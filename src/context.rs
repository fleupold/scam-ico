use crate::contract::Contract;
use crate::truffle::{Artifact, ArtifactError};
use std::path::{Path, PathBuf};
use thiserror::Error;
use web3::contract::Error as Web3ContractError;
use web3::error::Error as Web3Error;
use web3::futures::future::{self, Either};
use web3::futures::Future;
use web3::types::{Address, U256};
use web3::{Transport, Web3};

pub struct Context<T: Transport> {
    pub web3: Web3<T>,
    pub ico: Contract<T>,
    pub weth: Contract<T>,
    pub scm: Contract<T>,
}

impl<T: Transport> Context<T> {
    pub fn new<P>(
        web3: Web3<T>,
        truffle_project: P,
    ) -> impl Future<Item = Context<T>, Error = ContextError>
    where
        P: AsRef<Path>,
    {
        Context::with_ico_contract_factory(web3, truffle_project, |web3, artifact| {
            Contract::new(web3, artifact).map_err(Into::into)
        })
    }

    pub fn with_ico_address<P>(
        web3: Web3<T>,
        truffle_project: P,
        address: Address,
    ) -> impl Future<Item = Context<T>, Error = ContextError>
    where
        P: AsRef<Path>,
    {
        Context::with_ico_contract_factory(web3, truffle_project, move |web3, artifact| {
            future::ok(Contract::at(web3, address, artifact))
        })
    }

    fn with_ico_contract_factory<P, R, F>(
        web3: Web3<T>,
        truffle_project: P,
        factory: F,
    ) -> impl Future<Item = Context<T>, Error = ContextError>
    where
        P: AsRef<Path>,
        R: Future<Item = Contract<T>, Error = ContextError>,
        F: FnOnce(Web3<T>, Artifact) -> R,
    {
        use Either::*;
        macro_rules! try_result {
            ($result:expr) => {
                match $result {
                    Ok(a) => a,
                    Err(e) => return B(future::err(e.into())),
                };
            };
        }

        // create an owned copy of our truffle project path so we can move it
        // through our futures chain
        let tp = PathBuf::from(truffle_project.as_ref());

        let ico_artifact = try_result!(Artifact::load(&tp, "ScamIco"));
        A(factory(web3.clone(), ico_artifact)
            .and_then(move |ico| {
                let weth_artifact = try_result!(Artifact::load(&tp, "WETH9"));
                A(ico
                    .call("weth", ())
                    .map_err(Into::into)
                    .and_then(move |weth_address| {
                        let weth = Contract::at(web3.clone(), weth_address, weth_artifact);
                        Ok((tp, web3, ico, weth))
                    }))
            })
            .and_then(|(tp, web3, ico, weth)| {
                let scm_artifact = try_result!(Artifact::load(&tp, "Scam"));
                A(ico
                    .call("scm", ())
                    .map_err(Into::into)
                    .and_then(move |scm_address| {
                        let scm = Contract::at(web3.clone(), scm_address, scm_artifact);
                        Ok(Context {
                            web3,
                            ico,
                            weth,
                            scm,
                        })
                    }))
            }))
    }

    pub fn remaining(&self) -> f64 {
        match self.ico.call::<_, _, U256>("remaining", ()).wait() {
            Ok(r) => u256_to_f64_amount(r, 18),
            Err(_) => -1.0,
        }
    }

    pub fn balances(
        &self,
        account: Address,
    ) -> impl Future<Item = (f64, f64, f64), Error = ContextError> {
        Future::join3(
            self.web3
                .eth()
                .balance(account, None)
                .map(|balance| u256_to_f64_amount(balance, 18))
                .map_err(Into::into),
            erc20_balance(self.weth.clone(), account),
            erc20_balance(self.scm.clone(), account),
        )
    }

    pub fn purchase_weth(
        &self,
        account: Address,
        amount: f64,
    ) -> impl Future<Item = (), Error = ContextError> {
        let weth = self.weth.clone();
        weth.call::<_, _, U256>("decimals", ())
            .map(|decimals| decimals.as_u32() as i32)
            .map_err(ContextError::from)
            .and_then(move |decimals| {
                let amount = f64_amount_to_u256(amount, decimals);
                weth.function("deposit", ())
                    .value(Some(amount))
                    .from(account)
                    .send()
                    .map(|_| ())
                    .map_err(ContextError::from)
            })
    }

    pub fn magic_weth(
        &self,
        account: Address,
        amount: f64,
    ) -> impl Future<Item = (), Error = ContextError> {
        let weth = self.weth.clone();
        weth.call::<_, _, U256>("decimals", ())
            .map(|decimals| decimals.as_u32() as i32)
            .map_err(ContextError::from)
            .and_then(move |decimals| {
                let amount = f64_amount_to_u256(amount, decimals);
                weth.function("magicallyCreate", (account, amount))
                    .send()
                    .map(|_| ())
                    .map_err(ContextError::from)
            })
    }

    pub fn fund(
        &self,
        account: Address,
        amount: f64,
    ) -> impl Future<Item = (), Error = ContextError> {
        let ico = self.ico.clone();
        let weth = self.weth.clone();

        unimplemented!();
        ico.call::<_, _, U256>("fund", ())
            .map(|decimals| decimals.as_u32() as i32)
            .map_err(ContextError::from)
            .and_then(move |decimals| {
                let amount = f64_amount_to_u256(amount, decimals);
                weth.function("magicallyCreate", (account, amount))
                    .send()
                    .map(|_| ())
                    .map_err(ContextError::from)
            })
    }
}

fn erc20_balance<T>(
    token: Contract<T>,
    account: Address,
) -> impl Future<Item = f64, Error = ContextError>
where
    T: Transport,
{
    token
        .call::<_, _, U256>("decimals", ())
        .and_then(move |decimals| {
            token
                .call::<_, _, U256>("balanceOf", account)
                .map(move |balance| (decimals, balance))
        })
        .and_then(|(decimals, balance)| Ok(u256_to_f64_amount(balance, decimals.as_u32() as _)))
        .map_err(Into::into)
}

fn u256_to_f64_amount(a: U256, decimals: i32) -> f64 {
    let div = U256::from(10).pow(decimals.into());
    let (q, r) = a.div_mod(div);

    let whole = q.low_u32() as f64;
    let fraction = (r.low_u64() as f64) * (10.0f64).powi(-decimals);

    (whole + fraction)
}

fn f64_amount_to_u256(a: f64, decimals: i32) -> U256 {
    let (t, f) = (a.trunc() as u64, a.fract());
    let whole = U256::from(t) * U256::from(10).pow(decimals.into());
    let fraction = U256::from((f * (10.0f64).powi(decimals)) as u64);

    (whole + fraction)
}

#[derive(Debug, Error)]
pub enum ContextError {
    #[error("failed to load artifact: {0}")]
    Artifact(#[from] ArtifactError),

    #[error("web3 error: {0}")]
    Web3(#[from] Web3Error),

    #[error("web3 contract error: {0}")]
    Web3Contract(#[from] Web3ContractError),
}
