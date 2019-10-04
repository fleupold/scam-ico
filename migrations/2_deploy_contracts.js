const ScamIco = artifacts.require("ScamIco");
const Weth9 = artifacts.require("Weth9");
const wethArtifact = require("canonical-weth");
const truffleConfig = require("../truffle-config");

function networkId(network) {
  return truffleConfig.networks[network].network_id;
}

module.exports = async function(deployer, network) {
  let wethAddress;
  switch (network) {
  case "development":
    await deployer.deploy(Weth9);
    wethAddress = Weth9.address;
    break;
  default:
    wethAddress = wethArtifact.networks[networkId(network)];
    break;
  }
  
  await deployer.deploy(ScamIco, wethAddress);
};
