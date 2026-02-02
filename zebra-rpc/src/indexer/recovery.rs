//! Indexer recovery parsing and tracking utilities.
//!
//! This module provides utilities for indexers to:
//! - Parse recovery messages (config, request, approve, cancel) from transaction memos
//! - Track recovery configuration lifecycle
//! - Monitor guardian approvals and timelock states
//! - Calculate recovery execution eligibility
//!
//! # Overview
//!
//! Recovery messages (types 0xF0-0xF3) enable social account recovery using
//! guardian-based M-of-N secret sharing. The recovery system follows a
//! multi-phase process:
//!
//! 1. **Configuration Phase**: User designates trusted guardians and sets threshold
//! 2. **Request Phase**: New device initiates recovery with proof of identity
//! 3. **Approval Phase**: Guardians verify identity out-of-band and approve
//! 4. **Timelock Phase**: 7-day waiting period for owner to cancel if unauthorized
//! 5. **Execution Phase**: After timelock, recovery can be finalized
//!
//! # Recovery Message Types
//!
//! - `RecoveryConfig (0xF0)`: Sets up guardians and recovery parameters
//! - `RecoveryRequest (0xF1)`: Initiates a recovery attempt
//! - `RecoveryApprove (0xF2)`: Guardian approves a recovery request
//! - `RecoveryCancel (0xF3)`: Owner cancels an unauthorized recovery attempt
//!
//! # Usage
//!
//! ```ignore
//! use zebra_chain::transaction::Memo;
//! use zebra_rpc::indexer::recovery::{parse_recovery_memo, IndexedRecovery};
//!
//! let recovery = parse_recovery_memo(&memo, "txid123", 1000)?;
//! match recovery {
//!     IndexedRecovery::Config(config) => {
//!         println!("New recovery config: {} guardians, threshold {}",
//!                  config.guardian_count, config.threshold);
//!     }
//!     IndexedRecovery::Request(request) => {
//!         println!("Recovery requested for {}", request.target_address);
//!     }
//!     IndexedRecovery::Approve(approve) => {
//!         println!("Guardian approved: {}", approve.guardian_address);
//!     }
//!     IndexedRecovery::Cancel(cancel) => {
//!         println!("Recovery cancelled by owner");
//!     }
//! }
//! ```

use std::fmt;

use serde::{Deserialize, Serialize};
use zebra_chain::transaction::{
    social::{SocialMessage, SocialMessageType, SocialParseError},
    Memo,
};

/// Default timelock duration in blocks (~7 days at 60s blocks).
pub const DEFAULT_RECOVERY_TIMELOCK_BLOCKS: u32 = 10080;

/// Minimum timelock duration in blocks (~1 day).
pub const MIN_RECOVERY_TIMELOCK_BLOCKS: u32 = 1440;

/// Maximum timelock duration in blocks (~70 days).
pub const MAX_RECOVERY_TIMELOCK_BLOCKS: u32 = 100800;

/// Minimum number of guardians required.
pub const MIN_GUARDIANS: usize = 1;

/// Maximum number of guardians allowed.
pub const MAX_GUARDIANS: usize = 15;

/// Recovery status based on lifecycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum RecoveryState {
    /// Recovery is configured and ready to use.
    #[default]
    Active,
    /// A recovery request is pending guardian approval.
    Pending,
    /// Enough guardians have approved; waiting for timelock.
    Approved,
    /// Timelock period is active; owner can still cancel.
    Timelocked,
    /// Recovery was successfully executed.
    Executed,
    /// Recovery was cancelled by the owner.
    Cancelled,
    /// Recovery request expired (timelock passed without execution).
    Expired,
}

impl RecoveryState {
    /// Returns the string representation of this state.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Timelocked => "timelocked",
            Self::Executed => "executed",
            Self::Cancelled => "cancelled",
            Self::Expired => "expired",
        }
    }
}

impl fmt::Display for RecoveryState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// An indexed recovery configuration extracted from a memo.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedRecoveryConfig {
    /// The transaction ID containing this configuration.
    pub tx_id: String,

    /// Block height where this configuration was created.
    pub created_at_block: u32,

    /// The address this recovery config protects.
    pub owner_address: String,

    /// Recovery configuration ID (derived from tx_id hash).
    pub recovery_id: String,

    /// Number of guardians registered.
    pub guardian_count: u8,

    /// Number of guardian approvals required (M of N).
    pub threshold: u8,

    /// Timelock duration in blocks.
    pub timelock_blocks: u32,

    /// Protocol version.
    pub version: u8,

    /// List of guardian address hashes (SHA256 truncated).
    pub guardian_hashes: Vec<String>,
}

impl IndexedRecoveryConfig {
    /// Creates a new indexed recovery configuration from parsed data.
    pub fn new(
        tx_id: &str,
        created_at_block: u32,
        owner_address: String,
        guardian_count: u8,
        threshold: u8,
        timelock_blocks: u32,
        version: u8,
        guardian_hashes: Vec<String>,
    ) -> Self {
        let recovery_id = derive_recovery_id(tx_id);

        Self {
            tx_id: tx_id.to_string(),
            created_at_block,
            owner_address,
            recovery_id,
            guardian_count,
            threshold,
            timelock_blocks,
            version,
            guardian_hashes,
        }
    }

    /// Returns true if the given guardian hash is in this configuration.
    pub fn has_guardian(&self, guardian_hash: &str) -> bool {
        self.guardian_hashes.iter().any(|h| h == guardian_hash)
    }

    /// Returns the number of guardians still needed for threshold.
    pub fn guardians_needed(&self, approvals: u8) -> u8 {
        self.threshold.saturating_sub(approvals)
    }
}

impl fmt::Display for IndexedRecoveryConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RecoveryConfig {{ id: {}..., guardians: {}, threshold: {} }}",
            &self.recovery_id[..8.min(self.recovery_id.len())],
            self.guardian_count,
            self.threshold
        )
    }
}

/// An indexed recovery request extracted from a memo.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedRecoveryRequest {
    /// The transaction ID containing this request.
    pub tx_id: String,

    /// Block height where this request was made.
    pub requested_at_block: u32,

    /// The address being recovered.
    pub target_address: String,

    /// Recovery configuration ID this request is for.
    pub recovery_id: String,

    /// Unique request ID (derived from tx_id hash).
    pub request_id: String,

    /// The new public key to rotate to (hex-encoded, 33 bytes).
    pub new_pubkey: String,

    /// Block height when the timelock expires.
    pub timelock_expires_block: u32,

    /// Number of approvals received.
    pub approvals_count: u8,

    /// Addresses of guardians who have approved.
    pub approved_guardians: Vec<String>,
}

