use ethabi::Contract;
use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::path::Path;
use web3::types::Address;

#[derive(Debug, Deserialize)]
pub struct Artifact {
    pub abi: Contract,
    pub networks: HashMap<String, Network>,
}

impl Artifact {
    pub fn load<P, S>(truffle_project: P, name: S) -> Result<Artifact, Box<dyn Error>>
    where
        P: AsRef<Path>,
        S: AsRef<str>,
    {
        let json = File::open(
            truffle_project
                .as_ref()
                .join("build")
                .join("contracts")
                .join(format!("{}.json", name.as_ref())),
        )?;
        let artifact = serde_json::from_reader(json)?;

        Ok(artifact)
    }
}

#[derive(Debug, Deserialize)]
pub struct Network {
    pub address: Address,
}
