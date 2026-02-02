//! Indexer protocol upgrade tracking utilities.
//!
//! This module provides utilities for indexers to:
//! - Track version bit signaling across blocks
//! - Determine soft fork deployment states
//! - Calculate activation windows and timelines
//! - Generate upgrade progress reports
//!
//! # Overview
//!
//! Protocol upgrades in Botcash use version bit signaling, where miners set
//! specific bits in the block version field to indicate support for proposed
//! soft forks. This module tracks signals and determines when thresholds are met.
//!
//! # Signaling Process
//!
//! 1. **Defined**: Deployment is proposed but signaling hasn't started
//! 2. **Started**: Miners can signal support by setting version bits
//! 3. **Locked-In**: 75% threshold reached; activation locked
//! 4. **Active**: Soft fork rules are now enforced
//! 5. **Failed**: Timeout reached without activation
//!
//! # Usage
//!
//! ```ignore
//! use zebra_rpc::indexer::protocol_upgrades::{
//!     DeploymentTracker, SignalingWindowStats, parse_block_signals
//! };
//!
//! let signals = parse_block_signals(block_version);
//! tracker.record_block(height, block_version);
//! let stats = tracker.get_window_stats(deployment_id, window_number);
//! ```

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};
use zebra_chain::{
    block::Height,
    parameters::protocol_upgrades::{
        calculate_activation_height, create_signaling_version, parse_version_bits,
        supports_signaling, window_end_height, window_number, window_start_height, DeploymentState,
        SignalingStats, SoftForkDeployment, ACTIVATION_THRESHOLD_PERCENT, GRACE_PERIOD_BLOCKS,
        MAX_CONCURRENT_DEPLOYMENTS, SIGNALING_BIT_RANGE, SIGNALING_MIN_VERSION,
        SIGNALING_WINDOW_BLOCKS,
    },
};

// ============================================================================
// Indexed Block Signal
// ============================================================================

/// Parsed version bit signals from a block header.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedBlockSignal {
    /// Block height.
    pub height: Height,

    /// Raw block version field.
    pub version: u32,

    /// Whether this version supports signaling (>= SIGNALING_MIN_VERSION).
    pub supports_signaling: bool,

    /// Map of bit position to signal state (true = signaling support).
    pub signals: BTreeMap<u8, bool>,

    /// List of bits that are actively signaling (set to true).
    pub active_bits: Vec<u8>,
}

impl IndexedBlockSignal {
    /// Parses signals from a block version.
    pub fn from_version(height: Height, version: u32) -> Self {
        let supports = supports_signaling(version);
        let signals = if supports {
            parse_version_bits(version)
        } else {
            BTreeMap::new()
        };

        let active_bits = signals
            .iter()
            .filter_map(|(&bit, &set)| if set { Some(bit) } else { None })
            .collect();

        Self {
            height,
            version,
            supports_signaling: supports,
            signals,
            active_bits,
        }
    }

    /// Returns true if this block signals support for the given bit.
    pub fn signals_bit(&self, bit: u8) -> bool {
        self.signals.get(&bit).copied().unwrap_or(false)
    }

    /// Returns true if this block signals support for the given deployment.
    pub fn signals_deployment(&self, deployment: &SoftForkDeployment) -> bool {
        self.supports_signaling && self.signals_bit(deployment.bit)
    }
}

// ============================================================================
// Signaling Window Statistics
// ============================================================================

/// Statistics for a single signaling window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalingWindowStats {
    /// Window number (0-indexed).
    pub window: u64,

    /// Start height of this window.
    pub start_height: Height,

    /// End height of this window (inclusive).
    pub end_height: Height,

    /// Total blocks recorded in this window.
    pub total_blocks: u32,

    /// Number of blocks signaling support.
    pub signaling_blocks: u32,

    /// Signaling percentage (0-100).
    pub signaling_percent: f64,

    /// Whether threshold (75%) has been reached.
    pub threshold_reached: bool,

    /// Whether this window is complete (all SIGNALING_WINDOW_BLOCKS recorded).
    pub is_complete: bool,

    /// Heights of blocks that signaled support.
    pub signaling_heights: Vec<Height>,
}

