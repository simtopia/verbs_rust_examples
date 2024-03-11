// SPDX-License-Identifier: BUSL-1.1
pragma solidity 0.8.10;

contract MockAggregator {
  int256 private _latestAnswer;

  event PriceUpdate(int256 indexed old_price, int256 indexed new_price);

  constructor(int256 initialAnswer) {
    _latestAnswer = initialAnswer;
    emit PriceUpdate(initialAnswer, initialAnswer);
  }

  function latestAnswer() external view returns (int256) {
    return _latestAnswer;
  }

  function getTokenType() external pure returns (uint256) {
    return 1;
  }

  function decimals() external pure returns (uint8) {
    return 8;
  }

  function setValue(int256 value) public returns (bool) {
    emit PriceUpdate(_latestAnswer, value);
    _latestAnswer = value;
    return true;
  }
}
