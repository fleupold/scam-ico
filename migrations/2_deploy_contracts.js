const Scam = artifacts.require("Scam");

module.exports = function(deployer) {
  deployer.deploy(Scam);
};
