//! Attention Market Parameters for Botcash Social Protocol.
//!
//! This module defines the governance parameters for the Botcash attention market,
//! a circular economy where paid rankings are redistributed as tip credits with
//! time-limited expiration.
//!
//! These parameters are configurable via on-chain voting (see specs/governance.md).
//! The default values are designed to create a balanced economy that rewards
//! active participants while maintaining velocity through credit expiration.

use crate::amount::{Amount, NonNegative, COIN};
use crate::block::HeightDiff;

/// The percentage of attention market payments redistributed as credits.
///
/// When users pay BCASH to boost content visibility, this percentage of the payment
/// is redistributed back to participants in the form of tip credits. The remaining
/// percentage (100% - REDISTRIBUTION_RATE) is burned or goes to miners.
///
/// Default: 80% (80 out of 100)
///
/// This creates a circular economy where paying for visibility also earns you
/// credits to tip others, encouraging continued participation.
pub const REDISTRIBUTION_RATE_PERCENT: u8 = 80;

/// The number of blocks before earned credits expire.
///
/// Credits earned from redistribution must be used within this many blocks
/// or they expire. This creates velocity in the economy and prevents hoarding.
///
/// Default: 10,080 blocks (7 days at 60-second block time)
///
/// Calculation: 7 days * 24 hours * 60 minutes = 10,080 minutes = 10,080 blocks
pub const CREDIT_TTL_BLOCKS: HeightDiff = 10_080;

/// The number of blocks in one redistribution epoch.
///
/// At the end of each epoch, all attention market payments during that epoch
/// are tallied and credits are calculated for redistribution to participants.
///
/// Default: 1,440 blocks (1 day at 60-second block time)
///
/// Calculation: 24 hours * 60 minutes = 1,440 minutes = 1,440 blocks
pub const EPOCH_LENGTH_BLOCKS: HeightDiff = 1_440;

/// The weight multiplier for tips in the Attention Units (AU) calculation.
///
/// Tips are considered organic signals of quality (as opposed to self-promotion
/// through boosts), so they count more heavily in the ranking algorithm.
///
/// Default: 2.0 (tips count 2x compared to paid boosts)
///
/// Stored as fixed-point with 1 decimal place: 20 = 2.0
pub const TIP_WEIGHT_FIXED: u8 = 20;

/// The decay half-life for the "hot" feed ranking algorithm, in blocks.
///
/// Content's effective AU decays exponentially over time. After this many blocks,
/// content has half its original effective AU. This ensures fresh content can
/// compete with older high-AU content.
///
/// Default: 1,440 blocks (1 day at 60-second block time)
pub const DECAY_HALF_LIFE_BLOCKS: HeightDiff = 1_440;

/// The minimum amount (in zatoshis) required for an attention boost.
///
/// This prevents spam by requiring a minimum economic commitment for boosts.
///
/// Default: 100,000 zatoshis (0.001 BCASH)
///
/// Note: 1 BCASH = 100,000,000 zatoshis (same as Zcash's COIN constant)
pub const MIN_BOOST_AMOUNT: u64 = COIN as u64 / 1000; // 0.001 BCASH = 100,000 zatoshis

/// The maximum duration (in blocks) for an attention boost.
///
/// Boosts cannot last forever - this caps how long content can maintain
/// boosted status from a single payment.
///
/// Default: 43,200 blocks (~30 days at 60-second block time)
///
/// Calculation: 30 days * 24 hours * 60 minutes = 43,200 blocks
pub const MAX_BOOST_DURATION_BLOCKS: HeightDiff = 43_200;

/// The default duration (in blocks) for an attention boost when not specified.
///
/// Default: 1,440 blocks (~1 day at 60-second block time)
pub const DEFAULT_BOOST_DURATION_BLOCKS: HeightDiff = 1_440;

/// The maximum number of items returned in a single market feed request.
///
/// This prevents excessive resource usage from large feed queries.
///
/// Default: 1,000 items
pub const MAX_MARKET_FEED_LIMIT: u32 = 1_000;

/// The default number of items returned in a market feed request.
///
/// Default: 50 items
pub const DEFAULT_MARKET_FEED_LIMIT: u32 = 50;

/// Category codes for the attention market.
///
/// Content can be categorized to allow filtered feeds. Categories 0-6 are
/// predefined, and 7-255 are reserved for future governance decisions.
pub mod categories {
    /// General/uncategorized content
    pub const GENERAL: u8 = 0;
    /// Services and offerings
    pub const SERVICES: u8 = 1;
    /// Jobs and employment
    pub const JOBS: u8 = 2;
    /// For sale / marketplace
    pub const MARKETPLACE: u8 = 3;
    /// Events and meetups
    pub const EVENTS: u8 = 4;
    /// Education and tutorials
    pub const EDUCATION: u8 = 5;
    /// Community discussions
    pub const COMMUNITY: u8 = 6;

