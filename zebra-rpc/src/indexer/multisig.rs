//! Indexer multi-sig identity parsing and tracking utilities.
//!
//! This module provides utilities for indexers to:
//! - Parse multi-sig setup messages (0xF5) from transaction memos
//! - Parse multi-sig action messages (0xF6) from transaction memos
//! - Track multi-sig identity configurations
//! - Verify signature counts against thresholds
//!
//! # Overview
//!
//! Multi-sig identities (types 0xF5-0xF6) enable M-of-N signature requirements
//! for high-value accounts (influencers, businesses, agents with significant stake).
//!
//! The multi-sig system consists of two message types:
//!
//! 1. **Setup (0xF5)**: Configures an address as a multi-sig identity
//! 2. **Action (0xF6)**: A social action with multiple signatures
//!
//! # Multi-Sig Message Types
//!
//! - `MultisigSetup (0xF5)`: Configures M-of-N signature requirements
//! - `MultisigAction (0xF6)`: Wraps a social action with required signatures
//!
//! # Usage
//!
//! ```ignore
//! use zebra_chain::transaction::Memo;
//! use zebra_rpc::indexer::multisig::{parse_multisig_memo, IndexedMultisig};
//!
//! let multisig = parse_multisig_memo(&memo, "txid123", 1000)?;
//! match multisig {
//!     IndexedMultisig::Setup(setup) => {
//!         println!("Multi-sig setup: {}-of-{} keys",
//!                  setup.threshold, setup.key_count);
//!     }
//!     IndexedMultisig::Action(action) => {
//!         println!("Multi-sig action: {} type with {} sigs",
//!                  action.action_type, action.signature_count);
//!     }
//! }
//! ```

use std::fmt;

use serde::{Deserialize, Serialize};
use zebra_chain::transaction::{
    social::{SocialMessage, SocialMessageType, SocialParseError},
    Memo,
};

/// Minimum number of keys for a multi-sig identity.
pub const MIN_MULTISIG_KEYS: usize = 2;

/// Maximum number of keys for a multi-sig identity.
pub const MAX_MULTISIG_KEYS: usize = 15;

/// Size of a compressed public key in bytes.
pub const COMPRESSED_PUBKEY_SIZE: usize = 33;

/// Size of a Schnorr signature in bytes.
pub const SCHNORR_SIGNATURE_SIZE: usize = 64;

/// Multi-sig identity status.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum MultisigState {
    /// Multi-sig is active and operational.
    #[default]
    Active,
    /// Multi-sig setup is pending (waiting for confirmation).
    Pending,
    /// Multi-sig has been revoked/disabled.
    Revoked,
}

impl MultisigState {
    /// Returns the string representation of this state.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Pending => "pending",
            Self::Revoked => "revoked",
        }
    }
}

impl fmt::Display for MultisigState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// An indexed multi-sig setup extracted from a memo.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedMultisigSetup {
    /// The transaction ID containing this setup.
    pub tx_id: String,

    /// Block height where this setup was created.
    pub setup_block: u32,

    /// The address configured as multi-sig.
    pub address: String,

    /// Number of keys in the multi-sig.
    pub key_count: u8,

    /// Number of signatures required (M of N).
    pub threshold: u8,

    /// Protocol version.
    pub version: u8,

    /// The compressed public keys (hex-encoded).
    pub public_keys: Vec<String>,

    /// Current state of the multi-sig.
    pub state: MultisigState,
}

impl IndexedMultisigSetup {
    /// Creates a new indexed multi-sig setup from parsed data.
    pub fn new(
        tx_id: &str,
        setup_block: u32,
        address: &str,
        key_count: u8,
        threshold: u8,
        version: u8,
        public_keys: Vec<String>,
    ) -> Self {
        Self {
            tx_id: tx_id.to_string(),
            setup_block,
            address: address.to_string(),
            key_count,
            threshold,
            version,
            public_keys,
            state: MultisigState::Active,
        }
    }

