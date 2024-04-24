use alloy_primitives::Address;
use alloy_sol_types::SolValue;
use verbs_rs::env::{Env, Validator};
use verbs_rs::utils::constructor_data;
use verbs_rs::LocalDB;

use super::periphery_bytecode;

pub struct PeripheryAddresses {
    pub faucet: Address,
    pub token_a: Address,
    pub token_b: Address,
    pub weth: Address,
}

pub fn deploy_periphery_contracts<V: Validator>(
    mut env: Env<LocalDB, V>,
    admin_address: Address,
) -> (Env<LocalDB, V>, PeripheryAddresses) {
    let faucet_address = env.deploy_contract(
        admin_address,
        "Faucet",
        constructor_data(
            periphery_bytecode::FAUCET,
            Some((admin_address, false).abi_encode_params()),
        ),
    );

    let token_a_address = env.deploy_contract(
        admin_address,
        "Token A",
        constructor_data(
            periphery_bytecode::MINTABLE_ERC20,
            Some(
                (String::from("A"), String::from("A"), 18u128, faucet_address).abi_encode_params(),
            ),
        ),
    );

    let token_b_address = env.deploy_contract(
        admin_address,
        "Token B",
        constructor_data(
            periphery_bytecode::MINTABLE_ERC20,
            Some(
                (String::from("B"), String::from("B"), 18u128, faucet_address).abi_encode_params(),
            ),
        ),
    );

    let weth_address = env.deploy_contract(
        admin_address,
        "WETH token",
        constructor_data(
            periphery_bytecode::WETH_MINTABLE_ERC20,
            Some((String::from("WETH"), String::from("WETH"), faucet_address).abi_encode_params()),
        ),
    );
    // Token a and token b need to be ordered so that all interactions with
    // Uniswap make sense. Most of the Uniswap's functions require sqrtPriceX96
    // as an input, which is interpreted as the price of token0 in terms of token1.
    // We will use make the calculations considering that token_a = token0 and token_b=token
    // and so we need tokan_a < token_b
    if token_a_address < token_b_address {
        (
            env,
            PeripheryAddresses {
                faucet: faucet_address,
                token_a: token_a_address,
                token_b: token_b_address,
                weth: weth_address,
            },
        )
    } else {
        (
            env,
            PeripheryAddresses {
                faucet: faucet_address,
                token_a: token_b_address,
                token_b: token_a_address,
                weth: weth_address,
            },
        )
    }
}
