//! Price Oracle Parameters for Botcash Dynamic Fee Adjustment.
//!
//! This module defines the decentralized price oracle system where miners signal
//! BCASH/USD prices in their block nonces. The aggregated price is used to calculate
//! USD-stable transaction fees, keeping fees affordable regardless of price volatility.
//!
//! These parameters are configurable via on-chain voting (see specs/governance.md).

use crate::amount::COIN;
use crate::block::HeightDiff;

// ============================================================================
// Core Constants
// ============================================================================

/// Target transaction fee in USD.
///
/// Fees are dynamically adjusted to maintain this approximate USD value.
/// At $0.00001 per transaction, Botcash remains essentially free to use.
///
/// Default: $0.00001 USD (one hundred-thousandth of a dollar)
///
/// Stored as nano-USD (1e-9 USD) for precision: 10,000 nano-USD = $0.00001
/// (since 1 nano-USD = $0.000000001, we need 10,000 of them for $0.00001)
pub const TARGET_FEE_NANO_USD: u64 = 10_000;

/// Minimum transaction fee in zatoshis.
///
/// This is the absolute minimum fee regardless of price oracle output.
/// Prevents dust spam even if BCASH price skyrockets.
///
/// Default: 1,000 zatoshis (0.00001 BCASH)
pub const MIN_FEE_ZATOSHIS: u64 = 1_000;

/// Maximum transaction fee in zatoshis.
///
/// This is the absolute maximum fee regardless of price oracle output.
/// Protects users if BCASH price crashes dramatically.
///
/// Default: 1,000,000 zatoshis (0.01 BCASH)
pub const MAX_FEE_ZATOSHIS: u64 = 1_000_000;

/// Number of recent blocks used for price aggregation.
///
/// The oracle uses the median price from the last N blocks to resist manipulation.
/// 100 blocks ≈ 100 minutes at 60-second block time.
///
/// Default: 100 blocks
pub const PRICE_AGGREGATION_BLOCKS: u32 = 100;

/// Minimum number of blocks with valid price signals required for oracle activation.
///
/// If fewer than this many blocks have price signals, the oracle returns None
/// and the system falls back to static fees.
///
/// Default: 51 blocks (majority of aggregation window)
pub const MIN_VALID_PRICE_SIGNALS: u32 = 51;

/// Maximum allowed deviation from median for a price signal to be considered valid.
///
/// Signals that deviate more than this percentage from the current median are
/// rejected as outliers to prevent manipulation.
///
/// Default: 50% (signals must be within 50% of median)
pub const MAX_PRICE_DEVIATION_PERCENT: u8 = 50;

/// Maximum daily fee adjustment rate as a percentage.
///
/// Fees can only change by this percentage per day to prevent sudden shocks.
/// This applies to the overall fee level, not individual transaction fees.
///
/// Default: 10% per day
pub const MAX_DAILY_ADJUSTMENT_PERCENT: u8 = 10;

/// Number of blocks in one day at 60-second block time.
///
/// Used for daily adjustment rate calculations.
pub const BLOCKS_PER_DAY: HeightDiff = 1_440;

// ============================================================================
// Price Signal Encoding
// ============================================================================

/// Magic prefix for price signals in nonce field.
///
/// Miners embed price signals in the first 4 bytes of the 32-byte nonce.
/// Format: [PRICE_SIGNAL_MAGIC (4 bytes)][price_nano_usd (4 bytes)][pow_nonce (24 bytes)]
///
/// The magic bytes "BCPR" (Botcash Price) identify valid price signals.
pub const PRICE_SIGNAL_MAGIC: [u8; 4] = [0x42, 0x43, 0x50, 0x52]; // "BCPR"

/// Size of the price signal data in the nonce (magic + price).
pub const PRICE_SIGNAL_SIZE: usize = 8;

/// Remaining nonce space for PoW after price signal.
pub const POW_NONCE_SIZE: usize = 24;

/// A price signal embedded in a block nonce by a miner.
///
/// Miners voluntarily include BCASH/USD price signals to contribute to the
/// decentralized price oracle. The price is stored in nano-USD (1e-9 USD)
/// to allow for both very small and very large BCASH prices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PriceSignal {
    /// Price in nano-USD (1e-9 USD).
    ///
    /// For example:
    /// - price_nano_usd = 100_000_000 means BCASH = $0.10
    /// - price_nano_usd = 1_000_000_000 means BCASH = $1.00
    /// - price_nano_usd = 10_000_000_000 means BCASH = $10.00
    pub price_nano_usd: u64,
}

