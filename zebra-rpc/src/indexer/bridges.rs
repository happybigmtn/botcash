//! Indexer bridge parsing utilities.
//!
//! This module provides utilities for indexers to parse bridge messages from
//! transaction memos and extract cross-platform identity linking information.
//!
//! # Overview
//!
//! Bridge messages (types 0xB0-0xB3) enable cross-platform identity linking
//! between Botcash addresses and external platforms like Telegram, Discord,
//! Nostr, Mastodon, and Twitter. Indexers need to track link state and parse
//! the relevant data from each message type.
//!
//! # Bridge Types
//!
//! - `BridgeLink` (0xB0): Links an external platform identity to a Botcash address
//! - `BridgeUnlink` (0xB1): Removes a platform identity link
//! - `BridgePost` (0xB2): Cross-posts content from an external platform
//! - `BridgeVerify` (0xB3): Requests verification challenge for linking
//!
//! # Usage
//!
//! ```ignore
//! use zebra_chain::transaction::Memo;
//! use zebra_rpc::indexer::bridges::{parse_bridge_memo, IndexedBridge};
//!
//! let bridge = parse_bridge_memo(&memo, "txid123", 1000)?;
//! match bridge {
//!     IndexedBridge::Link(link) => {
//!         println!("Linked {} identity {} to Botcash", link.platform, link.platform_id);
//!     }
//!     IndexedBridge::Unlink(unlink) => {
//!         println!("Unlinked {} identity {}", unlink.platform, unlink.platform_id);
//!     }
//!     IndexedBridge::Post(post) => {
//!         println!("Cross-posted from {} (original: {})", post.platform, post.original_id);
//!     }
//!     IndexedBridge::Verify(verify) => {
//!         println!("Verification request for {} identity {}", verify.platform, verify.platform_id);
//!     }
//! }
//! ```

use std::fmt;

use serde::{Deserialize, Serialize};
use zebra_chain::transaction::{
    social::{
        BridgeMessage, BridgePlatform, SocialMessage, SocialMessageType, SocialParseError,
    },
    Memo,
};

/// An indexed bridge link event extracted from a memo.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedBridgeLink {
    /// The transaction ID containing this bridge link.
    pub tx_id: String,

    /// Block height where this transaction was included.
    pub block_height: u32,

    /// The platform being linked.
    pub platform: String,

    /// The platform-specific user identifier.
    pub platform_id: String,

    /// The verification challenge (32 bytes hex-encoded).
    pub challenge: String,

    /// The signature proving ownership (hex-encoded).
    pub signature: String,

    /// Protocol version.
    pub version: u8,
}

impl IndexedBridgeLink {
    /// Creates a new indexed bridge link from parsed data.
    pub fn new(
        tx_id: &str,
        block_height: u32,
        platform: String,
        platform_id: String,
        challenge: String,
        signature: String,
        version: u8,
    ) -> Self {
        Self {
            tx_id: tx_id.to_string(),
            block_height,
            platform,
            platform_id,
            challenge,
            signature,
            version,
        }
    }
}

impl fmt::Display for IndexedBridgeLink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BridgeLink {{ tx: {}..., platform: {}, id: {} }}",
            &self.tx_id[..8.min(self.tx_id.len())],
            self.platform,
            self.platform_id
        )
    }
}

/// An indexed bridge unlink event extracted from a memo.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedBridgeUnlink {
    /// The transaction ID containing this bridge unlink.
    pub tx_id: String,

    /// Block height where this transaction was included.
    pub block_height: u32,

    /// The platform being unlinked.
    pub platform: String,

    /// The platform-specific user identifier being unlinked.
    pub platform_id: String,

    /// Protocol version.
    pub version: u8,
}

impl IndexedBridgeUnlink {
    /// Creates a new indexed bridge unlink from parsed data.
    pub fn new(
        tx_id: &str,
        block_height: u32,
        platform: String,
        platform_id: String,
        version: u8,
    ) -> Self {
        Self {
            tx_id: tx_id.to_string(),
            block_height,
            platform,
            platform_id,
            version,
        }
    }
}

impl fmt::Display for IndexedBridgeUnlink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BridgeUnlink {{ tx: {}..., platform: {}, id: {} }}",
            &self.tx_id[..8.min(self.tx_id.len())],
            self.platform,
            self.platform_id
        )
    }
}

/// An indexed bridge cross-post event extracted from a memo.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedBridgePost {
    /// The transaction ID containing this bridge post.
    pub tx_id: String,

    /// Block height where this transaction was included.
    pub block_height: u32,

    /// The source platform.
    pub platform: String,

    /// The original post ID on the source platform.
    pub original_id: String,

    /// The content that was cross-posted.
    pub content: String,

    /// Protocol version.
    pub version: u8,
}

