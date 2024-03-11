use crate::aave::calls;
use crate::aave::protocol::uniswap_abi;

use alloy_primitives::{Address, Uint, U256};
use funty::Fundamental;
use rand::RngCore;
use rand_distr::{Distribution, Normal};
use verbs_rs::agent::{Agent, RecordedAgent};
use verbs_rs::contract::Transaction;
use verbs_rs::env::Env;
use verbs_rs::utils::div_u256;
use verbs_rs::DB;

pub struct UniswapPriceAgent {
    address: Address,
    pool: Address,
    fee: u32,
    swap_router: Address,
    token_b: Address, // stable coin (debt in Aave in simulation)
    token0: Address,
    token1: Address,
    external_market: Gbm,
    step: u32,
    transient_price_impact: f64,
    dt: f64,
}

impl UniswapPriceAgent {
    pub fn new<D>(
        network: &mut Env<D>,
        idx: usize,
        pool: Address,
        fee: u32,
        swap_router: Address,
        token_b: Address,
        token_a_price: i128,
        token_b_price: i128,
        mu: f64,
        dt: f64,
        sigma: f64,
    ) -> Self
    where
        D: DB,
    {
        let address = Address::from(Uint::from(idx));
        let external_market = Gbm::new(dt, mu, sigma, token_a_price, token_a_price, token_b_price);
        let token0 = calls::get_token0(network, address, pool);
        let token1 = calls::get_token1(network, address, pool);

        Self {
            address,
            pool,
            fee,
            swap_router,
            token_b,
            token0,
            token1,
            external_market,
            step: 0u32,
            transient_price_impact: 0.,
            dt,
        }
    }

    /// Gets the swap parameters so that, after the swap, the price in Uniswap is the same as the price in the external market.
    /// We know that in Uniswap v3 (or v2), we have
    /// L = \frac{\Delta y}{\Delta \sqrt{P}}
    /// where y is the numeraire (in our case the debt asset), and P is the price of the collateral in terms of the numeraire.
    pub fn get_swap_size_to_increase_uniswap_price(
        &mut self,
        sqrt_price_external_market: u128,
        sqrt_price_uniswap_x96: u128,
        liquidity: u128,
    ) -> Option<uniswap_abi::SwapRouter::ExactInputSingleParams> {
        let change_sqrt_price = sqrt_price_external_market - sqrt_price_uniswap_x96;

        let mut change_token1 = U256::from(liquidity) / U256::from(2).pow(U256::from(48));
        change_token1 *= U256::from(change_sqrt_price) / U256::from(2).pow(U256::from(48));

        if change_token1 > U256::ZERO {
            Some(uniswap_abi::SwapRouter::ExactInputSingleParams {
                amountIn: change_token1,
                amountOutMinimum: U256::ZERO,
                deadline: U256::MAX,
                fee: self.fee,
                recipient: self.address,
                sqrtPriceLimitX96: U256::MIN,
                tokenIn: self.token1,
                tokenOut: self.token0,
            })
        } else {
            None
        }
    }

    pub fn get_swap_size_to_decrease_uniswap_price(
        &mut self,
        sqrt_price_external_market: u128,
        sqrt_price_uniswap_x96: u128,
        liquidity: u128,
    ) -> Option<uniswap_abi::SwapRouter::ExactOutputSingleParams> {
        let change_sqrt_price = sqrt_price_uniswap_x96 - sqrt_price_external_market;

        let mut change_token1 = U256::from(liquidity) / U256::from(2).pow(U256::from(48));
        change_token1 *= U256::from(change_sqrt_price) / U256::from(2).pow(U256::from(48));

        if change_token1 > U256::ZERO {
            Some(uniswap_abi::SwapRouter::ExactOutputSingleParams {
                amountOut: change_token1,
                amountInMaximum: U256::MAX,
                deadline: U256::MAX,
                fee: self.fee,
                recipient: self.address,
                sqrtPriceLimitX96: U256::MIN, //U256::from(sqrt_price_external_market),
                tokenIn: self.token0,
                tokenOut: self.token1,
            })
        } else {
            None
        }
    }

    fn get_price_impact(&mut self, sqrt_price_uniswap_x96: U256) -> f64 {
        // price_uniswap is price of token0 in terms of token1
        let price_uniswap: f64 = f64::powi(
            div_u256(
                sqrt_price_uniswap_x96,
                U256::from(2).pow(U256::from(96)),
                10,
            ),
            2,
        );
        let price_external_market = self.external_market.get_price_token_a();
        // we check whether we need to invert price_uniswap
        if self.token_b == self.token1 {
            price_uniswap - price_external_market
        } else {
            1. / price_uniswap - price_external_market
        }
    }
}