impl IndexedRecoveryRequest {
    /// Creates a new indexed recovery request from parsed data.
    pub fn new(
        tx_id: &str,
        requested_at_block: u32,
        target_address: String,
        recovery_id: String,
        new_pubkey: String,
        timelock_blocks: u32,
    ) -> Self {
        let request_id = derive_request_id(tx_id);
        let timelock_expires_block = requested_at_block.saturating_add(timelock_blocks);

        Self {
            tx_id: tx_id.to_string(),
            requested_at_block,
            target_address,
            recovery_id,
            request_id,
            new_pubkey,
            timelock_expires_block,
            approvals_count: 0,
            approved_guardians: Vec::new(),
        }
    }

    /// Returns the current state of this request at the given block height.
    pub fn state_at(&self, current_height: u32, threshold: u8, is_cancelled: bool) -> RecoveryState {
        if is_cancelled {
            return RecoveryState::Cancelled;
        }

        if self.approvals_count >= threshold {
            if current_height >= self.timelock_expires_block {
                RecoveryState::Executed
            } else {
                RecoveryState::Timelocked
            }
        } else if current_height >= self.timelock_expires_block {
            RecoveryState::Expired
        } else {
            RecoveryState::Pending
        }
    }

    /// Returns true if this request can be executed at the given height.
    pub fn can_execute(&self, current_height: u32, threshold: u8, is_cancelled: bool) -> bool {
        !is_cancelled
            && self.approvals_count >= threshold
            && current_height >= self.timelock_expires_block
    }

    /// Returns true if the timelock period is currently active.
    pub fn is_in_timelock(&self, current_height: u32, threshold: u8) -> bool {
        self.approvals_count >= threshold && current_height < self.timelock_expires_block
    }

    /// Adds an approval from a guardian.
    pub fn add_approval(&mut self, guardian_address: String) {
        if !self.approved_guardians.contains(&guardian_address) {
            self.approved_guardians.push(guardian_address);
            self.approvals_count = self.approvals_count.saturating_add(1);
        }
    }
}

impl fmt::Display for IndexedRecoveryRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RecoveryRequest {{ id: {}..., target: {}..., approvals: {} }}",
            &self.request_id[..8.min(self.request_id.len())],
            &self.target_address[..8.min(self.target_address.len())],
            self.approvals_count
        )
    }
}

/// An indexed recovery approval extracted from a memo.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedRecoveryApproval {
    /// The transaction ID containing this approval.
    pub tx_id: String,

    /// Block height where this approval was made.
    pub approved_at_block: u32,

    /// The guardian's address who approved.
    pub guardian_address: String,

    /// The request ID being approved.
    pub request_id: String,

    /// The encrypted Shamir share (hex-encoded).
    pub encrypted_share: String,
}

impl IndexedRecoveryApproval {
    /// Creates a new indexed recovery approval from parsed data.
    pub fn new(
        tx_id: &str,
        approved_at_block: u32,
        guardian_address: String,
        request_id: String,
        encrypted_share: String,
    ) -> Self {
        Self {
            tx_id: tx_id.to_string(),
            approved_at_block,
            guardian_address,
            request_id,
            encrypted_share,
        }
    }
}

impl fmt::Display for IndexedRecoveryApproval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RecoveryApproval {{ request: {}..., guardian: {}... }}",
            &self.request_id[..8.min(self.request_id.len())],
            &self.guardian_address[..8.min(self.guardian_address.len())]
        )
    }
}

/// An indexed recovery cancellation extracted from a memo.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedRecoveryCancel {
    /// The transaction ID containing this cancellation.
    pub tx_id: String,

    /// Block height where this cancellation was made.
    pub cancelled_at_block: u32,

    /// The request ID being cancelled.
    pub request_id: String,

    /// The owner's address who cancelled.
    pub owner_address: String,
}

impl IndexedRecoveryCancel {
    /// Creates a new indexed recovery cancellation from parsed data.
    pub fn new(
        tx_id: &str,
        cancelled_at_block: u32,
        request_id: String,
        owner_address: String,
    ) -> Self {
        Self {
            tx_id: tx_id.to_string(),
            cancelled_at_block,
            request_id,
            owner_address,
        }
    }
}

impl fmt::Display for IndexedRecoveryCancel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RecoveryCancel {{ request: {}..., owner: {}... }}",
            &self.request_id[..8.min(self.request_id.len())],
            &self.owner_address[..8.min(self.owner_address.len())]
        )
    }
}

/// An indexed key rotation extracted from a memo.
///
/// Key rotation allows users to migrate their social identity (followers,
/// karma, etc.) to a new address. This can be initiated after successful
/// social recovery or proactively for security reasons.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedKeyRotation {
    /// The transaction ID containing this rotation.
    pub tx_id: String,

    /// Block height where this rotation was submitted.
    pub rotation_block: u32,

    /// The old (source) address being rotated from.
    pub old_address: String,

    /// The new (target) address being rotated to.
    pub new_address: String,

    /// Whether this rotation was performed via social recovery.
    pub via_recovery: bool,

    /// The old key signature (hex-encoded).
    pub old_signature: String,

    /// The new key signature (hex-encoded).
    pub new_signature: String,

    /// Optional reason for the rotation.
    pub reason: Option<String>,
}

impl IndexedKeyRotation {
    /// Creates a new indexed key rotation from parsed data.
    pub fn new(
        tx_id: &str,
        rotation_block: u32,
        old_address: String,
        new_address: String,
        via_recovery: bool,
        old_signature: String,
        new_signature: String,
        reason: Option<String>,
    ) -> Self {
        Self {
            tx_id: tx_id.to_string(),
            rotation_block,
            old_address,
            new_address,
            via_recovery,
            old_signature,
            new_signature,
            reason,
        }
    }

    /// Returns the migration ID (derived from old and new addresses).
    pub fn migration_id(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.old_address.hash(&mut hasher);
        self.new_address.hash(&mut hasher);
        self.tx_id.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }
}

impl fmt::Display for IndexedKeyRotation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "KeyRotation {{ old: {}... -> new: {}...{} }}",
            &self.old_address[..8.min(self.old_address.len())],
            &self.new_address[..8.min(self.new_address.len())],
            if self.via_recovery { " (via recovery)" } else { "" }
        )
    }
}

/// Unified enum for all indexed recovery message types.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexedRecovery {
    /// A recovery configuration message.
    Config(IndexedRecoveryConfig),
    /// A recovery request message.
    Request(IndexedRecoveryRequest),
    /// A recovery approval message.
    Approve(IndexedRecoveryApproval),
    /// A recovery cancellation message.
    Cancel(IndexedRecoveryCancel),
    /// A key rotation message.
    Rotation(IndexedKeyRotation),
}

