//! Indexer channel parsing utilities.
//!
//! This module provides utilities for indexers to parse channel messages from
//! transaction memos and extract channel state information.
//!
//! # Overview
//!
//! Channel messages (types 0xC0, 0xC1, 0xC2) enable Layer-2 social channels for
//! high-frequency interactions like chat. Indexers need to track channel state
//! (open, closing, settled) and parse the relevant data from each message type.
//!
//! # Channel Types
//!
//! - `ChannelOpen` (0xC0): Opens a new channel between parties with a deposit
//! - `ChannelClose` (0xC1): Initiates cooperative channel close
//! - `ChannelSettle` (0xC2): Finalizes channel with message hash proof
//!
//! # Usage
//!
//! ```ignore
//! use zebra_chain::transaction::Memo;
//! use zebra_rpc::indexer::channels::{parse_channel_memo, IndexedChannel};
//!
//! let channel = parse_channel_memo(&memo, "txid123", 1000)?;
//! match channel {
//!     IndexedChannel::Open(open) => {
//!         println!("Channel opened with {} parties", open.parties.len());
//!     }
//!     IndexedChannel::Close(close) => {
//!         println!("Channel {} closing at seq {}", close.channel_id, close.final_seq);
//!     }
//!     IndexedChannel::Settle(settle) => {
//!         println!("Channel {} settled", settle.channel_id);
//!     }
//! }
//! ```

use std::fmt;

use serde::{Deserialize, Serialize};
use zebra_chain::transaction::{
    social::{SocialMessage, SocialMessageType, SocialParseError},
    Memo,
};

/// Maximum number of parties in a channel.
pub const MAX_CHANNEL_PARTIES: usize = 10;

/// Minimum deposit for opening a channel (in zatoshis).
pub const MIN_CHANNEL_DEPOSIT: u64 = 100_000;

/// Default timeout for channel operations (in blocks).
pub const DEFAULT_CHANNEL_TIMEOUT_BLOCKS: u32 = 1440;

/// An indexed channel open event extracted from a memo.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedChannelOpen {
    /// The transaction ID containing this channel open.
    pub tx_id: String,

    /// Block height where this transaction was included.
    pub block_height: u32,

    /// List of party addresses in the channel.
    pub parties: Vec<String>,

    /// Deposit amount in zatoshis.
    pub deposit: u64,

    /// Timeout in blocks for channel operations.
    pub timeout_blocks: u32,

    /// Protocol version.
    pub version: u8,
}

impl IndexedChannelOpen {
    /// Creates a new indexed channel open from parsed data.
    pub fn new(
        tx_id: &str,
        block_height: u32,
        parties: Vec<String>,
        deposit: u64,
        timeout_blocks: u32,
        version: u8,
    ) -> Self {
        Self {
            tx_id: tx_id.to_string(),
            block_height,
            parties,
            deposit,
            timeout_blocks,
            version,
        }
    }

    /// Returns the number of parties in the channel.
    pub fn party_count(&self) -> usize {
        self.parties.len()
    }

    /// Returns true if the deposit meets the minimum requirement.
    pub fn has_valid_deposit(&self) -> bool {
        self.deposit >= MIN_CHANNEL_DEPOSIT
    }
}

impl fmt::Display for IndexedChannelOpen {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ChannelOpen {{ tx: {}..., parties: {}, deposit: {}, timeout: {} }}",
            &self.tx_id[..8.min(self.tx_id.len())],
            self.parties.len(),
            self.deposit,
            self.timeout_blocks
        )
    }
}

/// An indexed channel close event extracted from a memo.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedChannelClose {
    /// The transaction ID containing this channel close.
    pub tx_id: String,

    /// Block height where this transaction was included.
    pub block_height: u32,

    /// The channel ID being closed (32-byte hex string).
    pub channel_id: String,

    /// Final sequence number of messages in the channel.
    pub final_seq: u32,

    /// Protocol version.
    pub version: u8,
}

impl IndexedChannelClose {
    /// Creates a new indexed channel close from parsed data.
    pub fn new(
        tx_id: &str,
        block_height: u32,
        channel_id: String,
        final_seq: u32,
        version: u8,
    ) -> Self {
        Self {
            tx_id: tx_id.to_string(),
            block_height,
            channel_id,
            final_seq,
            version,
        }
    }
}

impl fmt::Display for IndexedChannelClose {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ChannelClose {{ tx: {}..., channel: {}..., final_seq: {} }}",
            &self.tx_id[..8.min(self.tx_id.len())],
            &self.channel_id[..8.min(self.channel_id.len())],
            self.final_seq
        )
    }
}

