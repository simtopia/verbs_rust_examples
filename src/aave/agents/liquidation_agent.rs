use crate::aave::calls;
use crate::aave::protocol::{aave_abi, uniswap_abi};
use crate::aave::types;
use alloy_primitives::{Address, Log, Uint, U256};
use alloy_sol_types::SolEvent;
use rand::Rng;
use std::mem;
use verbs_rs::agent::{Agent, RecordedAgent};
use verbs_rs::contract::Transaction;
use verbs_rs::env::{Env, RevertError, Validator};
use verbs_rs::utils::{div_u256, scale_data_value, Eth};
use verbs_rs::DB;

pub struct LiquidationAgent {
    address: Address,
    pool_address: Address,
    oracle_address: Address,
    collateral_token_address: Address,
    debt_token_address: Address,
    liquidation_addresses: Vec<Address>,
    current_user_data: types::UserData,
    adversarial: bool,
    uniswap_pool: Address,
    quoter: Address,
    swap_router: Address,
    uniswap_fee: u32,
    balance_debt_asset: Vec<U256>,
    balance_collateral_asset: Vec<U256>,
    step: i16,
}

fn scale_data(
    x: &(
        Address,
        aave_abi::Pool_Implementation::getUserAccountDataReturn,
    ),
) -> types::UserRecord {
    (
        scale_data_value(x.1.totalCollateralBase, 0, 0),
        scale_data_value(x.1.totalDebtBase, 0, 0),
        scale_data_value(x.1.availableBorrowsBase, 0, 0),
        scale_data_value(x.1.currentLiquidationThreshold, 0, 0),
        scale_data_value(x.1.ltv, 0, 0),
        scale_data_value(x.1.healthFactor, 18, 12),
    )
}

impl LiquidationAgent {
    pub fn new(
        idx: usize,
        pool_address: Address,
        oracle_address: Address,
        collateral_token_address: Address,
        debt_token_address: Address,
        liquidation_addresses: Vec<Address>,
        adversarial: bool,
        uniswap_pool: Address,
        quoter: Address,
        swap_router: Address,
        uniswap_fee: u32,
    ) -> Self {
        let address = Address::from(Uint::from(idx));

        LiquidationAgent {
            address,
            pool_address,
            oracle_address,
            collateral_token_address,
            debt_token_address,
            liquidation_addresses,
            current_user_data: Vec::new(),
            adversarial,
            uniswap_pool,
            quoter,
            swap_router,
            uniswap_fee,
            balance_debt_asset: Vec::new(),
            balance_collateral_asset: Vec::new(),
            step: 0i16,
        }
    }

    fn accountability<D, V>(&self, network: &mut Env<D, V>, user: Address, amount: U256) -> bool
    where
        D: DB,
        V: Validator,
    {
        let call_result: Result<
            (
                aave_abi::Pool_Implementation::liquidationCallReturn,
                Vec<Log>,
            ),
            RevertError,
        > = network.direct_call(
            self.address,
            self.pool_address,
            aave_abi::Pool_Implementation::liquidationCallCall {
                collateralAsset: self.collateral_token_address,
                debtAsset: self.debt_token_address,
                user,
                debtToCover: amount,
                receiveAToken: false,
            },
            U256::ZERO,
        );

        match call_result {
            Ok((_, events)) => {
                let event = events.last().unwrap().to_owned();

                let decoded_event =
                    aave_abi::Pool_Implementation::LiquidationCall::decode_log(&event, false)
                        .unwrap();

                // close short position in the external market
                let debt_to_cover = decoded_event.debtToCover;
                let liquidated_collateral_amount = decoded_event.liquidatedCollateralAmount;

                let quote = calls::quote_v2_exact_output_swap(
                    network,
                    self.address,
                    self.collateral_token_address,
                    self.debt_token_address,
                    self.uniswap_fee,
                    debt_to_cover,
                    self.quoter,
                );

                let amount_collateral_from_swap = match quote {
                    Some(q) => q.amountIn,
                    None => liquidated_collateral_amount,
                };
                liquidated_collateral_amount > amount_collateral_from_swap
            }
            // In the case the call is reverted we continue without liquidation
            Err(_) => false,
        }
    }

