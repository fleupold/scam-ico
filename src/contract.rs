#![allow(dead_code)]

use crate::truffle::Artifact;
use ethabi::{Contract as AbiContract, Function, Result as AbiResult};
use ethsign::{SecretKey, Signature};
use rlp::RlpStream;
use web3::api::Eth;
use web3::contract::tokens::{Detokenize, Tokenize};
use web3::contract::{Contract as Web3Contract, Error as Web3ContractError, QueryResult};
use web3::error::Error as Web3Error;
use web3::futures::future::{self, Either};
use web3::futures::Future;
use web3::types::{
    Address, BlockNumber, Bytes, CallRequest, TransactionCondition, TransactionRequest, H256, U256,
};
use web3::{Transport, Web3};

#[derive(Clone)]
pub struct Contract<T: Transport> {
    web3: Web3<T>,
    contract: Web3Contract<T>,
    abi: AbiContract,
}

impl<T: Transport> Contract<T> {
    pub fn new(
        web3: Web3<T>,
        artifact: Artifact,
    ) -> impl Future<Item = Contract<T>, Error = Web3Error> {
        web3.net().version().and_then(move |network_id| {
            let address = artifact.networks[&network_id].address;
            Ok(Contract::at(web3, address, artifact))
        })
    }

    pub fn at(web3: Web3<T>, address: Address, artifact: Artifact) -> Contract<T> {
        let contract = Web3Contract::new(web3.eth(), address, artifact.abi.clone());
        let abi = artifact.abi.clone();

        Contract {
            web3,
            contract,
            abi,
        }
    }

    pub fn address(&self) -> Address {
        self.contract.address()
    }

    pub fn function<S, P>(&self, name: S, params: P) -> ContractTransactionBuilder<T>
    where
        S: AsRef<str>,
        P: Tokenize,
    {
        self.try_function(name, params).unwrap()
    }

    pub fn try_function<S, P>(&self, name: S, params: P) -> AbiResult<ContractTransactionBuilder<T>>
    where
        S: AsRef<str>,
        P: Tokenize,
    {
        let function = self.abi.function(name.as_ref())?;
        let data = function.encode_input(&params.into_tokens())?;

        Ok(ContractTransactionBuilder::new(
            self.web3.eth(),
            function.clone(),
            self.address(),
            data.into(),
        ))
    }

    pub fn call<S, P, R>(
        &self,
        name: S,
        params: P,
    ) -> impl Future<Item = R, Error = Web3ContractError>
    where
        S: AsRef<str>,
        P: Tokenize,
        R: Detokenize,
    {
        self.function(name, params).call()
    }

    pub fn send<S, P>(&self, name: S, params: P) -> impl Future<Item = H256, Error = Web3Error>
    where
        S: AsRef<str>,
        P: Tokenize,
    {
        self.function(name, params).send()
    }
}

pub struct ContractTransactionBuilder<T: Transport> {
    eth: Eth<T>,
    function: Function,
    tx: TransactionRequest,
    block: Option<BlockNumber>,
    secret: Option<SecretKey>,
    chain_id: Option<u64>,
}

impl<T: Transport> ContractTransactionBuilder<T> {
    fn new(
        eth: Eth<T>,
        function: Function,
        contract: Address,
        data: Bytes,
    ) -> ContractTransactionBuilder<T> {
        ContractTransactionBuilder {
            eth,
            function,
            tx: TransactionRequest {
                from: Address::zero(),
                to: Some(contract),
                gas: None,
                gas_price: None,
                value: None,
                data: Some(data),
                nonce: None,
                condition: None,
            },
            block: None,
            secret: None,
            chain_id: None,
        }
    }

    pub fn from(mut self, from: Address) -> ContractTransactionBuilder<T> {
        self.tx.from = from;
        self
    }

    pub fn gas(mut self, gas: Option<U256>) -> ContractTransactionBuilder<T> {
        self.tx.gas = gas;
        self
    }

    pub fn gas_price(mut self, gas_price: Option<U256>) -> ContractTransactionBuilder<T> {
        self.tx.gas_price = gas_price;
        self
    }

    pub fn value(mut self, value: Option<U256>) -> ContractTransactionBuilder<T> {
        self.tx.value = value;
        self
    }

    pub fn nonce(mut self, nonce: Option<U256>) -> ContractTransactionBuilder<T> {
        self.tx.nonce = nonce;
        self
    }

    pub fn condition(
        mut self,
        condition: Option<TransactionCondition>,
    ) -> ContractTransactionBuilder<T> {
        self.tx.condition = condition;
        self
    }

    pub fn block(mut self, block: Option<BlockNumber>) -> ContractTransactionBuilder<T> {
        self.block = block;
        self
    }

