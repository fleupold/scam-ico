use bip39::{Mnemonic, Seed};
use ethsign::SecretKey;
use std::error::Error;
use web3::futures::Future;
use web3::types::Address;
use web3::{Transport, Web3};

pub struct Wallet {
    accounts: Vec<Account>,
}

struct Account {
    public: Address,
    private: Option<SecretKey>,
}

impl Wallet {
    pub fn with_mnemonic(mnemonic: Mnemonic, count: usize) -> Wallet {
        let accounts = (0..count)
            .map(|i| {
                let seed = Seed::new(&mnemonic, &format!("m/44'/60'/0'/0/{}", i));
                let private: SecretKey = unimplemented!();
                Account {
                    public: private.public().address().into(),
                    private: Some(private),
                }
            })
            .collect();
        Wallet { accounts }
    }

    pub fn local<T>(web3: Web3<T>) -> impl Future<Item = Wallet, Error = impl Error>
    where
        T: Transport,
    {
        web3.eth().accounts().and_then(|accounts| {
            let accounts = accounts
                .iter()
                .map(|account| Account {
                    public: *account,
                    private: None,
                })
                .collect();
            Ok(Wallet { accounts })
        })
    }

    pub fn accounts<'a>(&'a self) -> impl Iterator<Item = Address> + 'a {
        self.accounts.iter().map(|account| account.public)
    }
}
