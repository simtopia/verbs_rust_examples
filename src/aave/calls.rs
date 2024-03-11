use crate::aave::protocol::{aave_abi, periphery_abi, uniswap_abi};
use alloy_primitives::{Address, I256, U256};
use verbs_rs::{contract::Transaction, env::Env, DB};

pub fn supply_call(
    user_address: Address,
    pool_address: Address,
    token_address: Address,
    amount: U256,
) -> Transaction {
    Transaction::new(
        user_address,
        pool_address,
        aave_abi::Pool_Implementation::supplyCall {
            asset: token_address,
            amount,
            onBehalfOf: user_address,
            referralCode: 0,
        },
        U256::ZERO,
        true,
    )
}

pub fn borrow_call(
    user_address: Address,
    pool_address: Address,
    token_address: Address,
    amount: U256,
) -> Transaction {
    Transaction::new(
        user_address,
        pool_address,
        aave_abi::Pool_Implementation::borrowCall {
            asset: token_address,
            amount,
            interestRateMode: U256::from(2u128),
            referralCode: 0,
            onBehalfOf: user_address,
        },
        U256::ZERO,
        false,
    )
}

pub fn liquidation_call(
    collateral_token_address: Address,
    debt_token_address: Address,
    user_address: Address,
    pool_address: Address,
    liquidator_address: Address,
    amount: U256,
) -> Transaction {
    Transaction::new(
        liquidator_address,
        pool_address,
        aave_abi::Pool_Implementation::liquidationCallCall {
            collateralAsset: collateral_token_address,
            debtAsset: debt_token_address,
            user: user_address,
            debtToCover: amount,
            receiveAToken: false,
        },
        U256::ZERO,
        false,
    )
}

pub fn get_reserve_configuration_data<D>(
    network: &mut Env<D>,
    admin_address: Address,
    data_provider_address: Address,
    token_address: Address,
) -> aave_abi::PoolDataProvider::getReserveConfigurationDataReturn
where
    D: DB,
{
    network
        .direct_call(
            admin_address,
            data_provider_address,
            aave_abi::PoolDataProvider::getReserveConfigurationDataCall {
                asset: token_address,
            },
            U256::ZERO,
        )
        .unwrap()
        .0
}

pub fn get_asset_price<D>(
    network: &mut Env<D>,
    admin_address: Address,
    oracle_address: Address,
    token_address: Address,
) -> U256
where
    D: DB,
{
    network
        .direct_call(
            admin_address,
            oracle_address,
            aave_abi::AaveOracle::getAssetPriceCall {
                asset: token_address,
            },
            U256::ZERO,
        )
        .unwrap()
        .0
        ._0
}

// /// Get user account data
// ///
// /// * totalCollateralBase
// /// * totalDebtBase
// /// * availableBorrowsBase
// /// * currentLiquidationThreshold
// /// * ltv
// /// * healthFactor
// ///
pub fn get_user_data<D>(
    network: &mut Env<D>,
    admin_address: Address,
    pool_contract: Address,
    account_address: Address,
) -> aave_abi::Pool_Implementation::getUserAccountDataReturn
where
    D: DB,
{
    network
        .direct_call(
            admin_address,
            pool_contract,
            aave_abi::Pool_Implementation::getUserAccountDataCall {
                user: account_address,
            },
            U256::ZERO,
        )
        .unwrap()
        .0
}

// /// Manually set the price of a token via it's price oracle
pub fn set_token_price<D>(
    network: &mut Env<D>,
    admin_address: Address,
    token_oracle_address: Address,
    price: I256,
) where
    D: DB,
{
    network
        .direct_execute(
            admin_address,
            token_oracle_address,
            aave_abi::MockAggregator::setValueCall { value: price },
            U256::ZERO,
        )
        .unwrap();
}

