/// The BPS unit
pub const BPS: u64 = 10_000;

/// The WAD unit
pub const WAD: u64 = 1_000_000_000_000_000_000;

/// The EGLD symbol or ticker
pub const EGLD_SYMBOL: &[u8] = b"EGLD";

/// The USD symbol or ticker
pub const USD_SYMBOL: &[u8] = b"USD";

/// The USDC token identifier on mainnet
pub const USDC_TOKEN_ID_M: &[u8] = b"USDC-c76f1f";

/// The USDC token identifier on devnet
pub const USDC_TOKEN_ID_D: &[u8] = b"USDC-350c4e";

/// The USDT token identifier on mainnet
pub const USDT_TOKEN_ID_M: &[u8] = b"USDT-f8c08c";

/// The USDT token identifier on devnet
pub const USDT_TOKEN_ID_D: &[u8] = b"USDT-58d5d0";

/// The minimum first anchor tolerance allowed (0.25%)
pub const MIN_FIRST_ANCHOR_TOLERANCE: u64 = 2_500_000_000_000_000;

/// The maximum first anchor tolerance allowed (50%)
pub const MAX_FIRST_ANCHOR_TOLERANCE: u64 = 500_000_000_000_000_000;

/// The minimum last anchor tolerance allowed (1%)
pub const MIN_LAST_ANCHOR_TOLERANCE: u64 = 10_000_000_000_000_000;

/// The maximum last anchor tolerance allowed (100%)
pub const MAX_LAST_ANCHOR_TOLERANCE: u64 = 1_000_000_000_000_000_000;
