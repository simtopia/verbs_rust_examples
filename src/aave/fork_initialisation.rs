use super::agents::AgentStates;
use super::calls;
use super::deployment;
use super::protocol::{aave_abi, aave_bytecode, uniswap_abi, uniswap_bytecode};
use super::types::ForkedSimParameters;
use alloy_primitives::{Address, U256};
use alloy_sol_types::SolValue;
use verbs_rs::agent::AgentSet;
use verbs_rs::env::Env;
use verbs_rs::env::Validator;
use verbs_rs::utils::{address_from_hex, constructor_data};
use verbs_rs::ForkDb;

const WETH: &str = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
const DAI: &str = "0x6B175474E89094C44Da98b954EedeAC495271d0F";
const _CRV: &str = "0xD533a949740bb3306d119CC777fa900bA034cd52";
const _BAL: &str = "0xba100000625a3754423978a60c9317c58a424e3D";

const DAI_ADMIN: &str = "0x9759A6Ac90977b93B58547b4A71c78317f391A28";
const _CRV_MINTER: &str = "0xd061D61a4d941c39E5453435B6345Dc261C2fcE0";

const UNISWAP_FACTORY: &str = "0x1F98431c8aD98523631AE4a59f267346ea31F984";
const UNISWAP_SWAP_ROUTER: &str = "0xE592427A0AEce92De3Edee1F18E0157C05861564";
const UNISWAP_NFT_POSITION_MANAGER: &str = "0xC36442b4a4522E871399CD717aBDD847Ab11FE88";
const UNISWAP_QUOTER: &str = "0x61fFE014bA17989E743c5F6cB21bF9697530B21e";

const AAVE_DATA_PROVIDER: &str = "0x7B4EB56E7CD4b454BA8ff71E4518426369a138a3";
const AAVE_POOL: &str = "0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2";
const AAVE_ORACLE: &str = "0x54586bE62E3c3580375aE3723C145253060Ca0C2";
const AAVE_ADDRESS_PROVIDER: &str = "0x2f39d218133AFaB8F2B819B1066c7E434Ad94E9e";
const AAVE_ACL_MANAGER: &str = "0xc2aaCf6553D20d1e9d78E365AAba8032af9c85b0";

pub struct PeripheryAddresses {
    pub token_a: Address,
    pub token_b: Address,
}

pub struct UniswapAddresses {
    pub factory: Address,
    pub swap_router: Address,
    pub nft_position_manager: Address,
    pub quoter_address: Address,
}

pub struct AaveAddresses {
    pub data_provider: Address,
    pub pool: Address,
    pub oracle: Address,
    pub address_provider: Address,
    pub acl_manager: Address,
}