impl SignalingWindowStats {
    /// Creates a new window stats tracker.
    pub fn new(window: u64) -> Self {
        let start_height = Height((window * SIGNALING_WINDOW_BLOCKS as u64) as u32);
        let end_height = Height(start_height.0 + SIGNALING_WINDOW_BLOCKS - 1);

        Self {
            window,
            start_height,
            end_height,
            total_blocks: 0,
            signaling_blocks: 0,
            signaling_percent: 0.0,
            threshold_reached: false,
            is_complete: false,
            signaling_heights: Vec::new(),
        }
    }

    /// Records a block's signal in this window.
    pub fn record_block(&mut self, height: Height, signals: bool) {
        self.total_blocks += 1;
        if signals {
            self.signaling_blocks += 1;
            self.signaling_heights.push(height);
        }

        self.update_stats();
    }

    /// Updates derived statistics.
    fn update_stats(&mut self) {
        if self.total_blocks > 0 {
            self.signaling_percent =
                (self.signaling_blocks as f64 / self.total_blocks as f64) * 100.0;
        }
        self.threshold_reached = self.signaling_percent >= ACTIVATION_THRESHOLD_PERCENT as f64;
        self.is_complete = self.total_blocks >= SIGNALING_WINDOW_BLOCKS;
    }

    /// Returns the number of additional signals needed to reach threshold.
    pub fn signals_needed_for_threshold(&self) -> u32 {
        let blocks_remaining = SIGNALING_WINDOW_BLOCKS.saturating_sub(self.total_blocks);
        let total_expected = SIGNALING_WINDOW_BLOCKS as f64;
        let threshold_count =
            (total_expected * ACTIVATION_THRESHOLD_PERCENT as f64 / 100.0).ceil() as u32;

        threshold_count.saturating_sub(self.signaling_blocks)
    }

    /// Returns the minimum signals still achievable if all remaining blocks signal.
    pub fn max_achievable_percent(&self) -> f64 {
        let blocks_remaining = SIGNALING_WINDOW_BLOCKS.saturating_sub(self.total_blocks);
        let max_signaling = self.signaling_blocks + blocks_remaining;
        (max_signaling as f64 / SIGNALING_WINDOW_BLOCKS as f64) * 100.0
    }

    /// Returns true if threshold can still be reached in this window.
    pub fn can_still_reach_threshold(&self) -> bool {
        self.max_achievable_percent() >= ACTIVATION_THRESHOLD_PERCENT as f64
    }
}

// ============================================================================
// Deployment Tracker
// ============================================================================

/// Tracks the state and progress of a soft fork deployment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentTracker {
    /// The deployment being tracked.
    pub deployment: SoftForkDeployment,

    /// Current state of the deployment.
    pub state: DeploymentState,

    /// Statistics for each signaling window.
    pub window_stats: HashMap<u64, SignalingWindowStats>,

    /// Total blocks processed.
    pub total_blocks_processed: u32,

    /// Total blocks that signaled support.
    pub total_signaling_blocks: u32,

    /// Overall signaling percentage across all processed blocks.
    pub overall_signaling_percent: f64,

    /// The window where threshold was first reached (if any).
    pub threshold_window: Option<u64>,

    /// Lock-in height (if locked in or active).
    pub locked_in_height: Option<Height>,

    /// Activation height (if locked in or active).
    pub activation_height: Option<Height>,
}

impl DeploymentTracker {
    /// Creates a new deployment tracker.
    pub fn new(deployment: SoftForkDeployment) -> Self {
        Self {
            deployment,
            state: DeploymentState::Defined,
            window_stats: HashMap::new(),
            total_blocks_processed: 0,
            total_signaling_blocks: 0,
            overall_signaling_percent: 0.0,
            threshold_window: None,
            locked_in_height: None,
            activation_height: None,
        }
    }