impl IndexedRecovery {
    /// Returns the message type of this indexed recovery.
    pub fn message_type(&self) -> SocialMessageType {
        match self {
            Self::Config(_) => SocialMessageType::RecoveryConfig,
            Self::Request(_) => SocialMessageType::RecoveryRequest,
            Self::Approve(_) => SocialMessageType::RecoveryApprove,
            Self::Cancel(_) => SocialMessageType::RecoveryCancel,
            Self::Rotation(_) => SocialMessageType::KeyRotation,
        }
    }

    /// Returns the transaction ID of this indexed recovery.
    pub fn tx_id(&self) -> &str {
        match self {
            Self::Config(c) => &c.tx_id,
            Self::Request(r) => &r.tx_id,
            Self::Approve(a) => &a.tx_id,
            Self::Cancel(c) => &c.tx_id,
            Self::Rotation(r) => &r.tx_id,
        }
    }
}

impl fmt::Display for IndexedRecovery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Config(c) => write!(f, "{}", c),
            Self::Request(r) => write!(f, "{}", r),
            Self::Approve(a) => write!(f, "{}", a),
            Self::Cancel(c) => write!(f, "{}", c),
            Self::Rotation(r) => write!(f, "{}", r),
        }
    }
}

/// Error type for recovery parsing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecoveryParseError {
    /// The memo is not a valid social message.
    NotSocialMessage,
    /// The memo is not a recovery message type.
    NotRecoveryMessage,
    /// The message payload is malformed.
    MalformedPayload(String),
    /// Invalid guardian configuration.
    InvalidGuardians(String),
    /// Invalid threshold value.
    InvalidThreshold(String),
    /// Invalid timelock value.
    InvalidTimelock(String),
    /// Underlying social parse error.
    SocialError(SocialParseError),
}

impl fmt::Display for RecoveryParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotSocialMessage => write!(f, "memo is not a valid social message"),
            Self::NotRecoveryMessage => write!(f, "memo is not a recovery message type"),
            Self::MalformedPayload(msg) => write!(f, "malformed recovery payload: {}", msg),
            Self::InvalidGuardians(msg) => write!(f, "invalid guardians: {}", msg),
            Self::InvalidThreshold(msg) => write!(f, "invalid threshold: {}", msg),
            Self::InvalidTimelock(msg) => write!(f, "invalid timelock: {}", msg),
            Self::SocialError(err) => write!(f, "social parse error: {:?}", err),
        }
    }
}

impl std::error::Error for RecoveryParseError {}

impl From<SocialParseError> for RecoveryParseError {
    fn from(err: SocialParseError) -> Self {
        Self::SocialError(err)
    }
}

/// Checks if a memo contains a recovery message.
///
/// Returns true if the first byte of the memo matches a recovery message type
/// (0xF0-0xF4).
pub fn is_recovery_memo(memo: &Memo) -> bool {
    let bytes = memo.as_bytes();
    if bytes.is_empty() {
        return false;
    }

    matches!(bytes[0], 0xF0 | 0xF1 | 0xF2 | 0xF3 | 0xF4)
}

/// Parses a recovery message from a memo.
///
/// This function extracts recovery-related data from a memo and returns
/// the appropriate indexed recovery type. The `tx_id` and `block_height`
/// are used for metadata but don't come from the memo itself.
///
/// # Arguments
///
/// * `memo` - The memo to parse
/// * `tx_id` - The transaction ID containing this memo
/// * `block_height` - The block height where this memo was found
///
/// # Returns
///
/// Returns `Ok(IndexedRecovery)` if the memo contains a valid recovery message,
/// or an appropriate `RecoveryParseError` otherwise.
pub fn parse_recovery_memo(
    memo: &Memo,
    tx_id: &str,
    block_height: u32,
) -> Result<IndexedRecovery, RecoveryParseError> {
    // First, try to parse as a social message
    let social = SocialMessage::try_from(memo).map_err(RecoveryParseError::from)?;

    // Check if it's a recovery message type
    if !social.msg_type().is_recovery() {
        return Err(RecoveryParseError::NotRecoveryMessage);
    }

    let payload = social.payload();

    match social.msg_type() {
        SocialMessageType::RecoveryConfig => {
            parse_recovery_config(payload, tx_id, block_height)
        }
        SocialMessageType::RecoveryRequest => {
            parse_recovery_request(payload, tx_id, block_height)
        }
        SocialMessageType::RecoveryApprove => {
            parse_recovery_approval(payload, tx_id, block_height)
        }
        SocialMessageType::RecoveryCancel => {
            parse_recovery_cancel(payload, tx_id, block_height)
        }
        SocialMessageType::KeyRotation => {
            parse_key_rotation(payload, tx_id, block_height)
        }
        _ => Err(RecoveryParseError::NotRecoveryMessage),
    }
}

/// Parses a RecoveryConfig payload.
///
/// Expected format:
/// ```text
/// [version:1][guardian_count:1][threshold:1][timelock_blocks:4][owner_addr_len:1][owner_addr:N]
/// [guardian_hash_1:32][guardian_hash_2:32]...
/// ```
fn parse_recovery_config(
    payload: &[u8],
    tx_id: &str,
    block_height: u32,
) -> Result<IndexedRecovery, RecoveryParseError> {
    if payload.len() < 8 {
        return Err(RecoveryParseError::MalformedPayload(
            "payload too short for recovery config".to_string(),
        ));
    }

    let version = payload[0];
    let guardian_count = payload[1];
    let threshold = payload[2];

    // Validate guardian count
    if guardian_count < MIN_GUARDIANS as u8 || guardian_count > MAX_GUARDIANS as u8 {
        return Err(RecoveryParseError::InvalidGuardians(format!(
            "guardian count {} out of range [{}, {}]",
            guardian_count, MIN_GUARDIANS, MAX_GUARDIANS
        )));
    }

    // Validate threshold
    if threshold == 0 || threshold > guardian_count {
        return Err(RecoveryParseError::InvalidThreshold(format!(
            "threshold {} invalid for {} guardians",
            threshold, guardian_count
        )));
    }

    // Parse timelock (4 bytes little-endian)
    let timelock_blocks = u32::from_le_bytes([payload[3], payload[4], payload[5], payload[6]]);

    // Validate timelock
    if timelock_blocks < MIN_RECOVERY_TIMELOCK_BLOCKS
        || timelock_blocks > MAX_RECOVERY_TIMELOCK_BLOCKS
    {
        return Err(RecoveryParseError::InvalidTimelock(format!(
            "timelock {} blocks out of range [{}, {}]",
            timelock_blocks, MIN_RECOVERY_TIMELOCK_BLOCKS, MAX_RECOVERY_TIMELOCK_BLOCKS
        )));
    }

    let owner_addr_len = payload[7] as usize;
    if payload.len() < 8 + owner_addr_len {
        return Err(RecoveryParseError::MalformedPayload(
            "payload too short for owner address".to_string(),
        ));
    }

    let owner_address = String::from_utf8_lossy(&payload[8..8 + owner_addr_len]).to_string();

    // Parse guardian hashes (32 bytes each)
    let hash_start = 8 + owner_addr_len;
    let expected_hash_bytes = guardian_count as usize * 32;
    if payload.len() < hash_start + expected_hash_bytes {
        return Err(RecoveryParseError::MalformedPayload(
            "payload too short for guardian hashes".to_string(),
        ));
    }

    let mut guardian_hashes = Vec::with_capacity(guardian_count as usize);
    for i in 0..guardian_count as usize {
        let start = hash_start + i * 32;
        let hash = hex::encode(&payload[start..start + 32]);
        guardian_hashes.push(hash);
    }

    let config = IndexedRecoveryConfig::new(
        tx_id,
        block_height,
        owner_address,
        guardian_count,
        threshold,
        timelock_blocks,
        version,
        guardian_hashes,
    );

    Ok(IndexedRecovery::Config(config))
}

