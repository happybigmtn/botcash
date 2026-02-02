//! Indexer moderation parsing and trust/report utilities.
//!
//! This module provides utilities for indexers to:
//! - Parse trust messages (0xD0) from transaction memos
//! - Parse report messages (0xD1) from transaction memos
//! - Calculate trust scores using web of trust algorithms
//! - Track report status and stake handling
//!
//! # Overview
//!
//! Moderation in Botcash happens at the view layer, not the data layer:
//! - All content remains on-chain immutably
//! - Users/indexers filter based on trust relationships and reports
//! - Trust propagates through the social graph with decay
//! - Reports require stake to prevent spam (false reports forfeit stake)
//!
//! # Trust System (0xD0)
//!
//! Users can mark others as trusted, neutral, or distrusted:
//! ```text
//! trust_score = Σ(incoming_trusts) - Σ(incoming_distrusts)
//! ```
//!
//! Trust propagates transitively with decay:
//! ```text
//! transitive_trust = direct_trust * TRUST_DECAY_FACTOR^depth
//! ```
//!
//! # Report System (0xD1)
//!
//! Stake-weighted reports for content moderation:
//! - Reports require minimum 0.01 BCASH stake
//! - Valid reports return stake + reward
//! - False reports forfeit stake to the reported address
//!
//! # Usage
//!
//! ```ignore
//! use zebra_chain::transaction::Memo;
//! use zebra_rpc::indexer::moderation::{parse_moderation_memo, IndexedModeration};
//!
//! let mod_event = parse_moderation_memo(&memo, "txid123", 1000)?;
//! match mod_event {
//!     IndexedModeration::Trust(trust) => {
//!         println!("Trust: {} -> {} = {:?}", trust.from_address, trust.target_address, trust.level);
//!     }
//!     IndexedModeration::Report(report) => {
//!         println!("Report: {} on tx {} ({:?})", report.reporter_address, report.target_txid, report.category);
//!     }
//! }
//! ```

use std::fmt;

use serde::{Deserialize, Serialize};
use zebra_chain::transaction::{
    social::{
        ReportCategory as ChainReportCategory, ReportMessage, SocialMessage, SocialMessageType,
        SocialParseError, TrustLevel as ChainTrustLevel, TrustMessage,
    },
    Memo,
};

/// Trust decay factor for transitive trust propagation.
/// Each hop reduces trust by this factor (0.7 = 30% decay per hop).
pub const TRUST_DECAY_FACTOR: f64 = 0.7;

/// Maximum depth for transitive trust propagation.
pub const MAX_TRUST_DEPTH: u32 = 3;

/// Minimum stake required for reports (in zatoshis = 0.01 BCASH).
pub const MIN_REPORT_STAKE: u64 = 1_000_000;

/// Report expiration in blocks (~30 days at 60s blocks).
pub const REPORT_EXPIRATION_BLOCKS: u32 = 43200;

/// Trust level for moderation relationships.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TrustLevel {
    /// Actively distrust this address.
    Distrust = 0,
    /// Neutral/unknown relationship.
    Neutral = 1,
    /// Trust this address.
    Trusted = 2,
}

impl TrustLevel {
    /// Creates a TrustLevel from a byte value.
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Distrust),
            1 => Some(Self::Neutral),
            2 => Some(Self::Trusted),
            _ => None,
        }
    }

    /// Returns the byte value of this trust level.
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Returns the signed value for trust score calculation.
    /// Distrust = -1, Neutral = 0, Trusted = +1
    pub fn score_value(self) -> i64 {
        match self {
            Self::Distrust => -1,
            Self::Neutral => 0,
            Self::Trusted => 1,
        }
    }
}

impl From<ChainTrustLevel> for TrustLevel {
    fn from(chain_level: ChainTrustLevel) -> Self {
        match chain_level {
            ChainTrustLevel::Distrust => Self::Distrust,
            ChainTrustLevel::Neutral => Self::Neutral,
            ChainTrustLevel::Trusted => Self::Trusted,
        }
    }
}

impl Default for TrustLevel {
    fn default() -> Self {
        Self::Neutral
    }
}

impl fmt::Display for TrustLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Distrust => write!(f, "distrust"),
            Self::Neutral => write!(f, "neutral"),
            Self::Trusted => write!(f, "trusted"),
        }
    }
}

/// Report category for content moderation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReportCategory {
    /// Spam content.
    Spam = 0,
    /// Scam/fraud.
    Scam = 1,
    /// Harassment/abuse.
    Harassment = 2,
    /// Potentially illegal content.
    Illegal = 3,
    /// Other category.
    Other = 4,
}