    /// Records a block and updates the deployment state.
    pub fn record_block(&mut self, height: Height, version: u32) {
        let signal = IndexedBlockSignal::from_version(height, version);
        let signals = signal.signals_deployment(&self.deployment);

        // Update totals
        self.total_blocks_processed += 1;
        if signals {
            self.total_signaling_blocks += 1;
        }
        if self.total_blocks_processed > 0 {
            self.overall_signaling_percent =
                (self.total_signaling_blocks as f64 / self.total_blocks_processed as f64) * 100.0;
        }

        // Skip if before start height
        if height < self.deployment.start_height {
            return;
        }

        // Skip if already failed or active
        if matches!(
            self.state,
            DeploymentState::Active { .. } | DeploymentState::Failed
        ) {
            return;
        }

        // Record in window stats
        let window = window_number(height);
        let stats = self
            .window_stats
            .entry(window)
            .or_insert_with(|| SignalingWindowStats::new(window));
        stats.record_block(height, signals);

        // Check for state transitions
        self.update_state(height);
    }

    /// Updates the deployment state based on current data.
    fn update_state(&mut self, current_height: Height) {
        // If already in locked-in, check for activation
        if let DeploymentState::LockedIn {
            activation_height, ..
        } = self.state
        {
            if current_height >= activation_height {
                self.state = DeploymentState::Active {
                    activated_height: activation_height,
                };
            }
            return;
        }

        // Check for timeout (before checking lock-in to prevent race)
        if current_height >= self.deployment.timeout_height {
            self.state = DeploymentState::Failed;
            return;
        }

        // Before start, stay in Defined
        if current_height < self.deployment.start_height {
            self.state = DeploymentState::Defined;
            return;
        }

        // In started phase, check completed windows for threshold
        self.state = DeploymentState::Started;

        let current_window = window_number(current_height);
        for window in 0..current_window {
            if let Some(stats) = self.window_stats.get(&window) {
                if stats.is_complete && stats.threshold_reached {
                    let locked_in = stats.end_height;
                    let activation = calculate_activation_height(locked_in);

                    self.threshold_window = Some(window);
                    self.locked_in_height = Some(locked_in);
                    self.activation_height = Some(activation);

                    if current_height >= activation {
                        self.state = DeploymentState::Active {
                            activated_height: activation,
                        };
                    } else {
                        self.state = DeploymentState::LockedIn {
                            locked_in_height: locked_in,
                            activation_height: activation,
                        };
                    }
                    return;
                }
            }
        }
    }

    /// Returns the current window being processed.
    pub fn current_window(&self, height: Height) -> Option<&SignalingWindowStats> {
        let window = window_number(height);
        self.window_stats.get(&window)
    }

    /// Returns statistics for a specific window.
    pub fn get_window_stats(&self, window: u64) -> Option<&SignalingWindowStats> {
        self.window_stats.get(&window)
    }

    /// Returns an estimate of blocks until activation (if on track).
    pub fn blocks_until_activation(&self, current_height: Height) -> Option<u32> {
        match &self.state {
            DeploymentState::LockedIn {
                activation_height, ..
            } => Some(activation_height.0.saturating_sub(current_height.0)),
            DeploymentState::Active { .. } => Some(0),
            _ => None,
        }
    }

    /// Returns whether the deployment is still salvageable.
    pub fn is_salvageable(&self, current_height: Height) -> bool {
        if !matches!(self.state, DeploymentState::Started) {
            return matches!(
                self.state,
                DeploymentState::Defined
                    | DeploymentState::LockedIn { .. }
                    | DeploymentState::Active { .. }
            );
        }

        // Check if there are enough blocks remaining to potentially reach threshold
        let remaining_blocks = self
            .deployment
            .timeout_height
            .0
            .saturating_sub(current_height.0);
        let current_window = window_number(current_height);

        if let Some(stats) = self.window_stats.get(&current_window) {
            // If current window can still reach threshold, salvageable
            if stats.can_still_reach_threshold() {
                return true;
            }
        }

        // Check if there's a full window available before timeout
        let windows_until_timeout = remaining_blocks / SIGNALING_WINDOW_BLOCKS;
        windows_until_timeout >= 1
    }
}

