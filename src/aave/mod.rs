mod agents;
mod calls;
mod deployment;
mod fork_initialisation;
mod initialisation;
mod protocol;
pub mod types;

use verbs_rs::env::GasPriorityValidator;
use verbs_rs::sim_runner;

use self::initialisation::initialise_sim;

pub fn aave_sim_from_fork(
    seed: u64,
    n_steps: usize,
    params: types::ForkedSimParameters,
    alchemy_key: String,
) {
    println!("Initialising Simulation");

    let validator = GasPriorityValidator {};

    let (mut env, mut agent_sets, _, _, _) =
        fork_initialisation::initialise_sim(params, alchemy_key, validator);

    println!("Running");

    sim_runner::run(&mut env, &mut agent_sets, seed, n_steps);
}

pub fn aave_sim(seed: u64, n_steps: usize, params: types::SimParameters) {
    let validator = GasPriorityValidator {};
    let (mut env, mut agent_sets, _, _, _) = initialise_sim(params, validator);

    sim_runner::run(&mut env, &mut agent_sets, seed, n_steps);
}
