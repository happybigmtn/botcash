//! Indexer batch parsing utilities.
//!
//! This module provides utilities for indexers to parse batch messages from
//! transaction memos and expand them into individual indexed actions.
//!
//! # Overview
//!
//! Batch messages (type 0x80) allow multiple social actions to be combined
//! into a single transaction, reducing fees and chain bloat. Indexers need
//! to parse these batches and index each action individually.
//!
//! # Usage
//!
//! ```ignore
//! use zebra_chain::transaction::Memo;
//! use zebra_rpc::indexer::batch::{IndexedBatchAction, parse_batch_from_memo};
//!
//! let actions = parse_batch_from_memo(&memo, "txid123", 1000)?;
//! for action in actions {
//!     // Index each action individually
//!     index_social_action(&action);
//! }
//! ```

use std::fmt;

use serde::{Deserialize, Serialize};
use zebra_chain::transaction::{
    social::{BatchMessage, BatchParseError, SocialMessage, SocialMessageType, SocialParseError},
    Memo,
};

/// Maximum number of actions that can be in a batch (matches chain constant).
pub const MAX_BATCH_ACTIONS: usize = 5;

/// An individual action extracted from a batch, with indexing metadata.
///
/// This struct represents a single social action that was part of a batch
/// transaction, enriched with metadata needed for indexing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedBatchAction {
    /// The transaction ID containing this batch.
    pub tx_id: String,

    /// Zero-based index of this action within the batch.
    pub action_index: u8,

    /// The type of social action.
    pub action_type: u8,

    /// Human-readable name of the action type.
    pub action_type_name: String,

    /// The protocol version of this action.
    pub version: u8,

    /// The raw payload bytes of this action.
    #[serde(with = "hex")]
    pub payload: Vec<u8>,

    /// Whether this action involves value transfer.
    pub is_value_transfer: bool,

    /// Whether this action is an attention market action.
    pub is_attention_market: bool,

    /// Block height where this transaction was included.
    pub block_height: u32,
}

impl IndexedBatchAction {
    /// Creates a new indexed batch action from a social message.
    pub fn from_social_message(
        msg: &SocialMessage,
        tx_id: &str,
        action_index: u8,
        block_height: u32,
    ) -> Self {
        Self {
            tx_id: tx_id.to_string(),
            action_index,
            action_type: msg.msg_type().as_u8(),
            action_type_name: msg.msg_type().name().to_string(),
            version: msg.version(),
            payload: msg.payload().to_vec(),
            is_value_transfer: msg.is_value_transfer(),
            is_attention_market: msg.is_attention_market(),
            block_height,
        }
    }

    /// Returns the social message type if it can be parsed.
    pub fn social_message_type(&self) -> Option<SocialMessageType> {
        SocialMessageType::try_from(self.action_type).ok()
    }
}

impl fmt::Display for IndexedBatchAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BatchAction {{ tx: {}..., index: {}, type: {} ({:#04x}), payload_len: {} }}",
            &self.tx_id[..8.min(self.tx_id.len())],
            self.action_index,
            self.action_type_name,
            self.action_type,
            self.payload.len()
        )
    }
}

/// Summary information about a parsed batch.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchSummary {
    /// The transaction ID containing this batch.
    pub tx_id: String,

    /// Block height where this transaction was included.
    pub block_height: u32,

    /// Number of actions in the batch.
    pub action_count: u8,

    /// Total encoded size of the batch in bytes.
    pub encoded_size: usize,

    /// Protocol version of the batch.
    pub version: u8,

    /// List of action types in the batch (in order).
    pub action_types: Vec<u8>,

    /// Human-readable names of action types.
    pub action_type_names: Vec<String>,
}

