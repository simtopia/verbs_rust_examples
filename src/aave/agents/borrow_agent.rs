use crate::aave::calls;

use alloy_primitives::{Address, Uint, U256};
use rand::Rng;
use verbs_rs::agent::{Agent, RecordedAgent};
use verbs_rs::contract::Transaction;
use verbs_rs::env::{Env, Validator};
use verbs_rs::DB;

pub struct BorrowAgent {
    address: Address,
    activation_rate: f64,
    borrow_token_decimals: U256,
    _borrow_token_ltv: U256,
    has_supplied: bool,
    has_borrowed: bool,
    pool_address: Address,
    oracle_address: Address,
    supply_token_address: Address,
    borrow_token_address: Address,
}

impl BorrowAgent {
    pub fn new(
        idx: usize,
        activation_rate: f64,
        borrow_token_ltv: U256,
        borrow_token_decimals: U256,
        pool_address: Address,
        oracle_address: Address,
        supply_token_address: Address,
        borrow_token_address: Address,
    ) -> Self {
        let address = Address::from(Uint::from(idx));

        BorrowAgent {
            address,
            activation_rate,
            _borrow_token_ltv: borrow_token_ltv,
            borrow_token_decimals,
            has_supplied: false,
            has_borrowed: false,
            pool_address,
            oracle_address,
            supply_token_address,
            borrow_token_address,
        }
    }
}

impl Agent for BorrowAgent {
    fn update<D, V, R>(&mut self, rng: &mut R, network: &mut Env<D, V>) -> Vec<Transaction>
    where
        D: DB,
        V: Validator,
        R: Rng,
    {
        if rng.gen::<f64>() < self.activation_rate {
            if !self.has_supplied {
                let supply_call = calls::supply_call(
                    self.address,
                    self.pool_address,
                    self.supply_token_address,
                    U256::from(10_u128.pow(20)),
                );
                self.has_supplied = true;

                vec![supply_call]
            } else if !self.has_borrowed {
                let user_data =
                    calls::get_user_data(network, Address::ZERO, self.pool_address, self.address);
                let available_borrow_base = user_data.availableBorrowsBase;
                let borrow_asset_price = calls::get_asset_price(
                    network,
                    Address::ZERO,
                    self.oracle_address,
                    self.borrow_token_address,
                );

                let exp = U256::from(10u128).pow(self.borrow_token_decimals - U256::from(4u128));
                // Agent borrows a random fraction of the available to borrow amount
                let u = U256::from(rng.gen_range(9000..10000));
                let available_borrow = exp * available_borrow_base * u / borrow_asset_price;

                if available_borrow > U256::ZERO {
                    let borrow_call = calls::borrow_call(
                        self.address,
                        self.pool_address,
                        self.borrow_token_address,
                        available_borrow,
                    );
                    self.has_borrowed = true;
                    vec![borrow_call]
                } else {
                    Vec::default()
                }
            } else {
                Vec::default()
            }
        } else {
            Vec::default()
        }
    }

    fn get_address(&self) -> Address {
        self.address
    }
}

impl RecordedAgent<U256> for BorrowAgent {
    fn record<D: DB, V: Validator>(&mut self, _env: &mut Env<D, V>) -> U256 {
        U256::ZERO
    }
}
