//! Protocol Upgrade Signaling for Botcash Soft Forks.
//!
//! This module implements version bit signaling in block headers, allowing miners
//! to signal support for proposed soft forks. When a threshold of miners signal
//! support, the upgrade activates after a grace period.
//!
//! This is inspired by Bitcoin's BIP 9 mechanism but adapted for Botcash:
//! - 75% threshold (instead of 95%)
//! - 1000-block signaling window
//! - 1008-block grace period (~1 week at 60s blocks)
//!
//! See specs/governance.md for full specification.

use std::collections::BTreeMap;

use crate::block::Height;

// ============================================================================
// Signaling Parameters
// ============================================================================

/// Number of blocks in a signaling window.
///
/// Miners signal support during each window. If threshold is reached by
/// window end, the upgrade moves to LOCKED_IN state.
///
/// Default: 1000 blocks (~16.7 hours at 60-second block time)
pub const SIGNALING_WINDOW_BLOCKS: u32 = 1_000;

/// Percentage of blocks that must signal support for activation.
///
/// 75% is a balance between ensuring strong consensus and not allowing
/// a minority to indefinitely block useful upgrades.
///
/// Default: 75%
pub const ACTIVATION_THRESHOLD_PERCENT: u8 = 75;

/// Number of blocks after threshold is reached before activation.
///
/// This grace period allows node operators to upgrade their software
/// before the soft fork rules become enforced.
///
/// Default: 1008 blocks (~1 week at 60-second block time)
pub const GRACE_PERIOD_BLOCKS: u32 = 1_008;

/// Minimum number of blocks a deployment must signal before activation.
///
/// This prevents flash activations and ensures adequate community review.
///
/// Default: 2000 blocks (at least 2 full signaling windows)
pub const MIN_SIGNALING_BLOCKS: u32 = 2_000;

/// Maximum number of concurrent soft fork deployments.
///
/// Limited to prevent version bit exhaustion and coordination complexity.
///
/// Default: 29 (leaving bits 0-2 reserved for other purposes)
pub const MAX_CONCURRENT_DEPLOYMENTS: u8 = 29;

/// Version bits available for signaling (bits 3-31).
///
/// Bits 0-2 are reserved for potential future use.
/// Bit 31 is the sign bit (must be 0 per Zcash spec).
pub const SIGNALING_BIT_RANGE: std::ops::RangeInclusive<u8> = 3..=30;

/// Minimum block version that supports version bit signaling.
///
/// Botcash activates signaling from version 5 onwards.
/// Zcash's minimum version 4 is preserved for compatibility.
pub const SIGNALING_MIN_VERSION: u32 = 5;

// ============================================================================
// Deployment States
// ============================================================================

/// State of a soft fork deployment.
///
/// Each deployment progresses through these states:
/// ```text
/// DEFINED -> STARTED -> LOCKED_IN -> ACTIVE
///                   \-> FAILED (if timeout reached without activation)
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeploymentState {
    /// Deployment is defined but signaling has not started.
    ///
    /// This is the initial state for all deployments.
    Defined,

    /// Signaling period is active, miners can signal support.
    ///
    /// The deployment is in this state between start_height and timeout_height.
    Started,

    /// Threshold was reached, activation is locked in.
    ///
    /// The deployment will activate after the grace period.
    LockedIn {
        /// Block height when lock-in was achieved.
        locked_in_height: Height,
        /// Block height when activation will occur.
        activation_height: Height,
    },

    /// Soft fork rules are now enforced.
    ///
    /// All nodes must follow the new consensus rules.
    Active {
        /// Block height when activation occurred.
        activated_height: Height,
    },

    /// Deployment failed to achieve activation before timeout.
    ///
    /// The deployment is permanently failed and must be resubmitted
    /// with a new BIP number and different version bit if desired.
    Failed,
}

impl DeploymentState {
    /// Returns true if the deployment is currently accepting signals.
    pub fn is_signaling(&self) -> bool {
        matches!(self, DeploymentState::Started)
    }

    /// Returns true if the deployment is active (rules enforced).
    pub fn is_active(&self) -> bool {
        matches!(self, DeploymentState::Active { .. })
    }

    /// Returns true if the deployment has concluded (active or failed).
    pub fn is_final(&self) -> bool {
        matches!(
            self,
            DeploymentState::Active { .. } | DeploymentState::Failed
        )
    }
}

// ============================================================================
// Soft Fork Deployment
// ============================================================================