impl BatchSummary {
    /// Creates a batch summary from a parsed batch message.
    pub fn from_batch_message(
        batch: &BatchMessage,
        tx_id: &str,
        block_height: u32,
        encoded_size: usize,
    ) -> Self {
        let action_types: Vec<u8> = batch
            .actions()
            .iter()
            .map(|a| a.msg_type().as_u8())
            .collect();
        let action_type_names: Vec<String> = batch
            .actions()
            .iter()
            .map(|a| a.msg_type().name().to_string())
            .collect();

        Self {
            tx_id: tx_id.to_string(),
            block_height,
            action_count: batch.len() as u8,
            encoded_size,
            version: batch.version(),
            action_types,
            action_type_names,
        }
    }
}

impl fmt::Display for BatchSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BatchSummary {{ tx: {}..., actions: {}, types: [{}] }}",
            &self.tx_id[..8.min(self.tx_id.len())],
            self.action_count,
            self.action_type_names.join(", ")
        )
    }
}

/// Errors that can occur during batch indexing operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BatchIndexError {
    /// The memo is not a batch message.
    NotABatch,

    /// Failed to parse the batch message.
    ParseError(BatchParseError),

    /// Failed to parse a social message.
    SocialParseError(SocialParseError),

    /// Invalid transaction ID.
    InvalidTxId,
}

impl fmt::Display for BatchIndexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotABatch => write!(f, "memo is not a batch message"),
            Self::ParseError(e) => write!(f, "batch parse error: {}", e),
            Self::SocialParseError(e) => write!(f, "social parse error: {}", e),
            Self::InvalidTxId => write!(f, "invalid transaction ID"),
        }
    }
}

impl std::error::Error for BatchIndexError {}

impl From<BatchParseError> for BatchIndexError {
    fn from(err: BatchParseError) -> Self {
        Self::ParseError(err)
    }
}

impl From<SocialParseError> for BatchIndexError {
    fn from(err: SocialParseError) -> Self {
        Self::SocialParseError(err)
    }
}

/// Result of parsing a batch for indexing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedBatch {
    /// Summary information about the batch.
    pub summary: BatchSummary,

    /// Individual actions extracted from the batch.
    pub actions: Vec<IndexedBatchAction>,
}

impl ParsedBatch {
    /// Returns true if this batch is empty.
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    /// Returns the number of actions in this batch.
    pub fn len(&self) -> usize {
        self.actions.len()
    }
}

/// Checks if a memo contains a batch message.
///
/// This is a quick check that only looks at the first byte to determine
/// if the memo is a batch (0x80).
pub fn is_batch_memo(memo: &Memo) -> bool {
    let bytes = memo.as_bytes();
    bytes[0] == SocialMessageType::Batch.as_u8()
}

/// Parses a batch message from a memo and returns indexed actions.
///
/// # Arguments
///
/// * `memo` - The memo to parse
/// * `tx_id` - The transaction ID containing this memo
/// * `block_height` - The block height where the transaction was included
///
/// # Returns
///
/// A `ParsedBatch` containing the batch summary and individual actions,
/// or an error if the memo is not a valid batch.
///
/// # Example
///
/// ```ignore
/// let parsed = parse_batch_from_memo(&memo, "txid123", 1000)?;
/// println!("Found {} actions in batch", parsed.len());
/// for action in &parsed.actions {
///     println!("  - {}: {}", action.action_index, action.action_type_name);
/// }
/// ```
pub fn parse_batch_from_memo(
    memo: &Memo,
    tx_id: &str,
    block_height: u32,
) -> Result<ParsedBatch, BatchIndexError> {
    // Quick check for batch type
    if !is_batch_memo(memo) {
        return Err(BatchIndexError::NotABatch);
    }

    // Validate tx_id is not empty
    if tx_id.is_empty() {
        return Err(BatchIndexError::InvalidTxId);
    }

    // Parse the batch message
    let batch = BatchMessage::try_from_memo(memo)?;

    // Calculate encoded size
    let encoded_size = batch.encode().len();

    // Create summary
    let summary = BatchSummary::from_batch_message(&batch, tx_id, block_height, encoded_size);

    // Extract individual actions
    let actions: Vec<IndexedBatchAction> = batch
        .actions()
        .iter()
        .enumerate()
        .map(|(idx, msg)| IndexedBatchAction::from_social_message(msg, tx_id, idx as u8, block_height))
        .collect();

    Ok(ParsedBatch { summary, actions })
}

