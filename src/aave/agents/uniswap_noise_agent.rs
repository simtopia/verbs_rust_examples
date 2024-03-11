use crate::aave::calls;
use crate::aave::protocol::uniswap_abi;

use alloy_primitives::{Address, Uint, U256};
use rand::Rng;
use verbs_rs::agent::{Agent, RecordedAgent};
use verbs_rs::contract::Transaction;
use verbs_rs::env::Env;
use verbs_rs::DB;

pub struct UniswapNoiseAgent {
    address: Address,
    fee: u32,
    swap_router: Address,
    token_a: Address,
    token_b: Address,
    token_a_decimals: U256,
}

impl UniswapNoiseAgent {
    pub fn new<D>(
        network: &mut Env<D>,
        idx: usize,
        fee: u32,
        swap_router: Address,
        token_a: Address,
        token_b: Address,
    ) -> Self
    where
        D: DB,
    {
        let address = Address::from(Uint::from(idx));
        let token_a_decimals = calls::get_decimals(network, address, token_a);
        Self {
            address,
            fee,
            swap_router,
            token_a,
            token_b,
            token_a_decimals,
        }
    }
}

impl Agent for UniswapNoiseAgent {
    fn update<D, R>(&mut self, rng: &mut R, _network: &mut Env<D>) -> Vec<Transaction>
    where
        D: DB,
        R: Rng,
    {
        let mut calls: Vec<Transaction> = vec![];

        let amount_token_a = rng.gen::<f64>() * 10_f64.powi(6i32);
        let amount_token_a = U256::from(amount_token_a as u64)
            * U256::from(10).pow(self.token_a_decimals - U256::from(6));

        // Direction of the trade is random with p 0.5
        if rng.gen::<f64>() <= 0.5_f64 {
            // Buy token b and sell token a
            let call = calls::uniswap_swap_call(
                self.address,
                self.swap_router,
                uniswap_abi::SwapRouter::ExactInputSingleParams {
                    amountIn: amount_token_a,
                    amountOutMinimum: U256::ZERO,
                    deadline: U256::MAX,
                    fee: self.fee,
                    recipient: self.address,
                    sqrtPriceLimitX96: U256::ZERO,
                    tokenIn: self.token_a,
                    tokenOut: self.token_b,
                },
            );
            calls.push(call);
        } else {
            // Buy token a and sell token b
            let call = calls::uniswap_swap_call_exact_output(
                self.address,
                self.swap_router,
                uniswap_abi::SwapRouter::ExactOutputSingleParams {
                    amountInMaximum: U256::MAX,
                    amountOut: amount_token_a,
                    deadline: U256::MAX,
                    fee: self.fee,
                    recipient: self.address,
                    sqrtPriceLimitX96: U256::ZERO,
                    tokenIn: self.token_b,
                    tokenOut: self.token_a,
                },
            );
            calls.push(call);
        }

        calls
    }

    fn get_address(&self) -> Address {
        self.address
    }
}

impl RecordedAgent<U256> for UniswapNoiseAgent {
    fn record<D: DB>(&mut self, _env: &mut Env<D>) -> U256 {
        U256::ZERO
    }
}