/// A soft fork deployment that miners can signal support for.
///
/// Each deployment has a unique BIP number and version bit assignment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoftForkDeployment {
    /// Unique identifier for this deployment (e.g., "BSP-001").
    ///
    /// This follows the Botcash Improvement Proposal numbering.
    pub bip_id: String,

    /// Human-readable name for this deployment.
    pub name: String,

    /// Short description of what this soft fork enables.
    pub description: String,

    /// Version bit used for signaling (3-30).
    ///
    /// Must be unique among concurrent deployments.
    pub bit: u8,

    /// Block height when signaling starts.
    pub start_height: Height,

    /// Block height after which deployment fails if not activated.
    pub timeout_height: Height,
}

impl SoftForkDeployment {
    /// Creates a new soft fork deployment.
    ///
    /// # Arguments
    /// * `bip_id` - Unique identifier (e.g., "BSP-001")
    /// * `name` - Human-readable name
    /// * `description` - Short description
    /// * `bit` - Version bit (3-30)
    /// * `start_height` - When signaling begins
    /// * `timeout_height` - When deployment fails if not activated
    ///
    /// # Panics
    /// Panics if the bit is outside the valid range (3-30).
    pub fn new(
        bip_id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        bit: u8,
        start_height: Height,
        timeout_height: Height,
    ) -> Self {
        assert!(
            SIGNALING_BIT_RANGE.contains(&bit),
            "Version bit {} is outside valid range {:?}",
            bit,
            SIGNALING_BIT_RANGE
        );
        assert!(
            timeout_height > start_height,
            "Timeout height must be after start height"
        );

        Self {
            bip_id: bip_id.into(),
            name: name.into(),
            description: description.into(),
            bit,
            start_height,
            timeout_height,
        }
    }

    /// Returns the bitmask for this deployment's version bit.
    pub fn bit_mask(&self) -> u32 {
        1u32 << self.bit
    }

    /// Checks if a block version signals support for this deployment.
    pub fn version_signals(&self, version: u32) -> bool {
        version & self.bit_mask() != 0
    }
}

// ============================================================================
// Signal Tracking
// ============================================================================

/// Tracks signaling statistics for a deployment within a window.
#[derive(Debug, Clone, Default)]
pub struct SignalingStats {
    /// Total blocks in the signaling window.
    pub total_blocks: u32,

    /// Blocks that signaled support.
    pub signaling_blocks: u32,

    /// Block heights that signaled (for debugging/auditing).
    pub signaling_heights: Vec<Height>,
}

impl SignalingStats {
    /// Creates empty signaling stats.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a block's signal (or lack thereof).
    pub fn record_block(&mut self, height: Height, signals: bool) {
        self.total_blocks += 1;
        if signals {
            self.signaling_blocks += 1;
            self.signaling_heights.push(height);
        }
    }

    /// Returns the signaling percentage (0-100).
    pub fn signaling_percent(&self) -> f64 {
        if self.total_blocks == 0 {
            return 0.0;
        }
        (self.signaling_blocks as f64 / self.total_blocks as f64) * 100.0
    }

    /// Returns true if the activation threshold has been reached.
    pub fn threshold_reached(&self) -> bool {
        self.signaling_percent() >= ACTIVATION_THRESHOLD_PERCENT as f64
    }

    /// Returns the number of additional signals needed to reach threshold.
    pub fn signals_needed(&self) -> u32 {
        let threshold_count =
            (self.total_blocks as f64 * ACTIVATION_THRESHOLD_PERCENT as f64 / 100.0).ceil() as u32;
        threshold_count.saturating_sub(self.signaling_blocks)
    }
}

// ============================================================================
// Version Bit Utilities
// ============================================================================

/// Parses version bits from a block version field.
///
/// Returns a map of bit position to signal state (true = set, false = unset).
pub fn parse_version_bits(version: u32) -> BTreeMap<u8, bool> {
    let mut bits = BTreeMap::new();
    for bit in *SIGNALING_BIT_RANGE.start()..=*SIGNALING_BIT_RANGE.end() {
        bits.insert(bit, (version & (1u32 << bit)) != 0);
    }
    bits
}

/// Creates a block version with the specified bits set.
///
/// Starts from the base version (SIGNALING_MIN_VERSION) and sets the requested bits.
///
/// # Arguments
/// * `bits` - Iterator of bit positions to set (3-30)
///
/// # Panics
/// Panics if any bit is outside the valid range.
pub fn create_signaling_version(bits: impl IntoIterator<Item = u8>) -> u32 {
    let mut version = SIGNALING_MIN_VERSION;
    for bit in bits {
        assert!(
            SIGNALING_BIT_RANGE.contains(&bit),
            "Version bit {} is outside valid range {:?}",
            bit,
            SIGNALING_BIT_RANGE
        );
        version |= 1u32 << bit;
    }
    version
}

