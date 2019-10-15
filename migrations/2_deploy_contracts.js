const ScamIco = artifacts.require("ScamIco");
const Weth9 = artifacts.require("WETH9");
const MagicWeth = artifacts.require("MagicWeth");
const wethArtifact = require("canonical-weth");

async function deployWeth(deployer) {
  switch (deployer.network) {
  case "development":
    // if we are on the development network, we also want to deploy the WETH9
    // contract to use for our Scam ICO.
    await deployer.deploy(Weth9);
    return Weth9;
  case "test":
    // if we are on a test network, then we just use the MagicWeth contract
    // so we don't have to collect 100 eth from a faucet.
    await deployer.deploy(MagicWeth);
    return MagicWeth;
  default:
    // anywhere else we do our best to figure out the address of an existing
    // WETH9 contract.
    await Weth9.deployed();
    return Weth9;
  }
}

module.exports = async function(deployer) {
  const Weth = await deployWeth(deployer);
  await deployer.deploy(ScamIco, Weth.address);
};