impl IndexedBridgePost {
    /// Creates a new indexed bridge post from parsed data.
    pub fn new(
        tx_id: &str,
        block_height: u32,
        platform: String,
        original_id: String,
        content: String,
        version: u8,
    ) -> Self {
        Self {
            tx_id: tx_id.to_string(),
            block_height,
            platform,
            original_id,
            content,
            version,
        }
    }

    /// Returns true if this post has content.
    pub fn has_content(&self) -> bool {
        !self.content.is_empty()
    }
}

impl fmt::Display for IndexedBridgePost {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BridgePost {{ tx: {}..., platform: {}, original: {} }}",
            &self.tx_id[..8.min(self.tx_id.len())],
            self.platform,
            self.original_id
        )
    }
}

/// An indexed bridge verification request extracted from a memo.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedBridgeVerify {
    /// The transaction ID containing this verification request.
    pub tx_id: String,

    /// Block height where this transaction was included.
    pub block_height: u32,

    /// The platform to verify.
    pub platform: String,

    /// The platform-specific user identifier to verify.
    pub platform_id: String,

    /// Nonce for the verification challenge.
    pub nonce: u64,

    /// Protocol version.
    pub version: u8,
}

impl IndexedBridgeVerify {
    /// Creates a new indexed bridge verification request from parsed data.
    pub fn new(
        tx_id: &str,
        block_height: u32,
        platform: String,
        platform_id: String,
        nonce: u64,
        version: u8,
    ) -> Self {
        Self {
            tx_id: tx_id.to_string(),
            block_height,
            platform,
            platform_id,
            nonce,
            version,
        }
    }
}

impl fmt::Display for IndexedBridgeVerify {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BridgeVerify {{ tx: {}..., platform: {}, id: {}, nonce: {} }}",
            &self.tx_id[..8.min(self.tx_id.len())],
            self.platform,
            self.platform_id,
            self.nonce
        )
    }
}

/// An indexed bridge event (link, unlink, post, or verify).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexedBridge {
    /// A bridge link event.
    Link(IndexedBridgeLink),
    /// A bridge unlink event.
    Unlink(IndexedBridgeUnlink),
    /// A bridge cross-post event.
    Post(IndexedBridgePost),
    /// A bridge verification request event.
    Verify(IndexedBridgeVerify),
}

impl IndexedBridge {
    /// Returns the transaction ID for this bridge event.
    pub fn tx_id(&self) -> &str {
        match self {
            Self::Link(link) => &link.tx_id,
            Self::Unlink(unlink) => &unlink.tx_id,
            Self::Post(post) => &post.tx_id,
            Self::Verify(verify) => &verify.tx_id,
        }
    }

    /// Returns the block height for this bridge event.
    pub fn block_height(&self) -> u32 {
        match self {
            Self::Link(link) => link.block_height,
            Self::Unlink(unlink) => unlink.block_height,
            Self::Post(post) => post.block_height,
            Self::Verify(verify) => verify.block_height,
        }
    }

    /// Returns the platform for this bridge event.
    pub fn platform(&self) -> &str {
        match self {
            Self::Link(link) => &link.platform,
            Self::Unlink(unlink) => &unlink.platform,
            Self::Post(post) => &post.platform,
            Self::Verify(verify) => &verify.platform,
        }
    }

    /// Returns true if this is a bridge link event.
    pub fn is_link(&self) -> bool {
        matches!(self, Self::Link(_))
    }

    /// Returns true if this is a bridge unlink event.
    pub fn is_unlink(&self) -> bool {
        matches!(self, Self::Unlink(_))
    }

    /// Returns true if this is a bridge post event.
    pub fn is_post(&self) -> bool {
        matches!(self, Self::Post(_))
    }

    /// Returns true if this is a bridge verify event.
    pub fn is_verify(&self) -> bool {
        matches!(self, Self::Verify(_))
    }

    /// Returns the event type name.
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::Link(_) => "link",
            Self::Unlink(_) => "unlink",
            Self::Post(_) => "post",
            Self::Verify(_) => "verify",
        }
    }
}

impl fmt::Display for IndexedBridge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Link(link) => write!(f, "{}", link),
            Self::Unlink(unlink) => write!(f, "{}", unlink),
            Self::Post(post) => write!(f, "{}", post),
            Self::Verify(verify) => write!(f, "{}", verify),
        }
    }
}

/// Errors that can occur during bridge indexing operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BridgeIndexError {
    /// The memo is not a bridge message.
    NotABridge,

    /// Failed to parse the social message.
    ParseError(SocialParseError),

    /// Invalid bridge link payload.
    InvalidBridgeLink(String),

    /// Invalid bridge unlink payload.
    InvalidBridgeUnlink(String),

    /// Invalid bridge post payload.
    InvalidBridgePost(String),

    /// Invalid bridge verify payload.
    InvalidBridgeVerify(String),

    /// Invalid transaction ID.
    InvalidTxId,
}

