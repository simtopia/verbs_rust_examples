use crate::aave::agents::{BorrowAgent, LiquidationAgent, UniswapNoiseAgent, UniswapPriceAgent};
use crate::aave::protocol::{aave_abi, periphery_abi};
use crate::aave::types;
use alloy_primitives::{Address, U256};
use verbs_rs::agent::{AgentVec, SingletonAgent};
use verbs_rs::env::Env;
use verbs_rs::DB;

pub fn admin_mint_and_supply<D>(
    mut env: Env<D>,
    admin_address: Address,
    faucet: Address,
    pool: Address,
    token: Address,
    amount: u128,
) -> Env<D>
where
    D: DB,
{
    let amount = U256::from(amount);

    env.direct_execute(
        faucet,
        faucet,
        periphery_abi::Faucet::mintCall {
            token,
            to: admin_address,
            amount,
        },
        U256::ZERO,
    )
    .unwrap();

    env.direct_execute(
        admin_address,
        token,
        periphery_abi::MintableERC20::approveCall {
            spender: pool,
            amount: U256::MAX,
        },
        U256::ZERO,
    )
    .unwrap();

    env.direct_execute(
        admin_address,
        pool,
        aave_abi::Pool_Implementation::supplyCall {
            asset: token,
            amount,
            onBehalfOf: admin_address,
            referralCode: 0u16,
        },
        U256::ZERO,
    )
    .unwrap();

    env
}

pub fn approve_and_mint<D>(
    mut env: Env<D>,
    addresses: Vec<Address>,
    faucet: Address,
    token: Address,
    pool: Address,
    amount: u128,
) -> Env<D>
where
    D: DB,
{
    let amount = U256::from(amount);

    for address in addresses {
        env.direct_execute(
            faucet,
            faucet,
            periphery_abi::Faucet::mintCall {
                token,
                to: address,
                amount,
            },
            U256::ZERO,
        )
        .unwrap();
        env.direct_execute(
            address,
            token,
            periphery_abi::MintableERC20::approveCall {
                spender: pool,
                amount: U256::MAX,
            },
            U256::ZERO,
        )
        .unwrap();
    }

    env
}

pub fn approve_and_mint_dai<D>(
    mut env: Env<D>,
    addresses: Vec<Address>,
    dai: Address,
    dai_admin: Address,
    pool: Address,
    amount: u128,
) -> Env<D>
where
    D: DB,
{
    let amount = U256::from(amount);

    for address in addresses {
        env.direct_execute(
            dai_admin,
            dai,
            periphery_abi::DAI::mintCall {
                usr: address,
                wad: amount,
            },
            U256::ZERO,
        )
        .unwrap();
        env.direct_execute(
            address,
            dai,
            periphery_abi::DAI::approveCall {
                usr: pool,
                wad: U256::MAX,
            },
            U256::ZERO,
        )
        .unwrap();
    }

    env
}

pub fn _approve_and_mint_bal<D>(
    mut env: Env<D>,
    admin_address: Address,
    addresses: Vec<Address>,
    bal: Address,
    pool: Address,
    amount: u128,
) -> Env<D>
where
    D: DB,
{
    let amount = U256::from(amount);

    let bal_admin = env
        .direct_call(
            admin_address,
            bal,
            super::protocol::periphery_abi::BAL::MINTER_ROLECall {},
            U256::ZERO,
        )
        .unwrap()
        .0
        ._0;
    let bal_admin = env
        .direct_call(
            admin_address,
            bal,
            super::protocol::periphery_abi::BAL::getRoleMemberCall {
                role: bal_admin,
                index: U256::ZERO,
            },
            U256::ZERO,
        )
        .unwrap()
        .0
        ._0;

    for address in addresses {
        env.direct_execute(
            bal_admin,
            bal,
            periphery_abi::BAL::mintCall {
                to: address,
                amount,
            },
            U256::ZERO,
        )
        .unwrap();
        env.direct_execute(
            address,
            bal,
            periphery_abi::BAL::approveCall {
                spender: pool,
                amount: U256::MAX,
            },
            U256::ZERO,
        )
        .unwrap();
    }

    env
}

pub fn approve_and_mint_weth<D>(
    mut env: Env<D>,
    addresses: Vec<Address>,
    weth: Address,
    pool: Address,
    amount: u128,
) -> Env<D>
where
    D: DB,
{
    let amount = U256::from(amount);

    for address in addresses {
        env.direct_execute(
            address,
            weth,
            periphery_abi::WETH::depositCall {},
            U256::from(amount),
        )
        .unwrap();
        env.direct_execute(
            address,
            weth,
            periphery_abi::WETH::approveCall {
                guy: pool,
                wad: U256::MAX,
            },
            U256::ZERO,
        )
        .unwrap();
    }

    env
}

pub fn initialise_borrow_agents(
    n_agents: usize,
    activation_rate: f64,
    borrow_token_ltv: U256,
    borrow_token_decimals: U256,
    supply_token: Address,
    borrow_token: Address,
    pool: Address,
    oracle: Address,
) -> AgentVec<U256, BorrowAgent> {
    let agents = (1000..1000 + n_agents)
        .map(|i| {
            BorrowAgent::new(
                i,
                activation_rate,
                borrow_token_ltv,
                borrow_token_decimals,
                pool,
                oracle,
                supply_token,
                borrow_token,
            )
        })
        .collect();
    AgentVec::from(agents)
}

pub fn initialise_liquidation_agents(
    n_agents: usize,
    collateral_token: Address,
    debt_token: Address,
    pool: Address,
    oracle: Address,
    liquidation_addresses: Vec<Address>,
    adversarial: bool,
    uniswap_pool: Address,
    quoter: Address,
    swap_router: Address,
    uniswap_fee: u32,
) -> AgentVec<types::UserData, LiquidationAgent> {
    let agents = (2000..2000 + n_agents)
        .map(|i| {
            LiquidationAgent::new(
                i,
                pool,
                oracle,
                collateral_token,
                debt_token,
                liquidation_addresses.clone(),
                adversarial,
                uniswap_pool,
                quoter,
                swap_router,
                uniswap_fee,
            )
        })
        .collect();
    AgentVec::from(agents)
}

pub fn initialise_uniswap_price_agent<D>(
    env: &mut Env<D>,
    pool: Address,
    fee: u32,
    swap_router: Address,
    token_b: Address,
    token_a_price: i128,
    token_b_price: i128,
    mu: f64,
    dt: f64,
    sigma: f64,
) -> SingletonAgent<(i128, i128), UniswapPriceAgent>
where
    D: DB,
{
    SingletonAgent::from(UniswapPriceAgent::new(
        env,
        3000,
        pool,
        fee,
        swap_router,
        token_b,
        token_a_price,
        token_b_price,
        mu,
        dt,
        sigma,
    ))
}

pub fn initialise_uniswap_noise_agents<D>(
    env: &mut Env<D>,
    n_agents: usize,
    fee: u32,
    swap_router: Address,
    token_a: Address,
    token_b: Address,
) -> AgentVec<U256, UniswapNoiseAgent>
where
    D: DB,
{
    let agents = (4000..4000 + n_agents)
        .map(|i| UniswapNoiseAgent::new(env, i, fee, swap_router, token_a, token_b))
        .collect();
    AgentVec::from(agents)
}