impl Agent for UniswapPriceAgent {
    fn update<D, R>(&mut self, rng: &mut R, network: &mut Env<D>) -> Vec<Transaction>
    where
        D: DB,
        R: RngCore,
    {
        // Uniswap returns price of token0 in terms of token1
        let sqrt_price_uniswap_x96: U256 =
            calls::get_slot0(network, self.address, self.pool).sqrtPriceX96;

        // Accumulate price impact with exponential decay
        if self.step > 0 {
            let current_price_impact = self.get_price_impact(sqrt_price_uniswap_x96);
            self.transient_price_impact =
                f64::exp(-2. * self.dt) * self.transient_price_impact + current_price_impact;
        }

        // external market updates
        self.external_market
            .update(rng, self.transient_price_impact);

        let mut calls: Vec<Transaction> = Vec::new();

        let sqrt_price_uniswap_x96: u128 = u128::try_from(sqrt_price_uniswap_x96).unwrap();
        let sqrt_price_external_market = if self.token1 == self.token_b {
            self.external_market.get_sqrt_price_token_a_x96()
        } else {
            self.external_market.get_sqrt_price_token_b_x96()
        };
        let liquidity = calls::get_liquidity(network, self.address, self.pool);

        // find swap parameters so that price of uniswap after the swap matches the price of the external market
        // sqrt_price_external_market > sqrt_price_uniswap_x96, the uniswap agent wants to buy collateral asset (and sell debt asset) to increase the price of Uniswap
        // sqrt_price_external_market < sqrt_price_uniswap_x96, the uniswap agent wants to sell collateral asset (and buy debt asset) to decrease the price of Uniswap
        if sqrt_price_external_market > sqrt_price_uniswap_x96 {
            let params_swap = self.get_swap_size_to_increase_uniswap_price(
                sqrt_price_external_market,
                sqrt_price_uniswap_x96,
                liquidity,
            );
            if let Some(params) = params_swap {
                let call = calls::uniswap_swap_call(self.address, self.swap_router, params);
                calls.push(call);
            }
        } else {
            let params_swap = self.get_swap_size_to_decrease_uniswap_price(
                sqrt_price_external_market,
                sqrt_price_uniswap_x96,
                liquidity,
            );
            if let Some(params) = params_swap {
                let call =
                    calls::uniswap_swap_call_exact_output(self.address, self.swap_router, params);
                calls.push(call);
            }
        }
        self.step += 1;
        calls
    }

    fn get_address(&self) -> Address {
        self.address
    }
}

impl RecordedAgent<(i128, i128)> for UniswapPriceAgent {
    fn record<D: DB>(&mut self, _env: &mut Env<D>) -> (i128, i128) {
        (
            self.external_market.token_a_price,
            self.external_market.token_b_price,
        )
    }
}

pub struct Gbm {
    token_a_price: i128,
    token_b_price: i128,
    token_a_price_with_impact: i128,
    // gbm drift
    mu: f64,
    // gbm vol
    sigma: f64,
    normal: Normal<f64>,
    dt: f64,
}

impl Gbm {
    pub fn new(
        dt: f64,
        mu: f64,
        sigma: f64,
        token_a_price: i128,
        token_a_price_with_impact: i128,
        token_b_price: i128,
    ) -> Self {
        Self {
            token_a_price,
            token_b_price,
            token_a_price_with_impact,
            mu,
            sigma,
            normal: Normal::new(0., 1.).unwrap(),
            dt,
        }
    }

    pub fn update<R: RngCore>(&mut self, rng: &mut R, price_impact: f64) {
        let z1 = self.normal.sample(rng);

        // We keep the price of token_b constant
        let new_price_a = self.token_a_price.as_f64()
            * f64::exp((self.mu - 0.5 * self.sigma) * self.dt + self.sigma * self.dt.sqrt() * z1);
        let new_price_a_with_impact: f64 = new_price_a + price_impact;
        let new_price_b = self.token_b_price; //self.token_b_price.as_f64() * z2;
        self.token_a_price = new_price_a.as_i128();
        self.token_b_price = new_price_b.as_i128();
        self.token_a_price_with_impact = new_price_a_with_impact.as_i128();
    }

    pub fn get_sqrt_price_token_a_x96(&mut self) -> u128 {
        let n: i32 = 20;
        let price: f64 = f64::sqrt(div_u256(
            U256::from(self.token_a_price_with_impact),
            U256::from(self.token_b_price),
            10,
        )) * 2f64.powi(n);
        let mut price: u128 = price.as_u128();
        // `<<` is a left shift operator, which is equivalent to multiplying times 2^n
        price <<= 96 - n;
        price
    }

    pub fn get_sqrt_price_token_b_x96(&mut self) -> u128 {
        let n: i32 = 20;
        let price: f64 = f64::sqrt(div_u256(
            U256::from(self.token_b_price),
            U256::from(self.token_a_price_with_impact),
            10,
        )) * 2f64.powi(n);
        let mut price: u128 = price.as_u128();
        price <<= 96 - n;
        price
    }

    pub fn get_price_token_a(&mut self) -> f64 {
        let price: f64 = div_u256(
            U256::from(self.token_a_price_with_impact),
            U256::from(self.token_b_price),
            10,
        );
        price
    }
}