impl fmt::Display for BridgeIndexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotABridge => write!(f, "memo is not a bridge message"),
            Self::ParseError(e) => write!(f, "parse error: {}", e),
            Self::InvalidBridgeLink(msg) => write!(f, "invalid bridge link: {}", msg),
            Self::InvalidBridgeUnlink(msg) => write!(f, "invalid bridge unlink: {}", msg),
            Self::InvalidBridgePost(msg) => write!(f, "invalid bridge post: {}", msg),
            Self::InvalidBridgeVerify(msg) => write!(f, "invalid bridge verify: {}", msg),
            Self::InvalidTxId => write!(f, "invalid transaction ID"),
        }
    }
}

impl std::error::Error for BridgeIndexError {}

impl From<SocialParseError> for BridgeIndexError {
    fn from(err: SocialParseError) -> Self {
        Self::ParseError(err)
    }
}

/// Checks if a memo contains a bridge message.
///
/// This is a quick check that only looks at the first byte to determine
/// if the memo is a bridge message (0xB0, 0xB1, 0xB2, or 0xB3).
pub fn is_bridge_memo(memo: &Memo) -> bool {
    let bytes = memo.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    matches!(bytes[0], 0xB0 | 0xB1 | 0xB2 | 0xB3)
}

/// Returns the bridge message type from a memo, if it is a bridge message.
pub fn bridge_type_from_memo(memo: &Memo) -> Option<SocialMessageType> {
    let bytes = memo.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    match bytes[0] {
        0xB0 => Some(SocialMessageType::BridgeLink),
        0xB1 => Some(SocialMessageType::BridgeUnlink),
        0xB2 => Some(SocialMessageType::BridgePost),
        0xB3 => Some(SocialMessageType::BridgeVerify),
        _ => None,
    }
}

/// Converts a BridgePlatform to its display name.
fn platform_to_string(platform: BridgePlatform) -> String {
    match platform {
        BridgePlatform::Telegram => "telegram".to_string(),
        BridgePlatform::Discord => "discord".to_string(),
        BridgePlatform::Nostr => "nostr".to_string(),
        BridgePlatform::Mastodon => "mastodon".to_string(),
        BridgePlatform::Twitter => "twitter".to_string(),
    }
}

/// Parses a bridge link payload.
fn parse_bridge_link(
    bridge_msg: &BridgeMessage,
    tx_id: &str,
    block_height: u32,
    version: u8,
) -> Result<IndexedBridgeLink, BridgeIndexError> {
    let challenge = bridge_msg.challenge().ok_or_else(|| {
        BridgeIndexError::InvalidBridgeLink("missing challenge".to_string())
    })?;

    let signature = bridge_msg.signature().ok_or_else(|| {
        BridgeIndexError::InvalidBridgeLink("missing signature".to_string())
    })?;

    Ok(IndexedBridgeLink::new(
        tx_id,
        block_height,
        platform_to_string(bridge_msg.platform()),
        bridge_msg.platform_id().to_string(),
        hex::encode(challenge),
        hex::encode(signature),
        version,
    ))
}

/// Parses a bridge unlink payload.
fn parse_bridge_unlink(
    bridge_msg: &BridgeMessage,
    tx_id: &str,
    block_height: u32,
    version: u8,
) -> Result<IndexedBridgeUnlink, BridgeIndexError> {
    Ok(IndexedBridgeUnlink::new(
        tx_id,
        block_height,
        platform_to_string(bridge_msg.platform()),
        bridge_msg.platform_id().to_string(),
        version,
    ))
}

/// Parses a bridge post payload.
fn parse_bridge_post(
    bridge_msg: &BridgeMessage,
    tx_id: &str,
    block_height: u32,
    version: u8,
) -> Result<IndexedBridgePost, BridgeIndexError> {
    let original_id = bridge_msg.original_id().ok_or_else(|| {
        BridgeIndexError::InvalidBridgePost("missing original_id".to_string())
    })?;

    let content = bridge_msg.content().unwrap_or("");

    Ok(IndexedBridgePost::new(
        tx_id,
        block_height,
        platform_to_string(bridge_msg.platform()),
        original_id.to_string(),
        content.to_string(),
        version,
    ))
}

/// Parses a bridge verification request payload.
fn parse_bridge_verify(
    bridge_msg: &BridgeMessage,
    tx_id: &str,
    block_height: u32,
    version: u8,
) -> Result<IndexedBridgeVerify, BridgeIndexError> {
    let nonce = bridge_msg.nonce().ok_or_else(|| {
        BridgeIndexError::InvalidBridgeVerify("missing nonce".to_string())
    })?;

    Ok(IndexedBridgeVerify::new(
        tx_id,
        block_height,
        platform_to_string(bridge_msg.platform()),
        bridge_msg.platform_id().to_string(),
        nonce,
        version,
    ))
}