/// Checks if a block version supports version bit signaling.
///
/// Returns true if the version is >= SIGNALING_MIN_VERSION.
pub fn supports_signaling(version: u32) -> bool {
    version >= SIGNALING_MIN_VERSION
}

/// Returns the window number for a given block height.
///
/// Windows are numbered starting from 0, with each window containing
/// SIGNALING_WINDOW_BLOCKS blocks.
pub fn window_number(height: Height) -> u64 {
    height.0 as u64 / SIGNALING_WINDOW_BLOCKS as u64
}

/// Returns the start height of the window containing the given height.
pub fn window_start_height(height: Height) -> Height {
    let window = window_number(height);
    Height((window * SIGNALING_WINDOW_BLOCKS as u64) as u32)
}

/// Returns the end height (inclusive) of the window containing the given height.
pub fn window_end_height(height: Height) -> Height {
    let start = window_start_height(height);
    Height(start.0 + SIGNALING_WINDOW_BLOCKS - 1)
}

/// Calculates the activation height given a lock-in height.
///
/// Activation occurs at the start of the first window after the grace period.
pub fn calculate_activation_height(locked_in_height: Height) -> Height {
    let grace_end = Height(locked_in_height.0 + GRACE_PERIOD_BLOCKS);
    // Activation at next window boundary after grace period
    let window_after_grace = window_number(grace_end) + 1;
    Height((window_after_grace * SIGNALING_WINDOW_BLOCKS as u64) as u32)
}

// ============================================================================
// Deployment State Determination
// ============================================================================

/// Determines the state of a deployment at a given height.
///
/// # Arguments
/// * `deployment` - The soft fork deployment to check
/// * `height` - The block height to check state at
/// * `signaling_history` - Function that returns whether a block signaled support
///
/// # Returns
/// The deployment state at the given height.
pub fn get_deployment_state<F>(
    deployment: &SoftForkDeployment,
    height: Height,
    signaling_history: F,
) -> DeploymentState
where
    F: Fn(Height) -> Option<bool>,
{
    // Before start height
    if height < deployment.start_height {
        return DeploymentState::Defined;
    }

    // Check each completed window for lock-in
    let mut current_window = window_number(deployment.start_height);
    let height_window = window_number(height);

    while current_window < height_window {
        let window_start = Height((current_window * SIGNALING_WINDOW_BLOCKS as u64) as u32);
        let window_end = Height(window_start.0 + SIGNALING_WINDOW_BLOCKS - 1);

        // Only process windows within the deployment's active period
        if window_start >= deployment.start_height && window_end < deployment.timeout_height {
            // Count signals in this window
            let mut stats = SignalingStats::new();
            for h in window_start.0..=window_end.0 {
                let block_height = Height(h);
                if let Some(signals) = signaling_history(block_height) {
                    stats.record_block(block_height, signals);
                }
            }

            // Check if threshold was reached
            if stats.threshold_reached() && stats.total_blocks == SIGNALING_WINDOW_BLOCKS {
                let locked_in_height = window_end;
                let activation_height = calculate_activation_height(locked_in_height);

                // Check if we're past activation
                if height >= activation_height {
                    return DeploymentState::Active {
                        activated_height: activation_height,
                    };
                } else {
                    return DeploymentState::LockedIn {
                        locked_in_height,
                        activation_height,
                    };
                }
            }
        }

        current_window += 1;
    }

    // Check if we've passed timeout without activation
    if height >= deployment.timeout_height {
        return DeploymentState::Failed;
    }

    // Still in signaling period
    DeploymentState::Started
}

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during deployment operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeploymentError {
    /// Version bit is outside the valid range (3-30).
    InvalidBit {
        /// The invalid bit number.
        bit: u8,
    },

    /// Version bit is already assigned to another deployment.
    BitConflict {
        /// The conflicting bit number.
        bit: u8,
        /// BIP ID that already owns this bit.
        existing_bip: String,
        /// BIP ID attempting to use the same bit.
        new_bip: String,
    },

    /// Start height is in the past.
    StartHeightPassed {
        /// The proposed start height.
        start: Height,
        /// The current chain height.
        current: Height,
    },

    /// Timeout is too soon after start (less than MIN_SIGNALING_BLOCKS).
    TimeoutTooSoon {
        /// The proposed start height.
        start: Height,
        /// The proposed timeout height.
        timeout: Height,
        /// Minimum required gap between start and timeout.
        minimum_gap: u32,
    },

    /// Maximum concurrent deployments exceeded.
    TooManyDeployments {
        /// Maximum allowed concurrent deployments.
        max: u8,
        /// Current number of deployments.
        current: u8,
    },

    /// BIP ID is not unique.
    DuplicateBipId {
        /// The duplicate BIP ID.
        bip_id: String,
    },
}

