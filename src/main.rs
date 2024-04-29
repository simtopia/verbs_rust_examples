use aave::SimData;
use clap::Parser;
use kdam::TqdmParallelIterator;
use rayon::prelude::*;

mod aave;

use std::fs;

#[derive(Parser, Debug)]
#[command(about, long_about = None)]
struct Args {
    /// Random seed
    #[arg(long)]
    n_seeds: u64,
    /// Number of simulation steps
    #[arg(long)]
    n_steps: usize,
    /// Flag to run simulation from a live fork
    #[arg(long)]
    fork: bool,
    /// Optional Alchemy API key if running from fork
    #[arg(long)]
    key: Option<String>,
}

fn main() {
    let args = Args::parse();

    let seeds = Vec::from_iter(10..10 + args.n_seeds);
    let n_steps = args.n_steps;

    let results: Vec<SimData> = match args.fork {
        true => match args.key {
            Some(k) => {
                let params = aave::types::ForkedSimParameters {
                    n_borrowers: 10,
                    n_liquidators: 1,
                    prices_mu: 0f64,
                    prices_dt: 0.01f64,
                    prices_sigma: 0.4f64,
                    borrow_activation_rate: 0.1f64,
                    adversarial: false,
                    uniswap_fee: 500u32,
                    block_number: 18564279u64,
                };

                seeds
                    .par_iter()
                    .map(|i| {
                        let k = k.clone();
                        aave::aave_sim_from_fork(*i, n_steps, params, k)
                    })
                    .collect()
            }
            None => panic!("Alchemy key argument required for forked simulation"),
        },
        false => {
            let params = aave::types::SimParameters {
                n_borrowers: 10,
                n_liquidators: 1,
                prices_mu: 0f64,
                prices_dt: 0.01f64,
                prices_sigma: 0.3f64,
                borrow_activation_rate: 0.1f64,
                token_a_initial_price: 100000000000i128,
                token_b_initial_price: 100000000i128,
                token_a_liquidation_threshold: 8000u128,
                token_b_liquidation_threshold: 8500u128,
                token_a_base_ltv: 7500u128,
                token_b_base_ltv: 8000u128,
                liquidity: 10_f64.powf(5.),
                adversarial: false,
                uniswap_fee: 500u32,
            };
            seeds
                .par_iter()
                .tqdm()
                .map(|i| aave::aave_sim(*i, n_steps, params))
                .collect()
        }
    };
    let json = serde_json::to_string(&results).expect("Could not serialise to json string");
    let _ = fs::write("sim_dat.txt", json);
}