    fn _update<D, V, R>(&mut self, _rng: &mut R, network: &mut Env<D, V>) -> Vec<Transaction>
    where
        D: DB,
        V: Validator,
        R: Rng,
    {
        // Get current balance of the liquidator in the collateral asset and the debt asset
        let current_balance_collateral_asset =
            calls::balance_of(network, self.address, self.collateral_token_address);
        let current_balance_debt_asset =
            calls::balance_of(network, self.address, self.debt_token_address);

        // TODO: Can calculate amount to cover using this
        //  https://docs.aave.com/developers/guides/liquidations#executing-the-liquidation-call
        let user_data: Vec<(
            Address,
            aave_abi::Pool_Implementation::getUserAccountDataReturn,
        )> = self
            .liquidation_addresses
            .iter()
            .map(|x| {
                (
                    *x,
                    calls::get_user_data(network, Address::ZERO, self.pool_address, *x),
                )
            })
            .collect();

        self.current_user_data = user_data.iter().map(scale_data).collect();

        let risky_factors: Vec<(Address, U256)> = user_data
            .into_iter()
            .map(|x| (x.0, x.1.healthFactor))
            .filter(|x| x.1 < U256::to_weth(1u128))
            .collect();

        let profits: Vec<(Address, bool)> = risky_factors
            .into_iter()
            .map(|(x, _)| (x, self.accountability(network, x, U256::MAX)))
            .filter(|(_, y)| *y)
            .collect();

        // liquidation calls
        let mut calls: Vec<Transaction> = profits
            .into_iter()
            .map(|(x, _)| {
                calls::liquidation_call(
                    self.collateral_token_address,
                    self.debt_token_address,
                    x,
                    self.pool_address,
                    self.address,
                    U256::MAX,
                )
            })
            .collect();
        // Create Uniswap swap calls
        if self.step > 0 {
            // Check if liquidator has open short position
            if self
                .balance_debt_asset
                .last()
                .unwrap()
                .gt(&current_balance_debt_asset)
            {
                let debt = self.balance_debt_asset.last().unwrap() - current_balance_debt_asset;

                let swap_call = calls::uniswap_swap_call_exact_output(
                    self.address,
                    self.swap_router,
                    uniswap_abi::SwapRouter::ExactOutputSingleParams {
                        amountInMaximum: current_balance_collateral_asset,
                        amountOut: debt,
                        deadline: U256::MAX,
                        fee: self.uniswap_fee,
                        recipient: self.address,
                        sqrtPriceLimitX96: U256::ZERO,
                        tokenIn: self.collateral_token_address,
                        tokenOut: self.debt_token_address,
                    },
                );
                calls.push(swap_call);
            }
        }
        // Update running values
        self.balance_collateral_asset
            .push(current_balance_collateral_asset);
        self.balance_debt_asset.push(current_balance_debt_asset);
        self.step += 1_i16;

        calls
    }