    /// Checks if a given number of signatures meets the threshold.
    pub fn meets_threshold(&self, signature_count: u8) -> bool {
        signature_count >= self.threshold
    }
}

/// An indexed multi-sig action extracted from a memo.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedMultisigAction {
    /// The transaction ID containing this action.
    pub tx_id: String,

    /// Block height where this action was included.
    pub action_block: u32,

    /// The multi-sig address that performed this action.
    pub multisig_address: String,

    /// The type of wrapped action (e.g., "Post", "Follow").
    pub action_type: String,

    /// The wrapped action payload.
    pub action_payload: Vec<u8>,

    /// Number of signatures provided.
    pub signature_count: u8,

    /// List of (key_index, signature) pairs.
    pub signatures: Vec<IndexedSignature>,

    /// Protocol version.
    pub version: u8,
}

impl IndexedMultisigAction {
    /// Creates a new indexed multi-sig action from parsed data.
    pub fn new(
        tx_id: &str,
        action_block: u32,
        multisig_address: &str,
        action_type: &str,
        action_payload: Vec<u8>,
        signatures: Vec<IndexedSignature>,
        version: u8,
    ) -> Self {
        Self {
            tx_id: tx_id.to_string(),
            action_block,
            multisig_address: multisig_address.to_string(),
            action_type: action_type.to_string(),
            action_payload,
            signature_count: signatures.len() as u8,
            signatures,
            version,
        }
    }
}

/// An indexed signature within a multi-sig action.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedSignature {
    /// Index of the key that made this signature (0-based).
    pub key_index: u8,

    /// The signature (hex-encoded, 64 bytes).
    pub signature: String,
}

impl IndexedSignature {
    /// Creates a new indexed signature.
    pub fn new(key_index: u8, signature: String) -> Self {
        Self { key_index, signature }
    }
}

/// A unified enum for multi-sig indexed events.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexedMultisig {
    /// A multi-sig setup event.
    Setup(IndexedMultisigSetup),
    /// A multi-sig action event.
    Action(IndexedMultisigAction),
}

/// Errors that can occur when parsing multi-sig messages.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MultisigParseError {
    /// The memo does not contain a social message.
    NotSocialMessage,
    /// The memo is not a multi-sig message.
    NotMultisigMessage,
    /// Failed to parse the social message.
    SocialParseError(SocialParseError),
    /// The payload is too short.
    PayloadTooShort { actual: usize, minimum: usize },
    /// Invalid key count.
    InvalidKeyCount(u8),
    /// Invalid threshold.
    InvalidThreshold { threshold: u8, key_count: u8 },
    /// Invalid public key length.
    InvalidPublicKeyLength { index: usize, actual: usize },
    /// Invalid signature length.
    InvalidSignatureLength { index: usize, actual: usize },
}

impl fmt::Display for MultisigParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotSocialMessage => write!(f, "memo is not a social message"),
            Self::NotMultisigMessage => write!(f, "memo is not a multi-sig message"),
            Self::SocialParseError(e) => write!(f, "social parse error: {}", e),
            Self::PayloadTooShort { actual, minimum } => {
                write!(f, "payload too short: {} bytes, minimum {} required", actual, minimum)
            }
            Self::InvalidKeyCount(count) => {
                write!(f, "invalid key count: {} (must be 2-15)", count)
            }
            Self::InvalidThreshold { threshold, key_count } => {
                write!(f, "invalid threshold: {} for {} keys", threshold, key_count)
            }
            Self::InvalidPublicKeyLength { index, actual } => {
                write!(f, "invalid public key length at index {}: {} bytes", index, actual)
            }
            Self::InvalidSignatureLength { index, actual } => {
                write!(f, "invalid signature length at index {}: {} bytes", index, actual)
            }
        }
    }
}

impl std::error::Error for MultisigParseError {}

impl From<SocialParseError> for MultisigParseError {
    fn from(e: SocialParseError) -> Self {
        Self::SocialParseError(e)
    }
}

/// Checks if a memo contains a multi-sig message.
pub fn is_multisig_memo(memo: &Memo) -> bool {
    let bytes = memo.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    matches!(bytes[0], 0xF5 | 0xF6)
}

