/// The BPS unit
pub const BPS: u64 = 10_000;

/// The WAD unit
pub const WAD: u64 = 1_000_000_000_000_000_000;

/// The USH decimals
pub const USH_DECIMALS: usize = 18;

/// The Hatom USH name
pub const HUSH_NAME: &str = "HatomUSH";

/// The Hatom USH ticker
pub const HUSH_TICKER: &str = "HUSH";

/// The Hatom USH decimals
pub const HUSH_DECIMALS: usize = 8;

/// The exchange rate between USH and Hatom USH
pub const EXCHANGE_RATE: u128 = (10 as u128).pow(USH_DECIMALS as u32) * WAD as u128 / (10 as u128).pow(HUSH_DECIMALS as u32);

/// The maximum accrual time threshold allowed (1 day)
pub const MAX_ACCRUAL_TIME_THRESHOLD: u64 = 86400;

/// The amount of seconds in a year
pub const SECONDS_PER_YEAR: u64 = 31_556_926;

/// The maximum initial borrow rate allowed in wad (100% APR)
pub const MAX_INITIAL_BORROW_RATE: u64 = WAD / SECONDS_PER_YEAR;

/// The minimum time that has to elapse between borrow rate updates (1 day)
pub const BORROW_RATE_DELAY: u64 = 86400;

/// The maximum borrow rate change allowed in bps (10%)
pub const MAX_BORROW_RATE_CHANGE: u64 = 1_000;

/// The minimum close factor allowed (20%)
pub const MIN_CLOSE_FACTOR: u64 = 200_000_000_000_000_000;

/// The minimum liquidation incentive allowed (101%)
pub const MIN_LIQUIDATION_INCENTIVE: u64 = 1_010_000_000_000_000_000;