/// An indexed channel settle event extracted from a memo.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedChannelSettle {
    /// The transaction ID containing this channel settlement.
    pub tx_id: String,

    /// Block height where this transaction was included.
    pub block_height: u32,

    /// The channel ID being settled (32-byte hex string).
    pub channel_id: String,

    /// Final sequence number of messages in the channel.
    pub final_seq: u32,

    /// Merkle root hash of all messages in the channel (32-byte hex string).
    pub message_hash: String,

    /// Protocol version.
    pub version: u8,
}

impl IndexedChannelSettle {
    /// Creates a new indexed channel settle from parsed data.
    pub fn new(
        tx_id: &str,
        block_height: u32,
        channel_id: String,
        final_seq: u32,
        message_hash: String,
        version: u8,
    ) -> Self {
        Self {
            tx_id: tx_id.to_string(),
            block_height,
            channel_id,
            final_seq,
            message_hash,
            version,
        }
    }
}

impl fmt::Display for IndexedChannelSettle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ChannelSettle {{ tx: {}..., channel: {}..., final_seq: {}, hash: {}... }}",
            &self.tx_id[..8.min(self.tx_id.len())],
            &self.channel_id[..8.min(self.channel_id.len())],
            self.final_seq,
            &self.message_hash[..8.min(self.message_hash.len())]
        )
    }
}

/// An indexed channel event (open, close, or settle).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexedChannel {
    /// A channel open event.
    Open(IndexedChannelOpen),
    /// A channel close event.
    Close(IndexedChannelClose),
    /// A channel settle event.
    Settle(IndexedChannelSettle),
}

impl IndexedChannel {
    /// Returns the transaction ID for this channel event.
    pub fn tx_id(&self) -> &str {
        match self {
            Self::Open(open) => &open.tx_id,
            Self::Close(close) => &close.tx_id,
            Self::Settle(settle) => &settle.tx_id,
        }
    }

    /// Returns the block height for this channel event.
    pub fn block_height(&self) -> u32 {
        match self {
            Self::Open(open) => open.block_height,
            Self::Close(close) => close.block_height,
            Self::Settle(settle) => settle.block_height,
        }
    }

    /// Returns the channel ID if available (None for open events).
    pub fn channel_id(&self) -> Option<&str> {
        match self {
            Self::Open(_) => None,
            Self::Close(close) => Some(&close.channel_id),
            Self::Settle(settle) => Some(&settle.channel_id),
        }
    }

    /// Returns true if this is a channel open event.
    pub fn is_open(&self) -> bool {
        matches!(self, Self::Open(_))
    }

    /// Returns true if this is a channel close event.
    pub fn is_close(&self) -> bool {
        matches!(self, Self::Close(_))
    }

    /// Returns true if this is a channel settle event.
    pub fn is_settle(&self) -> bool {
        matches!(self, Self::Settle(_))
    }

    /// Returns the event type name.
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::Open(_) => "open",
            Self::Close(_) => "close",
            Self::Settle(_) => "settle",
        }
    }
}

impl fmt::Display for IndexedChannel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open(open) => write!(f, "{}", open),
            Self::Close(close) => write!(f, "{}", close),
            Self::Settle(settle) => write!(f, "{}", settle),
        }
    }
}

/// Errors that can occur during channel indexing operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChannelIndexError {
    /// The memo is not a channel message.
    NotAChannel,

    /// Failed to parse the social message.
    ParseError(SocialParseError),

    /// Invalid channel open payload.
    InvalidChannelOpen(String),

    /// Invalid channel close payload.
    InvalidChannelClose(String),

    /// Invalid channel settle payload.
    InvalidChannelSettle(String),

    /// Invalid transaction ID.
    InvalidTxId,
}

impl fmt::Display for ChannelIndexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAChannel => write!(f, "memo is not a channel message"),
            Self::ParseError(e) => write!(f, "parse error: {}", e),
            Self::InvalidChannelOpen(msg) => write!(f, "invalid channel open: {}", msg),
            Self::InvalidChannelClose(msg) => write!(f, "invalid channel close: {}", msg),
            Self::InvalidChannelSettle(msg) => write!(f, "invalid channel settle: {}", msg),
            Self::InvalidTxId => write!(f, "invalid transaction ID"),
        }
    }
}

impl std::error::Error for ChannelIndexError {}

impl From<SocialParseError> for ChannelIndexError {
    fn from(err: SocialParseError) -> Self {
        Self::ParseError(err)
    }
}

/// Checks if a memo contains a channel message.
///
/// This is a quick check that only looks at the first byte to determine
/// if the memo is a channel message (0xC0, 0xC1, or 0xC2).
pub fn is_channel_memo(memo: &Memo) -> bool {
    let bytes = memo.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    matches!(bytes[0], 0xC0 | 0xC1 | 0xC2)
}