impl PriceSignal {
    /// Creates a new price signal from a USD price.
    ///
    /// # Arguments
    /// * `price_usd` - The BCASH/USD price as a float (e.g., 0.15 for $0.15)
    ///
    /// # Returns
    /// A PriceSignal with the price encoded in nano-USD.
    pub fn from_usd(price_usd: f64) -> Self {
        // Convert to nano-USD (multiply by 1e9)
        let price_nano_usd = (price_usd * 1_000_000_000.0) as u64;
        Self { price_nano_usd }
    }

    /// Returns the price in USD as a float.
    pub fn to_usd(&self) -> f64 {
        self.price_nano_usd as f64 / 1_000_000_000.0
    }

    /// Encodes the price signal into 8 bytes for embedding in a nonce.
    ///
    /// Format: [PRICE_SIGNAL_MAGIC (4 bytes)][price_nano_usd as u32 (4 bytes)]
    ///
    /// Note: We use u32 for the wire format to save space, limiting max price
    /// to ~$4.29 at nano-USD precision. For higher prices, we scale down.
    pub fn encode(&self) -> [u8; PRICE_SIGNAL_SIZE] {
        let mut result = [0u8; PRICE_SIGNAL_SIZE];
        result[0..4].copy_from_slice(&PRICE_SIGNAL_MAGIC);

        // Scale price to fit in u32 if needed
        // If price > u32::MAX nano-USD (~$4.29), divide by 1000 and set high bit
        let (scaled_price, is_scaled) = if self.price_nano_usd > u32::MAX as u64 {
            ((self.price_nano_usd / 1000) as u32, true)
        } else {
            (self.price_nano_usd as u32, false)
        };

        // Use little-endian for consistency with other Zcash serialization
        let price_bytes = scaled_price.to_le_bytes();
        result[4..8].copy_from_slice(&price_bytes);

        // Set scaling flag in high bit of last byte if needed
        if is_scaled {
            result[7] |= 0x80;
        }

        result
    }

    /// Attempts to decode a price signal from nonce bytes.
    ///
    /// # Arguments
    /// * `nonce` - The 32-byte nonce from a block header
    ///
    /// # Returns
    /// Some(PriceSignal) if valid price signal found, None otherwise.
    pub fn decode(nonce: &[u8; 32]) -> Option<Self> {
        // Check magic prefix
        if nonce[0..4] != PRICE_SIGNAL_MAGIC {
            return None;
        }

        // Extract price bytes
        let mut price_bytes = [0u8; 4];
        price_bytes.copy_from_slice(&nonce[4..8]);

        // Check scaling flag
        let is_scaled = (price_bytes[3] & 0x80) != 0;
        price_bytes[3] &= 0x7F; // Clear flag for decoding

        let scaled_price = u32::from_le_bytes(price_bytes) as u64;

        let price_nano_usd = if is_scaled {
            scaled_price * 1000
        } else {
            scaled_price
        };

        // Reject zero prices as invalid
        if price_nano_usd == 0 {
            return None;
        }

        Some(Self { price_nano_usd })
    }

    /// Creates a 32-byte nonce with the price signal embedded.
    ///
    /// The first 8 bytes contain the price signal, the remaining 24 bytes
    /// can be used for PoW nonce searching.
    ///
    /// # Arguments
    /// * `pow_nonce` - The 24 bytes to use for PoW searching
    pub fn to_nonce(&self, pow_nonce: &[u8; POW_NONCE_SIZE]) -> [u8; 32] {
        let mut nonce = [0u8; 32];
        let signal = self.encode();
        nonce[0..PRICE_SIGNAL_SIZE].copy_from_slice(&signal);
        nonce[PRICE_SIGNAL_SIZE..32].copy_from_slice(pow_nonce);
        nonce
    }
}

// ============================================================================
// Price Aggregation
// ============================================================================