/// Determines the multi-sig message type from a memo.
pub fn multisig_type_from_memo(memo: &Memo) -> Option<SocialMessageType> {
    let bytes = memo.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    match bytes[0] {
        0xF5 => Some(SocialMessageType::MultisigSetup),
        0xF6 => Some(SocialMessageType::MultisigAction),
        _ => None,
    }
}

/// Parses a multi-sig message from a memo.
///
/// # Arguments
///
/// * `memo` - The transaction memo to parse
/// * `tx_id` - The transaction ID (for indexing)
/// * `block_height` - The block height (for indexing)
///
/// # Returns
///
/// The parsed multi-sig event, or an error if parsing fails.
pub fn parse_multisig_memo(
    memo: &Memo,
    tx_id: &str,
    block_height: u32,
) -> Result<IndexedMultisig, MultisigParseError> {
    // First try to parse as a social message
    let msg = SocialMessage::try_from(memo)?;

    match msg.msg_type() {
        SocialMessageType::MultisigSetup => {
            parse_multisig_setup(&msg, tx_id, block_height)
        }
        SocialMessageType::MultisigAction => {
            parse_multisig_action(&msg, tx_id, block_height)
        }
        _ => Err(MultisigParseError::NotMultisigMessage),
    }
}

/// Parses a multi-sig setup from a social message.
fn parse_multisig_setup(
    msg: &SocialMessage,
    tx_id: &str,
    block_height: u32,
) -> Result<IndexedMultisig, MultisigParseError> {
    let payload = msg.payload();

    // Format: [key_count(1)][pubkey1(33)]...[pubkeyN(33)][threshold(1)]
    // Minimum: 1 + 2*33 + 1 = 68 bytes for 2 keys
    if payload.is_empty() {
        return Err(MultisigParseError::PayloadTooShort {
            actual: 0,
            minimum: 68,
        });
    }

    let key_count = payload[0];

    // Validate key count
    if key_count < MIN_MULTISIG_KEYS as u8 || key_count > MAX_MULTISIG_KEYS as u8 {
        return Err(MultisigParseError::InvalidKeyCount(key_count));
    }

    let expected_len = 1 + (key_count as usize * COMPRESSED_PUBKEY_SIZE) + 1;
    if payload.len() < expected_len {
        return Err(MultisigParseError::PayloadTooShort {
            actual: payload.len(),
            minimum: expected_len,
        });
    }

    // Parse public keys
    let mut public_keys = Vec::with_capacity(key_count as usize);
    let mut offset = 1;
    for i in 0..key_count as usize {
        let key_end = offset + COMPRESSED_PUBKEY_SIZE;
        if key_end > payload.len() {
            return Err(MultisigParseError::InvalidPublicKeyLength {
                index: i,
                actual: payload.len() - offset,
            });
        }
        let key_bytes = &payload[offset..key_end];
        public_keys.push(hex::encode(key_bytes));
        offset = key_end;
    }

    // Parse threshold
    let threshold = payload[offset];

    // Validate threshold
    if threshold < 1 || threshold > key_count {
        return Err(MultisigParseError::InvalidThreshold { threshold, key_count });
    }

    // Use tx_id as the address for now (in practice, would be derived from sender)
    let setup = IndexedMultisigSetup::new(
        tx_id,
        block_height,
        tx_id, // Placeholder - would be sender address
        key_count,
        threshold,
        msg.version(),
        public_keys,
    );

    Ok(IndexedMultisig::Setup(setup))
}

