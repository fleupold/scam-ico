use crate::contract::Contract;
use crate::truffle::{Artifact, ArtifactError};
use std::path::{Path, PathBuf};
use thiserror::Error;
use web3::contract::Error as Web3ContractError;
use web3::error::Error as Web3Error;
use web3::futures::future::{self, Either};
use web3::futures::Future;
use web3::types::Address;
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
