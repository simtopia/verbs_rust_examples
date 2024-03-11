use alloy_sol_types::sol;

sol!(UniswapAggregator, "contracts/uniswap/UniswapAggregator.abi");
sol!(UniswapV3Factory, "contracts/uniswap/UniswapV3Factory.abi");
sol!(UniswapV3Pool, "contracts/uniswap/UniswapV3Pool.abi");
sol!(
    UniswapV3PoolDeployer,
    "contracts/uniswap/UniswapV3PoolDeployer.abi"
);
sol!(SwapRouter, "contracts/uniswap/SwapRouter.abi");
sol!(
    NonfungiblePositionManager,
    "contracts/uniswap/NonfungiblePositionManager.abi"
);
sol!(
    NonfungibleTokenPositionDescriptor,
    "contracts/uniswap/NonfungibleTokenPositionDescriptor.abi"
);
sol!(Quoter, "contracts/uniswap/Quoter.abi");
sol!(Quoter_v2, "contracts/uniswap/Quoter_v2.abi");