    /// The highest defined category code
    pub const MAX_DEFINED: u8 = 6;

    /// Category codes from this value and above are reserved for future use
    pub const RESERVED_START: u8 = 7;

    /// Returns true if the category code is valid (defined or general)
    pub fn is_valid(category: u8) -> bool {
        category <= MAX_DEFINED
    }

    /// Returns true if the category code is reserved for future use
    pub fn is_reserved(category: u8) -> bool {
        category >= RESERVED_START
    }

    /// Returns the name for a category code, or None if reserved/unknown
    pub fn name(category: u8) -> Option<&'static str> {
        match category {
            GENERAL => Some("General"),
            SERVICES => Some("Services"),
            JOBS => Some("Jobs"),
            MARKETPLACE => Some("Marketplace"),
            EVENTS => Some("Events"),
            EDUCATION => Some("Education"),
            COMMUNITY => Some("Community"),
            _ => None,
        }
    }
}

/// Attention market parameter bounds for validation.
///
/// These define the acceptable ranges for governance-adjustable parameters.
pub mod bounds {
    use super::*;

    /// Minimum redistribution rate (1%)
    pub const MIN_REDISTRIBUTION_RATE: u8 = 1;
    /// Maximum redistribution rate (99%)
    pub const MAX_REDISTRIBUTION_RATE: u8 = 99;

    /// Minimum credit TTL (1 day = 1,440 blocks)
    pub const MIN_CREDIT_TTL_BLOCKS: HeightDiff = 1_440;
    /// Maximum credit TTL (30 days = 43,200 blocks)
    pub const MAX_CREDIT_TTL_BLOCKS: HeightDiff = 43_200;

    /// Minimum epoch length (1 hour = 60 blocks)
    pub const MIN_EPOCH_LENGTH_BLOCKS: HeightDiff = 60;
    /// Maximum epoch length (7 days = 10,080 blocks)
    pub const MAX_EPOCH_LENGTH_BLOCKS: HeightDiff = 10_080;

    /// Minimum tip weight (1.0 = 10 in fixed-point)
    pub const MIN_TIP_WEIGHT_FIXED: u8 = 10;
    /// Maximum tip weight (10.0 = 100 in fixed-point)
    pub const MAX_TIP_WEIGHT_FIXED: u8 = 100;

    /// Minimum decay half-life (1 hour = 60 blocks)
    pub const MIN_DECAY_HALF_LIFE_BLOCKS: HeightDiff = 60;
    /// Maximum decay half-life (7 days = 10,080 blocks)
    pub const MAX_DECAY_HALF_LIFE_BLOCKS: HeightDiff = 10_080;

    /// Minimum boost amount (0.0001 BCASH = 10,000 zatoshis)
    pub const MIN_BOOST_AMOUNT_ZATOSHIS: u64 = COIN as u64 / 10_000;
    /// Maximum minimum boost amount (1 BCASH = 100,000,000 zatoshis)
    pub const MAX_BOOST_AMOUNT_ZATOSHIS: u64 = COIN as u64;

    /// Validates that the redistribution rate is within bounds
    pub fn validate_redistribution_rate(rate: u8) -> bool {
        rate >= MIN_REDISTRIBUTION_RATE && rate <= MAX_REDISTRIBUTION_RATE
    }

    /// Validates that the credit TTL is within bounds
    pub fn validate_credit_ttl(blocks: HeightDiff) -> bool {
        blocks >= MIN_CREDIT_TTL_BLOCKS && blocks <= MAX_CREDIT_TTL_BLOCKS
    }

    /// Validates that the epoch length is within bounds
    pub fn validate_epoch_length(blocks: HeightDiff) -> bool {
        blocks >= MIN_EPOCH_LENGTH_BLOCKS && blocks <= MAX_EPOCH_LENGTH_BLOCKS
    }

    /// Validates that the tip weight is within bounds
    pub fn validate_tip_weight(weight: u8) -> bool {
        weight >= MIN_TIP_WEIGHT_FIXED && weight <= MAX_TIP_WEIGHT_FIXED
    }

    /// Validates that the decay half-life is within bounds
    pub fn validate_decay_half_life(blocks: HeightDiff) -> bool {
        blocks >= MIN_DECAY_HALF_LIFE_BLOCKS && blocks <= MAX_DECAY_HALF_LIFE_BLOCKS
    }