/// Aggregated price oracle result from multiple block signals.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OraclePrice {
    /// Median price in nano-USD from valid signals.
    pub median_price_nano_usd: u64,

    /// Number of valid price signals used.
    pub signal_count: u32,

    /// Height of the most recent block in the aggregation window.
    pub as_of_height: u32,
}

impl OraclePrice {
    /// Returns the median price in USD.
    pub fn to_usd(&self) -> f64 {
        self.median_price_nano_usd as f64 / 1_000_000_000.0
    }

    /// Calculates the dynamic fee in zatoshis based on this oracle price.
    ///
    /// fee = target_fee_usd / bcash_price_usd
    /// fee = (target_nano_usd / 1e9) / (price_nano_usd / 1e9)
    /// fee = target_nano_usd / price_nano_usd (in BCASH)
    /// fee_zatoshis = (target_nano_usd / price_nano_usd) * COIN
    pub fn calculate_fee(&self) -> u64 {
        if self.median_price_nano_usd == 0 {
            return MIN_FEE_ZATOSHIS;
        }

        // Calculate fee: (target_fee / price) * COIN
        // Use u128 for intermediate calculation to avoid overflow
        let fee = (TARGET_FEE_NANO_USD as u128 * COIN as u128)
            / self.median_price_nano_usd as u128;

        let fee = fee as u64;

        // Apply bounds
        fee.clamp(MIN_FEE_ZATOSHIS, MAX_FEE_ZATOSHIS)
    }
}

/// Calculates the median of a slice of u64 values.
///
/// Returns None if the slice is empty.
pub fn calculate_median(values: &[u64]) -> Option<u64> {
    if values.is_empty() {
        return None;
    }

    let mut sorted = values.to_vec();
    sorted.sort_unstable();

    let len = sorted.len();
    if len % 2 == 0 {
        // Even number of elements: average of two middle values
        Some((sorted[len / 2 - 1] + sorted[len / 2]) / 2)
    } else {
        // Odd number of elements: middle value
        Some(sorted[len / 2])
    }
}

/// Filters price signals to remove outliers.
///
/// Signals that deviate more than `max_deviation_percent` from the median
/// are considered outliers and removed.
///
/// # Arguments
/// * `prices` - Raw price signals in nano-USD
/// * `max_deviation_percent` - Maximum allowed deviation from median
///
/// # Returns
/// Filtered list of valid prices.
pub fn filter_outliers(prices: &[u64], max_deviation_percent: u8) -> Vec<u64> {
    if prices.is_empty() {
        return Vec::new();
    }

    // Calculate initial median
    let median = match calculate_median(prices) {
        Some(m) => m,
        None => return Vec::new(),
    };

    if median == 0 {
        return Vec::new();
    }

    // Filter outliers
    let max_deviation = max_deviation_percent as u64;
    prices
        .iter()
        .copied()
        .filter(|&price| {
            // Calculate deviation as percentage
            let deviation = if price > median {
                ((price - median) * 100) / median
            } else {
                ((median - price) * 100) / median
            };
            deviation <= max_deviation
        })
        .collect()
}

/// Aggregates price signals from multiple blocks into an oracle price.
///
/// # Arguments
/// * `signals` - Price signals from recent blocks (newest first)
/// * `as_of_height` - Height of the most recent block
///
/// # Returns
/// Some(OraclePrice) if enough valid signals, None otherwise.
pub fn aggregate_prices(signals: &[PriceSignal], as_of_height: u32) -> Option<OraclePrice> {
    if signals.len() < MIN_VALID_PRICE_SIGNALS as usize {
        return None;
    }

    // Extract prices
    let prices: Vec<u64> = signals.iter().map(|s| s.price_nano_usd).collect();

    // Filter outliers
    let filtered = filter_outliers(&prices, MAX_PRICE_DEVIATION_PERCENT);

    // Check minimum signals after filtering
    if filtered.len() < MIN_VALID_PRICE_SIGNALS as usize {
        return None;
    }

    // Calculate median
    let median = calculate_median(&filtered)?;

    Some(OraclePrice {
        median_price_nano_usd: median,
        signal_count: filtered.len() as u32,
        as_of_height,
    })
}

// ============================================================================
// Parameter Bounds
// ============================================================================

/// Price oracle parameter bounds for validation.
///
/// These define the acceptable ranges for governance-adjustable parameters.
pub mod bounds {
    #![allow(unused_imports)]
    use super::*;

