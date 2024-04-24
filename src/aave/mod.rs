mod agents;
mod calls;
mod deployment;
mod fork_initialisation;
mod initialisation;
mod protocol;
pub mod types;

use agents::AgentData;
use verbs_rs::env::GasPriorityValidator;
use verbs_rs::{agent::RecordedAgentSet, sim_runner};

use self::initialisation::initialise_sim;

use serde_json::json;

pub fn aave_sim_from_fork(
    seed: u64,
    n_steps: usize,
    params: types::ForkedSimParameters,
    alchemy_key: String,
) -> serde_json::Value {
    println!("Initialising Simulation");

    let validator = GasPriorityValidator {};

    let (mut env, mut agent_sets, _, _, _) =
        fork_initialisation::initialise_sim(params, alchemy_key, validator);

    println!("Running");

    sim_runner::run(&mut env, &mut agent_sets, seed, n_steps);
    let sim_data = AgentData {
        borrow_agents: agent_sets.borrow_agents.take_records(),
        liquidation_agents: agent_sets.liquidation_agents.take_records(),
        uniswap_price_agent: agent_sets.uniswap_price_agent.take_records(),
        uniswap_noise_agents: agent_sets.uniswap_noise_agents.take_records(),
    };
    let sim_data = json!({
        "seed": seed,
        "sim_data": sim_data
    });
    sim_data
}

pub fn aave_sim(seed: u64, n_steps: usize, params: types::SimParameters) -> serde_json::Value {
    let validator = GasPriorityValidator {};
    let (mut env, mut agent_sets, _, _, _) = initialise_sim(params, validator);

    sim_runner::run(&mut env, &mut agent_sets, seed, n_steps);
    let sim_data = AgentData {
        borrow_agents: agent_sets.borrow_agents.take_records(),
        liquidation_agents: agent_sets.liquidation_agents.take_records(),
        uniswap_price_agent: agent_sets.uniswap_price_agent.take_records(),
        uniswap_noise_agents: agent_sets.uniswap_noise_agents.take_records(),
    };
    let sim_data = json!({
        "seed": seed,
        "sim_data": sim_data
    });
    sim_data
}
