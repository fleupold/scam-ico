mod contract;
mod truffle;

use crate::contract::Contract;
use crate::truffle::Artifact;
use std::path::PathBuf;
use structopt::StructOpt;
use web3::contract::Contract as Web3Contract;
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

    /// The Scam ICO contract address. If it is not specified the address in the truffle artifact will be used.
    #[structopt(short, long)]
    contract: Option<Address>,
}

fn main() {
    let opt = Opt::from_args();

    let (eloop, http) = Http::new(&opt.transport).expect("error setting up transport");
    eloop.into_remote();

    let web3 = Web3::new(http);
    let accounts = web3
        .eth()
        .accounts()
        .wait()
        .expect("failed to get account list");

    let ico_artifact = Artifact::load(&opt.truffle_project, "ScamIco")
        .expect("failed to load ICO truffle artifact");
    let ico = Contract::new(web3.clone(), ico_artifact)
        .wait()
        .expect("failed to get ICO contract");

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

    println!(
        "{:?} {:?} {:?}",
        ico.address(),
        weth.address(),
        scm.address()
    );
}