impl ReportCategory {
    /// Creates a ReportCategory from a byte value.
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Spam),
            1 => Some(Self::Scam),
            2 => Some(Self::Harassment),
            3 => Some(Self::Illegal),
            4 => Some(Self::Other),
            _ => None,
        }
    }

    /// Returns the byte value of this report category.
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Returns whether this category requires immediate filtering.
    pub fn requires_immediate_filtering(self) -> bool {
        matches!(self, Self::Illegal)
    }

    /// Returns the default stake multiplier for this category.
    /// Higher multipliers = higher potential reward for valid reports.
    pub fn stake_multiplier(self) -> f64 {
        match self {
            Self::Spam => 1.0,
            Self::Scam => 1.5,
            Self::Harassment => 1.2,
            Self::Illegal => 2.0,
            Self::Other => 1.0,
        }
    }
}

impl From<ChainReportCategory> for ReportCategory {
    fn from(chain_cat: ChainReportCategory) -> Self {
        match chain_cat {
            ChainReportCategory::Spam => Self::Spam,
            ChainReportCategory::Scam => Self::Scam,
            ChainReportCategory::Harassment => Self::Harassment,
            ChainReportCategory::Illegal => Self::Illegal,
            ChainReportCategory::Other => Self::Other,
        }
    }
}

impl Default for ReportCategory {
    fn default() -> Self {
        Self::Other
    }
}

impl fmt::Display for ReportCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spam => write!(f, "spam"),
            Self::Scam => write!(f, "scam"),
            Self::Harassment => write!(f, "harassment"),
            Self::Illegal => write!(f, "illegal"),
            Self::Other => write!(f, "other"),
        }
    }
}

/// Report status based on validation state.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ReportStatus {
    /// Report is pending review.
    #[default]
    Pending,
    /// Report has been validated (stake returned + reward).
    Validated,
    /// Report was rejected (stake forfeited).
    Rejected,
    /// Report expired without resolution.
    Expired,
}

impl ReportStatus {
    /// Returns the string representation of this status.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Validated => "validated",
            Self::Rejected => "rejected",
            Self::Expired => "expired",
        }
    }

    /// Returns true if the report is still active (not resolved).
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Pending)
    }
}

impl fmt::Display for ReportStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// An indexed trust relationship extracted from a memo.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedTrust {
    /// The transaction ID containing this trust.
    pub tx_id: String,

    /// Block height where this trust was created.
    pub block_height: u32,

    /// The address giving the trust (truster).
    pub from_address: String,

    /// The address receiving the trust (trustee).
    pub target_address: String,

    /// The trust level assigned.
    pub level: TrustLevel,

    /// Optional reason for the trust decision.
    pub reason: Option<String>,

    /// Protocol version.
    pub version: u8,
}

impl IndexedTrust {
    /// Creates a new indexed trust from parsed data.
    pub fn new(
        tx_id: &str,
        block_height: u32,
        from_address: String,
        target_address: String,
        level: TrustLevel,
        reason: Option<String>,
        version: u8,
    ) -> Self {
        Self {
            tx_id: tx_id.to_string(),
            block_height,
            from_address,
            target_address,
            level,
            reason,
            version,
        }
    }

    /// Returns the score contribution of this trust.
    pub fn score_contribution(&self) -> i64 {
        self.level.score_value()
    }
}

impl fmt::Display for IndexedTrust {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Trust {{ {} -> {}: {} }}",
            &self.from_address[..8.min(self.from_address.len())],
            &self.target_address[..8.min(self.target_address.len())],
            self.level
        )
    }
}

/// An indexed report extracted from a memo.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedReport {
    /// The transaction ID containing this report.
    pub tx_id: String,

    /// Block height where this report was submitted.
    pub block_height: u32,

    /// The reporter's address.
    pub reporter_address: String,

    /// The transaction ID of the reported content.
    pub target_txid: String,

    /// The report category.
    pub category: ReportCategory,

    /// The stake amount in zatoshis.
    pub stake: u64,

    /// Optional evidence text.
    pub evidence: Option<String>,

    /// Protocol version.
    pub version: u8,

    /// Current report status (computed by indexer).
    pub status: ReportStatus,
}

impl IndexedReport {
    /// Creates a new indexed report from parsed data.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        tx_id: &str,
        block_height: u32,
        reporter_address: String,
        target_txid: String,
        category: ReportCategory,
        stake: u64,
        evidence: Option<String>,
        version: u8,
    ) -> Self {
        Self {
            tx_id: tx_id.to_string(),
            block_height,
            reporter_address,
            target_txid,
            category,
            stake,
            evidence,
            version,
            status: ReportStatus::Pending,
        }
    }

    /// Returns the block height when this report expires.
    pub fn expiration_block(&self) -> u32 {
        self.block_height.saturating_add(REPORT_EXPIRATION_BLOCKS)
    }

    /// Returns the status of this report at a given block height.
    pub fn status_at(&self, current_height: u32) -> ReportStatus {
        if self.status != ReportStatus::Pending {
            return self.status;
        }
        if current_height >= self.expiration_block() {
            ReportStatus::Expired
        } else {
            ReportStatus::Pending
        }
    }

    /// Returns true if this report is still active at the given height.
    pub fn is_active_at(&self, current_height: u32) -> bool {
        self.status_at(current_height).is_active()
    }

    /// Returns the potential reward for a valid report.
    pub fn potential_reward(&self) -> u64 {
        let multiplier = self.category.stake_multiplier();
        (self.stake as f64 * (multiplier - 1.0)) as u64
    }
}