/// Parses a single social message from a memo for indexing.
///
/// This handles both batch and non-batch messages. For batch messages,
/// it returns all individual actions. For regular messages, it returns
/// a single-element vector.
///
/// # Arguments
///
/// * `memo` - The memo to parse
/// * `tx_id` - The transaction ID containing this memo
/// * `block_height` - The block height where the transaction was included
///
/// # Returns
///
/// A vector of indexed actions (1 for regular messages, 1-5 for batches),
/// or an error if the memo cannot be parsed.
pub fn parse_social_memo_for_indexing(
    memo: &Memo,
    tx_id: &str,
    block_height: u32,
) -> Result<Vec<IndexedBatchAction>, BatchIndexError> {
    // Validate tx_id
    if tx_id.is_empty() {
        return Err(BatchIndexError::InvalidTxId);
    }

    // Check if it's a batch
    if is_batch_memo(memo) {
        let parsed = parse_batch_from_memo(memo, tx_id, block_height)?;
        return Ok(parsed.actions);
    }

    // Try to parse as a regular social message
    let msg = SocialMessage::try_from(memo)?;

    // Create a single indexed action (index 0, not part of a batch)
    let action = IndexedBatchAction::from_social_message(&msg, tx_id, 0, block_height);

    Ok(vec![action])
}

/// Statistics about batch processing in a block.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockBatchStats {
    /// Block height.
    pub block_height: u32,

    /// Total number of transactions in the block.
    pub total_transactions: u32,

    /// Number of transactions containing batch messages.
    pub batch_transactions: u32,

    /// Total number of social actions (individual + expanded from batches).
    pub total_social_actions: u32,

    /// Number of individual (non-batched) social actions.
    pub individual_actions: u32,

    /// Number of actions expanded from batches.
    pub batched_actions: u32,

    /// Average actions per batch multiplied by 100 (fixed-point representation).
    /// Example: 350 means 3.50 actions per batch. 0 if no batches.
    pub avg_actions_per_batch_x100: u32,

    /// Space savings from batching (estimated bytes saved).
    pub estimated_space_savings: u32,
}

impl BlockBatchStats {
    /// Creates a new stats tracker for a block.
    pub fn new(block_height: u32) -> Self {
        Self {
            block_height,
            ..Default::default()
        }
    }

    /// Records a non-batch social transaction.
    pub fn record_individual(&mut self) {
        self.total_transactions += 1;
        self.total_social_actions += 1;
        self.individual_actions += 1;
    }

    /// Records a batch transaction with the given number of actions.
    pub fn record_batch(&mut self, action_count: u32, _encoded_size: u32) {
        self.total_transactions += 1;
        self.batch_transactions += 1;
        self.total_social_actions += action_count;
        self.batched_actions += action_count;

        // Estimate space savings: each individual tx would need ~200 bytes overhead
        // Batch saves: (action_count - 1) * 200 bytes
        if action_count > 1 {
            self.estimated_space_savings += (action_count - 1) * 200;
        }

        // Update average (fixed-point x100)
        if self.batch_transactions > 0 {
            self.avg_actions_per_batch_x100 =
                (self.batched_actions * 100) / self.batch_transactions;
        }
    }
}