/// Returns the channel message type from a memo, if it is a channel message.
pub fn channel_type_from_memo(memo: &Memo) -> Option<SocialMessageType> {
    let bytes = memo.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    match bytes[0] {
        0xC0 => Some(SocialMessageType::ChannelOpen),
        0xC1 => Some(SocialMessageType::ChannelClose),
        0xC2 => Some(SocialMessageType::ChannelSettle),
        _ => None,
    }
}

/// Parses a channel open payload.
///
/// Format: [parties_count(1)][party1_addr_len(1)][party1_addr]...[deposit(8)][timeout_blocks(4)]
fn parse_channel_open_payload(
    payload: &[u8],
    tx_id: &str,
    block_height: u32,
    version: u8,
) -> Result<IndexedChannelOpen, ChannelIndexError> {
    if payload.is_empty() {
        return Err(ChannelIndexError::InvalidChannelOpen(
            "empty payload".to_string(),
        ));
    }

    let parties_count = payload[0] as usize;
    if parties_count == 0 || parties_count > MAX_CHANNEL_PARTIES {
        return Err(ChannelIndexError::InvalidChannelOpen(format!(
            "invalid parties count: {}",
            parties_count
        )));
    }

    let mut offset = 1;
    let mut parties = Vec::with_capacity(parties_count);

    for i in 0..parties_count {
        if offset >= payload.len() {
            return Err(ChannelIndexError::InvalidChannelOpen(format!(
                "payload too short for party {} address length",
                i
            )));
        }
        let addr_len = payload[offset] as usize;
        offset += 1;

        if offset + addr_len > payload.len() {
            return Err(ChannelIndexError::InvalidChannelOpen(format!(
                "payload too short for party {} address",
                i
            )));
        }
        let addr_bytes = &payload[offset..offset + addr_len];
        let addr = String::from_utf8_lossy(addr_bytes).to_string();
        parties.push(addr);
        offset += addr_len;
    }

    // Need at least 12 more bytes for deposit (8) + timeout (4)
    if offset + 12 > payload.len() {
        return Err(ChannelIndexError::InvalidChannelOpen(
            "payload too short for deposit and timeout".to_string(),
        ));
    }

    let deposit =
        u64::from_le_bytes(payload[offset..offset + 8].try_into().map_err(|_| {
            ChannelIndexError::InvalidChannelOpen("invalid deposit bytes".to_string())
        })?);
    offset += 8;

    let timeout_blocks =
        u32::from_le_bytes(payload[offset..offset + 4].try_into().map_err(|_| {
            ChannelIndexError::InvalidChannelOpen("invalid timeout bytes".to_string())
        })?);

    Ok(IndexedChannelOpen::new(
        tx_id,
        block_height,
        parties,
        deposit,
        timeout_blocks,
        version,
    ))
}

/// Parses a channel close payload.
///
/// Format: [channel_id(32)][final_seq(4)]
fn parse_channel_close_payload(
    payload: &[u8],
    tx_id: &str,
    block_height: u32,
    version: u8,
) -> Result<IndexedChannelClose, ChannelIndexError> {
    // Need exactly 36 bytes: channel_id (32) + final_seq (4)
    if payload.len() < 36 {
        return Err(ChannelIndexError::InvalidChannelClose(format!(
            "payload too short: {} bytes, expected at least 36",
            payload.len()
        )));
    }

    let channel_id = hex::encode(&payload[0..32]);
    let final_seq = u32::from_le_bytes(payload[32..36].try_into().map_err(|_| {
        ChannelIndexError::InvalidChannelClose("invalid final_seq bytes".to_string())
    })?);

    Ok(IndexedChannelClose::new(
        tx_id,
        block_height,
        channel_id,
        final_seq,
        version,
    ))
}

/// Parses a channel settle payload.
///
/// Format: [channel_id(32)][final_seq(4)][message_hash(32)]
fn parse_channel_settle_payload(
    payload: &[u8],
    tx_id: &str,
    block_height: u32,
    version: u8,
) -> Result<IndexedChannelSettle, ChannelIndexError> {
    // Need exactly 68 bytes: channel_id (32) + final_seq (4) + message_hash (32)
    if payload.len() < 68 {
        return Err(ChannelIndexError::InvalidChannelSettle(format!(
            "payload too short: {} bytes, expected at least 68",
            payload.len()
        )));
    }

    let channel_id = hex::encode(&payload[0..32]);
    let final_seq = u32::from_le_bytes(payload[32..36].try_into().map_err(|_| {
        ChannelIndexError::InvalidChannelSettle("invalid final_seq bytes".to_string())
    })?);
    let message_hash = hex::encode(&payload[36..68]);

    Ok(IndexedChannelSettle::new(
        tx_id,
        block_height,
        channel_id,
        final_seq,
        message_hash,
        version,
    ))
}

