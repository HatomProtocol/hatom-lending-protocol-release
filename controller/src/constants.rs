/// The WAD unit
pub const WAD: u64 = 1_000_000_000_000_000_000;

/// The maximum collateral factor allowed (90%)
pub const MAX_COLLATERAL_FACTOR: u64 = 900_000_000_000_000_000;

/// The maximum number of markets an account can enter
pub const MAX_MARKETS_PER_ACCOUNT: usize = 8;

/// The maximum number of rewards batches per money market
pub const MAX_REWARDS_BATCHES: usize = 3;

/// The minimum percentage of rewards that must be distributed before removing a batch
pub const MIN_REWARDS_BATCH_TOLERANCE: u64 = 950_000_000_000_000_000;

/// The maximum slippage for configuration swaps
pub const MAX_SLIPPAGE: u64 = 100_000_000_000_000_000;

/// The maximum premium for boosting rewards
pub const MAX_PREMIUM: u64 = 100_000_000_000_000_000;

/// The required time delay for collateral factor decreases (1 day)
pub const TIMELOCK_COLLATERAL_FACTOR_DECREASE: u64 = 1 * 24 * 60 * 60;

/// The maximum decrease on collateral factor allowed (10%)
pub const MAX_COLLATERAL_FACTOR_DECREASE: u64 = 100_000_000_000_000_000;