impl fmt::Display for BlockBatchStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Block {} stats: {} txs ({} batches), {} actions ({} batched, {} individual)",
            self.block_height,
            self.total_transactions,
            self.batch_transactions,
            self.total_social_actions,
            self.batched_actions,
            self.individual_actions
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zebra_chain::transaction::social::SOCIAL_PROTOCOL_VERSION;

    fn create_memo(bytes: &[u8]) -> Memo {
        Memo::try_from(bytes).expect("valid memo bytes")
    }

    fn create_post_action(content: &[u8]) -> SocialMessage {
        SocialMessage::new(SocialMessageType::Post, SOCIAL_PROTOCOL_VERSION, content.to_vec())
    }

    fn create_follow_action(target: &[u8]) -> SocialMessage {
        SocialMessage::new(SocialMessageType::Follow, SOCIAL_PROTOCOL_VERSION, target.to_vec())
    }

    fn create_tip_action(target_txid: &[u8]) -> SocialMessage {
        SocialMessage::new(SocialMessageType::Tip, SOCIAL_PROTOCOL_VERSION, target_txid.to_vec())
    }

    // ========================================================================
    // Required Tests for P6.1c: Indexer Batch Parsing
    // ========================================================================

    #[test]
    fn test_is_batch_memo() {
        let _init_guard = zebra_test::init();

        // Batch memo (starts with 0x80)
        let batch_memo = create_memo(&[0x80, 0x01, 0x01, 0x04, 0x00, 0x20, 0x01, b'H', b'i']);
        assert!(is_batch_memo(&batch_memo));

        // Non-batch social memo (starts with 0x20 = Post)
        let post_memo = create_memo(&[0x20, 0x01, b'H', b'i']);
        assert!(!is_batch_memo(&post_memo));

        // Empty memo
        let empty_memo = create_memo(&[]);
        assert!(!is_batch_memo(&empty_memo));
    }

    #[test]
    fn test_parse_batch_from_memo() {
        let _init_guard = zebra_test::init();

        // Create a valid batch with 2 actions
        let actions = vec![
            create_post_action(b"Hello!"),
            create_follow_action(b"bs1target..."),
        ];
        let batch = BatchMessage::new(actions).expect("valid batch");
        let encoded = batch.encode();
        let memo = create_memo(&encoded);

        // Parse it
        let parsed = parse_batch_from_memo(&memo, "txid123abc", 1000).expect("should parse");

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed.summary.action_count, 2);
        assert_eq!(parsed.summary.tx_id, "txid123abc");
        assert_eq!(parsed.summary.block_height, 1000);

        // Check first action
        assert_eq!(parsed.actions[0].action_index, 0);
        assert_eq!(parsed.actions[0].action_type, 0x20); // Post
        assert_eq!(parsed.actions[0].action_type_name, "Post");
        assert_eq!(parsed.actions[0].payload, b"Hello!");
        assert!(!parsed.actions[0].is_value_transfer);

        // Check second action
        assert_eq!(parsed.actions[1].action_index, 1);
        assert_eq!(parsed.actions[1].action_type, 0x30); // Follow
        assert_eq!(parsed.actions[1].action_type_name, "Follow");
    }

    #[test]
    fn test_parse_batch_not_a_batch() {
        let _init_guard = zebra_test::init();

        // Regular post memo (not a batch)
        let post_memo = create_memo(&[0x20, 0x01, b'H', b'i']);
        let result = parse_batch_from_memo(&post_memo, "txid123", 1000);

        assert!(matches!(result, Err(BatchIndexError::NotABatch)));
    }

    #[test]
    fn test_parse_batch_invalid_tx_id() {
        let _init_guard = zebra_test::init();

        let actions = vec![create_post_action(b"Hello!")];
        let batch = BatchMessage::new(actions).expect("valid batch");
        let encoded = batch.encode();
        let memo = create_memo(&encoded);

        // Empty tx_id should fail
        let result = parse_batch_from_memo(&memo, "", 1000);
        assert!(matches!(result, Err(BatchIndexError::InvalidTxId)));
    }

    #[test]
    fn test_parse_social_memo_for_indexing_batch() {
        let _init_guard = zebra_test::init();

        // Create a batch with 3 actions
        let actions = vec![
            create_post_action(b"Post 1"),
            create_post_action(b"Post 2"),
            create_follow_action(b"target"),
        ];
        let batch = BatchMessage::new(actions).expect("valid batch");
        let encoded = batch.encode();
        let memo = create_memo(&encoded);

        let indexed = parse_social_memo_for_indexing(&memo, "txid456", 2000).expect("should parse");

        assert_eq!(indexed.len(), 3);
        assert_eq!(indexed[0].action_type_name, "Post");
        assert_eq!(indexed[1].action_type_name, "Post");
        assert_eq!(indexed[2].action_type_name, "Follow");
    }

    #[test]
    fn test_parse_social_memo_for_indexing_single() {
        let _init_guard = zebra_test::init();

        // Single post memo (not a batch)
        let post_memo = create_memo(&[0x20, 0x01, b'T', b'e', b's', b't']);

        let indexed =
            parse_social_memo_for_indexing(&post_memo, "txid789", 3000).expect("should parse");

        assert_eq!(indexed.len(), 1);
        assert_eq!(indexed[0].action_index, 0);
        assert_eq!(indexed[0].action_type, 0x20);
        assert_eq!(indexed[0].action_type_name, "Post");
        assert_eq!(indexed[0].tx_id, "txid789");
        assert_eq!(indexed[0].block_height, 3000);
    }

    #[test]
    fn test_indexed_batch_action_value_transfer() {
        let _init_guard = zebra_test::init();

        // Create batch with value transfer actions
        let actions = vec![
            create_post_action(b"Hello"),
            create_tip_action(&[0xAB; 32]),
        ];
        let batch = BatchMessage::new(actions).expect("valid batch");
        let encoded = batch.encode();
        let memo = create_memo(&encoded);

        let parsed = parse_batch_from_memo(&memo, "txid_tip", 4000).expect("should parse");

        // Post is not value transfer
        assert!(!parsed.actions[0].is_value_transfer);

        // Tip is value transfer
        assert!(parsed.actions[1].is_value_transfer);
    }

    #[test]
    fn test_batch_summary_display() {
        let _init_guard = zebra_test::init();

        let actions = vec![
            create_post_action(b"Post"),
            create_follow_action(b"Follow"),
        ];
        let batch = BatchMessage::new(actions).expect("valid batch");
        let encoded = batch.encode();
        let summary = BatchSummary::from_batch_message(&batch, "txid12345678", 5000, encoded.len());

        let display = format!("{}", summary);
        assert!(display.contains("txid1234"));
        assert!(display.contains("actions: 2"));
        assert!(display.contains("Post"));
        assert!(display.contains("Follow"));
    }

    #[test]
    fn test_indexed_batch_action_display() {
        let _init_guard = zebra_test::init();

        let msg = create_post_action(b"Content");
        let action = IndexedBatchAction::from_social_message(&msg, "txid_abcdef12", 0, 6000);

        let display = format!("{}", action);
        assert!(display.contains("txid_abc"));
        assert!(display.contains("index: 0"));
        assert!(display.contains("Post"));
        assert!(display.contains("0x20"));
    }

    #[test]
    fn test_block_batch_stats() {
        let _init_guard = zebra_test::init();

        let mut stats = BlockBatchStats::new(7000);

        // Record 2 individual transactions
        stats.record_individual();
        stats.record_individual();

        // Record 1 batch with 3 actions
        stats.record_batch(3, 100);

        // Record 1 batch with 5 actions
        stats.record_batch(5, 150);

        assert_eq!(stats.block_height, 7000);
        assert_eq!(stats.total_transactions, 4);
        assert_eq!(stats.batch_transactions, 2);
        assert_eq!(stats.individual_actions, 2);
        assert_eq!(stats.batched_actions, 8);
        assert_eq!(stats.total_social_actions, 10);
        // 8 actions / 2 batches = 4.0 avg, represented as 400 (x100)
        assert_eq!(stats.avg_actions_per_batch_x100, 400);
        // Space savings: (3-1)*200 + (5-1)*200 = 400 + 800 = 1200
        assert_eq!(stats.estimated_space_savings, 1200);
    }

    #[test]
    fn test_batch_index_error_display() {
        let _init_guard = zebra_test::init();

        let err = BatchIndexError::NotABatch;
        assert_eq!(format!("{}", err), "memo is not a batch message");

        let err = BatchIndexError::InvalidTxId;
        assert_eq!(format!("{}", err), "invalid transaction ID");
    }

    #[test]
    fn test_parse_batch_max_actions() {
        let _init_guard = zebra_test::init();

        // Create batch with maximum allowed actions (5)
        let actions: Vec<SocialMessage> = (0..5)
            .map(|i| create_post_action(format!("Post {}", i).as_bytes()))
            .collect();
        let batch = BatchMessage::new(actions).expect("valid batch");
        let encoded = batch.encode();
        let memo = create_memo(&encoded);

        let parsed = parse_batch_from_memo(&memo, "txid_max", 8000).expect("should parse");

        assert_eq!(parsed.len(), 5);
        assert_eq!(parsed.summary.action_count, 5);
    }

    #[test]
    fn test_parse_batch_mixed_action_types() {
        let _init_guard = zebra_test::init();

        // Create batch with different action types
        let actions = vec![
            SocialMessage::new(SocialMessageType::Profile, SOCIAL_PROTOCOL_VERSION, b"name=Test".to_vec()),
            SocialMessage::new(SocialMessageType::Post, SOCIAL_PROTOCOL_VERSION, b"Hello world".to_vec()),
            SocialMessage::new(SocialMessageType::Dm, SOCIAL_PROTOCOL_VERSION, b"encrypted_msg".to_vec()),
            SocialMessage::new(SocialMessageType::AttentionBoost, SOCIAL_PROTOCOL_VERSION, vec![0xAB; 37]),
        ];
        let batch = BatchMessage::new(actions).expect("valid batch");
        let encoded = batch.encode();
        let memo = create_memo(&encoded);

        let parsed = parse_batch_from_memo(&memo, "txid_mixed", 9000).expect("should parse");

        assert_eq!(parsed.len(), 4);
        assert_eq!(parsed.actions[0].action_type_name, "Profile");
        assert_eq!(parsed.actions[1].action_type_name, "Post");
        assert_eq!(parsed.actions[2].action_type_name, "DM");
        assert_eq!(parsed.actions[3].action_type_name, "AttentionBoost");

        // AttentionBoost should be attention market
        assert!(parsed.actions[3].is_attention_market);
        assert!(parsed.actions[3].is_value_transfer);
    }

    #[test]
    fn test_indexed_action_social_message_type() {
        let _init_guard = zebra_test::init();

        let msg = create_post_action(b"Test");
        let action = IndexedBatchAction::from_social_message(&msg, "txid", 0, 1000);

        let msg_type = action.social_message_type();
        assert_eq!(msg_type, Some(SocialMessageType::Post));
    }

    #[test]
    fn test_parsed_batch_is_empty() {
        let _init_guard = zebra_test::init();

        let actions = vec![create_post_action(b"Single")];
        let batch = BatchMessage::new(actions).expect("valid batch");
        let encoded = batch.encode();
        let memo = create_memo(&encoded);

        let parsed = parse_batch_from_memo(&memo, "txid", 1000).expect("should parse");

        assert!(!parsed.is_empty());
        assert_eq!(parsed.len(), 1);
    }

    #[test]
    fn test_block_batch_stats_display() {
        let _init_guard = zebra_test::init();

        let mut stats = BlockBatchStats::new(10000);
        stats.record_individual();
        stats.record_batch(3, 100);

        let display = format!("{}", stats);
        assert!(display.contains("Block 10000"));
        assert!(display.contains("2 txs"));
        assert!(display.contains("1 batches"));
        assert!(display.contains("4 actions"));
    }
}
