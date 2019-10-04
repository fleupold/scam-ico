const ScamIco = artifacts.require("ScamIco");
const Weth9 = artifacts.require("Weth9");
const wethArtifact = require("canonical-weth");
const truffleConfig = require("../truffle-config");

module.exports = async function(deployer, network) {
  // if we are on our development network then we also want to deploy the WETH9
  // contract to use for our Scam ICO; otherwise we try our best to figure out
  // the address of an existing WETH9 contract
  let wethAddress;
  if (network === "development") {
    await deployer.deploy(Weth9);
    wethAddress = Weth9.address;
  } else {
    let networkId = (truffleConfig.networks[network] || {}).network_id;
    wethAddress = wethArtifact.networks[networkId];
  }

  if (wethAddress === undefined) {
    throw new Error(`unable to locate WETH9 contract address for network ${network}`);
  }
  
  await deployer.deploy(ScamIco, wethAddress);
};
