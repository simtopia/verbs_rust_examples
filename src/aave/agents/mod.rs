mod borrow_agent;
mod liquidation_agent;
mod uniswap_agent;
mod uniswap_noise_agent;

use super::types::UserData;
use alloy_primitives::U256;
pub use borrow_agent::BorrowAgent;
pub use liquidation_agent::LiquidationAgent;
use rand::RngCore;
use serde::{Deserialize, Serialize};
pub use uniswap_agent::UniswapPriceAgent;
pub use uniswap_noise_agent::UniswapNoiseAgent;
use verbs_rs::agent::{AgentSet, AgentVec, SimState, SingletonAgent};
use verbs_rs::contract::Transaction;
use verbs_rs::env::{Env, Validator};
use verbs_rs::DB;

#[derive(SimState)]
pub struct AgentStates {
    pub borrow_agents: AgentVec<U256, BorrowAgent>,
    pub liquidation_agents: AgentVec<UserData, LiquidationAgent>,
    pub uniswap_price_agent: SingletonAgent<(i128, i128), UniswapPriceAgent>,
    pub uniswap_noise_agents: AgentVec<U256, UniswapNoiseAgent>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentData {
    pub borrow_agents: Vec<Vec<U256>>,
    pub liquidation_agents: Vec<Vec<UserData>>,
    pub uniswap_price_agent: Vec<(i128, i128)>,
    pub uniswap_noise_agents: Vec<Vec<U256>>,
}
