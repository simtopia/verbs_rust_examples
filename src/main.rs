use aave::aave_sim;
use clap::Parser;
mod aave;


#[derive(Parser, Debug)]
#[command(about, long_about = None)]
struct Args {
    /// Random seed
    #[arg(long)]
    seed: u64,
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

    let seed = args.seed;
    let n_steps = args.n_steps;

    let results = match args.fork {
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

                aave::aave_sim_from_fork(seed, n_steps, params, k)
            }
            None => panic!("Alchemy key argument required for forked simulation"),
        },
        false => {
            let params = aave::types::SimParameters {
                n_borrowers: 10,
                n_liquidators: 1,
                prices_mu: 0f64,
                prices_dt: 0.01f64,
                prices_sigma: 0.4f64,
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
            aave_sim(seed, n_steps, params)
        }
    };
}