    /// Validates that the minimum boost amount is within bounds
    pub fn validate_min_boost_amount(amount: u64) -> bool {
        amount >= MIN_BOOST_AMOUNT_ZATOSHIS && amount <= MAX_BOOST_AMOUNT_ZATOSHIS
    }
}

/// Utility functions for attention market calculations.
impl AttentionParams {
    /// Calculates the epoch number for a given block height.
    ///
    /// Epochs are numbered starting from 0 at genesis.
    pub fn epoch_for_height(height: crate::block::Height, epoch_length: HeightDiff) -> u64 {
        height.0 as u64 / epoch_length as u64
    }

    /// Calculates the start block of a given epoch.
    pub fn epoch_start_height(epoch: u64, epoch_length: HeightDiff) -> crate::block::Height {
        crate::block::Height((epoch * epoch_length as u64) as u32)
    }

    /// Calculates the end block of a given epoch (inclusive).
    pub fn epoch_end_height(epoch: u64, epoch_length: HeightDiff) -> crate::block::Height {
        crate::block::Height(((epoch + 1) * epoch_length as u64 - 1) as u32)
    }

    /// Calculates the block at which credits granted at `grant_height` will expire.
    pub fn credit_expiration_height(
        grant_height: crate::block::Height,
        ttl_blocks: HeightDiff,
    ) -> crate::block::Height {
        crate::block::Height(grant_height.0.saturating_add(ttl_blocks as u32))
    }

    /// Converts the fixed-point tip weight to a floating-point value.
    pub fn tip_weight_as_f64(weight_fixed: u8) -> f64 {
        weight_fixed as f64 / 10.0
    }

    /// Calculates the decay factor for content age.
    ///
    /// Returns a value between 0.0 and 1.0 representing how much of the original
    /// AU should remain after `age_blocks` have passed.
    pub fn calculate_decay(age_blocks: HeightDiff, half_life_blocks: HeightDiff) -> f64 {
        if half_life_blocks == 0 {
            return 0.0;
        }
        0.5_f64.powf(age_blocks as f64 / half_life_blocks as f64)
    }

    /// Calculates Attention Units (AU) for content.
    ///
    /// AU = (bcash_paid * 1.0) + (tips_received * tip_weight)
    pub fn calculate_au(bcash_paid: u64, tips_received: u64, tip_weight_fixed: u8) -> f64 {
        let tip_weight = Self::tip_weight_as_f64(tip_weight_fixed);
        bcash_paid as f64 + (tips_received as f64 * tip_weight)
    }

    /// Calculates the time-decayed rank for the "hot" feed.
    ///
    /// rank = AU * decay_factor * boost_multiplier
    pub fn calculate_hot_rank(
        bcash_paid: u64,
        tips_received: u64,
        age_blocks: HeightDiff,
        is_boosted: bool,
        tip_weight_fixed: u8,
        half_life_blocks: HeightDiff,
    ) -> f64 {
        let au = Self::calculate_au(bcash_paid, tips_received, tip_weight_fixed);
        let decay = Self::calculate_decay(age_blocks, half_life_blocks);
        let boost_multiplier = if is_boosted { 1.5 } else { 1.0 };
        au * decay * boost_multiplier
    }

    /// Calculates the credit amount to redistribute to a participant.
    ///
    /// A participant's share is proportional to their payment relative to
    /// total epoch payments, multiplied by the redistribution rate.
    pub fn calculate_credit_share(
        participant_paid: u64,
        total_epoch_paid: u64,
        redistribution_rate: u8,
    ) -> u64 {
        if total_epoch_paid == 0 {
            return 0;
        }
        let pool = (total_epoch_paid as u128 * redistribution_rate as u128) / 100;
        ((pool * participant_paid as u128) / total_epoch_paid as u128) as u64
    }
}

