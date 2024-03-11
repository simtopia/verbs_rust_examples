use super::agents;
use super::calls;
use super::types::SimParameters;
use super::{deployment, protocol};

use self::agents::AgentStates;
use self::protocol::uniswap_abi;
use alloy_primitives::{I256, U256};
use verbs_rs::agent::AgentSet;
use verbs_rs::env::Env;
use verbs_rs::utils::address_from_hex;
use verbs_rs::utils::div_u256;
use verbs_rs::LocalDB;

use std::collections::HashMap;

/// Initial ticks where Uniswap is initialized and where liquidity is minted
fn get_init_ticks(params: &SimParameters) -> (i32, i32) {
    let _tick_spacing: HashMap<u32, f64> =
        HashMap::from([(100, 1.), (500, 10.), (3000, 60.), (10000, 200.)]);
    let price: f64 = div_u256(
        U256::from(params.token_a_initial_price),
        U256::from(params.token_b_initial_price),
        10,
    );
    let tick = price.log(1.0001).floor();
    let tick_spacing = _tick_spacing.get(&params.uniswap_fee).copied().unwrap();
    let tick_lower: f64 = tick - tick.rem_euclid(tick_spacing);
    let tick_upper: f64 = tick_lower + tick_spacing;

    // We make the tick range larger so that there is liquidity in a wide tick range and there can be tick crossing
    // TODO: monitor if we should make it larger.
    let width_tick_range = 2000.;
    let tick_lower = (tick_lower - width_tick_range * tick_spacing) as i32;
    let tick_upper = (tick_upper + width_tick_range * tick_spacing) as i32;

    (tick_lower, tick_upper)
}

/// Get Amount of each token is minted initially minted, according to the CPMM equations
/// L^2 = xy, P = y/x ==> x = L / sqrt(P), y = L * sqrt(P)
fn get_init_token_amounts(params: &SimParameters) -> (f64, f64) {
    let price: f64 = div_u256(
        U256::from(params.token_a_initial_price),
        U256::from(params.token_b_initial_price),
        10,
    );
    (
        params.liquidity / price.sqrt(),
        params.liquidity * price.sqrt(),
    )
}

/// get the initial sqrt price for Uniswap pool initialisation
fn get_sqrt_price_token_a_x96(params: &SimParameters) -> u128 {
    let n: i32 = 20;
    let price: f64 = f64::sqrt(div_u256(
        U256::from(params.token_a_initial_price),
        U256::from(params.token_b_initial_price),
        10,
    )) * 2f64.powi(n);
    let mut price: u128 = price as u128;
    // `<<` is a left shift operator, which is equivalent to multiplying times 2^n
    price <<= 96 - n;
    price
}