/// Parses a channel message from a memo and returns an indexed channel event.
///
/// # Arguments
///
/// * `memo` - The memo to parse
/// * `tx_id` - The transaction ID containing this memo
/// * `block_height` - The block height where the transaction was included
///
/// # Returns
///
/// An `IndexedChannel` variant (Open, Close, or Settle), or an error if the
/// memo is not a valid channel message.
///
/// # Example
///
/// ```ignore
/// let channel = parse_channel_memo(&memo, "txid123", 1000)?;
/// match channel {
///     IndexedChannel::Open(open) => println!("Opened with {} parties", open.parties.len()),
///     IndexedChannel::Close(close) => println!("Closing channel {}", close.channel_id),
///     IndexedChannel::Settle(settle) => println!("Settled channel {}", settle.channel_id),
/// }
/// ```
pub fn parse_channel_memo(
    memo: &Memo,
    tx_id: &str,
    block_height: u32,
) -> Result<IndexedChannel, ChannelIndexError> {
    // Validate tx_id
    if tx_id.is_empty() {
        return Err(ChannelIndexError::InvalidTxId);
    }

    // Quick check for channel type
    if !is_channel_memo(memo) {
        return Err(ChannelIndexError::NotAChannel);
    }

    // Parse the social message
    let msg = SocialMessage::try_from(memo)?;
    let version = msg.version();
    let payload = msg.payload();

    match msg.msg_type() {
        SocialMessageType::ChannelOpen => {
            let open = parse_channel_open_payload(payload, tx_id, block_height, version)?;
            Ok(IndexedChannel::Open(open))
        }
        SocialMessageType::ChannelClose => {
            let close = parse_channel_close_payload(payload, tx_id, block_height, version)?;
            Ok(IndexedChannel::Close(close))
        }
        SocialMessageType::ChannelSettle => {
            let settle = parse_channel_settle_payload(payload, tx_id, block_height, version)?;
            Ok(IndexedChannel::Settle(settle))
        }
        _ => Err(ChannelIndexError::NotAChannel),
    }
}

/// Statistics about channel activity in a block.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockChannelStats {
    /// Block height.
    pub block_height: u32,

    /// Total number of channel transactions.
    pub total_channel_txs: u32,

    /// Number of channel open transactions.
    pub channel_opens: u32,

    /// Number of channel close transactions.
    pub channel_closes: u32,

    /// Number of channel settle transactions.
    pub channel_settles: u32,

    /// Total deposit amount in opened channels (zatoshis).
    pub total_deposits: u64,

    /// Average parties per opened channel (multiplied by 100 for fixed-point).
    pub avg_parties_per_channel_x100: u32,
}

impl BlockChannelStats {
    /// Creates a new stats tracker for a block.
    pub fn new(block_height: u32) -> Self {
        Self {
            block_height,
            ..Default::default()
        }
    }

    /// Records a channel open event.
    pub fn record_open(&mut self, party_count: u32, deposit: u64) {
        self.total_channel_txs += 1;
        self.channel_opens += 1;
        self.total_deposits += deposit;

        // Update average parties (fixed-point x100)
        if self.channel_opens > 0 {
            let total_parties =
                (self.avg_parties_per_channel_x100 * (self.channel_opens - 1) / 100) + party_count;
            self.avg_parties_per_channel_x100 = (total_parties * 100) / self.channel_opens;
        }
    }

    /// Records a channel close event.
    pub fn record_close(&mut self) {
        self.total_channel_txs += 1;
        self.channel_closes += 1;
    }

    /// Records a channel settle event.
    pub fn record_settle(&mut self) {
        self.total_channel_txs += 1;
        self.channel_settles += 1;
    }

    /// Records an indexed channel event.
    pub fn record_channel(&mut self, channel: &IndexedChannel) {
        match channel {
            IndexedChannel::Open(open) => {
                self.record_open(open.parties.len() as u32, open.deposit);
            }
            IndexedChannel::Close(_) => {
                self.record_close();
            }
            IndexedChannel::Settle(_) => {
                self.record_settle();
            }
        }
    }
}