/// Parses a bridge message from a memo and returns an indexed bridge event.
///
/// # Arguments
///
/// * `memo` - The memo to parse
/// * `tx_id` - The transaction ID containing this memo
/// * `block_height` - The block height where the transaction was included
///
/// # Returns
///
/// An `IndexedBridge` variant (Link, Unlink, Post, or Verify), or an error if the
/// memo is not a valid bridge message.
///
/// # Example
///
/// ```ignore
/// let bridge = parse_bridge_memo(&memo, "txid123", 1000)?;
/// match bridge {
///     IndexedBridge::Link(link) => println!("Linked {} to {}", link.platform_id, link.platform),
///     IndexedBridge::Unlink(unlink) => println!("Unlinked {}", unlink.platform_id),
///     IndexedBridge::Post(post) => println!("Cross-posted: {}", post.content),
///     IndexedBridge::Verify(verify) => println!("Verify request for {}", verify.platform_id),
/// }
/// ```
pub fn parse_bridge_memo(
    memo: &Memo,
    tx_id: &str,
    block_height: u32,
) -> Result<IndexedBridge, BridgeIndexError> {
    // Validate tx_id
    if tx_id.is_empty() {
        return Err(BridgeIndexError::InvalidTxId);
    }

    // Quick check for bridge type
    if !is_bridge_memo(memo) {
        return Err(BridgeIndexError::NotABridge);
    }

    // Parse the social message
    let msg = SocialMessage::try_from(memo)?;
    let version = msg.version();
    let payload = msg.payload();
    let msg_type = msg.msg_type();

    // Parse the bridge message from payload
    let bridge_msg = BridgeMessage::parse(msg_type, payload).map_err(|e| match msg_type {
        SocialMessageType::BridgeLink => BridgeIndexError::InvalidBridgeLink(e.to_string()),
        SocialMessageType::BridgeUnlink => BridgeIndexError::InvalidBridgeUnlink(e.to_string()),
        SocialMessageType::BridgePost => BridgeIndexError::InvalidBridgePost(e.to_string()),
        SocialMessageType::BridgeVerify => BridgeIndexError::InvalidBridgeVerify(e.to_string()),
        _ => BridgeIndexError::NotABridge,
    })?;

    match msg_type {
        SocialMessageType::BridgeLink => {
            let link = parse_bridge_link(&bridge_msg, tx_id, block_height, version)?;
            Ok(IndexedBridge::Link(link))
        }
        SocialMessageType::BridgeUnlink => {
            let unlink = parse_bridge_unlink(&bridge_msg, tx_id, block_height, version)?;
            Ok(IndexedBridge::Unlink(unlink))
        }
        SocialMessageType::BridgePost => {
            let post = parse_bridge_post(&bridge_msg, tx_id, block_height, version)?;
            Ok(IndexedBridge::Post(post))
        }
        SocialMessageType::BridgeVerify => {
            let verify = parse_bridge_verify(&bridge_msg, tx_id, block_height, version)?;
            Ok(IndexedBridge::Verify(verify))
        }
        _ => Err(BridgeIndexError::NotABridge),
    }
}

/// Statistics about bridge activity in a block.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockBridgeStats {
    /// Block height.
    pub block_height: u32,

    /// Total number of bridge transactions.
    pub total_bridge_txs: u32,

    /// Number of bridge link transactions.
    pub bridge_links: u32,

    /// Number of bridge unlink transactions.
    pub bridge_unlinks: u32,

    /// Number of bridge post transactions.
    pub bridge_posts: u32,

    /// Number of bridge verify transactions.
    pub bridge_verifies: u32,

    /// Platform breakdown: telegram links.
    pub telegram_links: u32,

    /// Platform breakdown: discord links.
    pub discord_links: u32,

    /// Platform breakdown: nostr links.
    pub nostr_links: u32,

    /// Platform breakdown: mastodon links.
    pub mastodon_links: u32,

    /// Platform breakdown: twitter links.
    pub twitter_links: u32,
}

impl BlockBridgeStats {
    /// Creates a new stats tracker for a block.
    pub fn new(block_height: u32) -> Self {
        Self {
            block_height,
            ..Default::default()
        }
    }

    /// Records a bridge link event.
    pub fn record_link(&mut self, platform: &str) {
        self.total_bridge_txs += 1;
        self.bridge_links += 1;
        self.record_platform(platform);
    }

    /// Records a bridge unlink event.
    pub fn record_unlink(&mut self) {
        self.total_bridge_txs += 1;
        self.bridge_unlinks += 1;
    }

    /// Records a bridge post event.
    pub fn record_post(&mut self) {
        self.total_bridge_txs += 1;
        self.bridge_posts += 1;
    }

    /// Records a bridge verify event.
    pub fn record_verify(&mut self) {
        self.total_bridge_txs += 1;
        self.bridge_verifies += 1;
    }

    /// Records platform-specific link stats.
    fn record_platform(&mut self, platform: &str) {
        match platform {
            "telegram" => self.telegram_links += 1,
            "discord" => self.discord_links += 1,
            "nostr" => self.nostr_links += 1,
            "mastodon" => self.mastodon_links += 1,
            "twitter" => self.twitter_links += 1,
            _ => {}
        }
    }