/// Parses a RecoveryRequest payload.
///
/// Expected format:
/// ```text
/// [version:1][recovery_id_len:1][recovery_id:N][target_addr_len:1][target_addr:M]
/// [new_pubkey:33][proof_len:2][proof:P]
/// ```
fn parse_recovery_request(
    payload: &[u8],
    tx_id: &str,
    block_height: u32,
) -> Result<IndexedRecovery, RecoveryParseError> {
    if payload.len() < 3 {
        return Err(RecoveryParseError::MalformedPayload(
            "payload too short for recovery request".to_string(),
        ));
    }

    let _version = payload[0];
    let recovery_id_len = payload[1] as usize;

    if payload.len() < 2 + recovery_id_len + 1 {
        return Err(RecoveryParseError::MalformedPayload(
            "payload too short for recovery ID".to_string(),
        ));
    }

    let recovery_id = hex::encode(&payload[2..2 + recovery_id_len]);

    let target_offset = 2 + recovery_id_len;
    let target_addr_len = payload[target_offset] as usize;

    if payload.len() < target_offset + 1 + target_addr_len + 33 {
        return Err(RecoveryParseError::MalformedPayload(
            "payload too short for target address and pubkey".to_string(),
        ));
    }

    let target_address = String::from_utf8_lossy(
        &payload[target_offset + 1..target_offset + 1 + target_addr_len],
    )
    .to_string();

    let pubkey_offset = target_offset + 1 + target_addr_len;
    let new_pubkey = hex::encode(&payload[pubkey_offset..pubkey_offset + 33]);

    // Use default timelock for now (would be looked up from recovery config in full implementation)
    let request = IndexedRecoveryRequest::new(
        tx_id,
        block_height,
        target_address,
        recovery_id,
        new_pubkey,
        DEFAULT_RECOVERY_TIMELOCK_BLOCKS,
    );

    Ok(IndexedRecovery::Request(request))
}

/// Parses a RecoveryApprove payload.
///
/// Expected format:
/// ```text
/// [version:1][request_id_len:1][request_id:N][guardian_addr_len:1][guardian_addr:M]
/// [share_len:2][encrypted_share:S]
/// ```
fn parse_recovery_approval(
    payload: &[u8],
    tx_id: &str,
    block_height: u32,
) -> Result<IndexedRecovery, RecoveryParseError> {
    if payload.len() < 3 {
        return Err(RecoveryParseError::MalformedPayload(
            "payload too short for recovery approval".to_string(),
        ));
    }

    let _version = payload[0];
    let request_id_len = payload[1] as usize;

    if payload.len() < 2 + request_id_len + 1 {
        return Err(RecoveryParseError::MalformedPayload(
            "payload too short for request ID".to_string(),
        ));
    }

    let request_id = hex::encode(&payload[2..2 + request_id_len]);

    let guardian_offset = 2 + request_id_len;
    let guardian_addr_len = payload[guardian_offset] as usize;

    if payload.len() < guardian_offset + 1 + guardian_addr_len + 2 {
        return Err(RecoveryParseError::MalformedPayload(
            "payload too short for guardian address".to_string(),
        ));
    }

    let guardian_address = String::from_utf8_lossy(
        &payload[guardian_offset + 1..guardian_offset + 1 + guardian_addr_len],
    )
    .to_string();

    let share_len_offset = guardian_offset + 1 + guardian_addr_len;
    let share_len =
        u16::from_le_bytes([payload[share_len_offset], payload[share_len_offset + 1]]) as usize;

    if payload.len() < share_len_offset + 2 + share_len {
        return Err(RecoveryParseError::MalformedPayload(
            "payload too short for encrypted share".to_string(),
        ));
    }

    let encrypted_share =
        hex::encode(&payload[share_len_offset + 2..share_len_offset + 2 + share_len]);

    let approval = IndexedRecoveryApproval::new(
        tx_id,
        block_height,
        guardian_address,
        request_id,
        encrypted_share,
    );

    Ok(IndexedRecovery::Approve(approval))
}

/// Parses a RecoveryCancel payload.
///
/// Expected format:
/// ```text
/// [version:1][request_id_len:1][request_id:N][owner_addr_len:1][owner_addr:M]
/// ```
fn parse_recovery_cancel(
    payload: &[u8],
    tx_id: &str,
    block_height: u32,
) -> Result<IndexedRecovery, RecoveryParseError> {
    if payload.len() < 3 {
        return Err(RecoveryParseError::MalformedPayload(
            "payload too short for recovery cancel".to_string(),
        ));
    }

    let _version = payload[0];
    let request_id_len = payload[1] as usize;

    if payload.len() < 2 + request_id_len + 1 {
        return Err(RecoveryParseError::MalformedPayload(
            "payload too short for request ID".to_string(),
        ));
    }

    let request_id = hex::encode(&payload[2..2 + request_id_len]);

    let owner_offset = 2 + request_id_len;
    let owner_addr_len = payload[owner_offset] as usize;

    if payload.len() < owner_offset + 1 + owner_addr_len {
        return Err(RecoveryParseError::MalformedPayload(
            "payload too short for owner address".to_string(),
        ));
    }

    let owner_address =
        String::from_utf8_lossy(&payload[owner_offset + 1..owner_offset + 1 + owner_addr_len])
            .to_string();

    let cancel = IndexedRecoveryCancel::new(tx_id, block_height, request_id, owner_address);

    Ok(IndexedRecovery::Cancel(cancel))
}

