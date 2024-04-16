use alloy_primitives::{Address, U256};
use alloy_sol_types::SolValue;
use verbs_rs::env::Validator;
use verbs_rs::utils::constructor_data;
use verbs_rs::{env::Env, LocalDB};

use super::{deploy_periphery::PeripheryAddresses, uniswap_abi, uniswap_bytecode};

pub struct UniswapAddresses {
    pub pool: Address,
    pub aggregator: Address,
    pub swap_router: Address,
    pub nft_position_manager: Address,
    pub quoter_address: Address,
}

pub fn deploy_uniswap_contracts<V: Validator>(
    mut env: Env<LocalDB, V>,
    admin_address: Address,
    periphery_addresses: &PeripheryAddresses,
    uniswap_fee: u32,
) -> (Env<LocalDB, V>, UniswapAddresses) {
    let factory_address = env.deploy_contract(
        admin_address,
        "Uniswap factory",
        constructor_data(uniswap_bytecode::UNISWAP_V3_FACTORY, None),
    );

    let pool_address = env
        .direct_execute(
            admin_address,
            factory_address,
            uniswap_abi::UniswapV3Factory::createPoolCall {
                tokenA: periphery_addresses.token_a,
                tokenB: periphery_addresses.token_b,
                fee: uniswap_fee,
            },
            U256::ZERO,
        )
        .unwrap()
        .0
        .pool;

    let aggregator_address = env.deploy_contract(
        admin_address,
        "Uniswap aggregator",
        constructor_data(
            uniswap_bytecode::UNISWAP_AGGREGATOR,
            Some(
                (
                    pool_address,
                    periphery_addresses.token_a,
                    periphery_addresses.token_b,
                )
                    .abi_encode_params(),
            ),
        ),
    );

    let swap_router_address: Address = env.deploy_contract(
        admin_address,
        "Swap router",
        constructor_data(
            uniswap_bytecode::SWAP_ROUTER,
            Some((factory_address, periphery_addresses.weth).abi_encode_params()),
        ),
    );

    let nft_position_descriptor_address: Address = env.deploy_contract(
        admin_address,
        "Uniswap NFT descriptor",
        constructor_data(
            uniswap_bytecode::NON_FUNGIBLE_TOKEN_POSITION_DESCRIPTOR,
            Some(periphery_addresses.weth.abi_encode()),
        ),
    );

    let nft_position_manager_address: Address = env.deploy_contract(
        admin_address,
        "Uniswap NFT position manager",
        constructor_data(
            uniswap_bytecode::NON_FUNGIBLE_POSITION_MANAGER,
            Some(
                (
                    factory_address,
                    periphery_addresses.weth,
                    nft_position_descriptor_address,
                )
                    .abi_encode_params(),
            ),
        ),
    );

    let quoter_address_v2 = env.deploy_contract(
        admin_address,
        "Quoter",
        constructor_data(
            uniswap_bytecode::QUOTERV2,
            Some((factory_address, periphery_addresses.weth).abi_encode_params()),
        ),
    );
    (
        env,
        UniswapAddresses {
            pool: pool_address,
            aggregator: aggregator_address,
            swap_router: swap_router_address,
            nft_position_manager: nft_position_manager_address,
            quoter_address: quoter_address_v2,
        },
    )
}
