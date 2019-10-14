mod contract;
mod truffle;
mod wallet;

use crate::contract::Contract;
use crate::truffle::Artifact;
use crate::wallet::Wallet;
use bip39::{Language, Mnemonic};
use std::error::Error;
use std::path::PathBuf;
use std::str::FromStr;
use structopt::StructOpt;
use web3::futures::Future;
use web3::transports::Http;
use web3::types::Address;
use web3::Web3;

#[derive(Debug, StructOpt)]
#[structopt(name = "scam-ico", about = "Scam ICO Client.")]
struct Opt {
    /// The web3 transport to use.
    #[structopt(short, long, default_value = "http://localhost:7545")]
    transport: String,

    /// Path to truffle project.
    #[structopt(short = "p", long, default_value = ".")]
    truffle_project: PathBuf,

    /// The Scam ICO contract address. If it is not specified the address in the
    /// truffle artifact will be used.
    #[structopt(short, long)]
    contract: Option<Address>,

    /// The BIP-0039 mnemonic to use for generating BIP-0043 accounts. If it is
    /// not specified then it will use web3 to get the list of accounts and for
    /// signing.
    #[structopt(short, long)]
    mnemonic: Option<MnemonicArg>,

    /// The number of accounts to generating when using a mnemonic. Has no effect
    /// otherwise
    #[structopt(long, default_value = "3")]
    accounts: usize,
}

#[derive(Debug)]
pub struct MnemonicArg(Mnemonic);

impl FromStr for MnemonicArg {
    type Err = Box<dyn Error + 'static>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mnemonic = Mnemonic::from_phrase(s, Language::English)?;
        Ok(MnemonicArg(mnemonic))
    }
}

impl MnemonicArg {
    fn as_inner(&self) -> Mnemonic {
        self.0.clone()
    }
}

fn main() {
    let opt = Opt::from_args();

    let (eloop, http) = Http::new(&opt.transport).expect("error setting up transport");
    eloop.into_remote();

    let web3 = Web3::new(http);

    let wallet = if let Some(mnemonic) = &opt.mnemonic {
        Wallet::with_mnemonic(mnemonic.as_inner(), opt.accounts)
    } else {
        Wallet::local(web3.clone())
            .wait()
            .expect("failed to get local accounts")
    };

    let ico_artifact = Artifact::load(&opt.truffle_project, "ScamIco")
        .expect("failed to load ICO truffle artifact");
    let ico = if let Some(ico_address) = opt.contract {
        Contract::at(web3.clone(), ico_address, ico_artifact)
    } else {
        Contract::new(web3.clone(), ico_artifact)
            .wait()
            .expect("failed to get ICO contract")
    };

    let weth_artifact = Artifact::load(&opt.truffle_project, "WETH9")
        .expect("failed to load WETH truffle artifact");
    let weth_address = ico
        .call("weth", ())
        .wait()
        .expect("failed to get WETH address from ICO");
    let weth = Contract::at(web3.clone(), weth_address, weth_artifact);

    let scm_artifact =
        Artifact::load(&opt.truffle_project, "Scam").expect("failed to load SCM truffle artifact");
    let scm_address = ico
        .call("scm", ())
        .wait()
        .expect("failed to get SCM address from ICO");
    let scm = Contract::at(web3.clone(), scm_address, scm_artifact);

    println!("Scam ICO @ {:?}", ico.address());
    println!("WETH     @ {:?}", weth.address());
    println!("SCM      @ {:?}", scm.address());
    println!("Wallets  @");
    for account in wallet.accounts() {
        println!(" - {:?}", account);
    }
}
