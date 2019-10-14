pragma solidity ^0.5.0;

import "canonical-weth/contracts/WETH9.sol";

// Just canonical WETH with the ability to create WETH from thin air like magic.
// This just makes testing of the Scam ICO easier since it takes a wile for a
// faucet to give you 100 Eth in order to completely fund the ICO.
contract FakeWeth is WETH9 {
    function magicallyCreate(address receiver, uint256 amount) public returns (uint256) {
        balanceOf[receiver] += amount;
        emit Deposit(receiver, amount);
    }
}