impl fmt::Display for BlockChannelStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Block {} channel stats: {} txs ({} opens, {} closes, {} settles), deposits: {} zatoshis",
            self.block_height,
            self.total_channel_txs,
            self.channel_opens,
            self.channel_closes,
            self.channel_settles,
            self.total_deposits
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

    /// Creates a channel open payload.
    /// Format: [parties_count(1)][party1_addr_len(1)][party1_addr]...[deposit(8)][timeout_blocks(4)]
    fn create_channel_open_payload(parties: &[&str], deposit: u64, timeout_blocks: u32) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.push(parties.len() as u8);
        for party in parties {
            payload.push(party.len() as u8);
            payload.extend_from_slice(party.as_bytes());
        }
        payload.extend_from_slice(&deposit.to_le_bytes());
        payload.extend_from_slice(&timeout_blocks.to_le_bytes());
        payload
    }

    /// Creates a channel close payload.
    /// Format: [channel_id(32)][final_seq(4)]
    fn create_channel_close_payload(channel_id: &[u8; 32], final_seq: u32) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(channel_id);
        payload.extend_from_slice(&final_seq.to_le_bytes());
        payload
    }

    /// Creates a channel settle payload.
    /// Format: [channel_id(32)][final_seq(4)][message_hash(32)]
    fn create_channel_settle_payload(
        channel_id: &[u8; 32],
        final_seq: u32,
        message_hash: &[u8; 32],
    ) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(channel_id);
        payload.extend_from_slice(&final_seq.to_le_bytes());
        payload.extend_from_slice(message_hash);
        payload
    }

    fn create_social_memo(msg_type: SocialMessageType, payload: &[u8]) -> Memo {
        let msg = SocialMessage::new(msg_type, SOCIAL_PROTOCOL_VERSION, payload.to_vec());
        let encoded = msg.encode();
        create_memo(&encoded)
    }

    // ========================================================================
    // Tests for is_channel_memo
    // ========================================================================

    #[test]
    fn test_is_channel_memo() {
        let _init_guard = zebra_test::init();

        // Channel open memo
        let open_memo = create_memo(&[0xC0, 0x01, 0x00]);
        assert!(is_channel_memo(&open_memo));

        // Channel close memo
        let close_memo = create_memo(&[0xC1, 0x01, 0x00]);
        assert!(is_channel_memo(&close_memo));

        // Channel settle memo
        let settle_memo = create_memo(&[0xC2, 0x01, 0x00]);
        assert!(is_channel_memo(&settle_memo));

        // Non-channel memo (Post = 0x20)
        let post_memo = create_memo(&[0x20, 0x01, b'H', b'i']);
        assert!(!is_channel_memo(&post_memo));

        // Empty memo
        let empty_memo = create_memo(&[]);
        assert!(!is_channel_memo(&empty_memo));
    }

    #[test]
    fn test_channel_type_from_memo() {
        let _init_guard = zebra_test::init();

        let open_memo = create_memo(&[0xC0, 0x01]);
        assert_eq!(
            channel_type_from_memo(&open_memo),
            Some(SocialMessageType::ChannelOpen)
        );

        let close_memo = create_memo(&[0xC1, 0x01]);
        assert_eq!(
            channel_type_from_memo(&close_memo),
            Some(SocialMessageType::ChannelClose)
        );

        let settle_memo = create_memo(&[0xC2, 0x01]);
        assert_eq!(
            channel_type_from_memo(&settle_memo),
            Some(SocialMessageType::ChannelSettle)
        );

        let post_memo = create_memo(&[0x20, 0x01]);
        assert_eq!(channel_type_from_memo(&post_memo), None);

        let empty_memo = create_memo(&[]);
        assert_eq!(channel_type_from_memo(&empty_memo), None);
    }

    // ========================================================================
    // Tests for parse_channel_memo - Open
    // ========================================================================

    #[test]
    fn test_parse_channel_open() {
        let _init_guard = zebra_test::init();

        let parties = &["bs1alice12345", "bs1bob67890"];
        let deposit = 1_000_000u64;
        let timeout = 1440u32;

        let payload = create_channel_open_payload(parties, deposit, timeout);
        let memo = create_social_memo(SocialMessageType::ChannelOpen, &payload);

        let result = parse_channel_memo(&memo, "txid_open_123", 5000).expect("should parse");

        match result {
            IndexedChannel::Open(open) => {
                assert_eq!(open.tx_id, "txid_open_123");
                assert_eq!(open.block_height, 5000);
                assert_eq!(open.parties.len(), 2);
                assert_eq!(open.parties[0], "bs1alice12345");
                assert_eq!(open.parties[1], "bs1bob67890");
                assert_eq!(open.deposit, 1_000_000);
                assert_eq!(open.timeout_blocks, 1440);
                assert!(open.has_valid_deposit());
            }
            _ => panic!("expected Open variant"),
        }
    }

    #[test]
    fn test_parse_channel_open_single_party() {
        let _init_guard = zebra_test::init();

        let parties = &["bs1solo"];
        let payload = create_channel_open_payload(parties, 500_000, 720);
        let memo = create_social_memo(SocialMessageType::ChannelOpen, &payload);

        let result = parse_channel_memo(&memo, "txid_solo", 6000).expect("should parse");

        if let IndexedChannel::Open(open) = result {
            assert_eq!(open.parties.len(), 1);
            assert_eq!(open.parties[0], "bs1solo");
        } else {
            panic!("expected Open variant");
        }
    }

    #[test]
    fn test_parse_channel_open_invalid_deposit() {
        let _init_guard = zebra_test::init();

        // Deposit below minimum
        let payload = create_channel_open_payload(&["bs1addr"], 50_000, 1440);
        let memo = create_social_memo(SocialMessageType::ChannelOpen, &payload);

        let result = parse_channel_memo(&memo, "txid_low", 7000).expect("should parse");

        if let IndexedChannel::Open(open) = result {
            assert!(!open.has_valid_deposit());
        } else {
            panic!("expected Open variant");
        }
    }

    // ========================================================================
    // Tests for parse_channel_memo - Close
    // ========================================================================

    #[test]
    fn test_parse_channel_close() {
        let _init_guard = zebra_test::init();

        let channel_id: [u8; 32] = [0xAB; 32];
        // Use non-zero bytes to avoid trailing zero trimming
        let final_seq: u32 = 0x01020304;

        let payload = create_channel_close_payload(&channel_id, final_seq);
        let memo = create_social_memo(SocialMessageType::ChannelClose, &payload);

        let result = parse_channel_memo(&memo, "txid_close_456", 8000).expect("should parse");

        match result {
            IndexedChannel::Close(close) => {
                assert_eq!(close.tx_id, "txid_close_456");
                assert_eq!(close.block_height, 8000);
                assert_eq!(close.channel_id, hex::encode([0xAB; 32]));
                assert_eq!(close.final_seq, 0x01020304);
            }
            _ => panic!("expected Close variant"),
        }
    }

    // ========================================================================
    // Tests for parse_channel_memo - Settle
    // ========================================================================

    #[test]
    fn test_parse_channel_settle() {
        let _init_guard = zebra_test::init();

        let channel_id: [u8; 32] = [0xCD; 32];
        // Use non-zero bytes to avoid trailing zero trimming
        let final_seq: u32 = 0x05060708;
        let message_hash: [u8; 32] = [0xEF; 32];

        let payload = create_channel_settle_payload(&channel_id, final_seq, &message_hash);
        let memo = create_social_memo(SocialMessageType::ChannelSettle, &payload);

        let result = parse_channel_memo(&memo, "txid_settle_789", 9000).expect("should parse");

        match result {
            IndexedChannel::Settle(settle) => {
                assert_eq!(settle.tx_id, "txid_settle_789");
                assert_eq!(settle.block_height, 9000);
                assert_eq!(settle.channel_id, hex::encode([0xCD; 32]));
                assert_eq!(settle.final_seq, 0x05060708);
                assert_eq!(settle.message_hash, hex::encode([0xEF; 32]));
            }
            _ => panic!("expected Settle variant"),
        }
    }

    // ========================================================================
    // Tests for error cases
    // ========================================================================

    #[test]
    fn test_parse_channel_not_a_channel() {
        let _init_guard = zebra_test::init();

        // Post memo (not a channel)
        let post_memo = create_memo(&[0x20, 0x01, b'H', b'i']);
        let result = parse_channel_memo(&post_memo, "txid", 1000);

        assert!(matches!(result, Err(ChannelIndexError::NotAChannel)));
    }

    #[test]
    fn test_parse_channel_invalid_tx_id() {
        let _init_guard = zebra_test::init();

        let payload = create_channel_open_payload(&["bs1addr"], 500_000, 1440);
        let memo = create_social_memo(SocialMessageType::ChannelOpen, &payload);

        let result = parse_channel_memo(&memo, "", 1000);
        assert!(matches!(result, Err(ChannelIndexError::InvalidTxId)));
    }

    #[test]
    fn test_parse_channel_open_empty_payload() {
        let _init_guard = zebra_test::init();

        let memo = create_social_memo(SocialMessageType::ChannelOpen, &[]);
        let result = parse_channel_memo(&memo, "txid", 1000);

        assert!(matches!(
            result,
            Err(ChannelIndexError::InvalidChannelOpen(_))
        ));
    }

    #[test]
    fn test_parse_channel_close_short_payload() {
        let _init_guard = zebra_test::init();

        // Only 30 bytes instead of 36
        let short_payload = vec![0xAB; 30];
        let memo = create_social_memo(SocialMessageType::ChannelClose, &short_payload);
        let result = parse_channel_memo(&memo, "txid", 1000);

        assert!(matches!(
            result,
            Err(ChannelIndexError::InvalidChannelClose(_))
        ));
    }

    #[test]
    fn test_parse_channel_settle_short_payload() {
        let _init_guard = zebra_test::init();

        // Only 60 bytes instead of 68
        let short_payload = vec![0xAB; 60];
        let memo = create_social_memo(SocialMessageType::ChannelSettle, &short_payload);
        let result = parse_channel_memo(&memo, "txid", 1000);

        assert!(matches!(
            result,
            Err(ChannelIndexError::InvalidChannelSettle(_))
        ));
    }

    // ========================================================================
    // Tests for IndexedChannel methods
    // ========================================================================

    #[test]
    fn test_indexed_channel_methods() {
        let _init_guard = zebra_test::init();

        let open = IndexedChannel::Open(IndexedChannelOpen::new(
            "txid1",
            1000,
            vec!["a".to_string(), "b".to_string()],
            500_000,
            1440,
            1,
        ));

        assert_eq!(open.tx_id(), "txid1");
        assert_eq!(open.block_height(), 1000);
        assert!(open.channel_id().is_none());
        assert!(open.is_open());
        assert!(!open.is_close());
        assert!(!open.is_settle());
        assert_eq!(open.event_type(), "open");

        let close = IndexedChannel::Close(IndexedChannelClose::new(
            "txid2",
            2000,
            "abc123".to_string(),
            100,
            1,
        ));

        assert_eq!(close.tx_id(), "txid2");
        assert_eq!(close.block_height(), 2000);
        assert_eq!(close.channel_id(), Some("abc123"));
        assert!(!close.is_open());
        assert!(close.is_close());
        assert!(!close.is_settle());
        assert_eq!(close.event_type(), "close");

        let settle = IndexedChannel::Settle(IndexedChannelSettle::new(
            "txid3",
            3000,
            "def456".to_string(),
            200,
            "hash789".to_string(),
            1,
        ));

        assert_eq!(settle.tx_id(), "txid3");
        assert_eq!(settle.block_height(), 3000);
        assert_eq!(settle.channel_id(), Some("def456"));
        assert!(!settle.is_open());
        assert!(!settle.is_close());
        assert!(settle.is_settle());
        assert_eq!(settle.event_type(), "settle");
    }

    // ========================================================================
    // Tests for Display implementations
    // ========================================================================

    #[test]
    fn test_indexed_channel_open_display() {
        let _init_guard = zebra_test::init();

        let open = IndexedChannelOpen::new(
            "txid_abcdef12",
            1000,
            vec!["alice".to_string(), "bob".to_string()],
            500_000,
            1440,
            1,
        );

        let display = format!("{}", open);
        assert!(display.contains("txid_abc"));
        assert!(display.contains("parties: 2"));
        assert!(display.contains("deposit: 500000"));
        assert!(display.contains("timeout: 1440"));
    }

    #[test]
    fn test_indexed_channel_close_display() {
        let _init_guard = zebra_test::init();

        let close =
            IndexedChannelClose::new("txid_12345678", 2000, "channel_abcdef".to_string(), 150, 1);

        let display = format!("{}", close);
        assert!(display.contains("txid_123"));
        assert!(display.contains("channel_a"));
        assert!(display.contains("final_seq: 150"));
    }

    #[test]
    fn test_indexed_channel_settle_display() {
        let _init_guard = zebra_test::init();

        let settle = IndexedChannelSettle::new(
            "txid_87654321",
            3000,
            "channel_fedcba".to_string(),
            250,
            "hash_0123456789".to_string(),
            1,
        );

        let display = format!("{}", settle);
        assert!(display.contains("txid_876"));
        assert!(display.contains("channel_f"));
        assert!(display.contains("final_seq: 250"));
        assert!(display.contains("hash_012"));
    }

    #[test]
    fn test_indexed_channel_display() {
        let _init_guard = zebra_test::init();

        let open = IndexedChannel::Open(IndexedChannelOpen::new(
            "txid1234",
            1000,
            vec!["a".to_string()],
            100_000,
            1440,
            1,
        ));
        let display = format!("{}", open);
        assert!(display.contains("ChannelOpen"));

        let close = IndexedChannel::Close(IndexedChannelClose::new(
            "txid5678",
            2000,
            "ch123456".to_string(),
            50,
            1,
        ));
        let display = format!("{}", close);
        assert!(display.contains("ChannelClose"));

        let settle = IndexedChannel::Settle(IndexedChannelSettle::new(
            "txid9012",
            3000,
            "ch789012".to_string(),
            100,
            "hash345678".to_string(),
            1,
        ));
        let display = format!("{}", settle);
        assert!(display.contains("ChannelSettle"));
    }

    // ========================================================================
    // Tests for BlockChannelStats
    // ========================================================================

    #[test]
    fn test_block_channel_stats() {
        let _init_guard = zebra_test::init();

        let mut stats = BlockChannelStats::new(10000);

        // Record some channel events
        stats.record_open(2, 500_000);
        stats.record_open(3, 750_000);
        stats.record_close();
        stats.record_settle();

        assert_eq!(stats.block_height, 10000);
        assert_eq!(stats.total_channel_txs, 4);
        assert_eq!(stats.channel_opens, 2);
        assert_eq!(stats.channel_closes, 1);
        assert_eq!(stats.channel_settles, 1);
        assert_eq!(stats.total_deposits, 1_250_000);
    }

    #[test]
    fn test_block_channel_stats_record_channel() {
        let _init_guard = zebra_test::init();

        let mut stats = BlockChannelStats::new(11000);

        let open = IndexedChannel::Open(IndexedChannelOpen::new(
            "tx1",
            11000,
            vec!["a".to_string(), "b".to_string()],
            600_000,
            1440,
            1,
        ));
        stats.record_channel(&open);

        let close = IndexedChannel::Close(IndexedChannelClose::new(
            "tx2",
            11000,
            "ch1".to_string(),
            50,
            1,
        ));
        stats.record_channel(&close);

        let settle = IndexedChannel::Settle(IndexedChannelSettle::new(
            "tx3",
            11000,
            "ch2".to_string(),
            100,
            "hash".to_string(),
            1,
        ));
        stats.record_channel(&settle);

        assert_eq!(stats.total_channel_txs, 3);
        assert_eq!(stats.channel_opens, 1);
        assert_eq!(stats.channel_closes, 1);
        assert_eq!(stats.channel_settles, 1);
        assert_eq!(stats.total_deposits, 600_000);
    }

    #[test]
    fn test_block_channel_stats_display() {
        let _init_guard = zebra_test::init();

        let mut stats = BlockChannelStats::new(12000);
        stats.record_open(2, 1_000_000);
        stats.record_close();

        let display = format!("{}", stats);
        assert!(display.contains("Block 12000"));
        assert!(display.contains("2 txs"));
        assert!(display.contains("1 opens"));
        assert!(display.contains("1 closes"));
        assert!(display.contains("0 settles"));
        assert!(display.contains("1000000 zatoshis"));
    }

    // ========================================================================
    // Tests for ChannelIndexError
    // ========================================================================

    #[test]
    fn test_channel_index_error_display() {
        let _init_guard = zebra_test::init();

        assert_eq!(
            format!("{}", ChannelIndexError::NotAChannel),
            "memo is not a channel message"
        );
        assert_eq!(
            format!("{}", ChannelIndexError::InvalidTxId),
            "invalid transaction ID"
        );
        assert_eq!(
            format!(
                "{}",
                ChannelIndexError::InvalidChannelOpen("test".to_string())
            ),
            "invalid channel open: test"
        );
        assert_eq!(
            format!(
                "{}",
                ChannelIndexError::InvalidChannelClose("test".to_string())
            ),
            "invalid channel close: test"
        );
        assert_eq!(
            format!(
                "{}",
                ChannelIndexError::InvalidChannelSettle("test".to_string())
            ),
            "invalid channel settle: test"
        );
    }

    // ========================================================================
    // Tests for IndexedChannelOpen helpers
    // ========================================================================

    #[test]
    fn test_indexed_channel_open_party_count() {
        let _init_guard = zebra_test::init();

        let open = IndexedChannelOpen::new(
            "tx",
            1000,
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
            500_000,
            1440,
            1,
        );

        assert_eq!(open.party_count(), 3);
    }

    #[test]
    fn test_indexed_channel_open_has_valid_deposit() {
        let _init_guard = zebra_test::init();

        // Valid deposit (exactly at minimum)
        let valid = IndexedChannelOpen::new(
            "tx",
            1000,
            vec!["a".to_string()],
            MIN_CHANNEL_DEPOSIT,
            1440,
            1,
        );
        assert!(valid.has_valid_deposit());

        // Valid deposit (above minimum)
        let above = IndexedChannelOpen::new(
            "tx",
            1000,
            vec!["a".to_string()],
            MIN_CHANNEL_DEPOSIT + 1,
            1440,
            1,
        );
        assert!(above.has_valid_deposit());

        // Invalid deposit (below minimum)
        let invalid = IndexedChannelOpen::new(
            "tx",
            1000,
            vec!["a".to_string()],
            MIN_CHANNEL_DEPOSIT - 1,
            1440,
            1,
        );
        assert!(!invalid.has_valid_deposit());
    }
}