    /// Minimum target fee in nano-USD ($0.000001 = 1,000 nano-USD)
    pub const MIN_TARGET_FEE_NANO_USD: u64 = 1_000;
    /// Maximum target fee in nano-USD ($0.001 = 1,000,000,000 nano-USD)
    pub const MAX_TARGET_FEE_NANO_USD: u64 = 1_000_000_000;

    /// Minimum fee floor in zatoshis (100 = 0.000001 BCASH)
    pub const MIN_FEE_FLOOR_ZATOSHIS: u64 = 100;
    /// Maximum fee floor in zatoshis (100,000 = 0.001 BCASH)
    pub const MAX_FEE_FLOOR_ZATOSHIS: u64 = 100_000;

    /// Minimum fee ceiling in zatoshis (100,000 = 0.001 BCASH)
    pub const MIN_FEE_CEILING_ZATOSHIS: u64 = 100_000;
    /// Maximum fee ceiling in zatoshis (10,000,000 = 0.1 BCASH)
    pub const MAX_FEE_CEILING_ZATOSHIS: u64 = 10_000_000;

    /// Minimum price aggregation window (10 blocks)
    pub const MIN_AGGREGATION_BLOCKS: u32 = 10;
    /// Maximum price aggregation window (1000 blocks)
    pub const MAX_AGGREGATION_BLOCKS: u32 = 1_000;

    /// Minimum valid signals required (must be > 50% of window)
    pub const MIN_VALID_SIGNALS: u32 = 6; // For 10 block minimum
    /// Maximum valid signals required (can't exceed window)
    pub const MAX_VALID_SIGNALS: u32 = 1_000;

    /// Minimum max deviation (10%)
    pub const MIN_MAX_DEVIATION_PERCENT: u8 = 10;
    /// Maximum max deviation (90%)
    pub const MAX_MAX_DEVIATION_PERCENT: u8 = 90;

    /// Minimum daily adjustment rate (1%)
    pub const MIN_DAILY_ADJUSTMENT_PERCENT: u8 = 1;
    /// Maximum daily adjustment rate (50%)
    pub const MAX_DAILY_ADJUSTMENT_PERCENT: u8 = 50;

    /// Validates that the target fee is within bounds
    pub fn validate_target_fee(fee: u64) -> bool {
        fee >= MIN_TARGET_FEE_NANO_USD && fee <= MAX_TARGET_FEE_NANO_USD
    }

    /// Validates that the fee floor is within bounds
    pub fn validate_fee_floor(fee: u64) -> bool {
        fee >= MIN_FEE_FLOOR_ZATOSHIS && fee <= MAX_FEE_FLOOR_ZATOSHIS
    }

    /// Validates that the fee ceiling is within bounds
    pub fn validate_fee_ceiling(fee: u64) -> bool {
        fee >= MIN_FEE_CEILING_ZATOSHIS && fee <= MAX_FEE_CEILING_ZATOSHIS
    }

    /// Validates that the aggregation window is within bounds
    pub fn validate_aggregation_blocks(blocks: u32) -> bool {
        blocks >= MIN_AGGREGATION_BLOCKS && blocks <= MAX_AGGREGATION_BLOCKS
    }

    /// Validates that the minimum valid signals is within bounds
    pub fn validate_min_valid_signals(signals: u32, window: u32) -> bool {
        signals >= MIN_VALID_SIGNALS
            && signals <= MAX_VALID_SIGNALS
            && signals <= window
            && signals > window / 2 // Must be majority
    }

    /// Validates that the max deviation percentage is within bounds
    pub fn validate_max_deviation(percent: u8) -> bool {
        percent >= MIN_MAX_DEVIATION_PERCENT && percent <= MAX_MAX_DEVIATION_PERCENT
    }