pub fn initialise_sim<V: Validator>(
    params: ForkedSimParameters,
    alchemy_key: String,
    validator: V,
) -> (
    Env<ForkDb, V>,
    AgentStates,
    PeripheryAddresses,
    UniswapAddresses,
    AaveAddresses,
) {
    let start_balance = 10u128.pow(35);
    let admin_address = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266";

    let url_str = format!("https://eth-mainnet.g.alchemy.com/v2/{}", alchemy_key);
    let url_str = url_str.as_str();

    let mut env = Env::<ForkDb, V>::init(url_str, Some(params.block_number), validator);

    let admin_address = address_from_hex(admin_address);

    env.insert_account(admin_address, U256::from(start_balance));

    let periphery_addresses: PeripheryAddresses = PeripheryAddresses {
        token_a: address_from_hex(WETH), // risky asset that we use as collateral
        token_b: address_from_hex(DAI),  // stablecoin that we use as debt asset
    };

    let uniswap_addresses = UniswapAddresses {
        factory: address_from_hex(UNISWAP_FACTORY),
        swap_router: address_from_hex(UNISWAP_SWAP_ROUTER),
        nft_position_manager: address_from_hex(UNISWAP_NFT_POSITION_MANAGER),
        quoter_address: address_from_hex(UNISWAP_QUOTER),
    };

    let aave_addresses = AaveAddresses {
        data_provider: address_from_hex(AAVE_DATA_PROVIDER),
        pool: address_from_hex(AAVE_POOL),
        oracle: address_from_hex(AAVE_ORACLE),
        address_provider: address_from_hex(AAVE_ADDRESS_PROVIDER),
        acl_manager: address_from_hex(AAVE_ACL_MANAGER),
    };

    let token_a_config = calls::get_reserve_configuration_data(
        &mut env,
        admin_address,
        aave_addresses.data_provider,
        periphery_addresses.token_a,
    );
    let _token_a_decimals = token_a_config.decimals;
    let _token_a_ltv = token_a_config.ltv;

    let token_b_config = calls::get_reserve_configuration_data(
        &mut env,
        admin_address,
        aave_addresses.data_provider,
        periphery_addresses.token_b,
    );
    let token_b_decimals = token_b_config.decimals;
    let token_b_ltv = token_b_config.ltv;

    let uniswap_pool_address = env
        .direct_call(
            admin_address,
            uniswap_addresses.factory,
            uniswap_abi::UniswapV3Factory::getPoolCall {
                _0: periphery_addresses.token_a,
                _1: periphery_addresses.token_b,
                _2: params.uniswap_fee,
            },
            U256::ZERO,
        )
        .unwrap()
        .0
        ._0;

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

    let liquidation_agents = deployment::initialise_liquidation_agents(
        params.n_liquidators,
        periphery_addresses.token_a,
        periphery_addresses.token_b,
        aave_addresses.pool,
        aave_addresses.oracle,
        borrow_agents.get_addresses(),
        params.adversarial,
        uniswap_pool_address,
        uniswap_addresses.quoter_address,
        uniswap_addresses.swap_router,
        params.uniswap_fee,
    );

    // Get initial prices from the fork
    let initial_prices = env
        .direct_call(
            admin_address,
            aave_addresses.oracle,
            aave_abi::AaveOracle::getAssetsPricesCall {
                assets: vec![periphery_addresses.token_a, periphery_addresses.token_b],
            },
            U256::ZERO,
        )
        .unwrap()
        .0
        ._0;

    let token_a_price = initial_prices[0];
    let token_b_price = initial_prices[1];

    let uniswap_price_agent = deployment::initialise_uniswap_price_agent(
        &mut env,
        uniswap_pool_address,
        params.uniswap_fee,
        uniswap_addresses.swap_router,
        periphery_addresses.token_b,
        token_a_price.try_into().unwrap(),
        token_b_price.try_into().unwrap(),
        params.prices_mu,
        params.prices_dt,
        params.prices_sigma,
    );

    // Uniswap noise agent
    let uniswap_noise_agents = deployment::initialise_uniswap_noise_agents(
        &mut env,
        1,
        params.uniswap_fee,
        uniswap_addresses.swap_router,
        periphery_addresses.token_a,
        periphery_addresses.token_b,
    );

    env.insert_accounts(start_balance, borrow_agents.get_addresses());
    env.insert_accounts(start_balance, liquidation_agents.get_addresses());
    env.insert_accounts(start_balance, uniswap_price_agent.get_addresses());
    env.insert_accounts(start_balance, uniswap_noise_agents.get_addresses());

    // Initialise accounts used to mint dai and dal tokens
    let dai_admin = address_from_hex(DAI_ADMIN);
    env.insert_account(dai_admin, alloy_primitives::U256::from(start_balance));

    let initial_token_amount = 10u128.pow(25);

    let env = deployment::approve_and_mint_weth(
        env,
        borrow_agents.get_addresses(),
        periphery_addresses.token_a,
        aave_addresses.pool,
        initial_token_amount,
    );

    let env = deployment::approve_and_mint_weth(
        env,
        liquidation_agents.get_addresses(),
        periphery_addresses.token_a,
        uniswap_addresses.swap_router,
        initial_token_amount,
    );

    let env = deployment::approve_and_mint_dai(
        env,
        liquidation_agents.get_addresses(),
        periphery_addresses.token_b,
        dai_admin,
        aave_addresses.pool,
        initial_token_amount,
    );

    let env = deployment::approve_and_mint_weth(
        env,
        uniswap_price_agent.get_addresses(),
        periphery_addresses.token_a,
        uniswap_addresses.nft_position_manager,
        initial_token_amount,
    );

    let env = deployment::approve_and_mint_dai(
        env,
        uniswap_price_agent.get_addresses(),
        periphery_addresses.token_b,
        dai_admin,
        uniswap_addresses.nft_position_manager,
        initial_token_amount,
    );

    let env = deployment::approve_and_mint_weth(
        env,
        uniswap_price_agent.get_addresses(),
        periphery_addresses.token_a,
        uniswap_addresses.swap_router,
        initial_token_amount,
    );

    let env = deployment::approve_and_mint_dai(
        env,
        uniswap_price_agent.get_addresses(),
        periphery_addresses.token_b,
        dai_admin,
        uniswap_addresses.swap_router,
        initial_token_amount,
    );

    let env = deployment::approve_and_mint_weth(
        env,
        uniswap_noise_agents.get_addresses(),
        periphery_addresses.token_a,
        uniswap_addresses.swap_router,
        initial_token_amount,
    );

    let mut env = deployment::approve_and_mint_dai(
        env,
        uniswap_noise_agents.get_addresses(),
        periphery_addresses.token_b,
        dai_admin,
        uniswap_addresses.swap_router,
        initial_token_amount,
    );

    // Replace chainlink with our price aggregation
    let token_a_aggregator_address: Address = env.deploy_contract(
        admin_address,
        "Uniswap aggregator",
        constructor_data(
            uniswap_bytecode::UNISWAP_AGGREGATOR,
            Some(
                (
                    uniswap_pool_address,
                    periphery_addresses.token_a,
                    periphery_addresses.token_b,
                )
                    .abi_encode_params(),
            ),
        ),
    );

    let token_b_aggregator_address = env.deploy_contract(
        admin_address,
        "Token B Aggregator",
        constructor_data(
            aave_bytecode::MOCK_AGGREGATOR,
            Some(token_b_price.abi_encode()),
        ),
    );

    let aave_acl_admin = env
        .direct_call(
            admin_address,
            aave_addresses.address_provider,
            aave_abi::PoolAddressesProvider::getACLAdminCall {},
            U256::ZERO,
        )
        .unwrap()
        .0
        ._0;

    let pool_admin_role = env
        .direct_call(
            admin_address,
            aave_addresses.acl_manager,
            aave_abi::ACLManager::POOL_ADMIN_ROLECall {},
            U256::ZERO,
        )
        .unwrap()
        .0
        ._0;

    env.direct_execute(
        aave_acl_admin,
        aave_addresses.acl_manager,
        aave_abi::ACLManager::grantRoleCall {
            role: pool_admin_role,
            account: admin_address,
        },
        U256::ZERO,
    )
    .unwrap();

    env.direct_execute(
        admin_address,
        aave_addresses.oracle,
        aave_abi::AaveOracle::setAssetSourcesCall {
            assets: vec![periphery_addresses.token_a, periphery_addresses.token_b],
            sources: vec![token_a_aggregator_address, token_b_aggregator_address],
        },
        U256::ZERO,
    )
    .unwrap();

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