/// Parses a multi-sig action from a social message.
fn parse_multisig_action(
    msg: &SocialMessage,
    tx_id: &str,
    block_height: u32,
) -> Result<IndexedMultisig, MultisigParseError> {
    let payload = msg.payload();

    // Format: [action_type(1)][action_len(2)][action][sig_count(1)][sig1_idx(1)][sig1(64)]...
    // Minimum: 1 + 2 + 0 + 1 + 1 + 64 = 69 bytes for 1 signature
    if payload.len() < 5 {
        return Err(MultisigParseError::PayloadTooShort {
            actual: payload.len(),
            minimum: 5,
        });
    }

    let action_type_byte = payload[0];
    let action_len = u16::from_le_bytes([payload[1], payload[2]]) as usize;

    // Validate we have enough bytes for the action
    let action_end = 3 + action_len;
    if payload.len() < action_end + 1 {
        return Err(MultisigParseError::PayloadTooShort {
            actual: payload.len(),
            minimum: action_end + 1,
        });
    }

    let action_payload = payload[3..action_end].to_vec();
    let sig_count = payload[action_end];

    // Parse signatures
    let mut signatures = Vec::with_capacity(sig_count as usize);
    let mut offset = action_end + 1;

    for i in 0..sig_count as usize {
        // Each signature: [key_index(1)][signature(64)]
        if offset + 1 + SCHNORR_SIGNATURE_SIZE > payload.len() {
            return Err(MultisigParseError::InvalidSignatureLength {
                index: i,
                actual: payload.len().saturating_sub(offset + 1),
            });
        }

        let key_index = payload[offset];
        let sig_bytes = &payload[offset + 1..offset + 1 + SCHNORR_SIGNATURE_SIZE];
        signatures.push(IndexedSignature::new(key_index, hex::encode(sig_bytes)));
        offset += 1 + SCHNORR_SIGNATURE_SIZE;
    }

    // Convert action type byte to name
    let action_type_name = match SocialMessageType::try_from(action_type_byte) {
        Ok(t) => t.name().to_string(),
        Err(_) => format!("Unknown(0x{:02X})", action_type_byte),
    };

    let action = IndexedMultisigAction::new(
        tx_id,
        block_height,
        tx_id, // Placeholder - would be sender address
        &action_type_name,
        action_payload,
        signatures,
        msg.version(),
    );

    Ok(IndexedMultisig::Action(action))
}

/// Statistics for multi-sig events in a block.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockMultisigStats {
    /// Number of multi-sig setups in this block.
    pub setup_count: u32,
    /// Number of multi-sig actions in this block.
    pub action_count: u32,
    /// Total number of keys across all setups.
    pub total_keys: u32,
    /// Total number of signatures across all actions.
    pub total_signatures: u32,
}

impl BlockMultisigStats {
    /// Creates new empty stats.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a multi-sig setup.
    pub fn record_setup(&mut self, key_count: u8) {
        self.setup_count += 1;
        self.total_keys += key_count as u32;
    }

    /// Records a multi-sig action.
    pub fn record_action(&mut self, signature_count: u8) {
        self.action_count += 1;
        self.total_signatures += signature_count as u32;
    }