/// Parses a KeyRotation payload.
///
/// Expected format:
/// ```text
/// [version:1][flags:1][old_addr_len:1][old_addr:N][new_addr_len:1][new_addr:M]
/// [old_sig_len:1][old_sig:S1][new_sig_len:1][new_sig:S2][reason_len:2][reason:R]?
/// ```
///
/// Flags byte:
/// - bit 0: via_recovery (1 = key rotation via social recovery)
fn parse_key_rotation(
    payload: &[u8],
    tx_id: &str,
    block_height: u32,
) -> Result<IndexedRecovery, RecoveryParseError> {
    if payload.len() < 4 {
        return Err(RecoveryParseError::MalformedPayload(
            "payload too short for key rotation".to_string(),
        ));
    }

    let _version = payload[0];
    let flags = payload[1];
    let via_recovery = (flags & 0x01) != 0;

    let old_addr_len = payload[2] as usize;
    if payload.len() < 3 + old_addr_len + 1 {
        return Err(RecoveryParseError::MalformedPayload(
            "payload too short for old address".to_string(),
        ));
    }

    let old_address = String::from_utf8_lossy(&payload[3..3 + old_addr_len]).to_string();

    let new_addr_offset = 3 + old_addr_len;
    let new_addr_len = payload[new_addr_offset] as usize;

    if payload.len() < new_addr_offset + 1 + new_addr_len + 1 {
        return Err(RecoveryParseError::MalformedPayload(
            "payload too short for new address".to_string(),
        ));
    }

    let new_address =
        String::from_utf8_lossy(&payload[new_addr_offset + 1..new_addr_offset + 1 + new_addr_len])
            .to_string();

    // Parse old signature
    let old_sig_offset = new_addr_offset + 1 + new_addr_len;
    if payload.len() < old_sig_offset + 1 {
        return Err(RecoveryParseError::MalformedPayload(
            "payload too short for old signature length".to_string(),
        ));
    }
    let old_sig_len = payload[old_sig_offset] as usize;

    if payload.len() < old_sig_offset + 1 + old_sig_len + 1 {
        return Err(RecoveryParseError::MalformedPayload(
            "payload too short for old signature".to_string(),
        ));
    }
    let old_signature =
        hex::encode(&payload[old_sig_offset + 1..old_sig_offset + 1 + old_sig_len]);

    // Parse new signature
    let new_sig_offset = old_sig_offset + 1 + old_sig_len;
    let new_sig_len = payload[new_sig_offset] as usize;

    if payload.len() < new_sig_offset + 1 + new_sig_len {
        return Err(RecoveryParseError::MalformedPayload(
            "payload too short for new signature".to_string(),
        ));
    }
    let new_signature =
        hex::encode(&payload[new_sig_offset + 1..new_sig_offset + 1 + new_sig_len]);

    // Parse optional reason
    let reason_offset = new_sig_offset + 1 + new_sig_len;
    let reason = if payload.len() >= reason_offset + 2 {
        let reason_len =
            u16::from_le_bytes([payload[reason_offset], payload[reason_offset + 1]]) as usize;
        if reason_len > 0 && payload.len() >= reason_offset + 2 + reason_len {
            Some(
                String::from_utf8_lossy(&payload[reason_offset + 2..reason_offset + 2 + reason_len])
                    .to_string(),
            )
        } else {
            None
        }
    } else {
        None
    };

    let rotation = IndexedKeyRotation::new(
        tx_id,
        block_height,
        old_address,
        new_address,
        via_recovery,
        old_signature,
        new_signature,
        reason,
    );

    Ok(IndexedRecovery::Rotation(rotation))
}

/// Derives a recovery ID from a transaction ID.
///
/// Uses SHA256 hash of the tx_id, hex-encoded.
fn derive_recovery_id(tx_id: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(tx_id.as_bytes());
    hex::encode(hasher.finalize())
}

/// Derives a request ID from a transaction ID.
///
/// Uses SHA256 hash of the tx_id with "request:" prefix, hex-encoded.
fn derive_request_id(tx_id: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(b"request:");
    hasher.update(tx_id.as_bytes());
    hex::encode(hasher.finalize())
}

/// Statistics for recovery messages in a block.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockRecoveryStats {
    /// Number of recovery configurations created.
    pub configs_created: u32,
    /// Number of recovery requests initiated.
    pub requests_initiated: u32,
    /// Number of guardian approvals.
    pub approvals: u32,
    /// Number of cancellations.
    pub cancellations: u32,
    /// Number of key rotations.
    pub key_rotations: u32,
    /// Total recovery-related transactions.
    pub total_recovery_txs: u32,
}

impl BlockRecoveryStats {
    /// Creates new empty statistics.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a recovery message to the statistics.
    pub fn add(&mut self, recovery: &IndexedRecovery) {
        self.total_recovery_txs += 1;
        match recovery {
            IndexedRecovery::Config(_) => self.configs_created += 1,
            IndexedRecovery::Request(_) => self.requests_initiated += 1,
            IndexedRecovery::Approve(_) => self.approvals += 1,
            IndexedRecovery::Cancel(_) => self.cancellations += 1,
            IndexedRecovery::Rotation(_) => self.key_rotations += 1,
        }
    }

    /// Returns true if there are no recovery transactions.
    pub fn is_empty(&self) -> bool {
        self.total_recovery_txs == 0
    }
}