// ============================================================================
// Block Statistics
// ============================================================================

/// Per-block protocol upgrade statistics for indexing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockUpgradeStats {
    /// Block height.
    pub height: Height,

    /// Block version.
    pub version: u32,

    /// Whether the block supports signaling.
    pub supports_signaling: bool,

    /// Number of bits actively signaling.
    pub active_signal_count: u8,

    /// List of active signal bits.
    pub active_bits: Vec<u8>,

    /// Window number for this block.
    pub window: u64,

    /// Position within the window (0 to SIGNALING_WINDOW_BLOCKS-1).
    pub window_position: u32,
}

impl Default for BlockUpgradeStats {
    fn default() -> Self {
        Self {
            height: Height(0),
            version: 0,
            supports_signaling: false,
            active_signal_count: 0,
            active_bits: Vec::new(),
            window: 0,
            window_position: 0,
        }
    }
}

impl BlockUpgradeStats {
    /// Creates block statistics from a block header.
    pub fn from_block(height: Height, version: u32) -> Self {
        let signal = IndexedBlockSignal::from_version(height, version);
        let window = window_number(height);
        let window_start = window_start_height(height);
        let window_position = height.0 - window_start.0;

        Self {
            height,
            version,
            supports_signaling: signal.supports_signaling,
            active_signal_count: signal.active_bits.len() as u8,
            active_bits: signal.active_bits,
            window,
            window_position,
        }
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Parses version bit signals from a block version.
///
/// Returns an `IndexedBlockSignal` with all signal information.
pub fn parse_block_signals(height: Height, version: u32) -> IndexedBlockSignal {
    IndexedBlockSignal::from_version(height, version)
}

/// Checks if a version signals support for a specific bit.
pub fn version_signals_bit(version: u32, bit: u8) -> bool {
    if !SIGNALING_BIT_RANGE.contains(&bit) {
        return false;
    }
    supports_signaling(version) && (version & (1u32 << bit)) != 0
}

/// Returns a human-readable description of a deployment state.
pub fn describe_deployment_state(state: &DeploymentState) -> String {
    match state {
        DeploymentState::Defined => "Defined (waiting for start height)".to_string(),
        DeploymentState::Started => "Started (accepting signals)".to_string(),
        DeploymentState::LockedIn {
            locked_in_height,
            activation_height,
        } => format!(
            "Locked-In at {} (activates at {})",
            locked_in_height.0, activation_height.0
        ),
        DeploymentState::Active { activated_height } => {
            format!("Active since height {}", activated_height.0)
        }
        DeploymentState::Failed => "Failed (timeout without activation)".to_string(),
    }
}

/// Returns a progress summary for a deployment.
pub fn deployment_progress_summary(tracker: &DeploymentTracker) -> String {
    let state_desc = describe_deployment_state(&tracker.state);

    match &tracker.state {
        DeploymentState::Started => {
            format!(
                "{}: {} - Overall signaling: {:.1}%, {} blocks processed",
                tracker.deployment.bip_id,
                state_desc,
                tracker.overall_signaling_percent,
                tracker.total_blocks_processed
            )
        }
        DeploymentState::LockedIn {
            activation_height, ..
        } => {
            format!(
                "{}: {} ({}% supported in window {})",
                tracker.deployment.bip_id,
                state_desc,
                tracker.overall_signaling_percent as u32,
                tracker.threshold_window.unwrap_or(0)
            )
        }
        _ => format!("{}: {}", tracker.deployment.bip_id, state_desc),
    }
}

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during protocol upgrade indexing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpgradeIndexError {
    /// Invalid version bit in signal.
    InvalidBit {
        /// The invalid bit number.
        bit: u8,
    },

    /// Block height is out of sequence.
    HeightOutOfSequence {
        /// The expected height.
        expected: Height,
        /// The actual height.
        actual: Height,
    },

    /// Deployment not found.
    DeploymentNotFound {
        /// The BIP ID that wasn't found.
        bip_id: String,
    },
}

impl std::fmt::Display for UpgradeIndexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpgradeIndexError::InvalidBit { bit } => {
                write!(
                    f,
                    "invalid version bit {}: must be in range {:?}",
                    bit, SIGNALING_BIT_RANGE
                )
            }
            UpgradeIndexError::HeightOutOfSequence { expected, actual } => {
                write!(
                    f,
                    "block height out of sequence: expected {}, got {}",
                    expected.0, actual.0
                )
            }
            UpgradeIndexError::DeploymentNotFound { bip_id } => {
                write!(f, "deployment not found: {}", bip_id)
            }
        }
    }
}