pub fn _uniswap_liquidity<D>(
    network: &mut Env<D>,
    admin_address: Address,
    uniswap_pool_address: Address,
) -> u128
where
    D: DB,
{
    network
        .direct_call(
            admin_address,
            uniswap_pool_address,
            uniswap_abi::UniswapV3Pool::liquidityCall {},
            U256::ZERO,
        )
        .unwrap()
        .0
        ._0
}

pub fn uniswap_swap_call(
    recipient: Address,
    swap_router: Address,
    params: uniswap_abi::SwapRouter::ExactInputSingleParams,
) -> Transaction {
    Transaction::new(
        recipient,
        swap_router,
        uniswap_abi::SwapRouter::exactInputSingleCall { params },
        U256::ZERO,
        true,
    )
}

pub fn uniswap_swap_call_exact_output(
    recipient: Address,
    swap_router: Address,
    params: uniswap_abi::SwapRouter::ExactOutputSingleParams,
) -> Transaction {
    Transaction::new(
        recipient,
        swap_router,
        uniswap_abi::SwapRouter::exactOutputSingleCall { params },
        U256::ZERO,
        true,
    )
}

pub fn balance_of<D>(network: &mut Env<D>, caller: Address, token: Address) -> U256
where
    D: DB,
{
    network
        .direct_call(
            caller,
            token,
            periphery_abi::MintableERC20::balanceOfCall { account: caller },
            U256::ZERO,
        )
        .unwrap()
        .0
        ._0
}

pub fn get_slot0<D>(
    network: &mut Env<D>,
    caller: Address,
    pool: Address,
) -> uniswap_abi::UniswapV3Pool::slot0Return
where
    D: DB,
{
    network
        .direct_call(
            caller,
            pool,
            uniswap_abi::UniswapV3Pool::slot0Call {},
            U256::ZERO,
        )
        .unwrap()
        .0
}

pub fn get_liquidity<D>(network: &mut Env<D>, caller: Address, pool: Address) -> u128
where
    D: DB,
{
    network
        .direct_call(
            caller,
            pool,
            uniswap_abi::UniswapV3Pool::liquidityCall {},
            U256::ZERO,
        )
        .unwrap()
        .0
        ._0
}

pub fn quote_v2_exact_output_swap<D>(
    network: &mut Env<D>,
    caller: Address,
    token_in: Address,
    token_out: Address,
    fee: u32,
    amount_out: U256,
    quoter: Address,
) -> Option<uniswap_abi::Quoter_v2::quoteExactOutputSingleReturn>
where
    D: DB,
{
    let quote = network.direct_call(
        caller,
        quoter,
        uniswap_abi::Quoter_v2::quoteExactOutputSingleCall {
            params: uniswap_abi::Quoter_v2::QuoteExactOutputSingleParams {
                amount: amount_out,
                fee,
                sqrtPriceLimitX96: U256::ZERO,
                tokenIn: token_in,
                tokenOut: token_out,
            },
        },
        U256::ZERO,
    );
    match quote {
        Ok(result) => Some(result.0),
        Err(_) => None,
    }
}

pub fn get_decimals<D>(network: &mut Env<D>, caller: Address, token: Address) -> U256
where
    D: DB,
{
    let decimals: U256 = network
        .direct_call(
            caller,
            token,
            periphery_abi::MintableERC20::decimalsCall {},
            U256::ZERO,
        )
        .unwrap()
        .0
        ._0
        .try_into()
        .unwrap();
    decimals
}

pub fn get_token0<D>(network: &mut Env<D>, caller: Address, pool: Address) -> Address
where
    D: DB,
{
    network
        .direct_call(
            caller,
            pool,
            uniswap_abi::UniswapV3Pool::token0Call {},
            U256::ZERO,
        )
        .unwrap()
        .0
        ._0
}

pub fn get_token1<D>(network: &mut Env<D>, caller: Address, pool: Address) -> Address
where
    D: DB,
{
    network
        .direct_call(
            caller,
            pool,
            uniswap_abi::UniswapV3Pool::token1Call {},
            U256::ZERO,
        )
        .unwrap()
        .0
        ._0
}