impl std::fmt::Display for DeploymentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DeploymentError::InvalidBit { bit } => {
                write!(
                    f,
                    "version bit {} is outside valid range {:?}",
                    bit, SIGNALING_BIT_RANGE
                )
            }
            DeploymentError::BitConflict {
                bit,
                existing_bip,
                new_bip,
            } => {
                write!(
                    f,
                    "version bit {} already assigned to {} (conflict with {})",
                    bit, existing_bip, new_bip
                )
            }
            DeploymentError::StartHeightPassed { start, current } => {
                write!(
                    f,
                    "start height {} is in the past (current height {})",
                    start.0, current.0
                )
            }
            DeploymentError::TimeoutTooSoon {
                start,
                timeout,
                minimum_gap,
            } => {
                write!(
                    f,
                    "timeout {} is too soon after start {} (minimum gap: {} blocks)",
                    timeout.0, start.0, minimum_gap
                )
            }
            DeploymentError::TooManyDeployments { max, current } => {
                write!(
                    f,
                    "maximum {} concurrent deployments exceeded (current: {})",
                    max, current
                )
            }
            DeploymentError::DuplicateBipId { bip_id } => {
                write!(f, "BIP ID '{}' is already in use", bip_id)
            }
        }
    }
}

impl std::error::Error for DeploymentError {}

// ============================================================================
// Governance Bounds (for parameter adjustment via voting)
// ============================================================================

/// Validates a proposed activation threshold change.
///
/// Threshold must be between 50% (simple majority) and 95% (near-unanimous).
pub fn validate_threshold_change(new_threshold: u8) -> Result<(), &'static str> {
    if new_threshold < 50 {
        return Err("threshold cannot be below 50% (simple majority)");
    }
    if new_threshold > 95 {
        return Err("threshold cannot exceed 95%");
    }
    Ok(())
}

/// Validates a proposed signaling window change.
///
/// Window must be between 100 blocks (quick) and 10,000 blocks (~1 week).
pub fn validate_window_change(new_window: u32) -> Result<(), &'static str> {
    if new_window < 100 {
        return Err("signaling window cannot be less than 100 blocks");
    }
    if new_window > 10_000 {
        return Err("signaling window cannot exceed 10,000 blocks");
    }
    Ok(())
}

