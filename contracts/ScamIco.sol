pragma solidity ^0.5.0;

import "./Scam.sol";
import "canonical-weth/contracts/WETH9.sol";

contract ScamIco {
  enum State { FUNDING, CLOSED, FINISHED }

  WETH9 public weth;
  Scam public scm;

  uint256 constant target = 100 ether;
  uint256 public remaining;
  uint256 public close;
  mapping(address => uint256) public contributions;

  constructor(WETH9 weth_) public {
    weth = weth_;
    scm = new Scam();

    remaining = target;
    close = uint256(-1);
  }

  function state() public view returns (State) {
    if (close == uint256(-1)) {
      return State.FUNDING;
    } else {
      require(remaining == 0, "Internal state error"); // just a sanity check
      if (now < close + 2 hours) {
        return State.CLOSED;
      } else {
        return State.FINISHED;
      }
    }
  }

  function fund(uint256 amount) public returns (uint256) {
    require(remaining > 0, "ICO is closed");
    // a little unclear on what to do if more than remaining WETH being raised
    // gets transferred into the contract so lets revert for now
    require(amount <= remaining, "amount being funded is too large");
    require(amount <= weth.allowance(msg.sender, address(this)), "not enough funds available for transfer");

    weth.transferFrom(msg.sender, address(this), amount);
    contributions[msg.sender] += amount;
    remaining -= amount; // safe from overflow since we require amount <= remaining

    if (remaining == 0) {
      close = now;
    }

    return amount;
  }

  function claim() public returns (uint256) {
    require(state() == State.FINISHED, "Tokens not ready to be claimed yet");

    uint256 amount = contributions[msg.sender];
    require(amount > 0, "Nothing to claim");

    contributions[msg.sender] = 0;
    scm.mint(msg.sender, amount);

    return amount;
  }
}
