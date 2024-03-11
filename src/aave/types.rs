pub type UserRecord = (f64, f64, f64, f64, f64, f64);
pub type UserData = Vec<UserRecord>;

#[derive(Clone, Copy)]
pub struct SimParameters {
    pub n_borrowers: usize,
    pub n_liquidators: usize,
    pub prices_mu: f64,
    pub prices_dt: f64,
    pub prices_sigma: f64,
    pub borrow_activation_rate: f64,
    pub token_a_initial_price: i128,
    pub token_b_initial_price: i128,
    pub token_a_liquidation_threshold: u128,
    pub token_b_liquidation_threshold: u128,
    pub token_a_base_ltv: u128,
    pub token_b_base_ltv: u128,
    pub liquidity: f64,
    pub adversarial: bool,
    pub uniswap_fee: u32,
}

#[derive(Clone, Copy)]
pub struct ForkedSimParameters {
    pub n_borrowers: usize,
    pub n_liquidators: usize,
    pub prices_mu: f64,
    pub prices_dt: f64,
    pub prices_sigma: f64,
    pub borrow_activation_rate: f64,
    pub adversarial: bool,
    pub uniswap_fee: u32,
    pub block_number: u64,
}