impl fmt::Display for BlockRecoveryStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RecoveryStats {{ configs: {}, requests: {}, approvals: {}, cancels: {}, rotations: {} }}",
            self.configs_created, self.requests_initiated, self.approvals, self.cancellations, self.key_rotations
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== RecoveryState Tests ====================

    #[test]
    fn recovery_state_as_str() {
        assert_eq!(RecoveryState::Active.as_str(), "active");
        assert_eq!(RecoveryState::Pending.as_str(), "pending");
        assert_eq!(RecoveryState::Approved.as_str(), "approved");
        assert_eq!(RecoveryState::Timelocked.as_str(), "timelocked");
        assert_eq!(RecoveryState::Executed.as_str(), "executed");
        assert_eq!(RecoveryState::Cancelled.as_str(), "cancelled");
        assert_eq!(RecoveryState::Expired.as_str(), "expired");
    }

    #[test]
    fn recovery_state_display() {
        assert_eq!(format!("{}", RecoveryState::Active), "active");
        assert_eq!(format!("{}", RecoveryState::Pending), "pending");
        assert_eq!(format!("{}", RecoveryState::Timelocked), "timelocked");
    }

    #[test]
    fn recovery_state_default() {
        assert_eq!(RecoveryState::default(), RecoveryState::Active);
    }

    // ==================== IndexedRecoveryConfig Tests ====================

    #[test]
    fn indexed_recovery_config_new() {
        let config = IndexedRecoveryConfig::new(
            "txid123",
            1000,
            "bs1owner".to_string(),
            3,
            2,
            10080,
            1,
            vec!["hash1".to_string(), "hash2".to_string(), "hash3".to_string()],
        );

        assert_eq!(config.created_at_block, 1000);
        assert_eq!(config.owner_address, "bs1owner");
        assert_eq!(config.guardian_count, 3);
        assert_eq!(config.threshold, 2);
        assert_eq!(config.timelock_blocks, 10080);
        assert_eq!(config.guardian_hashes.len(), 3);
        assert!(!config.recovery_id.is_empty());
    }

    #[test]
    fn indexed_recovery_config_has_guardian() {
        let config = IndexedRecoveryConfig::new(
            "txid123",
            1000,
            "bs1owner".to_string(),
            2,
            1,
            10080,
            1,
            vec!["hash1".to_string(), "hash2".to_string()],
        );

        assert!(config.has_guardian("hash1"));
        assert!(config.has_guardian("hash2"));
        assert!(!config.has_guardian("hash3"));
    }

    #[test]
    fn indexed_recovery_config_guardians_needed() {
        let config = IndexedRecoveryConfig::new(
            "txid123",
            1000,
            "bs1owner".to_string(),
            5,
            3,
            10080,
            1,
            vec!["h1".to_string(), "h2".to_string(), "h3".to_string(), "h4".to_string(), "h5".to_string()],
        );

        assert_eq!(config.guardians_needed(0), 3);
        assert_eq!(config.guardians_needed(1), 2);
        assert_eq!(config.guardians_needed(2), 1);
        assert_eq!(config.guardians_needed(3), 0);
        assert_eq!(config.guardians_needed(5), 0); // saturating
    }

    // ==================== IndexedRecoveryRequest Tests ====================

    #[test]
    fn indexed_recovery_request_new() {
        let request = IndexedRecoveryRequest::new(
            "txid456",
            2000,
            "bs1target".to_string(),
            "recovery_id".to_string(),
            "02abcdef".to_string(),
            10080,
        );

        assert_eq!(request.requested_at_block, 2000);
        assert_eq!(request.target_address, "bs1target");
        assert_eq!(request.timelock_expires_block, 2000 + 10080);
        assert_eq!(request.approvals_count, 0);
        assert!(request.approved_guardians.is_empty());
    }

    #[test]
    fn indexed_recovery_request_state_at() {
        let request = IndexedRecoveryRequest::new(
            "txid456",
            1000,
            "bs1target".to_string(),
            "recovery_id".to_string(),
            "02abcdef".to_string(),
            10080,
        );

        // No approvals yet
        assert_eq!(request.state_at(5000, 2, false), RecoveryState::Pending);

        // Still pending (timelock not expired, no approvals)
        assert_eq!(request.state_at(10000, 2, false), RecoveryState::Pending);

        // Expired (timelock passed, not enough approvals)
        assert_eq!(request.state_at(12000, 2, false), RecoveryState::Expired);

        // Cancelled
        assert_eq!(request.state_at(5000, 2, true), RecoveryState::Cancelled);
    }

    #[test]
    fn indexed_recovery_request_state_with_approvals() {
        let mut request = IndexedRecoveryRequest::new(
            "txid456",
            1000,
            "bs1target".to_string(),
            "recovery_id".to_string(),
            "02abcdef".to_string(),
            10080, // timelock expires at 11080
        );

        request.add_approval("guardian1".to_string());
        request.add_approval("guardian2".to_string());
        assert_eq!(request.approvals_count, 2);

        // Threshold met (2), but still in timelock
        assert_eq!(request.state_at(5000, 2, false), RecoveryState::Timelocked);

        // After timelock expires - can be executed
        assert_eq!(request.state_at(12000, 2, false), RecoveryState::Executed);
    }

    #[test]
    fn indexed_recovery_request_add_approval() {
        let mut request = IndexedRecoveryRequest::new(
            "txid456",
            1000,
            "bs1target".to_string(),
            "recovery_id".to_string(),
            "02abcdef".to_string(),
            10080,
        );

        request.add_approval("guardian1".to_string());
        assert_eq!(request.approvals_count, 1);
        assert_eq!(request.approved_guardians.len(), 1);

        // Duplicate approval should be ignored
        request.add_approval("guardian1".to_string());
        assert_eq!(request.approvals_count, 1);

        request.add_approval("guardian2".to_string());
        assert_eq!(request.approvals_count, 2);
    }

    #[test]
    fn indexed_recovery_request_can_execute() {
        let mut request = IndexedRecoveryRequest::new(
            "txid456",
            1000,
            "bs1target".to_string(),
            "recovery_id".to_string(),
            "02abcdef".to_string(),
            10080,
        );

        // Can't execute with no approvals
        assert!(!request.can_execute(12000, 2, false));

        request.add_approval("g1".to_string());
        request.add_approval("g2".to_string());

        // Can't execute before timelock
        assert!(!request.can_execute(5000, 2, false));

        // Can execute after timelock with threshold met
        assert!(request.can_execute(12000, 2, false));

        // Can't execute if cancelled
        assert!(!request.can_execute(12000, 2, true));
    }

    #[test]
    fn indexed_recovery_request_is_in_timelock() {
        let mut request = IndexedRecoveryRequest::new(
            "txid456",
            1000,
            "bs1target".to_string(),
            "recovery_id".to_string(),
            "02abcdef".to_string(),
            10080,
        );

        // Not in timelock without approvals
        assert!(!request.is_in_timelock(5000, 2));

        request.add_approval("g1".to_string());
        request.add_approval("g2".to_string());

        // In timelock after threshold met but before expiry
        assert!(request.is_in_timelock(5000, 2));

        // Not in timelock after expiry
        assert!(!request.is_in_timelock(12000, 2));
    }

    // ==================== IndexedRecoveryApproval Tests ====================

    #[test]
    fn indexed_recovery_approval_new() {
        let approval = IndexedRecoveryApproval::new(
            "txid789",
            3000,
            "bs1guardian".to_string(),
            "request123".to_string(),
            "encrypted_share_hex".to_string(),
        );

        assert_eq!(approval.tx_id, "txid789");
        assert_eq!(approval.approved_at_block, 3000);
        assert_eq!(approval.guardian_address, "bs1guardian");
        assert_eq!(approval.request_id, "request123");
        assert_eq!(approval.encrypted_share, "encrypted_share_hex");
    }

    // ==================== IndexedRecoveryCancel Tests ====================

    #[test]
    fn indexed_recovery_cancel_new() {
        let cancel = IndexedRecoveryCancel::new(
            "txid_cancel",
            4000,
            "request123".to_string(),
            "bs1owner".to_string(),
        );

        assert_eq!(cancel.tx_id, "txid_cancel");
        assert_eq!(cancel.cancelled_at_block, 4000);
        assert_eq!(cancel.request_id, "request123");
        assert_eq!(cancel.owner_address, "bs1owner");
    }

    // ==================== IndexedRecovery Tests ====================

    #[test]
    fn indexed_recovery_message_type() {
        let config = IndexedRecovery::Config(IndexedRecoveryConfig::new(
            "tx1", 1000, "owner".to_string(), 2, 1, 10080, 1, vec!["h1".to_string()],
        ));
        assert_eq!(config.message_type(), SocialMessageType::RecoveryConfig);

        let request = IndexedRecovery::Request(IndexedRecoveryRequest::new(
            "tx2", 2000, "target".to_string(), "rid".to_string(), "pk".to_string(), 10080,
        ));
        assert_eq!(request.message_type(), SocialMessageType::RecoveryRequest);

        let approve = IndexedRecovery::Approve(IndexedRecoveryApproval::new(
            "tx3", 3000, "guardian".to_string(), "req".to_string(), "share".to_string(),
        ));
        assert_eq!(approve.message_type(), SocialMessageType::RecoveryApprove);

        let cancel = IndexedRecovery::Cancel(IndexedRecoveryCancel::new(
            "tx4", 4000, "req".to_string(), "owner".to_string(),
        ));
        assert_eq!(cancel.message_type(), SocialMessageType::RecoveryCancel);
    }

    #[test]
    fn indexed_recovery_tx_id() {
        let config = IndexedRecovery::Config(IndexedRecoveryConfig::new(
            "config_tx", 1000, "owner".to_string(), 2, 1, 10080, 1, vec![],
        ));
        assert_eq!(config.tx_id(), "config_tx");
    }

    // ==================== BlockRecoveryStats Tests ====================

    #[test]
    fn block_recovery_stats_new() {
        let stats = BlockRecoveryStats::new();
        assert_eq!(stats.configs_created, 0);
        assert_eq!(stats.requests_initiated, 0);
        assert_eq!(stats.approvals, 0);
        assert_eq!(stats.cancellations, 0);
        assert_eq!(stats.total_recovery_txs, 0);
        assert!(stats.is_empty());
    }

    #[test]
    fn block_recovery_stats_add() {
        let mut stats = BlockRecoveryStats::new();

        let config = IndexedRecovery::Config(IndexedRecoveryConfig::new(
            "tx1", 1000, "owner".to_string(), 2, 1, 10080, 1, vec![],
        ));
        stats.add(&config);
        assert_eq!(stats.configs_created, 1);
        assert_eq!(stats.total_recovery_txs, 1);
        assert!(!stats.is_empty());

        let request = IndexedRecovery::Request(IndexedRecoveryRequest::new(
            "tx2", 2000, "target".to_string(), "rid".to_string(), "pk".to_string(), 10080,
        ));
        stats.add(&request);
        assert_eq!(stats.requests_initiated, 1);
        assert_eq!(stats.total_recovery_txs, 2);

        let approve = IndexedRecovery::Approve(IndexedRecoveryApproval::new(
            "tx3", 3000, "guardian".to_string(), "req".to_string(), "share".to_string(),
        ));
        stats.add(&approve);
        assert_eq!(stats.approvals, 1);

        let cancel = IndexedRecovery::Cancel(IndexedRecoveryCancel::new(
            "tx4", 4000, "req".to_string(), "owner".to_string(),
        ));
        stats.add(&cancel);
        assert_eq!(stats.cancellations, 1);
        assert_eq!(stats.total_recovery_txs, 4);
    }

    // ==================== ID Derivation Tests ====================

    #[test]
    fn derive_recovery_id_deterministic() {
        let id1 = derive_recovery_id("txid123");
        let id2 = derive_recovery_id("txid123");
        assert_eq!(id1, id2);

        let id3 = derive_recovery_id("txid456");
        assert_ne!(id1, id3);
    }

    #[test]
    fn derive_request_id_deterministic() {
        let id1 = derive_request_id("txid123");
        let id2 = derive_request_id("txid123");
        assert_eq!(id1, id2);

        // Request ID should differ from recovery ID for same tx
        let recovery_id = derive_recovery_id("txid123");
        assert_ne!(id1, recovery_id);
    }

    // ==================== Constants Tests ====================

    #[test]
    fn recovery_constants() {
        assert_eq!(DEFAULT_RECOVERY_TIMELOCK_BLOCKS, 10080);
        assert_eq!(MIN_RECOVERY_TIMELOCK_BLOCKS, 1440);
        assert_eq!(MAX_RECOVERY_TIMELOCK_BLOCKS, 100800);
        assert_eq!(MIN_GUARDIANS, 1);
        assert_eq!(MAX_GUARDIANS, 15);
    }

    // ==================== RecoveryParseError Tests ====================

    #[test]
    fn recovery_parse_error_display() {
        assert_eq!(
            format!("{}", RecoveryParseError::NotSocialMessage),
            "memo is not a valid social message"
        );
        assert_eq!(
            format!("{}", RecoveryParseError::NotRecoveryMessage),
            "memo is not a recovery message type"
        );
        assert_eq!(
            format!("{}", RecoveryParseError::MalformedPayload("test".to_string())),
            "malformed recovery payload: test"
        );
        assert_eq!(
            format!("{}", RecoveryParseError::InvalidGuardians("bad".to_string())),
            "invalid guardians: bad"
        );
        assert_eq!(
            format!("{}", RecoveryParseError::InvalidThreshold("bad".to_string())),
            "invalid threshold: bad"
        );
        assert_eq!(
            format!("{}", RecoveryParseError::InvalidTimelock("bad".to_string())),
            "invalid timelock: bad"
        );
    }

    // ==================== Display Tests ====================

    #[test]
    fn indexed_recovery_config_display() {
        let config = IndexedRecoveryConfig::new(
            "txid_long_enough_for_display",
            1000,
            "bs1owner".to_string(),
            3,
            2,
            10080,
            1,
            vec!["h1".to_string(), "h2".to_string(), "h3".to_string()],
        );
        let display = format!("{}", config);
        assert!(display.contains("RecoveryConfig"));
        assert!(display.contains("guardians: 3"));
        assert!(display.contains("threshold: 2"));
    }

    #[test]
    fn indexed_recovery_request_display() {
        let request = IndexedRecoveryRequest::new(
            "txid_for_request_display",
            2000,
            "bs1target_address".to_string(),
            "recovery_id_here".to_string(),
            "02abcdef1234567890".to_string(),
            10080,
        );
        let display = format!("{}", request);
        assert!(display.contains("RecoveryRequest"));
        assert!(display.contains("target:"));
        assert!(display.contains("approvals: 0"));
    }

    #[test]
    fn indexed_recovery_approval_display() {
        let approval = IndexedRecoveryApproval::new(
            "tx_approval_123",
            3000,
            "bs1guardian_address".to_string(),
            "request_id_here".to_string(),
            "encrypted_share".to_string(),
        );
        let display = format!("{}", approval);
        assert!(display.contains("RecoveryApproval"));
        assert!(display.contains("request:"));
        assert!(display.contains("guardian:"));
    }

    #[test]
    fn indexed_recovery_cancel_display() {
        let cancel = IndexedRecoveryCancel::new(
            "tx_cancel_456",
            4000,
            "request_id_here".to_string(),
            "bs1owner_address".to_string(),
        );
        let display = format!("{}", cancel);
        assert!(display.contains("RecoveryCancel"));
        assert!(display.contains("request:"));
        assert!(display.contains("owner:"));
    }

    #[test]
    fn block_recovery_stats_display() {
        let mut stats = BlockRecoveryStats::new();
        stats.configs_created = 2;
        stats.requests_initiated = 1;
        stats.approvals = 5;
        stats.cancellations = 1;
        let display = format!("{}", stats);
        assert!(display.contains("configs: 2"));
        assert!(display.contains("requests: 1"));
        assert!(display.contains("approvals: 5"));
        assert!(display.contains("cancels: 1"));
    }

    // ==================== IndexedKeyRotation Tests ====================

    #[test]
    fn indexed_key_rotation_new() {
        let rotation = IndexedKeyRotation::new(
            "txid_rotation",
            5000,
            "bs1old_address".to_string(),
            "bs1new_address".to_string(),
            false,
            "old_sig_hex".to_string(),
            "new_sig_hex".to_string(),
            Some("Security upgrade".to_string()),
        );

        assert_eq!(rotation.tx_id, "txid_rotation");
        assert_eq!(rotation.rotation_block, 5000);
        assert_eq!(rotation.old_address, "bs1old_address");
        assert_eq!(rotation.new_address, "bs1new_address");
        assert!(!rotation.via_recovery);
        assert_eq!(rotation.old_signature, "old_sig_hex");
        assert_eq!(rotation.new_signature, "new_sig_hex");
        assert_eq!(rotation.reason, Some("Security upgrade".to_string()));
    }

    #[test]
    fn indexed_key_rotation_via_recovery() {
        let rotation = IndexedKeyRotation::new(
            "txid_recovery_rotation",
            6000,
            "bs1lost_key".to_string(),
            "bs1new_key".to_string(),
            true,
            "recovery_old_sig".to_string(),
            "recovery_new_sig".to_string(),
            None,
        );

        assert!(rotation.via_recovery);
        assert!(rotation.reason.is_none());
    }

    #[test]
    fn indexed_key_rotation_migration_id() {
        let rotation1 = IndexedKeyRotation::new(
            "tx1",
            1000,
            "old1".to_string(),
            "new1".to_string(),
            false,
            "sig1".to_string(),
            "sig2".to_string(),
            None,
        );

        let rotation2 = IndexedKeyRotation::new(
            "tx1",
            1000,
            "old1".to_string(),
            "new1".to_string(),
            false,
            "sig1".to_string(),
            "sig2".to_string(),
            None,
        );

        // Same inputs should produce same migration ID
        assert_eq!(rotation1.migration_id(), rotation2.migration_id());

        // Different tx_id should produce different migration ID
        let rotation3 = IndexedKeyRotation::new(
            "tx2",
            1000,
            "old1".to_string(),
            "new1".to_string(),
            false,
            "sig1".to_string(),
            "sig2".to_string(),
            None,
        );
        assert_ne!(rotation1.migration_id(), rotation3.migration_id());
    }

    #[test]
    fn indexed_key_rotation_display() {
        let rotation = IndexedKeyRotation::new(
            "txid_rotation_display",
            5000,
            "bs1old_address_long".to_string(),
            "bs1new_address_long".to_string(),
            false,
            "sig1".to_string(),
            "sig2".to_string(),
            None,
        );
        let display = format!("{}", rotation);
        assert!(display.contains("KeyRotation"));
        assert!(display.contains("old:"));
        assert!(display.contains("new:"));
        assert!(!display.contains("via recovery"));

        // Test with via_recovery
        let rotation_recovery = IndexedKeyRotation::new(
            "txid_recovery",
            5000,
            "bs1old_addr".to_string(),
            "bs1new_addr".to_string(),
            true,
            "sig1".to_string(),
            "sig2".to_string(),
            None,
        );
        let display_recovery = format!("{}", rotation_recovery);
        assert!(display_recovery.contains("(via recovery)"));
    }

    #[test]
    fn indexed_recovery_rotation_message_type() {
        let rotation = IndexedRecovery::Rotation(IndexedKeyRotation::new(
            "tx_rot",
            5000,
            "old".to_string(),
            "new".to_string(),
            false,
            "sig1".to_string(),
            "sig2".to_string(),
            None,
        ));
        assert_eq!(rotation.message_type(), SocialMessageType::KeyRotation);
    }

    #[test]
    fn indexed_recovery_rotation_tx_id() {
        let rotation = IndexedRecovery::Rotation(IndexedKeyRotation::new(
            "rotation_tx_123",
            5000,
            "old".to_string(),
            "new".to_string(),
            false,
            "sig1".to_string(),
            "sig2".to_string(),
            None,
        ));
        assert_eq!(rotation.tx_id(), "rotation_tx_123");
    }

    #[test]
    fn block_recovery_stats_add_rotation() {
        let mut stats = BlockRecoveryStats::new();

        let rotation = IndexedRecovery::Rotation(IndexedKeyRotation::new(
            "tx_rot",
            5000,
            "old".to_string(),
            "new".to_string(),
            false,
            "sig1".to_string(),
            "sig2".to_string(),
            None,
        ));
        stats.add(&rotation);

        assert_eq!(stats.key_rotations, 1);
        assert_eq!(stats.total_recovery_txs, 1);
        assert!(!stats.is_empty());

        // Add another rotation
        stats.add(&rotation);
        assert_eq!(stats.key_rotations, 2);
        assert_eq!(stats.total_recovery_txs, 2);
    }

    #[test]
    fn block_recovery_stats_display_with_rotations() {
        let mut stats = BlockRecoveryStats::new();
        stats.key_rotations = 3;
        stats.total_recovery_txs = 3;
        let display = format!("{}", stats);
        assert!(display.contains("rotations: 3"));
    }
}