    /// Validates that the daily adjustment rate is within bounds
    pub fn validate_daily_adjustment(percent: u8) -> bool {
        percent >= MIN_DAILY_ADJUSTMENT_PERCENT && percent <= MAX_DAILY_ADJUSTMENT_PERCENT
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// A zero-sized struct for accessing oracle utility functions.
pub struct OracleParams;

impl OracleParams {
    /// Calculates the dynamic fee for a given BCASH/USD price.
    ///
    /// # Arguments
    /// * `price_usd` - Current BCASH price in USD
    ///
    /// # Returns
    /// Fee in zatoshis, clamped to bounds.
    pub fn calculate_fee_for_price(price_usd: f64) -> u64 {
        if price_usd <= 0.0 {
            return MIN_FEE_ZATOSHIS;
        }

        // target_fee_usd = TARGET_FEE_NANO_USD / 1e9
        // fee_bcash = target_fee_usd / price_usd
        // fee_zatoshis = fee_bcash * COIN
        let target_fee_usd = TARGET_FEE_NANO_USD as f64 / 1_000_000_000.0;
        let fee_bcash = target_fee_usd / price_usd;
        let fee_zatoshis = (fee_bcash * COIN as f64) as u64;

        fee_zatoshis.clamp(MIN_FEE_ZATOSHIS, MAX_FEE_ZATOSHIS)
    }

    /// Checks if a nonce contains a valid price signal.
    pub fn has_price_signal(nonce: &[u8; 32]) -> bool {
        nonce[0..4] == PRICE_SIGNAL_MAGIC
    }

    /// Calculates the maximum fee change allowed between two heights.
    ///
    /// Based on MAX_DAILY_ADJUSTMENT_PERCENT and the number of blocks elapsed.
    ///
    /// # Arguments
    /// * `current_fee` - The current fee in zatoshis
    /// * `blocks_elapsed` - Number of blocks since last adjustment
    ///
    /// # Returns
    /// Maximum allowed fee change in zatoshis.
    pub fn max_fee_change(current_fee: u64, blocks_elapsed: HeightDiff) -> u64 {
        // Daily rate as fraction of fee
        let daily_rate = MAX_DAILY_ADJUSTMENT_PERCENT as f64 / 100.0;

        // Scale by blocks elapsed
        let block_rate = daily_rate * (blocks_elapsed as f64 / BLOCKS_PER_DAY as f64);

        (current_fee as f64 * block_rate) as u64
    }

    /// Applies rate limiting to a fee change.
    ///
    /// # Arguments
    /// * `old_fee` - Previous fee in zatoshis
    /// * `new_fee` - Proposed new fee in zatoshis
    /// * `blocks_elapsed` - Blocks since last fee update
    ///
    /// # Returns
    /// Rate-limited fee in zatoshis.
    pub fn rate_limit_fee(old_fee: u64, new_fee: u64, blocks_elapsed: HeightDiff) -> u64 {
        let max_change = Self::max_fee_change(old_fee, blocks_elapsed);

        if new_fee > old_fee {
            let change = new_fee - old_fee;
            if change > max_change {
                old_fee.saturating_add(max_change)
            } else {
                new_fee
            }
        } else {
            let change = old_fee - new_fee;
            if change > max_change {
                old_fee.saturating_sub(max_change)
            } else {
                new_fee
            }
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests that all default oracle parameters have expected values.
    #[test]
    fn test_oracle_params_default() {
        let _init_guard = zebra_test::init();

        // Verify default parameter values
        assert_eq!(TARGET_FEE_NANO_USD, 10_000, "Target fee should be $0.00001 (10,000 nano-USD)");
        assert_eq!(MIN_FEE_ZATOSHIS, 1_000, "Min fee should be 0.00001 BCASH");
        assert_eq!(MAX_FEE_ZATOSHIS, 1_000_000, "Max fee should be 0.01 BCASH");
        assert_eq!(PRICE_AGGREGATION_BLOCKS, 100, "Should aggregate 100 blocks");
        assert_eq!(MIN_VALID_PRICE_SIGNALS, 51, "Need majority (51) of signals");
        assert_eq!(MAX_PRICE_DEVIATION_PERCENT, 50, "Max deviation should be 50%");
        assert_eq!(MAX_DAILY_ADJUSTMENT_PERCENT, 10, "Max daily adjustment should be 10%");
        assert_eq!(BLOCKS_PER_DAY, 1_440, "1 day = 1,440 blocks at 60s");
    }

    /// Tests price signal encoding and decoding.
    #[test]
    fn test_price_signal_roundtrip() {
        let _init_guard = zebra_test::init();

        // Test various price levels
        let test_prices = [
            0.001,   // Very low price
            0.10,    // 10 cents
            0.50,    // 50 cents
            1.00,    // $1
            10.00,   // $10
            100.00,  // $100 (tests scaling)
        ];

        for &price in &test_prices {
            let signal = PriceSignal::from_usd(price);
            let _encoded = signal.encode();

            // Create nonce with signal
            let pow_nonce = [0xABu8; POW_NONCE_SIZE];
            let nonce = signal.to_nonce(&pow_nonce);

            // Verify magic
            assert_eq!(&nonce[0..4], &PRICE_SIGNAL_MAGIC, "Magic should match");

            // Decode and verify
            let decoded = PriceSignal::decode(&nonce).expect("Should decode");

            // Allow small precision loss due to scaling
            let price_diff = (decoded.to_usd() - price).abs();
            let tolerance = if price > 4.0 { price * 0.001 } else { 0.0001 };
            assert!(
                price_diff < tolerance,
                "Price ${} decoded as ${}, diff {} > tolerance {}",
                price, decoded.to_usd(), price_diff, tolerance
            );
        }
    }

    /// Tests that invalid nonces return None.
    #[test]
    fn test_price_signal_invalid() {
        let _init_guard = zebra_test::init();

        // No magic prefix
        let nonce = [0u8; 32];
        assert!(PriceSignal::decode(&nonce).is_none(), "Should reject no magic");

        // Wrong magic
        let mut nonce = [0u8; 32];
        nonce[0..4].copy_from_slice(b"XXXX");
        assert!(PriceSignal::decode(&nonce).is_none(), "Should reject wrong magic");

        // Zero price (magic present but price is 0)
        let mut nonce = [0u8; 32];
        nonce[0..4].copy_from_slice(&PRICE_SIGNAL_MAGIC);
        // Price bytes are already 0
        assert!(PriceSignal::decode(&nonce).is_none(), "Should reject zero price");
    }

    /// Tests median calculation.
    #[test]
    fn test_calculate_median() {
        let _init_guard = zebra_test::init();

        // Empty slice
        assert_eq!(calculate_median(&[]), None);

        // Single element
        assert_eq!(calculate_median(&[100]), Some(100));

        // Odd count
        assert_eq!(calculate_median(&[1, 2, 3]), Some(2));
        assert_eq!(calculate_median(&[3, 1, 2]), Some(2)); // Unsorted

        // Even count (average of middle two)
        assert_eq!(calculate_median(&[1, 2, 3, 4]), Some(2)); // (2+3)/2 = 2.5 → 2
        assert_eq!(calculate_median(&[1, 3]), Some(2)); // (1+3)/2 = 2

        // Larger example
        let prices = [100, 150, 200, 250, 300];
        assert_eq!(calculate_median(&prices), Some(200));
    }

    /// Tests outlier filtering.
    #[test]
    fn test_filter_outliers() {
        let _init_guard = zebra_test::init();

        // No outliers (all within 50% of median)
        let prices = [100, 110, 120, 130, 140];
        let filtered = filter_outliers(&prices, 50);
        assert_eq!(filtered.len(), 5, "All prices should pass");

        // One extreme outlier
        let prices = [100, 110, 120, 130, 500]; // 500 is way above median ~120
        let filtered = filter_outliers(&prices, 50);
        // Median of [100,110,120,130,500] = 120
        // 500 deviates (500-120)/120 = 316% > 50%
        assert!(!filtered.contains(&500), "500 should be filtered");
        assert!(filtered.len() < 5, "Should filter outliers");

        // All within tolerance
        let prices = [100, 105, 110, 115, 120];
        let filtered = filter_outliers(&prices, 50);
        assert_eq!(filtered.len(), 5, "All within 50% should pass");
    }

    /// Tests price aggregation.
    #[test]
    fn test_aggregate_prices() {
        let _init_guard = zebra_test::init();

        // Not enough signals
        let few_signals: Vec<PriceSignal> = (0..50)
            .map(|i| PriceSignal::from_usd(0.10 + i as f64 * 0.001))
            .collect();
        assert!(aggregate_prices(&few_signals, 100).is_none(), "Need 51+ signals");

        // Enough valid signals
        let signals: Vec<PriceSignal> = (0..60)
            .map(|i| PriceSignal::from_usd(0.10 + i as f64 * 0.001))
            .collect();
        let oracle = aggregate_prices(&signals, 100).expect("Should aggregate");
        assert!(oracle.signal_count >= 51, "Should have enough signals");
        assert_eq!(oracle.as_of_height, 100);

        // Median should be around middle of range
        let median_usd = oracle.to_usd();
        assert!(median_usd > 0.10 && median_usd < 0.16, "Median should be reasonable");
    }

    /// Tests dynamic fee calculation.
    #[test]
    fn test_fee_calculation() {
        let _init_guard = zebra_test::init();

        // Test at various price points
        // target = $0.00001, so fee_bcash = $0.00001 / price

        // At $0.10 per BCASH:
        // fee = $0.00001 / $0.10 = 0.0001 BCASH = 10,000 zatoshis
        let fee = OracleParams::calculate_fee_for_price(0.10);
        assert_eq!(fee, 10_000, "Fee at $0.10 should be 10,000 zatoshis");

        // At $1.00 per BCASH:
        // fee = $0.00001 / $1.00 = 0.00001 BCASH = 1,000 zatoshis
        let fee = OracleParams::calculate_fee_for_price(1.00);
        assert_eq!(fee, 1_000, "Fee at $1.00 should be 1,000 zatoshis (min)");

        // At $0.001 per BCASH:
        // fee = $0.00001 / $0.001 = 0.01 BCASH = 1,000,000 zatoshis
        let fee = OracleParams::calculate_fee_for_price(0.001);
        assert_eq!(fee, 1_000_000, "Fee at $0.001 should be 1,000,000 zatoshis (max)");

        // At very high price (e.g., $100):
        // fee = $0.00001 / $100 = 0.0000001 BCASH = 10 zatoshis → clamped to min
        let fee = OracleParams::calculate_fee_for_price(100.0);
        assert_eq!(fee, MIN_FEE_ZATOSHIS, "Fee at $100 should be clamped to min");

        // At zero or negative price: return min fee
        assert_eq!(OracleParams::calculate_fee_for_price(0.0), MIN_FEE_ZATOSHIS);
        assert_eq!(OracleParams::calculate_fee_for_price(-1.0), MIN_FEE_ZATOSHIS);
    }

    /// Tests OraclePrice fee calculation.
    #[test]
    fn test_oracle_price_fee() {
        let _init_guard = zebra_test::init();

        let oracle = OraclePrice {
            median_price_nano_usd: 100_000_000, // $0.10
            signal_count: 60,
            as_of_height: 1000,
        };

        let fee = oracle.calculate_fee();
        assert_eq!(fee, 10_000, "Oracle fee should match calculated fee");
    }

    /// Tests fee rate limiting.
    #[test]
    fn test_fee_rate_limiting() {
        let _init_guard = zebra_test::init();

        let old_fee = 10_000u64;

        // After 1 day, max change is 10%
        let new_fee = 15_000u64; // 50% increase
        let limited = OracleParams::rate_limit_fee(old_fee, new_fee, BLOCKS_PER_DAY);
        // Max change = 10_000 * 0.10 = 1,000
        assert_eq!(limited, 11_000, "Should limit to 10% increase");

        // After half day, max change is 5%
        let limited = OracleParams::rate_limit_fee(old_fee, new_fee, BLOCKS_PER_DAY / 2);
        // Max change = 10_000 * 0.05 = 500
        assert_eq!(limited, 10_500, "Should limit to 5% increase");

        // Decrease should also be limited
        let new_fee = 5_000u64; // 50% decrease
        let limited = OracleParams::rate_limit_fee(old_fee, new_fee, BLOCKS_PER_DAY);
        assert_eq!(limited, 9_000, "Should limit to 10% decrease");

        // Small change within limits should pass through
        let new_fee = 10_500u64; // 5% increase
        let limited = OracleParams::rate_limit_fee(old_fee, new_fee, BLOCKS_PER_DAY);
        assert_eq!(limited, 10_500, "Small change should pass through");
    }

    /// Tests parameter bounds validation.
    #[test]
    fn test_oracle_params_bounds() {
        let _init_guard = zebra_test::init();

        // Target fee bounds (in nano-USD: $0.000001 = 1,000 nano-USD, $0.001 = 1,000,000,000 nano-USD)
        assert!(bounds::validate_target_fee(1_000), "Min target fee ($0.000001) should be valid");
        assert!(bounds::validate_target_fee(10_000), "Default target fee ($0.00001) should be valid");
        assert!(bounds::validate_target_fee(1_000_000_000), "Max target fee ($0.001) should be valid");
        assert!(!bounds::validate_target_fee(100), "Too low target fee should be invalid");
        assert!(!bounds::validate_target_fee(10_000_000_000), "Too high target fee should be invalid");

        // Fee floor bounds
        assert!(bounds::validate_fee_floor(100), "Min fee floor should be valid");
        assert!(bounds::validate_fee_floor(1_000), "Default fee floor should be valid");
        assert!(bounds::validate_fee_floor(100_000), "Max fee floor should be valid");
        assert!(!bounds::validate_fee_floor(50), "Too low fee floor should be invalid");

        // Fee ceiling bounds
        assert!(bounds::validate_fee_ceiling(100_000), "Min fee ceiling should be valid");
        assert!(bounds::validate_fee_ceiling(1_000_000), "Default fee ceiling should be valid");
        assert!(bounds::validate_fee_ceiling(10_000_000), "Max fee ceiling should be valid");
        assert!(!bounds::validate_fee_ceiling(50_000), "Too low fee ceiling should be invalid");

        // Aggregation window bounds
        assert!(bounds::validate_aggregation_blocks(10), "Min window should be valid");
        assert!(bounds::validate_aggregation_blocks(100), "Default window should be valid");
        assert!(bounds::validate_aggregation_blocks(1_000), "Max window should be valid");
        assert!(!bounds::validate_aggregation_blocks(5), "Too small window should be invalid");
        assert!(!bounds::validate_aggregation_blocks(2_000), "Too large window should be invalid");

        // Min valid signals bounds
        assert!(bounds::validate_min_valid_signals(6, 10), "Min signals for min window");
        assert!(bounds::validate_min_valid_signals(51, 100), "Default signals");
        assert!(!bounds::validate_min_valid_signals(3, 10), "Below minimum should be invalid");
        assert!(!bounds::validate_min_valid_signals(4, 10), "Not majority should be invalid");
        assert!(!bounds::validate_min_valid_signals(200, 100), "Exceeds window should be invalid");

        // Max deviation bounds
        assert!(bounds::validate_max_deviation(10), "Min deviation should be valid");
        assert!(bounds::validate_max_deviation(50), "Default deviation should be valid");
        assert!(bounds::validate_max_deviation(90), "Max deviation should be valid");
        assert!(!bounds::validate_max_deviation(5), "Too low deviation should be invalid");
        assert!(!bounds::validate_max_deviation(95), "Too high deviation should be invalid");

        // Daily adjustment bounds
        assert!(bounds::validate_daily_adjustment(1), "Min adjustment should be valid");
        assert!(bounds::validate_daily_adjustment(10), "Default adjustment should be valid");
        assert!(bounds::validate_daily_adjustment(50), "Max adjustment should be valid");
        assert!(!bounds::validate_daily_adjustment(0), "Zero adjustment should be invalid");
        assert!(!bounds::validate_daily_adjustment(60), "Too high adjustment should be invalid");
    }

    /// Tests has_price_signal utility.
    #[test]
    fn test_has_price_signal() {
        let _init_guard = zebra_test::init();

        let signal = PriceSignal::from_usd(0.15);
        let nonce = signal.to_nonce(&[0u8; POW_NONCE_SIZE]);
        assert!(OracleParams::has_price_signal(&nonce), "Should detect signal");

        let no_signal = [0u8; 32];
        assert!(!OracleParams::has_price_signal(&no_signal), "Should not detect signal");
    }

    /// Tests that price signal preserves PoW nonce space.
    #[test]
    fn test_pow_nonce_preserved() {
        let _init_guard = zebra_test::init();

        let signal = PriceSignal::from_usd(0.15);
        let pow_nonce = [0xDEu8; POW_NONCE_SIZE];
        let nonce = signal.to_nonce(&pow_nonce);

        // Verify PoW nonce is preserved in bytes 8-31
        assert_eq!(&nonce[PRICE_SIGNAL_SIZE..32], &pow_nonce[..], "PoW nonce should be preserved");
    }
}