    /// Records an indexed bridge event.
    pub fn record_bridge(&mut self, bridge: &IndexedBridge) {
        match bridge {
            IndexedBridge::Link(link) => {
                self.record_link(&link.platform);
            }
            IndexedBridge::Unlink(_) => {
                self.record_unlink();
            }
            IndexedBridge::Post(_) => {
                self.record_post();
            }
            IndexedBridge::Verify(_) => {
                self.record_verify();
            }
        }
    }
}

impl fmt::Display for BlockBridgeStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Block {} bridge stats: {} txs ({} links, {} unlinks, {} posts, {} verifies)",
            self.block_height,
            self.total_bridge_txs,
            self.bridge_links,
            self.bridge_unlinks,
            self.bridge_posts,
            self.bridge_verifies
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

    fn create_social_memo(msg_type: SocialMessageType, payload: &[u8]) -> Memo {
        let msg = SocialMessage::new(msg_type, SOCIAL_PROTOCOL_VERSION, payload.to_vec());
        let encoded = msg.encode();
        create_memo(&encoded)
    }

    // ========================================================================
    // Tests for is_bridge_memo
    // ========================================================================

    #[test]
    fn test_is_bridge_memo() {
        let _init_guard = zebra_test::init();

        // Bridge link memo
        let link_memo = create_memo(&[0xB0, 0x01, 0x00]);
        assert!(is_bridge_memo(&link_memo));

        // Bridge unlink memo
        let unlink_memo = create_memo(&[0xB1, 0x01, 0x00]);
        assert!(is_bridge_memo(&unlink_memo));

        // Bridge post memo
        let post_memo = create_memo(&[0xB2, 0x01, 0x00]);
        assert!(is_bridge_memo(&post_memo));

        // Bridge verify memo
        let verify_memo = create_memo(&[0xB3, 0x01, 0x00]);
        assert!(is_bridge_memo(&verify_memo));

        // Non-bridge memo (Post = 0x20)
        let social_post_memo = create_memo(&[0x20, 0x01, b'H', b'i']);
        assert!(!is_bridge_memo(&social_post_memo));

        // Empty memo
        let empty_memo = create_memo(&[]);
        assert!(!is_bridge_memo(&empty_memo));
    }

    #[test]
    fn test_bridge_type_from_memo() {
        let _init_guard = zebra_test::init();

        let link_memo = create_memo(&[0xB0, 0x01]);
        assert_eq!(
            bridge_type_from_memo(&link_memo),
            Some(SocialMessageType::BridgeLink)
        );

        let unlink_memo = create_memo(&[0xB1, 0x01]);
        assert_eq!(
            bridge_type_from_memo(&unlink_memo),
            Some(SocialMessageType::BridgeUnlink)
        );

        let post_memo = create_memo(&[0xB2, 0x01]);
        assert_eq!(
            bridge_type_from_memo(&post_memo),
            Some(SocialMessageType::BridgePost)
        );

        let verify_memo = create_memo(&[0xB3, 0x01]);
        assert_eq!(
            bridge_type_from_memo(&verify_memo),
            Some(SocialMessageType::BridgeVerify)
        );

        let social_post_memo = create_memo(&[0x20, 0x01]);
        assert_eq!(bridge_type_from_memo(&social_post_memo), None);

        let empty_memo = create_memo(&[]);
        assert_eq!(bridge_type_from_memo(&empty_memo), None);
    }

    // ========================================================================
    // Tests for parse_bridge_memo - Link
    // ========================================================================

    #[test]
    fn test_parse_bridge_link() {
        let _init_guard = zebra_test::init();

        let challenge = [0xAB; 32];
        let signature = vec![0xCD; 64];
        let bridge_msg = BridgeMessage::new_link(
            BridgePlatform::Telegram,
            "user123".to_string(),
            challenge,
            signature.clone(),
        );
        let payload = bridge_msg.encode(SocialMessageType::BridgeLink);
        let memo = create_social_memo(SocialMessageType::BridgeLink, &payload);

        let result = parse_bridge_memo(&memo, "txid_link_123", 5000).expect("should parse");

        match result {
            IndexedBridge::Link(link) => {
                assert_eq!(link.tx_id, "txid_link_123");
                assert_eq!(link.block_height, 5000);
                assert_eq!(link.platform, "telegram");
                assert_eq!(link.platform_id, "user123");
                assert_eq!(link.challenge, hex::encode(challenge));
                assert_eq!(link.signature, hex::encode(signature));
            }
            _ => panic!("expected Link variant"),
        }
    }

    // ========================================================================
    // Tests for parse_bridge_memo - Unlink
    // ========================================================================

    #[test]
    fn test_parse_bridge_unlink() {
        let _init_guard = zebra_test::init();

        let bridge_msg = BridgeMessage::new_unlink(
            BridgePlatform::Discord,
            "discord_user_456".to_string(),
        );
        let payload = bridge_msg.encode(SocialMessageType::BridgeUnlink);
        let memo = create_social_memo(SocialMessageType::BridgeUnlink, &payload);

        let result = parse_bridge_memo(&memo, "txid_unlink_456", 6000).expect("should parse");

        match result {
            IndexedBridge::Unlink(unlink) => {
                assert_eq!(unlink.tx_id, "txid_unlink_456");
                assert_eq!(unlink.block_height, 6000);
                assert_eq!(unlink.platform, "discord");
                assert_eq!(unlink.platform_id, "discord_user_456");
            }
            _ => panic!("expected Unlink variant"),
        }
    }

    // ========================================================================
    // Tests for parse_bridge_memo - Post
    // ========================================================================

    #[test]
    fn test_parse_bridge_post() {
        let _init_guard = zebra_test::init();

        let bridge_msg = BridgeMessage::new_post(
            BridgePlatform::Nostr,
            "note123abc".to_string(),
            "Hello from Nostr!".to_string(),
        );
        let payload = bridge_msg.encode(SocialMessageType::BridgePost);
        let memo = create_social_memo(SocialMessageType::BridgePost, &payload);

        let result = parse_bridge_memo(&memo, "txid_post_789", 7000).expect("should parse");

        match result {
            IndexedBridge::Post(post) => {
                assert_eq!(post.tx_id, "txid_post_789");
                assert_eq!(post.block_height, 7000);
                assert_eq!(post.platform, "nostr");
                assert_eq!(post.original_id, "note123abc");
                assert_eq!(post.content, "Hello from Nostr!");
                assert!(post.has_content());
            }
            _ => panic!("expected Post variant"),
        }
    }

    // ========================================================================
    // Tests for parse_bridge_memo - Verify
    // ========================================================================

    #[test]
    fn test_parse_bridge_verify() {
        let _init_guard = zebra_test::init();

        let bridge_msg = BridgeMessage::new_verify(
            BridgePlatform::Mastodon,
            "@user@mastodon.social".to_string(),
            0x0102030405060708u64,
        );
        let payload = bridge_msg.encode(SocialMessageType::BridgeVerify);
        let memo = create_social_memo(SocialMessageType::BridgeVerify, &payload);

        let result = parse_bridge_memo(&memo, "txid_verify_012", 8000).expect("should parse");

        match result {
            IndexedBridge::Verify(verify) => {
                assert_eq!(verify.tx_id, "txid_verify_012");
                assert_eq!(verify.block_height, 8000);
                assert_eq!(verify.platform, "mastodon");
                assert_eq!(verify.platform_id, "@user@mastodon.social");
                assert_eq!(verify.nonce, 0x0102030405060708);
            }
            _ => panic!("expected Verify variant"),
        }
    }

    // ========================================================================
    // Tests for error cases
    // ========================================================================

    #[test]
    fn test_parse_bridge_not_a_bridge() {
        let _init_guard = zebra_test::init();

        // Post memo (not a bridge)
        let post_memo = create_memo(&[0x20, 0x01, b'H', b'i']);
        let result = parse_bridge_memo(&post_memo, "txid", 1000);

        assert!(matches!(result, Err(BridgeIndexError::NotABridge)));
    }

    #[test]
    fn test_parse_bridge_invalid_tx_id() {
        let _init_guard = zebra_test::init();

        let bridge_msg = BridgeMessage::new_unlink(BridgePlatform::Telegram, "user".to_string());
        let payload = bridge_msg.encode(SocialMessageType::BridgeUnlink);
        let memo = create_social_memo(SocialMessageType::BridgeUnlink, &payload);

        let result = parse_bridge_memo(&memo, "", 1000);
        assert!(matches!(result, Err(BridgeIndexError::InvalidTxId)));
    }

    // ========================================================================
    // Tests for IndexedBridge methods
    // ========================================================================

    #[test]
    fn test_indexed_bridge_methods() {
        let _init_guard = zebra_test::init();

        let link = IndexedBridge::Link(IndexedBridgeLink::new(
            "txid1",
            1000,
            "telegram".to_string(),
            "user1".to_string(),
            "challenge".to_string(),
            "signature".to_string(),
            1,
        ));

        assert_eq!(link.tx_id(), "txid1");
        assert_eq!(link.block_height(), 1000);
        assert_eq!(link.platform(), "telegram");
        assert!(link.is_link());
        assert!(!link.is_unlink());
        assert!(!link.is_post());
        assert!(!link.is_verify());
        assert_eq!(link.event_type(), "link");

        let unlink = IndexedBridge::Unlink(IndexedBridgeUnlink::new(
            "txid2",
            2000,
            "discord".to_string(),
            "user2".to_string(),
            1,
        ));

        assert_eq!(unlink.tx_id(), "txid2");
        assert_eq!(unlink.block_height(), 2000);
        assert_eq!(unlink.platform(), "discord");
        assert!(!unlink.is_link());
        assert!(unlink.is_unlink());
        assert!(!unlink.is_post());
        assert!(!unlink.is_verify());
        assert_eq!(unlink.event_type(), "unlink");

        let post = IndexedBridge::Post(IndexedBridgePost::new(
            "txid3",
            3000,
            "nostr".to_string(),
            "note123".to_string(),
            "content".to_string(),
            1,
        ));

        assert_eq!(post.tx_id(), "txid3");
        assert_eq!(post.block_height(), 3000);
        assert_eq!(post.platform(), "nostr");
        assert!(!post.is_link());
        assert!(!post.is_unlink());
        assert!(post.is_post());
        assert!(!post.is_verify());
        assert_eq!(post.event_type(), "post");

        let verify = IndexedBridge::Verify(IndexedBridgeVerify::new(
            "txid4",
            4000,
            "mastodon".to_string(),
            "user4".to_string(),
            12345,
            1,
        ));

        assert_eq!(verify.tx_id(), "txid4");
        assert_eq!(verify.block_height(), 4000);
        assert_eq!(verify.platform(), "mastodon");
        assert!(!verify.is_link());
        assert!(!verify.is_unlink());
        assert!(!verify.is_post());
        assert!(verify.is_verify());
        assert_eq!(verify.event_type(), "verify");
    }

    // ========================================================================
    // Tests for Display implementations
    // ========================================================================

    #[test]
    fn test_indexed_bridge_link_display() {
        let _init_guard = zebra_test::init();

        let link = IndexedBridgeLink::new(
            "txid_abcdef12",
            1000,
            "telegram".to_string(),
            "user123".to_string(),
            "challenge".to_string(),
            "sig".to_string(),
            1,
        );

        let display = format!("{}", link);
        assert!(display.contains("txid_abc"));
        assert!(display.contains("telegram"));
        assert!(display.contains("user123"));
    }

    #[test]
    fn test_indexed_bridge_unlink_display() {
        let _init_guard = zebra_test::init();

        let unlink = IndexedBridgeUnlink::new(
            "txid_12345678",
            2000,
            "discord".to_string(),
            "user456".to_string(),
            1,
        );

        let display = format!("{}", unlink);
        assert!(display.contains("txid_123"));
        assert!(display.contains("discord"));
        assert!(display.contains("user456"));
    }

    #[test]
    fn test_indexed_bridge_post_display() {
        let _init_guard = zebra_test::init();

        let post = IndexedBridgePost::new(
            "txid_87654321",
            3000,
            "nostr".to_string(),
            "note789".to_string(),
            "content".to_string(),
            1,
        );

        let display = format!("{}", post);
        assert!(display.contains("txid_876"));
        assert!(display.contains("nostr"));
        assert!(display.contains("note789"));
    }

    #[test]
    fn test_indexed_bridge_verify_display() {
        let _init_guard = zebra_test::init();

        let verify = IndexedBridgeVerify::new(
            "txid_fedcba98",
            4000,
            "mastodon".to_string(),
            "user012".to_string(),
            99999,
            1,
        );

        let display = format!("{}", verify);
        assert!(display.contains("txid_fed"));
        assert!(display.contains("mastodon"));
        assert!(display.contains("user012"));
        assert!(display.contains("99999"));
    }

    #[test]
    fn test_indexed_bridge_display() {
        let _init_guard = zebra_test::init();

        let link = IndexedBridge::Link(IndexedBridgeLink::new(
            "txid1234",
            1000,
            "telegram".to_string(),
            "user".to_string(),
            "chal".to_string(),
            "sig".to_string(),
            1,
        ));
        let display = format!("{}", link);
        assert!(display.contains("BridgeLink"));

        let unlink = IndexedBridge::Unlink(IndexedBridgeUnlink::new(
            "txid5678",
            2000,
            "discord".to_string(),
            "user".to_string(),
            1,
        ));
        let display = format!("{}", unlink);
        assert!(display.contains("BridgeUnlink"));

        let post = IndexedBridge::Post(IndexedBridgePost::new(
            "txid9012",
            3000,
            "nostr".to_string(),
            "note".to_string(),
            "content".to_string(),
            1,
        ));
        let display = format!("{}", post);
        assert!(display.contains("BridgePost"));

        let verify = IndexedBridge::Verify(IndexedBridgeVerify::new(
            "txid3456",
            4000,
            "mastodon".to_string(),
            "user".to_string(),
            12345,
            1,
        ));
        let display = format!("{}", verify);
        assert!(display.contains("BridgeVerify"));
    }

    // ========================================================================
    // Tests for BlockBridgeStats
    // ========================================================================

    #[test]
    fn test_block_bridge_stats() {
        let _init_guard = zebra_test::init();

        let mut stats = BlockBridgeStats::new(10000);

        // Record some bridge events
        stats.record_link("telegram");
        stats.record_link("discord");
        stats.record_unlink();
        stats.record_post();
        stats.record_verify();

        assert_eq!(stats.block_height, 10000);
        assert_eq!(stats.total_bridge_txs, 5);
        assert_eq!(stats.bridge_links, 2);
        assert_eq!(stats.bridge_unlinks, 1);
        assert_eq!(stats.bridge_posts, 1);
        assert_eq!(stats.bridge_verifies, 1);
        assert_eq!(stats.telegram_links, 1);
        assert_eq!(stats.discord_links, 1);
    }

    #[test]
    fn test_block_bridge_stats_record_bridge() {
        let _init_guard = zebra_test::init();

        let mut stats = BlockBridgeStats::new(11000);

        let link = IndexedBridge::Link(IndexedBridgeLink::new(
            "tx1",
            11000,
            "telegram".to_string(),
            "user1".to_string(),
            "chal".to_string(),
            "sig".to_string(),
            1,
        ));
        stats.record_bridge(&link);

        let unlink = IndexedBridge::Unlink(IndexedBridgeUnlink::new(
            "tx2",
            11000,
            "discord".to_string(),
            "user2".to_string(),
            1,
        ));
        stats.record_bridge(&unlink);

        let post = IndexedBridge::Post(IndexedBridgePost::new(
            "tx3",
            11000,
            "nostr".to_string(),
            "note1".to_string(),
            "content".to_string(),
            1,
        ));
        stats.record_bridge(&post);

        let verify = IndexedBridge::Verify(IndexedBridgeVerify::new(
            "tx4",
            11000,
            "mastodon".to_string(),
            "user3".to_string(),
            12345,
            1,
        ));
        stats.record_bridge(&verify);

        assert_eq!(stats.total_bridge_txs, 4);
        assert_eq!(stats.bridge_links, 1);
        assert_eq!(stats.bridge_unlinks, 1);
        assert_eq!(stats.bridge_posts, 1);
        assert_eq!(stats.bridge_verifies, 1);
        assert_eq!(stats.telegram_links, 1);
    }

    #[test]
    fn test_block_bridge_stats_display() {
        let _init_guard = zebra_test::init();

        let mut stats = BlockBridgeStats::new(12000);
        stats.record_link("telegram");
        stats.record_unlink();
        stats.record_post();

        let display = format!("{}", stats);
        assert!(display.contains("Block 12000"));
        assert!(display.contains("3 txs"));
        assert!(display.contains("1 links"));
        assert!(display.contains("1 unlinks"));
        assert!(display.contains("1 posts"));
    }

    // ========================================================================
    // Tests for BridgeIndexError
    // ========================================================================

    #[test]
    fn test_bridge_index_error_display() {
        let _init_guard = zebra_test::init();

        assert_eq!(
            format!("{}", BridgeIndexError::NotABridge),
            "memo is not a bridge message"
        );
        assert_eq!(
            format!("{}", BridgeIndexError::InvalidTxId),
            "invalid transaction ID"
        );
        assert_eq!(
            format!(
                "{}",
                BridgeIndexError::InvalidBridgeLink("test".to_string())
            ),
            "invalid bridge link: test"
        );
        assert_eq!(
            format!(
                "{}",
                BridgeIndexError::InvalidBridgeUnlink("test".to_string())
            ),
            "invalid bridge unlink: test"
        );
        assert_eq!(
            format!(
                "{}",
                BridgeIndexError::InvalidBridgePost("test".to_string())
            ),
            "invalid bridge post: test"
        );
        assert_eq!(
            format!(
                "{}",
                BridgeIndexError::InvalidBridgeVerify("test".to_string())
            ),
            "invalid bridge verify: test"
        );
    }

    // ========================================================================
    // Tests for IndexedBridgePost helper
    // ========================================================================

    #[test]
    fn test_indexed_bridge_post_has_content() {
        let _init_guard = zebra_test::init();

        let with_content = IndexedBridgePost::new(
            "tx",
            1000,
            "nostr".to_string(),
            "note1".to_string(),
            "Hello!".to_string(),
            1,
        );
        assert!(with_content.has_content());

        let without_content = IndexedBridgePost::new(
            "tx",
            1000,
            "nostr".to_string(),
            "note2".to_string(),
            "".to_string(),
            1,
        );
        assert!(!without_content.has_content());
    }

    // ========================================================================
    // Tests for platform conversion
    // ========================================================================

    #[test]
    fn test_platform_to_string() {
        let _init_guard = zebra_test::init();

        assert_eq!(platform_to_string(BridgePlatform::Telegram), "telegram");
        assert_eq!(platform_to_string(BridgePlatform::Discord), "discord");
        assert_eq!(platform_to_string(BridgePlatform::Nostr), "nostr");
        assert_eq!(platform_to_string(BridgePlatform::Mastodon), "mastodon");
        assert_eq!(platform_to_string(BridgePlatform::Twitter), "twitter");
    }
}