impl std::error::Error for UpgradeIndexError {}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_indexed_block_signal_basic() {
        let signal = IndexedBlockSignal::from_version(Height(100), 5);
        assert!(signal.supports_signaling);
        assert_eq!(signal.version, 5);
        assert!(signal.active_bits.is_empty()); // Version 5 has no signaling bits set
    }

    #[test]
    fn test_indexed_block_signal_with_bits() {
        // Version with bits 5 and 7 set: 5 | (1<<5) | (1<<7) = 5 | 32 | 128 = 165
        let version = 5 | (1u32 << 5) | (1u32 << 7);
        let signal = IndexedBlockSignal::from_version(Height(100), version);

        assert!(signal.supports_signaling);
        assert!(signal.signals_bit(5));
        assert!(signal.signals_bit(7));
        assert!(!signal.signals_bit(6));
        assert_eq!(signal.active_bits.len(), 2);
    }

    #[test]
    fn test_indexed_block_signal_no_signaling_support() {
        let signal = IndexedBlockSignal::from_version(Height(100), 4); // Base version
        assert!(!signal.supports_signaling);
        assert!(signal.signals.is_empty());
        assert!(signal.active_bits.is_empty());
    }

    #[test]
    fn test_indexed_block_signal_deployment() {
        let deployment = SoftForkDeployment::new(
            "BSP-001",
            "Test",
            "Test deployment",
            5,
            Height(1000),
            Height(50000),
        );

        let version_signals = 5 | (1u32 << 5);
        let signal = IndexedBlockSignal::from_version(Height(1500), version_signals);
        assert!(signal.signals_deployment(&deployment));

        let version_no_signal = 5;
        let signal = IndexedBlockSignal::from_version(Height(1500), version_no_signal);
        assert!(!signal.signals_deployment(&deployment));
    }

    #[test]
    fn test_signaling_window_stats() {
        let mut stats = SignalingWindowStats::new(0);

        assert_eq!(stats.start_height, Height(0));
        assert_eq!(stats.end_height, Height(999));
        assert!(!stats.is_complete);
        assert!(!stats.threshold_reached);

        // Record 750 signaling blocks (75%)
        for i in 0..750 {
            stats.record_block(Height(i), true);
        }
        // Record 250 non-signaling blocks
        for i in 750..1000 {
            stats.record_block(Height(i), false);
        }

        assert!(stats.is_complete);
        assert!(stats.threshold_reached);
        assert_eq!(stats.total_blocks, 1000);
        assert_eq!(stats.signaling_blocks, 750);
        assert!((stats.signaling_percent - 75.0).abs() < 0.001);
    }

    #[test]
    fn test_signaling_window_stats_threshold_not_reached() {
        let mut stats = SignalingWindowStats::new(0);

        // Record only 700 signaling blocks (70%)
        for i in 0..700 {
            stats.record_block(Height(i), true);
        }
        for i in 700..1000 {
            stats.record_block(Height(i), false);
        }

        assert!(stats.is_complete);
        assert!(!stats.threshold_reached);
        assert!((stats.signaling_percent - 70.0).abs() < 0.001);
    }

    #[test]
    fn test_signaling_window_stats_partial() {
        let mut stats = SignalingWindowStats::new(0);

        // Record only 500 blocks, 400 signaling (80% so far)
        for i in 0..400 {
            stats.record_block(Height(i), true);
        }
        for i in 400..500 {
            stats.record_block(Height(i), false);
        }

        assert!(!stats.is_complete);
        assert!(stats.threshold_reached); // 80% > 75%
        assert_eq!(stats.total_blocks, 500);
        assert!(stats.can_still_reach_threshold());
    }

    #[test]
    fn test_signaling_window_stats_cannot_reach_threshold() {
        let mut stats = SignalingWindowStats::new(0);

        // Record 300 signaling, 500 non-signaling
        for i in 0..300 {
            stats.record_block(Height(i), true);
        }
        for i in 300..800 {
            stats.record_block(Height(i), false);
        }

        // 800 blocks recorded, 300 signaling = 37.5%
        // 200 blocks remaining, max achievable = (300+200)/1000 = 50%
        assert!(!stats.can_still_reach_threshold());
    }

    #[test]
    fn test_deployment_tracker_basic() {
        let deployment = SoftForkDeployment::new(
            "BSP-001",
            "Test Upgrade",
            "A test soft fork",
            5,
            Height(1000),
            Height(50000),
        );

        let mut tracker = DeploymentTracker::new(deployment);

        assert_eq!(tracker.state, DeploymentState::Defined);
        assert_eq!(tracker.total_blocks_processed, 0);
    }

    #[test]
    fn test_deployment_tracker_defined_to_started() {
        let deployment =
            SoftForkDeployment::new("BSP-001", "Test", "Test", 5, Height(1000), Height(50000));

        let mut tracker = DeploymentTracker::new(deployment);

        // Record blocks before start - stays Defined
        for i in 0..500 {
            tracker.record_block(Height(i), 5);
        }
        assert_eq!(tracker.state, DeploymentState::Defined);

        // Record block at start - transitions to Started
        tracker.record_block(Height(1000), 5);
        assert_eq!(tracker.state, DeploymentState::Started);
    }

    #[test]
    fn test_deployment_tracker_full_cycle() {
        let deployment =
            SoftForkDeployment::new("BSP-001", "Test", "Test", 5, Height(1000), Height(50000));

        let mut tracker = DeploymentTracker::new(deployment.clone());
        let signaling_version = 5 | (1u32 << 5);

        // Record a full window with 80% support (window 1: 1000-1999)
        for i in 1000..1800 {
            tracker.record_block(Height(i), signaling_version);
        }
        for i in 1800..2000 {
            tracker.record_block(Height(i), 5); // No signaling
        }

        // After window 1 complete, check for lock-in at window 2
        tracker.record_block(Height(2000), 5);

        match tracker.state {
            DeploymentState::LockedIn {
                locked_in_height,
                activation_height,
            } => {
                assert_eq!(locked_in_height, Height(1999));
                assert!(activation_height > locked_in_height);
            }
            _ => panic!("Expected LockedIn state, got {:?}", tracker.state),
        }
    }

    #[test]
    fn test_deployment_tracker_timeout() {
        let deployment = SoftForkDeployment::new(
            "BSP-001",
            "Test",
            "Test",
            5,
            Height(1000),
            Height(5000), // Short timeout for testing
        );

        let mut tracker = DeploymentTracker::new(deployment);

        // Record blocks without signaling until timeout
        for i in 1000..5001 {
            tracker.record_block(Height(i), 5); // No signaling
        }

        assert_eq!(tracker.state, DeploymentState::Failed);
    }

    #[test]
    fn test_block_upgrade_stats() {
        let version = 5 | (1u32 << 5) | (1u32 << 10);
        let stats = BlockUpgradeStats::from_block(Height(1500), version);

        assert_eq!(stats.height, Height(1500));
        assert!(stats.supports_signaling);
        assert_eq!(stats.active_signal_count, 2);
        assert!(stats.active_bits.contains(&5));
        assert!(stats.active_bits.contains(&10));
        assert_eq!(stats.window, 1);
        assert_eq!(stats.window_position, 500);
    }

    #[test]
    fn test_version_signals_bit() {
        let version = 5 | (1u32 << 7);

        assert!(version_signals_bit(version, 7));
        assert!(!version_signals_bit(version, 5));
        assert!(!version_signals_bit(version, 2)); // Outside range
        assert!(!version_signals_bit(4, 7)); // Base version doesn't support signaling
    }

    #[test]
    fn test_describe_deployment_state() {
        assert!(describe_deployment_state(&DeploymentState::Defined).contains("Defined"));
        assert!(describe_deployment_state(&DeploymentState::Started).contains("Started"));
        assert!(describe_deployment_state(&DeploymentState::Failed).contains("Failed"));

        let locked_in = DeploymentState::LockedIn {
            locked_in_height: Height(1000),
            activation_height: Height(3000),
        };
        let desc = describe_deployment_state(&locked_in);
        assert!(desc.contains("Locked-In"));
        assert!(desc.contains("1000"));
        assert!(desc.contains("3000"));
    }

    #[test]
    fn test_deployment_progress_summary() {
        let deployment =
            SoftForkDeployment::new("BSP-001", "Test", "Test", 5, Height(1000), Height(50000));

        let tracker = DeploymentTracker::new(deployment);
        let summary = deployment_progress_summary(&tracker);

        assert!(summary.contains("BSP-001"));
        assert!(summary.contains("Defined"));
    }

    #[test]
    fn test_upgrade_index_error_display() {
        let err = UpgradeIndexError::InvalidBit { bit: 2 };
        assert!(err.to_string().contains("invalid version bit 2"));

        let err = UpgradeIndexError::HeightOutOfSequence {
            expected: Height(100),
            actual: Height(99),
        };
        assert!(err.to_string().contains("100"));
        assert!(err.to_string().contains("99"));

        let err = UpgradeIndexError::DeploymentNotFound {
            bip_id: "BSP-001".to_string(),
        };
        assert!(err.to_string().contains("BSP-001"));
    }

    #[test]
    fn test_is_salvageable() {
        let deployment =
            SoftForkDeployment::new("BSP-001", "Test", "Test", 5, Height(1000), Height(50000));

        let mut tracker = DeploymentTracker::new(deployment);

        // Before start, salvageable (still defined)
        assert!(tracker.is_salvageable(Height(500)));

        // At start, salvageable
        tracker.record_block(Height(1000), 5);
        assert!(tracker.is_salvageable(Height(1000)));

        // Near timeout with poor signaling, may not be salvageable
        tracker.record_block(Height(49000), 5);
        // Depends on remaining time
    }

    #[test]
    fn test_window_calculation_consistency() {
        for height in [0, 999, 1000, 1500, 1999, 2000, 10000] {
            let h = Height(height);
            let window = window_number(h);
            let start = window_start_height(h);
            let end = window_end_height(h);

            assert!(start.0 <= h.0, "Start should be <= height");
            assert!(end.0 >= h.0, "End should be >= height");
            assert_eq!(
                end.0 - start.0 + 1,
                SIGNALING_WINDOW_BLOCKS,
                "Window should span exactly SIGNALING_WINDOW_BLOCKS"
            );
            assert_eq!(
                window,
                start.0 as u64 / SIGNALING_WINDOW_BLOCKS as u64,
                "Window number should match start/window_size"
            );
        }
    }
}
