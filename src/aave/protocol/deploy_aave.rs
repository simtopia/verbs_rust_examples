use super::{aave_abi, aave_bytecode};
use alloy_primitives::{Address, Uint, U256, U64};
use alloy_sol_types::{SolCall, SolValue};
use verbs_rs::env::{Env, Validator};
use verbs_rs::utils::{constructor_data, data_bytes_from_hex};
use verbs_rs::LocalDB;

use super::deploy_periphery::{deploy_periphery_contracts, PeripheryAddresses};
use super::deploy_uniswap::{deploy_uniswap_contracts, UniswapAddresses};
use super::periphery_bytecode;

pub struct AaveAddresses {
    pub data_provider: Address,
    pub token_a_oracle: Address,
    pub token_b_oracle: Address,
    pub pool: Address,
    pub oracle: Address,
}

pub fn deploy_aave_contracts<V: Validator>(
    mut env: Env<LocalDB, V>,
    admin_address: Address,
    token_a_liquidation_threshold: u128,
    token_b_liquidation_threshold: u128,
    token_a_base_ltv: u128,
    token_b_base_ltv: u128,
    uniswap_fee: u32,
) -> (
    Env<LocalDB, V>,
    PeripheryAddresses,
    UniswapAddresses,
    AaveAddresses,
) {
    let address_provider_registry_address = env.deploy_contract(
        admin_address,
        "address_provider_registry",
        constructor_data(
            aave_bytecode::POOL_ADDRESSES_PROVIDER_REGISTRY,
            Some(admin_address.abi_encode()),
        ),
    );

    env.direct_execute(
        admin_address,
        address_provider_registry_address,
        aave_abi::PoolAddressesProviderRegistry::transferOwnershipCall {
            newOwner: admin_address,
        },
        U256::ZERO,
    )
    .unwrap();

    let _supply_logic_address = env.deploy_contract(
        admin_address,
        "Supply logic",
        constructor_data(aave_bytecode::SUPPLY_LOGIC, None),
    );

    let _borrow_logic_address = env.deploy_contract(
        admin_address,
        "Borrow logic",
        constructor_data(aave_bytecode::BORROW_LOGIC, None),
    );

    let _liquidation_logic_address = env.deploy_contract(
        admin_address,
        "Liquidation logic",
        constructor_data(aave_bytecode::LIQUIDATION_LOGIC, None),
    );

    let _emode_logic_address = env.deploy_contract(
        admin_address,
        "EMode logic",
        constructor_data(aave_bytecode::EMODE_LOGIC, None),
    );

    let _bridge_logic_addresse_logic = env.deploy_contract(
        admin_address,
        "Bridge logic",
        constructor_data(aave_bytecode::BRIDGE_LOGIC, None),
    );
    let _configurator_logic_address = env.deploy_contract(
        admin_address,
        "Configuration logic",
        constructor_data(aave_bytecode::CONFIGURATOR_LOGIC, None),
    );

    let _flash_loan_logic_address = env.deploy_contract(
        admin_address,
        "Flash loan logic",
        constructor_data(aave_bytecode::FLASHLOAN_LOGIC, None),
    );

    let _pool_logic_address = env.deploy_contract(
        admin_address,
        "Pool logic",
        constructor_data(aave_bytecode::POOL_LOGIC, None),
    );

    let treasury_controller_address = env.deploy_contract(
        admin_address,
        "Treasury controller",
        constructor_data(
            aave_bytecode::TREASURY_CONTROLLER,
            Some(admin_address.abi_encode()),
        ),
    );

    let treasury_proxy_address = env.deploy_contract(
        admin_address,
        "Treasury proxy",
        constructor_data(aave_bytecode::TREASURY_PROXY, None),
    );

    let treasury_address = env.deploy_contract(
        admin_address,
        "Treasury implementation",
        constructor_data(aave_bytecode::TREASURY_IMPLEMENTATION, None),
    );

    env.direct_execute(
        admin_address,
        treasury_proxy_address,
        aave_abi::TreasuryProxy::initialize_0Call {
            logic: treasury_address,
            admin: admin_address,
            data: aave_abi::TreasuryImplementation::initializeCall {
                fundsAdmin: treasury_controller_address,
            }
            .abi_encode()
            .into(),
        },
        U256::ZERO,
    )
    .unwrap();

    let (env, periphery_addresses) = deploy_periphery_contracts(env, admin_address);

    let (mut env, uniswap_addresses) =
        deploy_uniswap_contracts(env, admin_address, &periphery_addresses, uniswap_fee);

    let aave_address = env.deploy_contract(
        admin_address,
        "AAVE token",
        constructor_data(
            periphery_bytecode::MINTABLE_ERC20,
            Some(
                (
                    String::from("AAVE"),
                    String::from("AAVE"),
                    18u128,
                    periphery_addresses.faucet,
                )
                    .abi_encode_params(),
            ),
        ),
    );

    let staked_aave_proxy_address = env.deploy_contract(
        admin_address,
        "Stake AAVE proxy",
        constructor_data(aave_bytecode::STAKEAAVE_PROXY, None),
    );

    let staked_aave_v1_address = env.deploy_contract(
        admin_address,
        "StakeAave-REV-1-Implementation",
        constructor_data(
            aave_bytecode::STAKE_AAVE_REV_1_IMPLEMENTATION,
            Some(
                (
                    aave_address,
                    aave_address,
                    3600u128,
                    1800u128,
                    admin_address,
                    admin_address,
                    3600000u128,
                )
                    .abi_encode_params(),
            ),
        ),
    );

    let staked_aave_v2_address = env.deploy_contract(
        admin_address,
        "StakeAave-REV-2-Implementation",
        constructor_data(
            aave_bytecode::STAKE_AAVE_REV_2_IMPLEMENTATION,
            Some(
                (
                    aave_address,
                    aave_address,
                    3600u128,
                    1800u128,
                    admin_address,
                    admin_address,
                    3600000u128,
                    Address::ZERO,
                )
                    .abi_encode_params(),
            ),
        ),
    );

    let staked_aave_v3_address = env.deploy_contract(
        admin_address,
        "StakeAave-REV-3-Implementation",
        constructor_data(
            aave_bytecode::STAKE_AAVE_REV_3_IMPLEMENTATION,
            Some(
                (
                    aave_address,
                    aave_address,
                    3600u128,
                    1800u128,
                    admin_address,
                    admin_address,
                    3600000u128,
                    "Staked AAVE",
                    "stkAAVE",
                    18u32,
                    Address::ZERO,
                )
                    .abi_encode_params(),
            ),
        ),
    );

    env.direct_execute(
        admin_address,
        staked_aave_proxy_address,
        aave_abi::StakeAave_Proxy::initialize_0Call {
            logic: staked_aave_v1_address,
            admin: admin_address,
            data: aave_abi::StakeAave_REV_1_Implementation::initializeCall {
                aaveGovernance: Address::ZERO,
                name: "REV1".to_string(),
                symbol: "REV1".to_string(),
                decimals: 18u8,
            }
            .abi_encode()
            .into(),
        },
        U256::ZERO,
    )
    .unwrap();

    env.direct_execute(
        admin_address,
        staked_aave_proxy_address,
        aave_abi::StakeAave_Proxy::upgradeToAndCallCall {
            newImplementation: staked_aave_v2_address,
            data: data_bytes_from_hex("8129fc1c").into(),
        },
        U256::ZERO,
    )
    .unwrap();

    env.direct_execute(
        admin_address,
        staked_aave_proxy_address,
        aave_abi::StakeAave_Proxy::upgradeToAndCallCall {
            newImplementation: staked_aave_v3_address,
            data: data_bytes_from_hex("8129fc1c").into(),
        },
        U256::ZERO,
    )
    .unwrap();

    let address_provider_address = env.deploy_contract(
        admin_address,
        "Pool address provider",
        constructor_data(
            aave_bytecode::POOL_ADDRESSES_PROVIDER,
            Some(("TEST", admin_address).abi_encode_params()),
        ),
    );

    env.direct_execute(
        admin_address,
        address_provider_registry_address,
        aave_abi::PoolAddressesProviderRegistry::registerAddressesProviderCall {
            provider: address_provider_address,
            id: Uint::from(U64::from(8080u128)),
        },
        U256::ZERO,
    )
    .unwrap();

    let data_provider_address = env.deploy_contract(
        admin_address,
        "Pool data provider",
        constructor_data(
            aave_bytecode::POOL_DATA_PROVIDER,
            Some(address_provider_address.abi_encode()),
        ),
    );

    env.direct_execute(
        admin_address,
        address_provider_address,
        aave_abi::PoolAddressesProvider::setPoolDataProviderCall {
            newDataProvider: data_provider_address,
        },
        U256::ZERO,
    )
    .unwrap();

    let token_a_aggregator_address = env.deploy_contract(
        admin_address,
        "Token A Aggregator",
        constructor_data(
            aave_bytecode::MOCK_AGGREGATOR,
            Some(30000000000u128.abi_encode()),
        ),
    );

    let token_b_aggregator_address = env.deploy_contract(
        admin_address,
        "Token B Aggregator",
        constructor_data(
            aave_bytecode::MOCK_AGGREGATOR,
            Some(30000000000u128.abi_encode()),
        ),
    );

    let aave_aggregator_address = env.deploy_contract(
        admin_address,
        "AAVE Aggregator",
        constructor_data(
            aave_bytecode::MOCK_AGGREGATOR,
            Some(1970000000u128.abi_encode()),
        ),
    );

    let weth_aggregator_address = env.deploy_contract(
        admin_address,
        "WETH Aggregator",
        constructor_data(
            aave_bytecode::MOCK_AGGREGATOR,
            Some(400000000000u128.abi_encode()),
        ),
    );

    let pool_address = env.deploy_contract(
        admin_address,
        "Pool implementation",
        constructor_data(
            aave_bytecode::POOL_IMPLEMENTATION,
            Some(address_provider_address.abi_encode()),
        ),
    );

    env.direct_execute(
        admin_address,
        pool_address,
        aave_abi::Pool_Implementation::initializeCall {
            provider: address_provider_address,
        },
        U256::ZERO,
    )
    .unwrap();

    env.direct_execute(
        admin_address,
        address_provider_address,
        aave_abi::PoolAddressesProvider::setPoolImplCall {
            newPoolImpl: pool_address,
        },
        U256::ZERO,
    )
    .unwrap();

    let pool_proxy_address = env
        .direct_call(
            admin_address,
            address_provider_address,
            aave_abi::PoolAddressesProvider::getPoolCall {},
            U256::ZERO,
        )
        .unwrap()
        .0
        ._0;

    let pool_configurator_address = env.deploy_contract(
        admin_address,
        "Pool configurator implementation",
        data_bytes_from_hex(aave_bytecode::POOL_CONFIGURATOR_IMPLEMENTATION),
    );

    env.direct_execute(
        admin_address,
        pool_configurator_address,
        aave_abi::PoolConfigurator_Implementation::initializeCall {
            provider: address_provider_address,
        },
        U256::ZERO,
    )
    .unwrap();

    env.direct_execute(
        admin_address,
        address_provider_address,
        aave_abi::PoolAddressesProvider::setPoolConfiguratorImplCall {
            newPoolConfiguratorImpl: pool_configurator_address,
        },
        U256::ZERO,
    )
    .unwrap();

    let pool_configurator_proxy_address = env
        .direct_call(
            admin_address,
            address_provider_address,
            aave_abi::PoolAddressesProvider::getPoolConfiguratorCall {},
            U256::ZERO,
        )
        .unwrap()
        .0
        ._0;

    let reserves_setup_helper_address = env.deploy_contract(
        admin_address,
        "Reserve setup helper",
        data_bytes_from_hex(aave_bytecode::RESERVES_SETUP_HELPER),
    );

    env.direct_execute(
        admin_address,
        address_provider_address,
        aave_abi::PoolAddressesProvider::setACLAdminCall {
            newAclAdmin: admin_address,
        },
        U256::ZERO,
    )
    .unwrap();

    let acl_manager_address = env.deploy_contract(
        admin_address,
        "ACL manager",
        constructor_data(
            aave_bytecode::ACL_MANAGER,
            Some(address_provider_address.abi_encode()),
        ),
    );

    env.direct_execute(
        admin_address,
        address_provider_address,
        aave_abi::PoolAddressesProvider::setACLManagerCall {
            newAclManager: acl_manager_address,
        },
        U256::ZERO,
    )
    .unwrap();

    env.direct_execute(
        admin_address,
        acl_manager_address,
        aave_abi::ACLManager::addPoolAdminCall {
            admin: admin_address,
        },
        U256::ZERO,
    )
    .unwrap();

    env.direct_execute(
        admin_address,
        acl_manager_address,
        aave_abi::ACLManager::addEmergencyAdminCall {
            admin: admin_address,
        },
        U256::ZERO,
    )
    .unwrap();

    let oracle_address = env.deploy_contract(
        admin_address,
        "Aave oracle",
        constructor_data(
            aave_bytecode::AAVE_ORACLE,
            Some(
                (
                    address_provider_address,
                    vec![
                        aave_address,
                        periphery_addresses.token_a,
                        periphery_addresses.token_b,
                        periphery_addresses.weth,
                    ],
                    vec![
                        aave_aggregator_address,
                        uniswap_addresses.aggregator,
                        token_b_aggregator_address,
                        weth_aggregator_address,
                    ],
                    Address::ZERO,
                    Address::ZERO,
                    100000000u128,
                )
                    .abi_encode_params(),
            ),
        ),
    );

    env.direct_execute(
        admin_address,
        address_provider_address,
        aave_abi::PoolAddressesProvider::setPriceOracleCall {
            newPriceOracle: oracle_address,
        },
        U256::ZERO,
    )
    .unwrap();

    env.direct_execute(
        admin_address,
        pool_configurator_proxy_address,
        aave_abi::PoolConfigurator_Implementation::updateFlashloanPremiumTotalCall {
            newFlashloanPremiumTotal: 9u128,
        },
        U256::ZERO,
    )
    .unwrap();

    env.direct_execute(
        admin_address,
        pool_configurator_proxy_address,
        aave_abi::PoolConfigurator_Implementation::updateFlashloanPremiumToProtocolCall {
            newFlashloanPremiumToProtocol: 0u128,
        },
        U256::ZERO,
    )
    .unwrap();

    let emission_manager_address = env.deploy_contract(
        admin_address,
        "Emission manager",
        constructor_data(
            aave_bytecode::EMISSION_MANAGER,
            Some(admin_address.abi_encode()),
        ),
    );

    let rewards_controller_address = env.deploy_contract(
        admin_address,
        "Incentive V2 Implementation",
        constructor_data(
            aave_bytecode::INCENTIVES_V2_IMPLEMENTATION,
            Some(emission_manager_address.abi_encode()),
        ),
    );

    env.direct_execute(
        admin_address,
        rewards_controller_address,
        aave_abi::IncentivesV2_Implementation::initializeCall { _0: Address::ZERO },
        U256::ZERO,
    )
    .unwrap();

    let id: [u8; 32] =
        data_bytes_from_hex("703c2c8634bed68d98c029c18f310e7f7ec0e5d6342c590190b3cb8b3ba54532")
            [..32]
            .try_into()
            .unwrap();

    env.direct_execute(
        admin_address,
        address_provider_address,
        aave_abi::PoolAddressesProvider::setAddressAsProxyCall {
            id: alloy_primitives::FixedBytes::from(id),
            newImplementationAddress: rewards_controller_address,
        },
        U256::ZERO,
    )
    .unwrap();

    let rewards_controller_proxy_address = env
        .direct_call(
            admin_address,
            address_provider_address,
            aave_abi::PoolAddressesProvider::getAddressCall {
                id: alloy_primitives::FixedBytes::from(id),
            },
            U256::ZERO,
        )
        .unwrap()
        .0
        ._0;

    env.direct_execute(
        admin_address,
        emission_manager_address,
        aave_abi::EmissionManager::setRewardsControllerCall {
            controller: rewards_controller_proxy_address,
        },
        U256::ZERO,
    )
    .unwrap();

    let _pull_rewards_transfer_address = env.deploy_contract(
        admin_address,
        "Pull rewards transfer strategy",
        constructor_data(
            aave_bytecode::PULL_REWARDS_TRANSFER_STRATEGY,
            Some(
                (
                    rewards_controller_proxy_address,
                    admin_address,
                    admin_address,
                )
                    .abi_encode_params(),
            ),
        ),
    );

    let _staked_token_transfer_strategy_address = env.deploy_contract(
        admin_address,
        "Staked transfer strategy",
        constructor_data(
            aave_bytecode::STAKED_TOKEN_TRANSFER_STRATEGY,
            Some(
                (
                    rewards_controller_proxy_address,
                    admin_address,
                    staked_aave_proxy_address,
                )
                    .abi_encode_params(),
            ),
        ),
    );

    env.direct_execute(
        admin_address,
        emission_manager_address,
        aave_abi::EmissionManager::transferOwnershipCall {
            newOwner: admin_address,
        },
        U256::ZERO,
    )
    .unwrap();

    let a_token_address = env.deploy_contract(
        admin_address,
        "AToken",
        constructor_data(
            aave_bytecode::A_TOKEN,
            Some(pool_proxy_address.abi_encode()),
        ),
    );

    env.direct_execute(
        admin_address,
        a_token_address,
        aave_abi::AToken::initializeCall {
            initializingPool: pool_proxy_address,
            treasury: Address::ZERO,
            underlyingAsset: Address::ZERO,
            incentivesController: Address::ZERO,
            aTokenDecimals: 0u8,
            aTokenName: "ATOKEN_IMPL".to_string(),
            aTokenSymbol: "ATOKEN_IMPL".to_string(),
            params: data_bytes_from_hex("00").into(),
        },
        U256::ZERO,
    )
    .unwrap();

    let delegation_aware_token_address = env.deploy_contract(
        admin_address,
        "Delegation aware Atoken",
        constructor_data(
            aave_bytecode::DELEGATION_AWARE_ATOKEN,
            Some(pool_proxy_address.abi_encode()),
        ),
    );

    env.direct_execute(
        admin_address,
        delegation_aware_token_address,
        aave_abi::DelegationAwareAToken::initializeCall {
            initializingPool: pool_proxy_address,
            treasury: Address::ZERO,
            underlyingAsset: Address::ZERO,
            incentivesController: Address::ZERO,
            aTokenDecimals: 0u8,
            aTokenName: "DELEGATION_AWARE_ATOKEN_IMPL".to_string(),
            aTokenSymbol: "DELEGATION_AWARE_ATOKEN_IMPL".to_string(),
            params: data_bytes_from_hex("00").into(),
        },
        U256::ZERO,
    )
    .unwrap();

    let stable_debt_token_address = env.deploy_contract(
        admin_address,
        "Stable debt token",
        constructor_data(
            aave_bytecode::STABLE_DEBT_TOKEN,
            Some(pool_proxy_address.abi_encode()),
        ),
    );

    env.direct_execute(
        admin_address,
        stable_debt_token_address,
        aave_abi::StableDebtToken::initializeCall {
            initializingPool: pool_proxy_address,
            underlyingAsset: Address::ZERO,
            incentivesController: Address::ZERO,
            debtTokenDecimals: 0u8,
            debtTokenName: "STABLE_DEBT_TOKEN_IMPL".to_string(),
            debtTokenSymbol: "STABLE_DEBT_TOKEN_IMPL".to_string(),
            params: data_bytes_from_hex("00").into(),
        },
        U256::ZERO,
    )
    .unwrap();

    let variable_debt_token_address = env.deploy_contract(
        admin_address,
        "Variable debt token",
        constructor_data(
            aave_bytecode::VARIABLE_DEBT_TOKEN,
            Some(pool_proxy_address.abi_encode()),
        ),
    );

    env.direct_execute(
        admin_address,
        variable_debt_token_address,
        aave_abi::VariableDebtToken::initializeCall {
            initializingPool: pool_proxy_address,
            underlyingAsset: Address::ZERO,
            incentivesController: Address::ZERO,
            debtTokenDecimals: 0u8,
            debtTokenName: "VARIABLE_DEBT_TOKEN_IMPL".to_string(),
            debtTokenSymbol: "VARIABLE_DEBT_TOKEN_IMPL".to_string(),
            params: data_bytes_from_hex("00").into(),
        },
        U256::ZERO,
    )
    .unwrap();

    let reserve_strategy_volatile_one_address = env.deploy_contract(
        admin_address,
        "Reserve strategy volatile 1",
        constructor_data(
            aave_bytecode::RESERVE_STRATEGY_RATE_STRATEGY_VOLATILE_ONE,
            Some(
                (
                    pool_proxy_address,
                    450000000000000000000000000u128,
                    0u128,
                    70000000000000000000000000u128,
                    3000000000000000000000000000u128,
                    0u128,
                    0u128,
                    20000000000000000000000000u128,
                    50000000000000000000000000u128,
                    200000000000000000000000000u128,
                )
                    .abi_encode_params(),
            ),
        ),
    );

    let reserve_strategy_stable_one_address = env.deploy_contract(
        admin_address,
        "Reserve strategy stable 1",
        constructor_data(
            aave_bytecode::RESERVE_STRATEGY_RATE_STRATEGY_STABLE_ONE,
            Some(
                (
                    pool_proxy_address,
                    900000000000000000000000000u128,
                    0u128,
                    40000000000000000000000000u128,
                    600000000000000000000000000u128,
                    20000000000000000000000000u128,
                    600000000000000000000000000u128,
                    20000000000000000000000000u128,
                    50000000000000000000000000u128,
                    200000000000000000000000000u128,
                )
                    .abi_encode_params(),
            ),
        ),
    );

    let reserve_strategy_stable_two_address = env.deploy_contract(
        admin_address,
        "Reserve strategy stable 2",
        constructor_data(
            aave_bytecode::RESERVE_STRATEGY_RATE_STRATEGY_STABLE_TWO,
            Some(
                (
                    pool_proxy_address,
                    800000000000000000000000000u128,
                    0u128,
                    40000000000000000000000000u128,
                    750000000000000000000000000u128,
                    20000000000000000000000000u128,
                    750000000000000000000000000u128,
                    20000000000000000000000000u128,
                    50000000000000000000000000u128,
                    200000000000000000000000000u128,
                )
                    .abi_encode_params(),
            ),
        ),
    );

    env.direct_execute(
        admin_address,
        pool_configurator_proxy_address,
        aave_abi::PoolConfigurator_Implementation::initReservesCall {
            input: vec![
                aave_abi::PoolConfigurator_Implementation::InitReserveInput {
                    aTokenImpl: a_token_address,
                    stableDebtTokenImpl: stable_debt_token_address,
                    variableDebtTokenImpl: variable_debt_token_address,
                    underlyingAssetDecimals: 18u8,
                    interestRateStrategyAddress: reserve_strategy_volatile_one_address,
                    underlyingAsset: aave_address,
                    treasury: treasury_proxy_address,
                    incentivesController: rewards_controller_proxy_address,
                    aTokenName: "AAVE".to_string(),
                    aTokenSymbol: "aAAVE".to_string(),
                    variableDebtTokenName: "AAVE Variable Debt".to_string(),
                    variableDebtTokenSymbol: "VariableDebtAAVE".to_string(),
                    stableDebtTokenName: "AAVE Stable Debt".to_string(),
                    stableDebtTokenSymbol: "StableDebtAAVE".to_string(),
                    params: data_bytes_from_hex("10").into(),
                },
                aave_abi::PoolConfigurator_Implementation::InitReserveInput {
                    aTokenImpl: a_token_address,
                    stableDebtTokenImpl: stable_debt_token_address,
                    variableDebtTokenImpl: variable_debt_token_address,
                    underlyingAssetDecimals: 18u8,
                    interestRateStrategyAddress: reserve_strategy_stable_one_address,
                    underlyingAsset: periphery_addresses.token_a,
                    treasury: treasury_proxy_address,
                    incentivesController: rewards_controller_proxy_address,
                    aTokenName: "A".to_string(),
                    aTokenSymbol: "aA".to_string(),
                    variableDebtTokenName: "A Variable Debt".to_string(),
                    variableDebtTokenSymbol: "VariableDebtA".to_string(),
                    stableDebtTokenName: "A Stable Debt".to_string(),
                    stableDebtTokenSymbol: "StableDebtA".to_string(),
                    params: data_bytes_from_hex("10").into(),
                },
                aave_abi::PoolConfigurator_Implementation::InitReserveInput {
                    aTokenImpl: a_token_address,
                    stableDebtTokenImpl: stable_debt_token_address,
                    variableDebtTokenImpl: variable_debt_token_address,
                    underlyingAssetDecimals: 18u8,
                    interestRateStrategyAddress: reserve_strategy_stable_two_address,
                    underlyingAsset: periphery_addresses.token_b,
                    treasury: treasury_proxy_address,
                    incentivesController: rewards_controller_proxy_address,
                    aTokenName: "B".to_string(),
                    aTokenSymbol: "aB".to_string(),
                    variableDebtTokenName: "B Variable Debt".to_string(),
                    variableDebtTokenSymbol: "VariableDebtB".to_string(),
                    stableDebtTokenName: "B Stable Debt".to_string(),
                    stableDebtTokenSymbol: "StableDebtB".to_string(),
                    params: data_bytes_from_hex("10").into(),
                },
                aave_abi::PoolConfigurator_Implementation::InitReserveInput {
                    aTokenImpl: a_token_address,
                    stableDebtTokenImpl: stable_debt_token_address,
                    variableDebtTokenImpl: variable_debt_token_address,
                    underlyingAssetDecimals: 18u8,
                    interestRateStrategyAddress: reserve_strategy_volatile_one_address,
                    underlyingAsset: periphery_addresses.weth,
                    treasury: treasury_proxy_address,
                    incentivesController: rewards_controller_proxy_address,
                    aTokenName: "WETH".to_string(),
                    aTokenSymbol: "aWETH".to_string(),
                    variableDebtTokenName: "WETH Variable Debt".to_string(),
                    variableDebtTokenSymbol: "VariableDebtWETH".to_string(),
                    stableDebtTokenName: "WETH Stable Debt".to_string(),
                    stableDebtTokenSymbol: "StableDebtWETH".to_string(),
                    params: data_bytes_from_hex("10").into(),
                },
            ],
        },
        U256::ZERO,
    )
    .unwrap();

    env.direct_execute(
        admin_address,
        acl_manager_address,
        aave_abi::ACLManager::addRiskAdminCall {
            admin: reserves_setup_helper_address,
        },
        U256::ZERO,
    )
    .unwrap();

    env.direct_execute(
        admin_address,
        reserves_setup_helper_address,
        aave_abi::ReservesSetupHelper::configureReservesCall {
            configurator: pool_configurator_proxy_address,
            inputParams: vec![
                aave_abi::ReservesSetupHelper::ConfigureReserveInput {
                    asset: aave_address,
                    baseLTV: Uint::from(5000u128),
                    liquidationThreshold: Uint::from(6500u128),
                    liquidationBonus: Uint::from(11000u128),
                    reserveFactor: Uint::from(0u128),
                    borrowCap: Uint::from(0u128),
                    supplyCap: Uint::from(0u128),
                    stableBorrowingEnabled: false,
                    borrowingEnabled: false,
                    flashLoanEnabled: true,
                },
                aave_abi::ReservesSetupHelper::ConfigureReserveInput {
                    asset: periphery_addresses.token_a,
                    baseLTV: Uint::from(token_a_base_ltv),
                    liquidationThreshold: Uint::from(token_a_liquidation_threshold),
                    liquidationBonus: Uint::from(10500u128),
                    reserveFactor: Uint::from(1000u128),
                    borrowCap: Uint::from(0u128),
                    supplyCap: Uint::from(0u128),
                    stableBorrowingEnabled: true,
                    borrowingEnabled: true,
                    flashLoanEnabled: true,
                },
                aave_abi::ReservesSetupHelper::ConfigureReserveInput {
                    asset: periphery_addresses.token_b,
                    baseLTV: Uint::from(token_b_base_ltv),
                    liquidationThreshold: Uint::from(token_b_liquidation_threshold),
                    liquidationBonus: Uint::from(10500u128),
                    reserveFactor: Uint::from(1000u128),
                    borrowCap: Uint::from(0u128),
                    supplyCap: Uint::from(0u128),
                    stableBorrowingEnabled: true,
                    borrowingEnabled: true,
                    flashLoanEnabled: true,
                },
                aave_abi::ReservesSetupHelper::ConfigureReserveInput {
                    asset: periphery_addresses.weth,
                    baseLTV: Uint::from(8000u128),
                    liquidationThreshold: Uint::from(8250u128),
                    liquidationBonus: Uint::from(10500u128),
                    reserveFactor: Uint::from(1000u128),
                    borrowCap: Uint::from(0u128),
                    supplyCap: Uint::from(0u128),
                    stableBorrowingEnabled: true,
                    borrowingEnabled: true,
                    flashLoanEnabled: true,
                },
            ],
        },
        U256::ZERO,
    )
    .unwrap();

    env.direct_execute(
        admin_address,
        acl_manager_address,
        aave_abi::ACLManager::removeRiskAdminCall {
            admin: reserves_setup_helper_address,
        },
        U256::ZERO,
    )
    .unwrap();

    let _weth_gateway_address = env.deploy_contract(
        admin_address,
        "Wrapped token gateway V3",
        constructor_data(
            aave_bytecode::WRAPPED_TOKEN_GATEWAY_V3,
            Some((periphery_addresses.weth, admin_address, pool_proxy_address).abi_encode_params()),
        ),
    );

    let _wallet_balance_provider_address = env.deploy_contract(
        admin_address,
        "Wallet balance provider",
        data_bytes_from_hex(aave_bytecode::WALLET_BALANCE_PROVIDER),
    );

    env.direct_execute(
        admin_address,
        pool_configurator_proxy_address,
        aave_abi::PoolConfigurator_Implementation::setBorrowableInIsolationCall {
            asset: periphery_addresses.token_a,
            borrowable: true,
        },
        U256::ZERO,
    )
    .unwrap();

    env.direct_execute(
        admin_address,
        pool_configurator_proxy_address,
        aave_abi::PoolConfigurator_Implementation::setBorrowableInIsolationCall {
            asset: periphery_addresses.token_b,
            borrowable: true,
        },
        U256::ZERO,
    )
    .unwrap();

    env.direct_execute(
        admin_address,
        pool_configurator_proxy_address,
        aave_abi::PoolConfigurator_Implementation::setEModeCategoryCall {
            categoryId: 1u8,
            ltv: 9800u16,
            liquidationThreshold: 9850u16,
            liquidationBonus: 10100u16,
            oracle: Address::ZERO,
            label: "Stable-EMode".to_string(),
        },
        U256::ZERO,
    )
    .unwrap();

    env.direct_execute(
        admin_address,
        pool_configurator_proxy_address,
        aave_abi::PoolConfigurator_Implementation::setAssetEModeCategoryCall {
            asset: periphery_addresses.token_a,
            newCategoryId: 1u8,
        },
        U256::ZERO,
    )
    .unwrap();

    env.direct_execute(
        admin_address,
        pool_configurator_proxy_address,
        aave_abi::PoolConfigurator_Implementation::setAssetEModeCategoryCall {
            asset: periphery_addresses.token_b,
            newCategoryId: 1u8,
        },
        U256::ZERO,
    )
    .unwrap();

    env.direct_execute(
        admin_address,
        pool_configurator_proxy_address,
        aave_abi::PoolConfigurator_Implementation::setLiquidationProtocolFeeCall {
            asset: aave_address,
            newFee: Uint::from(1000u128),
        },
        U256::ZERO,
    )
    .unwrap();

    env.direct_execute(
        admin_address,
        pool_configurator_proxy_address,
        aave_abi::PoolConfigurator_Implementation::setLiquidationProtocolFeeCall {
            asset: periphery_addresses.token_a,
            newFee: Uint::from(1000u128),
        },
        U256::ZERO,
    )
    .unwrap();

    env.direct_execute(
        admin_address,
        pool_configurator_proxy_address,
        aave_abi::PoolConfigurator_Implementation::setLiquidationProtocolFeeCall {
            asset: periphery_addresses.token_b,
            newFee: Uint::from(1000u128),
        },
        U256::ZERO,
    )
    .unwrap();

    env.direct_execute(
        admin_address,
        pool_configurator_proxy_address,
        aave_abi::PoolConfigurator_Implementation::setLiquidationProtocolFeeCall {
            asset: periphery_addresses.weth,
            newFee: Uint::from(1000u128),
        },
        U256::ZERO,
    )
    .unwrap();

    env.direct_execute(
        admin_address,
        pool_configurator_proxy_address,
        aave_abi::PoolConfigurator_Implementation::setPoolPauseCall { paused: false },
        U256::ZERO,
    )
    .unwrap();

    (
        env,
        periphery_addresses,
        uniswap_addresses,
        AaveAddresses {
            data_provider: data_provider_address,
            token_a_oracle: token_a_aggregator_address,
            token_b_oracle: token_b_aggregator_address,
            pool: pool_proxy_address,
            oracle: oracle_address,
        },
    )
}