/// A zero-sized struct for accessing attention parameter utility functions.
///
/// This pattern allows calling static methods via `AttentionParams::method_name()`.
pub struct AttentionParams;

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests that all default attention market parameters have expected values.
    #[test]
    fn test_attention_params_default() {
        let _init_guard = zebra_test::init();

        // Verify default parameter values from the implementation plan
        assert_eq!(REDISTRIBUTION_RATE_PERCENT, 80, "Redistribution rate should be 80%");
        assert_eq!(CREDIT_TTL_BLOCKS, 10_080, "Credit TTL should be 7 days (10,080 blocks)");
        assert_eq!(EPOCH_LENGTH_BLOCKS, 1_440, "Epoch length should be 1 day (1,440 blocks)");
        assert_eq!(TIP_WEIGHT_FIXED, 20, "Tip weight should be 2.0 (20 in fixed-point)");
        assert_eq!(DECAY_HALF_LIFE_BLOCKS, 1_440, "Decay half-life should be 1 day (1,440 blocks)");
        assert_eq!(MIN_BOOST_AMOUNT, 100_000, "Min boost should be 0.001 BCASH (100,000 zatoshis)");

        // Verify derived values
        assert_eq!(MAX_BOOST_DURATION_BLOCKS, 43_200, "Max boost duration should be 30 days");
        assert_eq!(DEFAULT_BOOST_DURATION_BLOCKS, 1_440, "Default boost should be 1 day");
        assert_eq!(MAX_MARKET_FEED_LIMIT, 1_000, "Max feed limit should be 1,000");
        assert_eq!(DEFAULT_MARKET_FEED_LIMIT, 50, "Default feed limit should be 50");

        // Verify category codes
        assert_eq!(categories::GENERAL, 0);
        assert_eq!(categories::MAX_DEFINED, 6);
        assert_eq!(categories::RESERVED_START, 7);
    }

    /// Tests that attention parameter bounds are correctly enforced.
    #[test]
    fn test_attention_params_bounds() {
        let _init_guard = zebra_test::init();

        // Test redistribution rate bounds
        assert!(bounds::validate_redistribution_rate(1), "1% should be valid");
        assert!(bounds::validate_redistribution_rate(80), "80% should be valid");
        assert!(bounds::validate_redistribution_rate(99), "99% should be valid");
        assert!(!bounds::validate_redistribution_rate(0), "0% should be invalid");
        assert!(!bounds::validate_redistribution_rate(100), "100% should be invalid");

        // Test credit TTL bounds
        assert!(bounds::validate_credit_ttl(1_440), "1 day should be valid");
        assert!(bounds::validate_credit_ttl(10_080), "7 days (default) should be valid");
        assert!(bounds::validate_credit_ttl(43_200), "30 days should be valid");
        assert!(!bounds::validate_credit_ttl(60), "1 hour should be invalid");
        assert!(!bounds::validate_credit_ttl(100_000), "100,000 blocks should be invalid");

        // Test epoch length bounds
        assert!(bounds::validate_epoch_length(60), "1 hour should be valid");
        assert!(bounds::validate_epoch_length(1_440), "1 day (default) should be valid");
        assert!(bounds::validate_epoch_length(10_080), "7 days should be valid");
        assert!(!bounds::validate_epoch_length(30), "30 blocks should be invalid");
        assert!(!bounds::validate_epoch_length(20_000), "20,000 blocks should be invalid");

        // Test tip weight bounds
        assert!(bounds::validate_tip_weight(10), "1.0x should be valid");
        assert!(bounds::validate_tip_weight(20), "2.0x (default) should be valid");
        assert!(bounds::validate_tip_weight(100), "10.0x should be valid");
        assert!(!bounds::validate_tip_weight(5), "0.5x should be invalid");
        assert!(!bounds::validate_tip_weight(150), "15.0x should be invalid");

        // Test decay half-life bounds
        assert!(bounds::validate_decay_half_life(60), "1 hour should be valid");
        assert!(bounds::validate_decay_half_life(1_440), "1 day (default) should be valid");
        assert!(bounds::validate_decay_half_life(10_080), "7 days should be valid");
        assert!(!bounds::validate_decay_half_life(30), "30 blocks should be invalid");
        assert!(!bounds::validate_decay_half_life(20_000), "20,000 blocks should be invalid");

        // Test minimum boost amount bounds
        assert!(bounds::validate_min_boost_amount(10_000), "0.0001 BCASH should be valid");
        assert!(bounds::validate_min_boost_amount(100_000), "0.001 BCASH (default) should be valid");
        assert!(bounds::validate_min_boost_amount(100_000_000), "1 BCASH should be valid");
        assert!(!bounds::validate_min_boost_amount(1_000), "0.00001 BCASH should be invalid");
        assert!(!bounds::validate_min_boost_amount(1_000_000_000), "10 BCASH should be invalid");
    }

    /// Tests category validation functions.
    #[test]
    fn test_category_validation() {
        let _init_guard = zebra_test::init();

        // Test valid categories
        for cat in 0..=categories::MAX_DEFINED {
            assert!(categories::is_valid(cat), "Category {} should be valid", cat);
            assert!(!categories::is_reserved(cat), "Category {} should not be reserved", cat);
            assert!(categories::name(cat).is_some(), "Category {} should have a name", cat);
        }

        // Test reserved categories
        for cat in categories::RESERVED_START..=255 {
            assert!(!categories::is_valid(cat), "Category {} should be invalid", cat);
            assert!(categories::is_reserved(cat), "Category {} should be reserved", cat);
            assert!(categories::name(cat).is_none(), "Category {} should not have a name", cat);
        }
    }

    /// Tests utility calculation functions.
    #[test]
    fn test_attention_calculations() {
        let _init_guard = zebra_test::init();

        // Test epoch calculations
        let epoch_len = EPOCH_LENGTH_BLOCKS;
        assert_eq!(
            AttentionParams::epoch_for_height(crate::block::Height(0), epoch_len),
            0
        );
        assert_eq!(
            AttentionParams::epoch_for_height(crate::block::Height(1_439), epoch_len),
            0
        );
        assert_eq!(
            AttentionParams::epoch_for_height(crate::block::Height(1_440), epoch_len),
            1
        );
        assert_eq!(
            AttentionParams::epoch_for_height(crate::block::Height(2_880), epoch_len),
            2
        );

        // Test epoch boundaries
        assert_eq!(
            AttentionParams::epoch_start_height(0, epoch_len),
            crate::block::Height(0)
        );
        assert_eq!(
            AttentionParams::epoch_end_height(0, epoch_len),
            crate::block::Height(1_439)
        );
        assert_eq!(
            AttentionParams::epoch_start_height(1, epoch_len),
            crate::block::Height(1_440)
        );

        // Test credit expiration
        assert_eq!(
            AttentionParams::credit_expiration_height(
                crate::block::Height(1000),
                CREDIT_TTL_BLOCKS
            ),
            crate::block::Height(11_080) // 1000 + 10,080
        );

        // Test tip weight conversion
        assert!((AttentionParams::tip_weight_as_f64(10) - 1.0).abs() < f64::EPSILON);
        assert!((AttentionParams::tip_weight_as_f64(20) - 2.0).abs() < f64::EPSILON);
        assert!((AttentionParams::tip_weight_as_f64(15) - 1.5).abs() < f64::EPSILON);

        // Test decay calculation
        assert!((AttentionParams::calculate_decay(0, 1_440) - 1.0).abs() < f64::EPSILON);
        assert!((AttentionParams::calculate_decay(1_440, 1_440) - 0.5).abs() < f64::EPSILON);
        assert!((AttentionParams::calculate_decay(2_880, 1_440) - 0.25).abs() < f64::EPSILON);

        // Test AU calculation
        // 100 paid + 50 tips at 2.0x weight = 100 + 100 = 200 AU
        assert!((AttentionParams::calculate_au(100, 50, 20) - 200.0).abs() < f64::EPSILON);

        // Test credit share calculation
        // Participant paid 100 out of 1000 total, 80% redistributed
        // Pool = 1000 * 0.80 = 800
        // Share = 800 * (100/1000) = 80
        assert_eq!(AttentionParams::calculate_credit_share(100, 1_000, 80), 80);
        assert_eq!(AttentionParams::calculate_credit_share(500, 1_000, 80), 400);
        assert_eq!(AttentionParams::calculate_credit_share(0, 1_000, 80), 0);
        assert_eq!(AttentionParams::calculate_credit_share(100, 0, 80), 0);
    }

    /// Tests hot rank calculation with various inputs.
    #[test]
    fn test_hot_rank_calculation() {
        let _init_guard = zebra_test::init();

        // Fresh unboosted content: 100 paid, 50 tips, age 0
        let rank1 = AttentionParams::calculate_hot_rank(
            100,
            50,
            0,
            false,
            TIP_WEIGHT_FIXED,
            DECAY_HALF_LIFE_BLOCKS,
        );
        // AU = 100 + 50*2 = 200, decay = 1.0, boost = 1.0
        assert!((rank1 - 200.0).abs() < f64::EPSILON);

        // Same content but boosted
        let rank2 = AttentionParams::calculate_hot_rank(
            100,
            50,
            0,
            true,
            TIP_WEIGHT_FIXED,
            DECAY_HALF_LIFE_BLOCKS,
        );
        // AU = 200, decay = 1.0, boost = 1.5 → 300
        assert!((rank2 - 300.0).abs() < f64::EPSILON);

        // Content after 1 day (half-life)
        let rank3 = AttentionParams::calculate_hot_rank(
            100,
            50,
            DECAY_HALF_LIFE_BLOCKS,
            false,
            TIP_WEIGHT_FIXED,
            DECAY_HALF_LIFE_BLOCKS,
        );
        // AU = 200, decay = 0.5, boost = 1.0 → 100
        assert!((rank3 - 100.0).abs() < f64::EPSILON);
    }
}