    /// Merges another stats into this one.
    pub fn merge(&mut self, other: &Self) {
        self.setup_count += other.setup_count;
        self.action_count += other.action_count;
        self.total_keys += other.total_keys;
        self.total_signatures += other.total_signatures;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_memo(bytes: &[u8]) -> Memo {
        Memo::try_from(bytes).expect("valid memo bytes")
    }

    #[test]
    fn is_multisig_memo_setup() {
        let _init_guard = zebra_test::init();

        let memo = create_memo(&[0xF5, 0x01, 2]);
        assert!(is_multisig_memo(&memo));
    }

    #[test]
    fn is_multisig_memo_action() {
        let _init_guard = zebra_test::init();

        let memo = create_memo(&[0xF6, 0x01, 0x20]);
        assert!(is_multisig_memo(&memo));
    }

    #[test]
    fn is_not_multisig_memo() {
        let _init_guard = zebra_test::init();

        // Post message
        let memo = create_memo(&[0x20, 0x01]);
        assert!(!is_multisig_memo(&memo));

        // Recovery config
        let memo = create_memo(&[0xF0, 0x01]);
        assert!(!is_multisig_memo(&memo));
    }

    #[test]
    fn multisig_type_from_memo_setup() {
        let _init_guard = zebra_test::init();

        let memo = create_memo(&[0xF5, 0x01, 2]);
        assert_eq!(multisig_type_from_memo(&memo), Some(SocialMessageType::MultisigSetup));
    }

    #[test]
    fn multisig_type_from_memo_action() {
        let _init_guard = zebra_test::init();

        let memo = create_memo(&[0xF6, 0x01, 0x20]);
        assert_eq!(multisig_type_from_memo(&memo), Some(SocialMessageType::MultisigAction));
    }

    #[test]
    fn multisig_type_from_memo_other() {
        let _init_guard = zebra_test::init();

        let memo = create_memo(&[0x20, 0x01]);
        assert_eq!(multisig_type_from_memo(&memo), None);
    }

    #[test]
    fn parse_multisig_setup_valid() {
        let _init_guard = zebra_test::init();

        // Create a valid 2-of-3 setup
        let mut payload = Vec::new();
        payload.push(3u8); // 3 keys

        // 3 public keys
        for i in 0..3 {
            let mut key = vec![if i % 2 == 0 { 0x02 } else { 0x03 }];
            key.extend_from_slice(&[i as u8; 32]);
            payload.extend_from_slice(&key);
        }

        payload.push(2u8); // threshold = 2

        let mut memo_bytes = vec![0xF5, 0x01]; // MultisigSetup, version 1
        memo_bytes.extend_from_slice(&payload);
        let memo = create_memo(&memo_bytes);

        let result = parse_multisig_memo(&memo, "txid123", 1000);
        assert!(result.is_ok());

        if let Ok(IndexedMultisig::Setup(setup)) = result {
            assert_eq!(setup.tx_id, "txid123");
            assert_eq!(setup.setup_block, 1000);
            assert_eq!(setup.key_count, 3);
            assert_eq!(setup.threshold, 2);
            assert_eq!(setup.public_keys.len(), 3);
            assert!(setup.meets_threshold(2));
            assert!(!setup.meets_threshold(1));
        } else {
            panic!("Expected Setup variant");
        }
    }

    #[test]
    fn parse_multisig_setup_invalid_key_count() {
        let _init_guard = zebra_test::init();

        // Only 1 key (minimum is 2)
        let mut memo_bytes = vec![0xF5, 0x01];
        memo_bytes.push(1u8); // 1 key (invalid)
        memo_bytes.extend_from_slice(&[0x02; 33]); // one key
        memo_bytes.push(1u8); // threshold

        let memo = create_memo(&memo_bytes);
        let result = parse_multisig_memo(&memo, "txid", 1000);

        assert!(matches!(result, Err(MultisigParseError::InvalidKeyCount(1))));
    }

    #[test]
    fn parse_multisig_setup_invalid_threshold() {
        let _init_guard = zebra_test::init();

        // Threshold > key count
        let mut memo_bytes = vec![0xF5, 0x01];
        memo_bytes.push(2u8); // 2 keys
        memo_bytes.extend_from_slice(&[0x02; 33]); // key 1
        memo_bytes.extend_from_slice(&[0x03; 33]); // key 2
        memo_bytes.push(3u8); // threshold = 3 (invalid, only 2 keys)

        let memo = create_memo(&memo_bytes);
        let result = parse_multisig_memo(&memo, "txid", 1000);

        assert!(matches!(
            result,
            Err(MultisigParseError::InvalidThreshold { threshold: 3, key_count: 2 })
        ));
    }

    #[test]
    fn parse_multisig_action_valid() {
        let _init_guard = zebra_test::init();

        // Create a valid multi-sig action (Post with 2 signatures)
        let mut payload = Vec::new();
        payload.push(0x20u8); // action_type = Post
        payload.extend_from_slice(&10u16.to_le_bytes()); // action_len = 10
        payload.extend_from_slice(b"Hello!!!!!"); // action content (10 bytes)
        payload.push(2u8); // 2 signatures

        // Signature 1
        payload.push(0u8); // key_index = 0
        payload.extend_from_slice(&[0xAA; 64]); // signature

        // Signature 2
        payload.push(2u8); // key_index = 2
        payload.extend_from_slice(&[0xBB; 64]); // signature

        let mut memo_bytes = vec![0xF6, 0x01]; // MultisigAction, version 1
        memo_bytes.extend_from_slice(&payload);
        let memo = create_memo(&memo_bytes);

        let result = parse_multisig_memo(&memo, "txid456", 2000);
        assert!(result.is_ok());

        if let Ok(IndexedMultisig::Action(action)) = result {
            assert_eq!(action.tx_id, "txid456");
            assert_eq!(action.action_block, 2000);
            assert_eq!(action.action_type, "Post");
            assert_eq!(action.action_payload, b"Hello!!!!!");
            assert_eq!(action.signature_count, 2);
            assert_eq!(action.signatures.len(), 2);
            assert_eq!(action.signatures[0].key_index, 0);
            assert_eq!(action.signatures[1].key_index, 2);
        } else {
            panic!("Expected Action variant");
        }
    }

    #[test]
    fn parse_multisig_action_payload_too_short() {
        let _init_guard = zebra_test::init();

        // Too short to even have header
        let memo = create_memo(&[0xF6, 0x01, 0x20]);
        let result = parse_multisig_memo(&memo, "txid", 1000);

        assert!(matches!(result, Err(MultisigParseError::PayloadTooShort { .. })));
    }

    #[test]
    fn multisig_state_display() {
        let _init_guard = zebra_test::init();

        assert_eq!(MultisigState::Active.as_str(), "active");
        assert_eq!(MultisigState::Pending.as_str(), "pending");
        assert_eq!(MultisigState::Revoked.as_str(), "revoked");

        assert_eq!(format!("{}", MultisigState::Active), "active");
    }

    #[test]
    fn block_multisig_stats() {
        let _init_guard = zebra_test::init();

        let mut stats = BlockMultisigStats::new();
        assert_eq!(stats.setup_count, 0);
        assert_eq!(stats.action_count, 0);

        stats.record_setup(3);
        assert_eq!(stats.setup_count, 1);
        assert_eq!(stats.total_keys, 3);

        stats.record_action(2);
        assert_eq!(stats.action_count, 1);
        assert_eq!(stats.total_signatures, 2);

        let mut other = BlockMultisigStats::new();
        other.record_setup(5);
        other.record_action(3);

        stats.merge(&other);
        assert_eq!(stats.setup_count, 2);
        assert_eq!(stats.action_count, 2);
        assert_eq!(stats.total_keys, 8);
        assert_eq!(stats.total_signatures, 5);
    }

    #[test]
    fn multisig_parse_error_display() {
        let _init_guard = zebra_test::init();

        let err = MultisigParseError::InvalidKeyCount(1);
        assert!(err.to_string().contains("invalid key count"));

        let err = MultisigParseError::InvalidThreshold { threshold: 5, key_count: 3 };
        assert!(err.to_string().contains("invalid threshold"));

        let err = MultisigParseError::PayloadTooShort { actual: 10, minimum: 68 };
        assert!(err.to_string().contains("too short"));
    }

    #[test]
    fn indexed_multisig_setup_meets_threshold() {
        let _init_guard = zebra_test::init();

        let setup = IndexedMultisigSetup::new(
            "tx123",
            1000,
            "address",
            5,
            3,
            1,
            vec!["key1".to_string(), "key2".to_string()],
        );

        assert!(setup.meets_threshold(3));
        assert!(setup.meets_threshold(4));
        assert!(setup.meets_threshold(5));
        assert!(!setup.meets_threshold(2));
        assert!(!setup.meets_threshold(1));
    }

    #[test]
    fn not_multisig_message_error() {
        let _init_guard = zebra_test::init();

        // A regular Post message
        let memo = create_memo(&[0x20, 0x01, 0x48, 0x69]); // Post, version 1, "Hi"
        let result = parse_multisig_memo(&memo, "txid", 1000);

        assert!(matches!(result, Err(MultisigParseError::NotMultisigMessage)));
    }
}