impl fmt::Display for IndexedReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Report {{ {} on {}...: {} ({} zatoshi) }}",
            &self.reporter_address[..8.min(self.reporter_address.len())],
            &self.target_txid[..8.min(self.target_txid.len())],
            self.category,
            self.stake
        )
    }
}

/// Unified moderation event type for indexing.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum IndexedModeration {
    /// A trust relationship.
    Trust(IndexedTrust),
    /// A content report.
    Report(IndexedReport),
}

impl IndexedModeration {
    /// Returns the transaction ID for this moderation event.
    pub fn tx_id(&self) -> &str {
        match self {
            Self::Trust(t) => &t.tx_id,
            Self::Report(r) => &r.tx_id,
        }
    }

    /// Returns the block height for this moderation event.
    pub fn block_height(&self) -> u32 {
        match self {
            Self::Trust(t) => t.block_height,
            Self::Report(r) => r.block_height,
        }
    }

    /// Returns true if this is a trust event.
    pub fn is_trust(&self) -> bool {
        matches!(self, Self::Trust(_))
    }

    /// Returns true if this is a report event.
    pub fn is_report(&self) -> bool {
        matches!(self, Self::Report(_))
    }
}

impl fmt::Display for IndexedModeration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Trust(t) => write!(f, "{}", t),
            Self::Report(r) => write!(f, "{}", r),
        }
    }
}

/// Error type for moderation indexing operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModerationIndexError {
    /// The memo is not a moderation message.
    NotModeration,
    /// The transaction ID is invalid.
    InvalidTxId,
    /// Failed to parse the social message.
    ParseError(SocialParseError),
    /// The payload is malformed.
    MalformedPayload(String),
    /// The stake amount is below minimum.
    InsufficientStake {
        /// The provided stake.
        provided: u64,
        /// The minimum required.
        required: u64,
    },
}

impl fmt::Display for ModerationIndexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotModeration => write!(f, "memo is not a moderation message"),
            Self::InvalidTxId => write!(f, "invalid transaction ID"),
            Self::ParseError(e) => write!(f, "parse error: {}", e),
            Self::MalformedPayload(msg) => write!(f, "malformed payload: {}", msg),
            Self::InsufficientStake { provided, required } => {
                write!(
                    f,
                    "insufficient stake: {} provided, {} required",
                    provided, required
                )
            }
        }
    }
}

impl std::error::Error for ModerationIndexError {}

impl From<SocialParseError> for ModerationIndexError {
    fn from(err: SocialParseError) -> Self {
        Self::ParseError(err)
    }
}

/// Checks if a memo is a moderation message (Trust or Report).
pub fn is_moderation_memo(memo: &Memo) -> bool {
    let bytes = memo.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    let msg_type = bytes[0];
    msg_type == SocialMessageType::Trust as u8 || msg_type == SocialMessageType::Report as u8
}

/// Parses a moderation memo into an indexed moderation event.
///
/// # Arguments
///
/// * `memo` - The transaction memo to parse
/// * `tx_id` - The transaction ID (used for indexing)
/// * `block_height` - The block height of this transaction
/// * `from_address` - The sender's address (for trust messages)
///
/// # Returns
///
/// An `IndexedModeration` event if successful, or a `ModerationIndexError`.
pub fn parse_moderation_memo(
    memo: &Memo,
    tx_id: &str,
    block_height: u32,
    from_address: &str,
) -> Result<IndexedModeration, ModerationIndexError> {
    // Validate tx_id
    if tx_id.is_empty() {
        return Err(ModerationIndexError::InvalidTxId);
    }

    // Quick check for moderation type
    if !is_moderation_memo(memo) {
        return Err(ModerationIndexError::NotModeration);
    }

    // Parse the social message
    let msg = SocialMessage::try_from(memo)?;
    let version = msg.version();
    let payload = msg.payload();

    match msg.msg_type() {
        SocialMessageType::Trust => {
            let trust = parse_trust_payload(payload, tx_id, block_height, from_address, version)?;
            Ok(IndexedModeration::Trust(trust))
        }
        SocialMessageType::Report => {
            let report = parse_report_payload(payload, tx_id, block_height, from_address, version)?;
            Ok(IndexedModeration::Report(report))
        }
        _ => Err(ModerationIndexError::NotModeration),
    }
}

/// Parses a trust message payload.
fn parse_trust_payload(
    payload: &[u8],
    tx_id: &str,
    block_height: u32,
    from_address: &str,
    version: u8,
) -> Result<IndexedTrust, ModerationIndexError> {
    let trust_msg = TrustMessage::parse(payload).map_err(|e| {
        ModerationIndexError::MalformedPayload(format!("failed to parse trust: {}", e))
    })?;

    Ok(IndexedTrust::new(
        tx_id,
        block_height,
        from_address.to_string(),
        trust_msg.target_address().to_string(),
        trust_msg.level().into(),
        trust_msg.reason().map(String::from),
        version,
    ))
}

