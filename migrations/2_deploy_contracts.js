const ScamIco = artifacts.require("ScamIco");
const Weth = artifacts.require("WETH9");
const wethArtifact = require("canonical-weth");

module.exports = async function(deployer) {
  // if we are on our development network then we also want to deploy the WETH9
  // contract to use for our Scam ICO; otherwise we try our best to figure out
  // the address of an existing WETH9 contract
  let wethAddress;
  if (deployer.network === "development") {
    await deployer.deploy(Weth);
    wethAddress = Weth.address;
  } else {
    wethAddress = wethArtifact.networks[deployer.network_id];
  }

  if (wethAddress === undefined) {
    throw new Error(`unable to locate WETH9 contract address for network ${network}`);
  }

  await deployer.deploy(ScamIco, wethAddress);
};