/// Validates a proposed grace period change.
///
/// Grace period must be between 144 blocks (~1 day) and 10,080 blocks (~1 week).
pub fn validate_grace_period_change(new_grace: u32) -> Result<(), &'static str> {
    if new_grace < 144 {
        return Err("grace period cannot be less than 144 blocks (~1 day)");
    }
    if new_grace > 10_080 {
        return Err("grace period cannot exceed 10,080 blocks (~1 week)");
    }
    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deployment_state_transitions() {
        let state = DeploymentState::Defined;
        assert!(!state.is_signaling());
        assert!(!state.is_active());
        assert!(!state.is_final());

        let state = DeploymentState::Started;
        assert!(state.is_signaling());
        assert!(!state.is_active());
        assert!(!state.is_final());

        let state = DeploymentState::LockedIn {
            locked_in_height: Height(1000),
            activation_height: Height(3000),
        };
        assert!(!state.is_signaling());
        assert!(!state.is_active());
        assert!(!state.is_final());

        let state = DeploymentState::Active {
            activated_height: Height(3000),
        };
        assert!(!state.is_signaling());
        assert!(state.is_active());
        assert!(state.is_final());

        let state = DeploymentState::Failed;
        assert!(!state.is_signaling());
        assert!(!state.is_active());
        assert!(state.is_final());
    }

    #[test]
    fn test_deployment_creation() {
        let deployment = SoftForkDeployment::new(
            "BSP-001",
            "Enhanced DMs",
            "Adds encryption version negotiation to DM protocol",
            5,
            Height(10_000),
            Height(100_000),
        );

        assert_eq!(deployment.bip_id, "BSP-001");
        assert_eq!(deployment.name, "Enhanced DMs");
        assert_eq!(deployment.bit, 5);
        assert_eq!(deployment.bit_mask(), 1u32 << 5);
    }

    #[test]
    #[should_panic(expected = "Version bit 2 is outside valid range")]
    fn test_deployment_invalid_bit_low() {
        SoftForkDeployment::new(
            "BSP-001",
            "Test",
            "Test",
            2, // Below minimum
            Height(10_000),
            Height(100_000),
        );
    }

    #[test]
    #[should_panic(expected = "Version bit 31 is outside valid range")]
    fn test_deployment_invalid_bit_high() {
        SoftForkDeployment::new(
            "BSP-001",
            "Test",
            "Test",
            31, // Sign bit
            Height(10_000),
            Height(100_000),
        );
    }

    #[test]
    fn test_version_signals() {
        let deployment = SoftForkDeployment::new(
            "BSP-001",
            "Test",
            "Test",
            5,
            Height(10_000),
            Height(100_000),
        );

        // Version with bit 5 set
        assert!(deployment.version_signals(0b0000_0000_0010_0101)); // bits 0, 2, 5
        assert!(deployment.version_signals(0b0000_0000_0010_0000)); // only bit 5

        // Version without bit 5
        assert!(!deployment.version_signals(0b0000_0000_0000_0101)); // bits 0, 2
        assert!(!deployment.version_signals(4)); // base version
    }

    #[test]
    fn test_signaling_stats() {
        let mut stats = SignalingStats::new();

        // Record 10 blocks, 8 signaling
        for i in 0..10 {
            stats.record_block(Height(i), i < 8);
        }

        assert_eq!(stats.total_blocks, 10);
        assert_eq!(stats.signaling_blocks, 8);
        assert!((stats.signaling_percent() - 80.0).abs() < 0.001);
        assert!(stats.threshold_reached()); // 80% >= 75%
        assert_eq!(stats.signaling_heights.len(), 8);
    }

    #[test]
    fn test_signaling_stats_threshold_not_reached() {
        let mut stats = SignalingStats::new();

        // Record 10 blocks, only 7 signaling (70%)
        for i in 0..10 {
            stats.record_block(Height(i), i < 7);
        }

        assert!(!stats.threshold_reached()); // 70% < 75%
        assert_eq!(stats.signals_needed(), 1); // Need 1 more for 8/10 = 80%
    }

    #[test]
    fn test_parse_version_bits() {
        let version = 0b0000_0000_0011_0100u32; // bits 2, 4, 5 set

        let bits = parse_version_bits(version);

        // Only bits 3-30 are reported
        assert!(!bits.get(&3).copied().unwrap_or(false));
        assert!(bits.get(&4).copied().unwrap_or(false));
        assert!(bits.get(&5).copied().unwrap_or(false));
        assert!(!bits.get(&6).copied().unwrap_or(false));
    }

    #[test]
    fn test_create_signaling_version() {
        let version = create_signaling_version([5, 7, 10]);

        assert_eq!(version & (1 << 5), 1 << 5);
        assert_eq!(version & (1 << 7), 1 << 7);
        assert_eq!(version & (1 << 10), 1 << 10);
        assert!(version >= SIGNALING_MIN_VERSION);
    }

    #[test]
    fn test_supports_signaling() {
        assert!(!supports_signaling(4)); // Zcash base version
        assert!(supports_signaling(5));
        assert!(supports_signaling(100));
    }

    #[test]
    fn test_window_calculations() {
        // Window 0: heights 0-999
        assert_eq!(window_number(Height(0)), 0);
        assert_eq!(window_number(Height(999)), 0);
        assert_eq!(window_start_height(Height(500)), Height(0));
        assert_eq!(window_end_height(Height(500)), Height(999));

        // Window 1: heights 1000-1999
        assert_eq!(window_number(Height(1000)), 1);
        assert_eq!(window_number(Height(1500)), 1);
        assert_eq!(window_start_height(Height(1500)), Height(1000));
        assert_eq!(window_end_height(Height(1500)), Height(1999));

        // Window 2: heights 2000-2999
        assert_eq!(window_number(Height(2000)), 2);
        assert_eq!(window_start_height(Height(2500)), Height(2000));
    }

    #[test]
    fn test_calculate_activation_height() {
        // Locked in at height 999 (end of window 0)
        // Grace period ends at 999 + 1008 = 2007 (in window 2)
        // Activation at start of window 3 = 3000
        let activation = calculate_activation_height(Height(999));
        assert_eq!(activation, Height(3000));

        // Locked in at height 1999 (end of window 1)
        // Grace period ends at 1999 + 1008 = 3007 (in window 3)
        // Activation at start of window 4 = 4000
        let activation = calculate_activation_height(Height(1999));
        assert_eq!(activation, Height(4000));
    }

    #[test]
    fn test_get_deployment_state_defined() {
        let deployment = SoftForkDeployment::new(
            "BSP-001",
            "Test",
            "Test",
            5,
            Height(10_000),
            Height(100_000),
        );

        // Before start height
        let state = get_deployment_state(&deployment, Height(5_000), |_| None);
        assert_eq!(state, DeploymentState::Defined);
    }

    #[test]
    fn test_get_deployment_state_started() {
        let deployment = SoftForkDeployment::new(
            "BSP-001",
            "Test",
            "Test",
            5,
            Height(10_000),
            Height(100_000),
        );

        // In signaling period but no complete windows yet
        let state = get_deployment_state(&deployment, Height(10_500), |_| Some(false));
        assert_eq!(state, DeploymentState::Started);
    }

    #[test]
    fn test_get_deployment_state_failed() {
        let deployment = SoftForkDeployment::new(
            "BSP-001",
            "Test",
            "Test",
            5,
            Height(10_000),
            Height(100_000),
        );

        // After timeout without activation
        let state = get_deployment_state(&deployment, Height(100_001), |_| Some(false));
        assert_eq!(state, DeploymentState::Failed);
    }

    #[test]
    fn test_validate_threshold_change() {
        assert!(validate_threshold_change(50).is_ok());
        assert!(validate_threshold_change(75).is_ok());
        assert!(validate_threshold_change(95).is_ok());

        assert!(validate_threshold_change(49).is_err());
        assert!(validate_threshold_change(96).is_err());
    }

    #[test]
    fn test_validate_window_change() {
        assert!(validate_window_change(100).is_ok());
        assert!(validate_window_change(1000).is_ok());
        assert!(validate_window_change(10_000).is_ok());

        assert!(validate_window_change(99).is_err());
        assert!(validate_window_change(10_001).is_err());
    }

    #[test]
    fn test_validate_grace_period_change() {
        assert!(validate_grace_period_change(144).is_ok());
        assert!(validate_grace_period_change(1008).is_ok());
        assert!(validate_grace_period_change(10_080).is_ok());

        assert!(validate_grace_period_change(143).is_err());
        assert!(validate_grace_period_change(10_081).is_err());
    }

    #[test]
    fn test_deployment_error_display() {
        let err = DeploymentError::InvalidBit { bit: 2 };
        assert!(err.to_string().contains("version bit 2"));

        let err = DeploymentError::BitConflict {
            bit: 5,
            existing_bip: "BSP-001".to_string(),
            new_bip: "BSP-002".to_string(),
        };
        assert!(err.to_string().contains("BSP-001"));
        assert!(err.to_string().contains("BSP-002"));
    }

    #[test]
    fn test_signaling_integration() {
        // Simulate a full deployment cycle
        let deployment = SoftForkDeployment::new(
            "BSP-001",
            "Enhanced Channels",
            "Improved off-chain channel protocol",
            5,
            Height(1_000),   // Start at window 1
            Height(50_000),  // Timeout at window 50
        );

        // Create a signaling history where window 1 (1000-1999) has 80% support
        let signaling_history = |height: Height| -> Option<bool> {
            if height.0 >= 1000 && height.0 < 2000 {
                // 80% signal in window 1
                Some(height.0 % 5 != 0) // 4 out of every 5 blocks signal
            } else {
                Some(false)
            }
        };

        // At height 1500 (during window 1), still started
        let state = get_deployment_state(&deployment, Height(1500), signaling_history);
        assert_eq!(state, DeploymentState::Started);

        // At height 2500 (after window 1 complete with threshold), locked in
        let state = get_deployment_state(&deployment, Height(2500), signaling_history);
        match state {
            DeploymentState::LockedIn {
                locked_in_height,
                activation_height,
            } => {
                assert_eq!(locked_in_height, Height(1999));
                // Activation should be at window boundary after grace period
                assert!(activation_height.0 > locked_in_height.0 + GRACE_PERIOD_BLOCKS);
            }
            _ => panic!("Expected LockedIn state, got {:?}", state),
        }
    }
}