    fn _update_adversarial<D, V, R>(
        &mut self,
        _rng: &mut R,
        network: &mut Env<D, V>,
    ) -> Vec<Transaction>
    where
        D: DB,
        V: Validator,
        R: Rng,
    {
        // Get current balacnce of the liquidator in the collateral asset and the debt asset
        let current_balance_collateral_asset =
            calls::balance_of(network, self.address, self.collateral_token_address);
        let current_balance_debt_asset =
            calls::balance_of(network, self.address, self.debt_token_address);
        // Decimals debt asset necessary for internal calculations
        let decimals_debt_asset =
            calls::get_decimals(network, self.address, self.debt_token_address);
        // TODO: Can calculate amount to cover using this
        //  https://docs.aave.com/developers/guides/liquidations#executing-the-liquidation-call
        let user_data: Vec<(
            Address,
            aave_abi::Pool_Implementation::getUserAccountDataReturn,
        )> = self
            .liquidation_addresses
            .iter()
            .map(|x| {
                (
                    *x,
                    calls::get_user_data(network, Address::ZERO, self.pool_address, *x),
                )
            })
            .collect();

        self.current_user_data = user_data.iter().map(scale_data).collect();

        // we get the prices of collateral in terms of debt
        let debt_asset_price = calls::get_asset_price(
            network,
            Address::ZERO,
            self.oracle_address,
            self.debt_token_address,
        );

        // We get the open positions that can be either:
        // - liquidated right now (their HF < 1)
        // - liquidated after the right price manipulation
        let current_sqrt_price_x96 =
            calls::get_slot0(network, self.address, self.uniswap_pool).sqrtPriceX96;
        let adversarial_liquidations: Vec<(Address, U256, U256)> = user_data
            .into_iter()
            .map(|x| (x.0, x.1.totalDebtBase, x.1.healthFactor))
            .filter(|x| {
                // We get the addresses and the debt amounts for the adversarial liquidations
                // x.1 is the debt of the user in the base currency.
                //  - we convert it to the actual number of debt tokens.
                //  - x.1 and debt_asset_price have the same number of decimals, so we do not need to re-scale anything
                // We add the number of decimals that the debt token has, in order to interact with Uniswap.
                let debt_to_cover: U256 = x.1 * U256::from(10).pow(decimals_debt_asset)
                    / (U256::from(2) * debt_asset_price);
                debt_to_cover.gt(&U256::ZERO)
            })
            .filter(|x| {
                // We recalculate the debt_to_cover
                // TODO: avoid calculating it twice
                let debt_to_cover: U256 = x.1 * U256::from(10).pow(decimals_debt_asset)
                    / (U256::from(2) * debt_asset_price);
                // we calculate the upper bound of HF such that adversarial liquidation is profitable.
                // See https://papers.ssrn.com/sol3/papers.cfm?abstract_id=4540333

                let quote = calls::quote_v2_exact_output_swap(
                    network,
                    self.address,
                    self.collateral_token_address,
                    self.debt_token_address,
                    self.uniswap_fee,
                    debt_to_cover,
                    self.quoter,
                );
                let sqrt_price_after_swap_x96 = match quote {
                    Some(q) => q.sqrtPriceX96After,
                    None => current_sqrt_price_x96,
                };
                let sqrt_upper_bound_hf =
                    div_u256(current_sqrt_price_x96, sqrt_price_after_swap_x96, 5);
                let upper_bound_hf = sqrt_upper_bound_hf.powi(2i32);

                // We filter the positions such that their hf < upper_bound_hf
                scale_data_value(x.2, 18, 12) < upper_bound_hf
            })
            .map(|x| (x.0, x.1, x.2))
            .collect();

        // // Liquidation
        let mut calls: Vec<Transaction> = adversarial_liquidations
            .iter()
            .filter(|x| x.2 < U256::to_weth(1u128))
            .map(|x| {
                calls::liquidation_call(
                    self.collateral_token_address,
                    self.debt_token_address,
                    x.0,
                    self.pool_address,
                    self.address,
                    U256::MAX,
                )
            })
            .collect();

        // Front-run trades - Price manipulation
        let trade_size_price_manipulation = adversarial_liquidations
            .iter()
            .filter(|x| x.2 > U256::to_weth(1u128))
            .map(|x| {
                // x.1 is the debt of the user in the base currency.
                // we convert it to the actual number of debt tokens
                // x.1 and debt_asset_price have the same number of decimals, so we do not need to re-scale anything
                let debt_to_cover: U256 = x.1 * U256::from(10).pow(decimals_debt_asset)
                    / (U256::from(2) * debt_asset_price);
                debt_to_cover
            })
            .sum();

        if trade_size_price_manipulation > U256::ZERO {
            let swap_call = calls::uniswap_swap_call_exact_output(
                self.address,
                self.swap_router,
                uniswap_abi::SwapRouter::ExactOutputSingleParams {
                    amountInMaximum: current_balance_collateral_asset,
                    amountOut: trade_size_price_manipulation,
                    deadline: U256::MAX,
                    fee: self.uniswap_fee,
                    recipient: self.address,
                    sqrtPriceLimitX96: U256::ZERO,
                    tokenIn: self.collateral_token_address,
                    tokenOut: self.debt_token_address,
                },
            );
            calls.push(swap_call);
        }

        // Update running values
        self.balance_collateral_asset
            .push(current_balance_collateral_asset);
        self.balance_debt_asset.push(current_balance_debt_asset);
        self.step += 1_i16;

        calls
    }
}

impl Agent for LiquidationAgent {
    fn update<D, V, R>(&mut self, _rng: &mut R, network: &mut Env<D, V>) -> Vec<Transaction>
    where
        D: DB,
        V: Validator,
        R: Rng,
    {
        if self.adversarial {
            self._update_adversarial(_rng, network)
        } else {
            self._update(_rng, network)
        }
    }

    fn get_address(&self) -> Address {
        self.address
    }
}

impl RecordedAgent<types::UserData> for LiquidationAgent {
    fn record<D: DB, V: Validator>(&mut self, _env: &mut Env<D, V>) -> types::UserData {
        mem::take(&mut self.current_user_data)
    }
}