/// Parses a report message payload.
fn parse_report_payload(
    payload: &[u8],
    tx_id: &str,
    block_height: u32,
    from_address: &str,
    version: u8,
) -> Result<IndexedReport, ModerationIndexError> {
    let report_msg = ReportMessage::parse(payload).map_err(|e| {
        ModerationIndexError::MalformedPayload(format!("failed to parse report: {}", e))
    })?;

    let stake = report_msg.stake();
    if stake < MIN_REPORT_STAKE {
        return Err(ModerationIndexError::InsufficientStake {
            provided: stake,
            required: MIN_REPORT_STAKE,
        });
    }

    Ok(IndexedReport::new(
        tx_id,
        block_height,
        from_address.to_string(),
        hex::encode(report_msg.target_txid()),
        report_msg.category().into(),
        stake,
        report_msg.evidence().map(String::from),
        version,
    ))
}

/// Calculates the trust score for an address from a list of incoming trusts.
///
/// # Arguments
///
/// * `trusts` - List of incoming trust relationships
///
/// # Returns
///
/// The calculated trust score (positive = net trusted, negative = net distrusted).
pub fn calculate_trust_score(trusts: &[IndexedTrust]) -> i64 {
    trusts.iter().map(|t| t.score_contribution()).sum()
}

/// Calculates transitive trust from a source to a target via an intermediary.
///
/// # Arguments
///
/// * `direct_trust` - The direct trust level from intermediary to target
/// * `intermediary_trust` - The trust score of the intermediary (from source's perspective)
/// * `depth` - The depth of the trust chain (1 = direct via intermediary)
///
/// # Returns
///
/// The transitive trust contribution (can be negative for distrust chains).
pub fn calculate_transitive_trust(
    direct_trust: TrustLevel,
    intermediary_trust: i64,
    depth: u32,
) -> f64 {
    if depth > MAX_TRUST_DEPTH || intermediary_trust == 0 {
        return 0.0;
    }

    let decay = TRUST_DECAY_FACTOR.powi(depth as i32);
    let trust_value = direct_trust.score_value() as f64;
    let intermediary_weight = (intermediary_trust as f64).signum(); // +1 or -1

    trust_value * intermediary_weight * decay
}

/// Aggregates report counts by category for a target.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportAggregation {
    /// Total number of reports.
    pub total_reports: u32,
    /// Total stake across all reports.
    pub total_stake: u64,
    /// Count by category.
    pub by_category: std::collections::HashMap<ReportCategory, u32>,
    /// Count by status.
    pub by_status: std::collections::HashMap<ReportStatus, u32>,
}

impl ReportAggregation {
    /// Creates a new empty aggregation.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a report to the aggregation.
    pub fn add_report(&mut self, report: &IndexedReport) {
        self.total_reports += 1;
        self.total_stake = self.total_stake.saturating_add(report.stake);
        *self.by_category.entry(report.category).or_insert(0) += 1;
        *self.by_status.entry(report.status).or_insert(0) += 1;
    }