    pub fn sign(
        mut self,
        secret: Option<SecretKey>,
        chain_id: Option<u64>,
    ) -> ContractTransactionBuilder<T> {
        if let Some(secret) = &secret {
            self.tx.from = secret.public().address().into()
        }
        self.secret = secret;
        self.chain_id = chain_id;
        self
    }

    fn build_raw_transaction(self) -> impl Future<Item = Bytes, Error = Web3Error> {
        use Either::*;

        let nonce = match &self.tx.nonce {
            Some(nonce) => A(future::ok(*nonce)),
            None => B(self.eth.transaction_count(self.tx.from, None)),
        };

        let gas = match &self.tx.gas {
            Some(gas) => A(future::ok(*gas)),
            None => B(self.eth.estimate_gas(
                CallRequest {
                    from: Some(self.tx.from),
                    to: self.tx.to.expect("contract address not set in transaction"),
                    gas: self.tx.gas,
                    gas_price: self.tx.gas_price,
                    value: self.tx.value,
                    data: self.tx.data.clone(),
                },
                None,
            )),
        };

        let gas_price = match &self.tx.gas_price {
            Some(gas_price) => A(future::ok(*gas_price)),
            None => B(self.eth.gas_price()),
        };

        nonce
            .join3(gas, gas_price)
            .and_then(move |(nonce, gas, gas_price)| {
                let tx = RawTransaction {
                    nonce,
                    gas_price,
                    gas,
                    to: self.tx.to,
                    value: self.tx.value.unwrap_or_default(),
                    data: self.tx.data.unwrap_or_default(),
                };

                if let Some(secret) = self.secret {
                    Ok(tx.sign(secret, self.chain_id))
                } else {
                    Ok(tx.into_raw(self.chain_id))
                }
            })
    }

    pub fn call<R>(self) -> impl Future<Item = R, Error = Web3ContractError>
    where
        R: Detokenize,
    {
        // no need to sign here since we are not modifying state
        QueryResult::new(
            self.eth.call(
                CallRequest {
                    from: Some(self.tx.from),
                    to: self.tx.to.unwrap_or_default(),
                    gas: self.tx.gas,
                    gas_price: self.tx.gas_price,
                    value: self.tx.value,
                    data: self.tx.data,
                },
                self.block,
            ),
            self.function,
        )
    }

    pub fn send(self) -> impl Future<Item = H256, Error = Web3Error> {
        use Either::*;

        if self.secret.is_some() {
            let eth = self.eth.clone();
            A(self
                .build_raw_transaction()
                .map_err(|e| Web3Error::Transport(e.to_string()))
                .and_then(move |tx| eth.send_raw_transaction(tx)))
        } else {
            B(self.eth.send_transaction(self.tx))
        }
    }
}

struct RawTransaction {
    pub nonce: U256,
    pub to: Option<Address>,
    pub value: U256,
    pub gas_price: U256,
    pub gas: U256,
    pub data: Bytes,
}

impl RawTransaction {
    pub fn sign(&self, key: SecretKey, chain_id: Option<u64>) -> Bytes {
        let mut rlp = RlpStream::new();
        self.rlp_append_unsigned(&mut rlp, chain_id);
        let hash = tiny_keccak::keccak256(&rlp.as_raw());
        rlp.clear();

        let sig = key.sign(&hash[..]).unwrap(); // TODO(nlordell): propagate this error
        self.rlp_append_signed(&mut rlp, sig, chain_id);

        rlp.out().into()
    }

    pub fn into_raw(&self, chain_id: Option<u64>) -> Bytes {
        let mut rlp = RlpStream::new();
        self.rlp_append_unsigned(&mut rlp, chain_id);
        rlp.out().into()
    }

    fn rlp_append_unsigned(&self, s: &mut RlpStream, chain_id: Option<u64>) {
        s.begin_list(if chain_id.is_some() { 9 } else { 6 });
        s.append(&self.nonce);
        s.append(&self.gas_price);
        s.append(&self.gas);
        s.append(&self.to.unwrap_or_default());
        s.append(&self.value);
        s.append(&self.data.0);
        if let Some(n) = chain_id {
            s.append(&n);
            s.append(&0u8);
            s.append(&0u8);
        }
    }

    fn rlp_append_signed(&self, s: &mut RlpStream, sig: Signature, chain_id: Option<u64>) {
        let v = RawTransaction::add_chain_replay_protection(sig.v as _, chain_id);

        s.begin_list(9);
        s.append(&self.nonce);
        s.append(&self.gas_price);
        s.append(&self.gas);
        s.append(&self.to.unwrap_or_default());
        s.append(&self.value);
        s.append(&self.data.0);
        s.append(&v);
        s.append(&U256::from(sig.r));
        s.append(&U256::from(sig.s));
    }

    fn add_chain_replay_protection(v: u64, chain_id: Option<u64>) -> u64 {
        v + if let Some(n) = chain_id {
            35 + n * 2
        } else {
            27
        }
    }
}