pub fn initialise_sim(
    params: SimParameters,
) -> (
    Env<LocalDB>,
    AgentStates,
    protocol::PeripheryAddresses,
    protocol::UniswapAddresses,
    protocol::AaveAddresses,
) {
    let start_balance = 10u128.pow(20);
    let admin_address = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266";
    let admin_address = address_from_hex(admin_address);

    let env = Env::<LocalDB>::init(U256::ZERO, U256::ZERO);

    let (mut env, periphery_addresses, uniswap_addresses, aave_addresses) =
        protocol::deploy_aave_contracts(
            env,
            admin_address,
            params.token_a_liquidation_threshold,
            params.token_b_liquidation_threshold,
            params.token_a_base_ltv,
            params.token_b_base_ltv,
            params.uniswap_fee,
        );

    let _token_a_config = calls::get_reserve_configuration_data(
        &mut env,
        admin_address,
        aave_addresses.data_provider,
        periphery_addresses.token_a,
    );
    let token_b_config = calls::get_reserve_configuration_data(
        &mut env,
        admin_address,
        aave_addresses.data_provider,
        periphery_addresses.token_b,
    );
    let token_b_decimals = token_b_config.decimals;
    let token_b_ltv = token_b_config.ltv;

    let borrow_agents = deployment::initialise_borrow_agents(
        params.n_borrowers,
        params.borrow_activation_rate,
        token_b_ltv,
        token_b_decimals,
        periphery_addresses.token_a,
        periphery_addresses.token_b,
        aave_addresses.pool,
        aave_addresses.oracle,
    );

    let borrower_addresses = borrow_agents.get_addresses();

    let liquidation_agents = deployment::initialise_liquidation_agents(
        params.n_liquidators,
        periphery_addresses.token_a,
        periphery_addresses.token_b,
        aave_addresses.pool,
        aave_addresses.oracle,
        borrower_addresses,
        params.adversarial,
        uniswap_addresses.pool,
        uniswap_addresses.quoter_address,
        uniswap_addresses.swap_router,
        params.uniswap_fee,
    );

    let uniswap_price_agent = deployment::initialise_uniswap_price_agent(
        &mut env,
        uniswap_addresses.pool,
        params.uniswap_fee,
        uniswap_addresses.swap_router,
        periphery_addresses.token_b,
        params.token_a_initial_price,
        params.token_b_initial_price,
        params.prices_mu,
        params.prices_dt,
        params.prices_sigma,
    );

    // Uniswap noise agent
    let uniswap_noise_agents = deployment::initialise_uniswap_noise_agents(
        &mut env,
        1_usize,
        params.uniswap_fee,
        uniswap_addresses.swap_router,
        periphery_addresses.token_a,
        periphery_addresses.token_b,
    );

    env.insert_accounts(start_balance, borrow_agents.get_addresses());
    env.insert_accounts(start_balance, liquidation_agents.get_addresses());
    env.insert_accounts(start_balance, uniswap_price_agent.get_addresses());
    env.insert_accounts(start_balance, uniswap_noise_agents.get_addresses());

    let env = deployment::approve_and_mint(
        env,
        borrow_agents.get_addresses(),
        periphery_addresses.faucet,
        periphery_addresses.token_a,
        aave_addresses.pool,
        10u128.pow(35),
    );

    // Approve Aave and Uniswap contracts to use liquidator_agents tokens
    let env = deployment::approve_and_mint(
        env,
        liquidation_agents.get_addresses(),
        periphery_addresses.faucet,
        periphery_addresses.token_b,
        aave_addresses.pool,
        10u128.pow(35),
    );

    let env = deployment::approve_and_mint(
        env,
        liquidation_agents.get_addresses(),
        periphery_addresses.faucet,
        periphery_addresses.token_a,
        uniswap_addresses.swap_router,
        10u128.pow(35),
    );

    // Approve Uniswap contracts to use uniswap_agent tokens
    let env = deployment::approve_and_mint(
        env,
        uniswap_price_agent.get_addresses(),
        periphery_addresses.faucet,
        periphery_addresses.token_a,
        uniswap_addresses.nft_position_manager,
        10u128.pow(35),
    );

    let env = deployment::approve_and_mint(
        env,
        uniswap_price_agent.get_addresses(),
        periphery_addresses.faucet,
        periphery_addresses.token_b,
        uniswap_addresses.nft_position_manager,
        10u128.pow(35),
    );

    let env = deployment::approve_and_mint(
        env,
        uniswap_price_agent.get_addresses(),
        periphery_addresses.faucet,
        periphery_addresses.token_a,
        uniswap_addresses.swap_router,
        10u128.pow(35),
    );

    let env = deployment::approve_and_mint(
        env,
        uniswap_price_agent.get_addresses(),
        periphery_addresses.faucet,
        periphery_addresses.token_b,
        uniswap_addresses.swap_router,
        10u128.pow(35),
    );

    // Approve Uniswap router contract to use uniswap_noise_agent tokens
    let env = deployment::approve_and_mint(
        env,
        uniswap_noise_agents.get_addresses(),
        periphery_addresses.faucet,
        periphery_addresses.token_a,
        uniswap_addresses.swap_router,
        10u128.pow(35),
    );

    let env = deployment::approve_and_mint(
        env,
        uniswap_noise_agents.get_addresses(),
        periphery_addresses.faucet,
        periphery_addresses.token_b,
        uniswap_addresses.swap_router,
        10u128.pow(35),
    );

    let mut env = deployment::admin_mint_and_supply(
        env,
        admin_address,
        periphery_addresses.faucet,
        aave_addresses.pool,
        periphery_addresses.token_b,
        10u128.pow(35),
    );

    // Uniswap pool initialisation
    let sqrt_price_x96 = get_sqrt_price_token_a_x96(&params);
    env.direct_execute(
        admin_address,
        uniswap_addresses.pool,
        uniswap_abi::UniswapV3Pool::initializeCall {
            sqrtPriceX96: U256::from(sqrt_price_x96),
        },
        U256::ZERO,
    )
    .unwrap();

    let (tick_lower, tick_upper) = get_init_ticks(&params);
    let caller = uniswap_price_agent.get_addresses()[0];
    let _slot0 = calls::get_slot0(&mut env, caller, uniswap_addresses.pool);
    let _price: u128 = _slot0.sqrtPriceX96.try_into().unwrap();
    let (init_token_a_amount, init_token_b_amount) = get_init_token_amounts(&params);
    let token_a_decimals = calls::get_decimals(&mut env, caller, periphery_addresses.token_a);
    let token_b_decimals = calls::get_decimals(&mut env, caller, periphery_addresses.token_b);
    let init_token_a_amount =
        U256::from(init_token_a_amount as i64) * U256::from(10).pow(token_a_decimals);
    let init_token_b_amount =
        U256::from(init_token_b_amount as i64) * U256::from(10).pow(token_b_decimals);

    let _output_mint = env
        .direct_execute(
            caller,
            uniswap_addresses.nft_position_manager,
            uniswap_abi::NonfungiblePositionManager::mintCall {
                params: uniswap_abi::NonfungiblePositionManager::MintParams {
                    amount0Desired: init_token_a_amount, //U256::from(10).pow(U256::from(19)),
                    amount0Min: U256::ZERO,
                    amount1Desired: init_token_b_amount, //U256::from(10).pow(U256::from(22)),
                    amount1Min: U256::ZERO,
                    deadline: U256::MAX,
                    fee: params.uniswap_fee,
                    recipient: caller,
                    tickLower: tick_lower, //tick_lower,
                    tickUpper: tick_upper, //tick_upper,
                    token0: periphery_addresses.token_a,
                    token1: periphery_addresses.token_b,
                },
            },
            U256::ZERO,
        )
        .unwrap();

    // sanity checks for Uniswap deployment and minting
    let token0 = env
        .direct_execute(
            caller,
            uniswap_addresses.pool,
            uniswap_abi::UniswapV3Pool::token0Call {},
            U256::ZERO,
        )
        .unwrap()
        .0;
    assert!(
        token0._0 == periphery_addresses.token_a,
        "We need Uniswap's token0 to be Aave's token A"
    );

    let liquidity = calls::get_liquidity(&mut env, caller, uniswap_addresses.pool);
    assert!(liquidity > 0, "Minting did not work");

    // This call is unnecessary
    calls::set_token_price(
        &mut env,
        admin_address,
        aave_addresses.token_a_oracle,
        I256::try_from(params.token_a_initial_price).unwrap(),
    );
    calls::set_token_price(
        &mut env,
        admin_address,
        aave_addresses.token_b_oracle,
        I256::try_from(params.token_b_initial_price).unwrap(),
    );

    (
        env,
        AgentStates {
            borrow_agents,
            liquidation_agents,
            uniswap_price_agent,
            uniswap_noise_agents,
        },
        periphery_addresses,
        uniswap_addresses,
        aave_addresses,
    )
}