    /// Returns true if there are any reports requiring immediate attention.
    pub fn has_immediate_reports(&self) -> bool {
        self.by_category
            .get(&ReportCategory::Illegal)
            .copied()
            .unwrap_or(0)
            > 0
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use zebra_chain::transaction::social::SOCIAL_PROTOCOL_VERSION;

    fn create_memo(bytes: &[u8]) -> Memo {
        Memo::try_from(bytes).expect("valid memo bytes")
    }

    fn create_social_memo(msg_type: SocialMessageType, payload: &[u8]) -> Memo {
        let msg = SocialMessage::new(msg_type, SOCIAL_PROTOCOL_VERSION, payload.to_vec());
        let encoded = msg.encode();
        create_memo(&encoded)
    }

    // ========================================================================
    // Tests for TrustLevel
    // ========================================================================

    #[test]
    fn test_trust_level_from_u8() {
        assert_eq!(TrustLevel::from_u8(0), Some(TrustLevel::Distrust));
        assert_eq!(TrustLevel::from_u8(1), Some(TrustLevel::Neutral));
        assert_eq!(TrustLevel::from_u8(2), Some(TrustLevel::Trusted));
        assert_eq!(TrustLevel::from_u8(3), None);
        assert_eq!(TrustLevel::from_u8(255), None);
    }

    #[test]
    fn test_trust_level_as_u8() {
        assert_eq!(TrustLevel::Distrust.as_u8(), 0);
        assert_eq!(TrustLevel::Neutral.as_u8(), 1);
        assert_eq!(TrustLevel::Trusted.as_u8(), 2);
    }

    #[test]
    fn test_trust_level_score_value() {
        assert_eq!(TrustLevel::Distrust.score_value(), -1);
        assert_eq!(TrustLevel::Neutral.score_value(), 0);
        assert_eq!(TrustLevel::Trusted.score_value(), 1);
    }

    #[test]
    fn test_trust_level_display() {
        assert_eq!(format!("{}", TrustLevel::Distrust), "distrust");
        assert_eq!(format!("{}", TrustLevel::Neutral), "neutral");
        assert_eq!(format!("{}", TrustLevel::Trusted), "trusted");
    }

    #[test]
    fn test_trust_level_default() {
        assert_eq!(TrustLevel::default(), TrustLevel::Neutral);
    }

    // ========================================================================
    // Tests for ReportCategory
    // ========================================================================

    #[test]
    fn test_report_category_from_u8() {
        assert_eq!(ReportCategory::from_u8(0), Some(ReportCategory::Spam));
        assert_eq!(ReportCategory::from_u8(1), Some(ReportCategory::Scam));
        assert_eq!(ReportCategory::from_u8(2), Some(ReportCategory::Harassment));
        assert_eq!(ReportCategory::from_u8(3), Some(ReportCategory::Illegal));
        assert_eq!(ReportCategory::from_u8(4), Some(ReportCategory::Other));
        assert_eq!(ReportCategory::from_u8(5), None);
    }

    #[test]
    fn test_report_category_as_u8() {
        assert_eq!(ReportCategory::Spam.as_u8(), 0);
        assert_eq!(ReportCategory::Scam.as_u8(), 1);
        assert_eq!(ReportCategory::Harassment.as_u8(), 2);
        assert_eq!(ReportCategory::Illegal.as_u8(), 3);
        assert_eq!(ReportCategory::Other.as_u8(), 4);
    }

    #[test]
    fn test_report_category_requires_immediate_filtering() {
        assert!(!ReportCategory::Spam.requires_immediate_filtering());
        assert!(!ReportCategory::Scam.requires_immediate_filtering());
        assert!(!ReportCategory::Harassment.requires_immediate_filtering());
        assert!(ReportCategory::Illegal.requires_immediate_filtering());
        assert!(!ReportCategory::Other.requires_immediate_filtering());
    }

    #[test]
    fn test_report_category_stake_multiplier() {
        assert!((ReportCategory::Spam.stake_multiplier() - 1.0).abs() < f64::EPSILON);
        assert!((ReportCategory::Scam.stake_multiplier() - 1.5).abs() < f64::EPSILON);
        assert!((ReportCategory::Harassment.stake_multiplier() - 1.2).abs() < f64::EPSILON);
        assert!((ReportCategory::Illegal.stake_multiplier() - 2.0).abs() < f64::EPSILON);
        assert!((ReportCategory::Other.stake_multiplier() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_report_category_display() {
        assert_eq!(format!("{}", ReportCategory::Spam), "spam");
        assert_eq!(format!("{}", ReportCategory::Scam), "scam");
        assert_eq!(format!("{}", ReportCategory::Harassment), "harassment");
        assert_eq!(format!("{}", ReportCategory::Illegal), "illegal");
        assert_eq!(format!("{}", ReportCategory::Other), "other");
    }

    // ========================================================================
    // Tests for ReportStatus
    // ========================================================================

    #[test]
    fn test_report_status_as_str() {
        assert_eq!(ReportStatus::Pending.as_str(), "pending");
        assert_eq!(ReportStatus::Validated.as_str(), "validated");
        assert_eq!(ReportStatus::Rejected.as_str(), "rejected");
        assert_eq!(ReportStatus::Expired.as_str(), "expired");
    }

    #[test]
    fn test_report_status_is_active() {
        assert!(ReportStatus::Pending.is_active());
        assert!(!ReportStatus::Validated.is_active());
        assert!(!ReportStatus::Rejected.is_active());
        assert!(!ReportStatus::Expired.is_active());
    }

    #[test]
    fn test_report_status_default() {
        assert_eq!(ReportStatus::default(), ReportStatus::Pending);
    }

    // ========================================================================
    // Tests for IndexedTrust
    // ========================================================================

    #[test]
    fn test_indexed_trust_new() {
        let trust = IndexedTrust::new(
            "txid123",
            1000,
            "bs1alice".to_string(),
            "bs1bob".to_string(),
            TrustLevel::Trusted,
            Some("Great developer".to_string()),
            1,
        );

        assert_eq!(trust.tx_id, "txid123");
        assert_eq!(trust.block_height, 1000);
        assert_eq!(trust.from_address, "bs1alice");
        assert_eq!(trust.target_address, "bs1bob");
        assert_eq!(trust.level, TrustLevel::Trusted);
        assert_eq!(trust.reason, Some("Great developer".to_string()));
        assert_eq!(trust.version, 1);
    }

    #[test]
    fn test_indexed_trust_score_contribution() {
        let trust_trusted = IndexedTrust::new(
            "tx1",
            100,
            "a".to_string(),
            "b".to_string(),
            TrustLevel::Trusted,
            None,
            1,
        );
        let trust_distrust = IndexedTrust::new(
            "tx2",
            100,
            "a".to_string(),
            "b".to_string(),
            TrustLevel::Distrust,
            None,
            1,
        );
        let trust_neutral = IndexedTrust::new(
            "tx3",
            100,
            "a".to_string(),
            "b".to_string(),
            TrustLevel::Neutral,
            None,
            1,
        );

        assert_eq!(trust_trusted.score_contribution(), 1);
        assert_eq!(trust_distrust.score_contribution(), -1);
        assert_eq!(trust_neutral.score_contribution(), 0);
    }

    #[test]
    fn test_indexed_trust_display() {
        let trust = IndexedTrust::new(
            "txid123",
            1000,
            "bs1alice".to_string(),
            "bs1bob".to_string(),
            TrustLevel::Trusted,
            None,
            1,
        );
        let display = format!("{}", trust);
        assert!(display.contains("bs1alice"));
        assert!(display.contains("bs1bob"));
        assert!(display.contains("trusted"));
    }

    // ========================================================================
    // Tests for IndexedReport
    // ========================================================================

    #[test]
    fn test_indexed_report_new() {
        let report = IndexedReport::new(
            "reporttxid",
            2000,
            "bs1reporter".to_string(),
            "contenttxid".to_string(),
            ReportCategory::Spam,
            MIN_REPORT_STAKE,
            Some("Repeated spam".to_string()),
            1,
        );

        assert_eq!(report.tx_id, "reporttxid");
        assert_eq!(report.block_height, 2000);
        assert_eq!(report.reporter_address, "bs1reporter");
        assert_eq!(report.target_txid, "contenttxid");
        assert_eq!(report.category, ReportCategory::Spam);
        assert_eq!(report.stake, MIN_REPORT_STAKE);
        assert_eq!(report.evidence, Some("Repeated spam".to_string()));
        assert_eq!(report.status, ReportStatus::Pending);
    }

    #[test]
    fn test_indexed_report_expiration_block() {
        let report = IndexedReport::new(
            "tx",
            1000,
            "a".to_string(),
            "b".to_string(),
            ReportCategory::Spam,
            MIN_REPORT_STAKE,
            None,
            1,
        );
        assert_eq!(report.expiration_block(), 1000 + REPORT_EXPIRATION_BLOCKS);
    }

    #[test]
    fn test_indexed_report_status_at() {
        let report = IndexedReport::new(
            "tx",
            1000,
            "a".to_string(),
            "b".to_string(),
            ReportCategory::Spam,
            MIN_REPORT_STAKE,
            None,
            1,
        );

        // Before expiration
        assert_eq!(report.status_at(1000), ReportStatus::Pending);
        assert_eq!(
            report.status_at(1000 + REPORT_EXPIRATION_BLOCKS - 1),
            ReportStatus::Pending
        );

        // At/after expiration
        assert_eq!(
            report.status_at(1000 + REPORT_EXPIRATION_BLOCKS),
            ReportStatus::Expired
        );
        assert_eq!(
            report.status_at(1000 + REPORT_EXPIRATION_BLOCKS + 100),
            ReportStatus::Expired
        );
    }

    #[test]
    fn test_indexed_report_is_active_at() {
        let report = IndexedReport::new(
            "tx",
            1000,
            "a".to_string(),
            "b".to_string(),
            ReportCategory::Spam,
            MIN_REPORT_STAKE,
            None,
            1,
        );

        assert!(report.is_active_at(1000));
        assert!(report.is_active_at(1000 + REPORT_EXPIRATION_BLOCKS - 1));
        assert!(!report.is_active_at(1000 + REPORT_EXPIRATION_BLOCKS));
    }

    #[test]
    fn test_indexed_report_potential_reward() {
        let spam_report = IndexedReport::new(
            "tx",
            1000,
            "a".to_string(),
            "b".to_string(),
            ReportCategory::Spam,
            MIN_REPORT_STAKE,
            None,
            1,
        );
        // Spam has 1.0x multiplier, so reward = stake * (1.0 - 1.0) = 0
        assert_eq!(spam_report.potential_reward(), 0);

        let scam_report = IndexedReport::new(
            "tx",
            1000,
            "a".to_string(),
            "b".to_string(),
            ReportCategory::Scam,
            2_000_000,
            None,
            1,
        );
        // Scam has 1.5x multiplier, so reward = 2M * 0.5 = 1M
        assert_eq!(scam_report.potential_reward(), 1_000_000);

        let illegal_report = IndexedReport::new(
            "tx",
            1000,
            "a".to_string(),
            "b".to_string(),
            ReportCategory::Illegal,
            MIN_REPORT_STAKE,
            None,
            1,
        );
        // Illegal has 2.0x multiplier, so reward = 1M * 1.0 = 1M
        assert_eq!(illegal_report.potential_reward(), 1_000_000);
    }

    // ========================================================================
    // Tests for IndexedModeration
    // ========================================================================

    #[test]
    fn test_indexed_moderation_trust() {
        let trust = IndexedTrust::new(
            "txid",
            100,
            "a".to_string(),
            "b".to_string(),
            TrustLevel::Trusted,
            None,
            1,
        );
        let moderation = IndexedModeration::Trust(trust.clone());

        assert_eq!(moderation.tx_id(), "txid");
        assert_eq!(moderation.block_height(), 100);
        assert!(moderation.is_trust());
        assert!(!moderation.is_report());
    }

    #[test]
    fn test_indexed_moderation_report() {
        let report = IndexedReport::new(
            "reporttxid",
            200,
            "a".to_string(),
            "b".to_string(),
            ReportCategory::Spam,
            MIN_REPORT_STAKE,
            None,
            1,
        );
        let moderation = IndexedModeration::Report(report.clone());

        assert_eq!(moderation.tx_id(), "reporttxid");
        assert_eq!(moderation.block_height(), 200);
        assert!(!moderation.is_trust());
        assert!(moderation.is_report());
    }

    // ========================================================================
    // Tests for ModerationIndexError
    // ========================================================================

    #[test]
    fn test_moderation_index_error_display() {
        assert_eq!(
            format!("{}", ModerationIndexError::NotModeration),
            "memo is not a moderation message"
        );
        assert_eq!(
            format!("{}", ModerationIndexError::InvalidTxId),
            "invalid transaction ID"
        );
        assert_eq!(
            format!(
                "{}",
                ModerationIndexError::InsufficientStake {
                    provided: 500_000,
                    required: 1_000_000
                }
            ),
            "insufficient stake: 500000 provided, 1000000 required"
        );
    }

    // ========================================================================
    // Tests for is_moderation_memo
    // ========================================================================

    #[test]
    fn test_is_moderation_memo_trust() {
        let memo = create_social_memo(SocialMessageType::Trust, &[]);
        assert!(is_moderation_memo(&memo));
    }

    #[test]
    fn test_is_moderation_memo_report() {
        let memo = create_social_memo(SocialMessageType::Report, &[]);
        assert!(is_moderation_memo(&memo));
    }

    #[test]
    fn test_is_moderation_memo_other_types() {
        // Test with a Post message - should not be moderation
        let memo = create_social_memo(SocialMessageType::Post, &[]);
        assert!(!is_moderation_memo(&memo));

        // Test with empty memo
        let empty_memo = Memo::try_from(&[][..]).unwrap();
        assert!(!is_moderation_memo(&empty_memo));
    }

    // ========================================================================
    // Tests for calculate_trust_score
    // ========================================================================

    #[test]
    fn test_calculate_trust_score_empty() {
        let trusts: Vec<IndexedTrust> = vec![];
        assert_eq!(calculate_trust_score(&trusts), 0);
    }

    #[test]
    fn test_calculate_trust_score_single_trust() {
        let trusts = vec![IndexedTrust::new(
            "tx",
            100,
            "a".to_string(),
            "b".to_string(),
            TrustLevel::Trusted,
            None,
            1,
        )];
        assert_eq!(calculate_trust_score(&trusts), 1);
    }

    #[test]
    fn test_calculate_trust_score_single_distrust() {
        let trusts = vec![IndexedTrust::new(
            "tx",
            100,
            "a".to_string(),
            "b".to_string(),
            TrustLevel::Distrust,
            None,
            1,
        )];
        assert_eq!(calculate_trust_score(&trusts), -1);
    }

    #[test]
    fn test_calculate_trust_score_mixed() {
        let trusts = vec![
            IndexedTrust::new(
                "tx1",
                100,
                "a".to_string(),
                "b".to_string(),
                TrustLevel::Trusted,
                None,
                1,
            ),
            IndexedTrust::new(
                "tx2",
                101,
                "c".to_string(),
                "b".to_string(),
                TrustLevel::Trusted,
                None,
                1,
            ),
            IndexedTrust::new(
                "tx3",
                102,
                "d".to_string(),
                "b".to_string(),
                TrustLevel::Distrust,
                None,
                1,
            ),
            IndexedTrust::new(
                "tx4",
                103,
                "e".to_string(),
                "b".to_string(),
                TrustLevel::Neutral,
                None,
                1,
            ),
        ];
        // 1 + 1 + (-1) + 0 = 1
        assert_eq!(calculate_trust_score(&trusts), 1);
    }

    // ========================================================================
    // Tests for calculate_transitive_trust
    // ========================================================================

    #[test]
    fn test_calculate_transitive_trust_depth_1() {
        // Direct trust via trusted intermediary
        let trust = calculate_transitive_trust(TrustLevel::Trusted, 1, 1);
        assert!((trust - TRUST_DECAY_FACTOR).abs() < f64::EPSILON);
    }

    #[test]
    fn test_calculate_transitive_trust_depth_2() {
        let trust = calculate_transitive_trust(TrustLevel::Trusted, 1, 2);
        let expected = TRUST_DECAY_FACTOR.powi(2);
        assert!((trust - expected).abs() < f64::EPSILON);
    }

    #[test]
    fn test_calculate_transitive_trust_max_depth() {
        let trust = calculate_transitive_trust(TrustLevel::Trusted, 1, MAX_TRUST_DEPTH);
        let expected = TRUST_DECAY_FACTOR.powi(MAX_TRUST_DEPTH as i32);
        assert!((trust - expected).abs() < f64::EPSILON);
    }

    #[test]
    fn test_calculate_transitive_trust_beyond_max_depth() {
        let trust = calculate_transitive_trust(TrustLevel::Trusted, 1, MAX_TRUST_DEPTH + 1);
        assert_eq!(trust, 0.0);
    }

    #[test]
    fn test_calculate_transitive_trust_zero_intermediary() {
        // No trust chain through neutral intermediary
        let trust = calculate_transitive_trust(TrustLevel::Trusted, 0, 1);
        assert_eq!(trust, 0.0);
    }

    #[test]
    fn test_calculate_transitive_trust_distrust_chain() {
        // Distrust via trusted intermediary = negative trust
        let trust = calculate_transitive_trust(TrustLevel::Distrust, 1, 1);
        assert!((trust + TRUST_DECAY_FACTOR).abs() < f64::EPSILON);
    }

    // ========================================================================
    // Tests for ReportAggregation
    // ========================================================================

    #[test]
    fn test_report_aggregation_new() {
        let agg = ReportAggregation::new();
        assert_eq!(agg.total_reports, 0);
        assert_eq!(agg.total_stake, 0);
        assert!(agg.by_category.is_empty());
        assert!(agg.by_status.is_empty());
    }

    #[test]
    fn test_report_aggregation_add_report() {
        let mut agg = ReportAggregation::new();
        let report = IndexedReport::new(
            "tx",
            100,
            "a".to_string(),
            "b".to_string(),
            ReportCategory::Spam,
            MIN_REPORT_STAKE,
            None,
            1,
        );
        agg.add_report(&report);

        assert_eq!(agg.total_reports, 1);
        assert_eq!(agg.total_stake, MIN_REPORT_STAKE);
        assert_eq!(agg.by_category.get(&ReportCategory::Spam), Some(&1));
        assert_eq!(agg.by_status.get(&ReportStatus::Pending), Some(&1));
    }

    #[test]
    fn test_report_aggregation_multiple_reports() {
        let mut agg = ReportAggregation::new();

        agg.add_report(&IndexedReport::new(
            "tx1",
            100,
            "a".to_string(),
            "b".to_string(),
            ReportCategory::Spam,
            MIN_REPORT_STAKE,
            None,
            1,
        ));
        agg.add_report(&IndexedReport::new(
            "tx2",
            101,
            "c".to_string(),
            "b".to_string(),
            ReportCategory::Spam,
            MIN_REPORT_STAKE * 2,
            None,
            1,
        ));
        agg.add_report(&IndexedReport::new(
            "tx3",
            102,
            "d".to_string(),
            "b".to_string(),
            ReportCategory::Scam,
            MIN_REPORT_STAKE,
            None,
            1,
        ));

        assert_eq!(agg.total_reports, 3);
        assert_eq!(agg.total_stake, MIN_REPORT_STAKE * 4);
        assert_eq!(agg.by_category.get(&ReportCategory::Spam), Some(&2));
        assert_eq!(agg.by_category.get(&ReportCategory::Scam), Some(&1));
    }

    #[test]
    fn test_report_aggregation_has_immediate_reports() {
        let mut agg = ReportAggregation::new();
        assert!(!agg.has_immediate_reports());

        agg.add_report(&IndexedReport::new(
            "tx",
            100,
            "a".to_string(),
            "b".to_string(),
            ReportCategory::Spam,
            MIN_REPORT_STAKE,
            None,
            1,
        ));
        assert!(!agg.has_immediate_reports());

        agg.add_report(&IndexedReport::new(
            "tx2",
            101,
            "c".to_string(),
            "b".to_string(),
            ReportCategory::Illegal,
            MIN_REPORT_STAKE,
            None,
            1,
        ));
        assert!(agg.has_immediate_reports());
    }

    // ========================================================================
    // Tests for parse_moderation_memo (integration)
    // ========================================================================

    #[test]
    fn test_parse_moderation_memo_invalid_txid() {
        let memo = create_social_memo(SocialMessageType::Trust, &[]);
        let result = parse_moderation_memo(&memo, "", 100, "bs1sender");
        assert_eq!(result, Err(ModerationIndexError::InvalidTxId));
    }

    #[test]
    fn test_parse_moderation_memo_not_moderation() {
        let memo = create_social_memo(
            SocialMessageType::Post,
            &[0, 5, b'h', b'e', b'l', b'l', b'o'],
        );
        let result = parse_moderation_memo(&memo, "txid", 100, "bs1sender");
        assert_eq!(result, Err(ModerationIndexError::NotModeration));
    }
}
