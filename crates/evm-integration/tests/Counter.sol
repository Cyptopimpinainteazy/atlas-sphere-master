// Simple Counter contract for EVM integration testing
// SPDX-License-Identifier: MIT

pragma solidity ^0.8.0;

contract Counter {
    uint256 public count;
    
    event CountIncremented(uint256 indexed newCount);
    event CountDecremented(uint256 indexed newCount);
    
    constructor() {
        count = 0;
    }
    
    function increment() public {
        count += 1;
        emit CountIncremented(count);
    }
    
    function decrement() public {
        require(count > 0, "Cannot go below zero");
        count -= 1;
        emit CountDecremented(count);
    }
    
    function setCount(uint256 newCount) public {
        count = newCount;
    }
    
    function getCount() public view returns (uint256) {
        return count;
    }
}
