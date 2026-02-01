//! Types for social protocol RPC methods.
//!
//! These types support the Botcash Social Protocol (BSP) RPC extensions
//! for social networking functionality built on the blockchain.

use derive_getters::Getters;
use derive_new::new;
use serde::{Deserialize, Serialize};

/// Request for creating a social post.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct SocialPostRequest {
    /// The sender's address (unified or shielded).
    pub from: String,

    /// The content of the post.
    pub content: String,

    /// Optional tags for the post.
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Response for creating a social post.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct SocialPostResponse {
    /// The transaction ID of the created post.
    #[serde(rename = "txid")]
    txid: String,

    /// The message type that was created.
    #[serde(rename = "messageType")]
    message_type: String,
}

/// Request for sending a direct message.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct SocialDmRequest {
    /// The sender's address.
    pub from: String,

    /// The recipient's address.
    pub to: String,

    /// The message content.
    pub content: String,
}

/// Response for sending a direct message.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct SocialDmResponse {
    /// The transaction ID of the sent DM.
    #[serde(rename = "txid")]
    txid: String,

    /// Whether the message was successfully queued.
    #[getter(copy)]
    success: bool,
}

/// Request for following a user.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct SocialFollowRequest {
    /// The follower's address.
    pub from: String,

    /// The address to follow.
    pub target: String,
}

/// Response for following a user.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct SocialFollowResponse {
    /// The transaction ID of the follow action.
    #[serde(rename = "txid")]
    txid: String,

    /// The target address that was followed.
    target: String,
}

/// Request for getting a social feed.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct SocialFeedRequest {
    /// Incoming viewing keys to scan for social messages.
    pub ivks: Vec<String>,

    /// Maximum number of posts to return.
    #[serde(default = "default_feed_limit")]
    pub limit: u32,

    /// Optional starting height for the scan.
    #[serde(rename = "startHeight")]
    pub start_height: Option<u32>,
}

fn default_feed_limit() -> u32 {
    50
}

/// A single post in the social feed.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct SocialFeedPost {
    /// The transaction ID containing this post.
    #[serde(rename = "txid")]
    txid: String,

    /// The type of social message (Post, Comment, etc.).
    #[serde(rename = "messageType")]
    message_type: String,

    /// The block height where this post was confirmed.
    #[getter(copy)]
    height: u32,

    /// The decoded content of the post (if decryptable).
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,

    /// The sender address (if known).
    #[serde(skip_serializing_if = "Option::is_none")]
    from: Option<String>,

    /// The Unix timestamp of the block containing this post.
    #[getter(copy)]
    timestamp: i64,

    /// Tags associated with this post.
    #[serde(default)]
    tags: Vec<String>,
}

impl Default for SocialFeedPost {
    fn default() -> Self {
        Self {
            txid: String::new(),
            message_type: String::new(),
            height: 0,
            content: None,
            from: None,
            timestamp: 0,
            tags: Vec::new(),
        }
    }
}

/// Response for getting a social feed.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct SocialFeedResponse {
    /// The posts in the feed.
    posts: Vec<SocialFeedPost>,

    /// The total number of posts found (may be more than returned if limit applied).
    #[getter(copy)]
    #[serde(rename = "totalCount")]
    total_count: u32,

    /// The height range that was scanned.
    #[serde(rename = "scannedRange")]
    scanned_range: ScannedRange,
}

/// The height range that was scanned for social messages.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct ScannedRange {
    /// The starting block height.
    #[getter(copy)]
    start: u32,

    /// The ending block height.
    #[getter(copy)]
    end: u32,
}

// ==================== Attention Market Types ====================

/// Request for boosting content visibility in the attention market.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct AttentionBoostRequest {
    /// The sender's address (unified or shielded).
    pub from: String,

    /// The transaction ID of the content to boost.
    #[serde(rename = "targetTxid")]
    pub target_txid: String,

    /// The amount of BCASH to spend on the boost (in zatoshis).
    pub amount: u64,

    /// How long the boost lasts (in blocks). Default is 1440 (~1 day at 60s blocks).
    #[serde(rename = "durationBlocks", default = "default_boost_duration")]
    pub duration_blocks: u32,

    /// Optional category code (0x00-0xFF).
    #[serde(default)]
    pub category: Option<u8>,
}

fn default_boost_duration() -> u32 {
    1440 // ~1 day at 60 second blocks
}

/// Response for boosting content.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct AttentionBoostResponse {
    /// The transaction ID of the boost transaction.
    #[serde(rename = "txid")]
    txid: String,

    /// The target content that was boosted.
    #[serde(rename = "targetTxid")]
    target_txid: String,

    /// The amount spent in zatoshis.
    #[getter(copy)]
    amount: u64,

    /// The block at which the boost expires.
    #[serde(rename = "expiresAtBlock")]
    #[getter(copy)]
    expires_at_block: u32,
}

/// Request for tipping with credits.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct CreditTipRequest {
    /// The sender's address.
    pub from: String,

    /// The transaction ID of the content to tip.
    #[serde(rename = "targetTxid")]
    pub target_txid: String,

    /// The amount of credits to tip (in zatoshis).
    #[serde(rename = "creditAmount")]
    pub credit_amount: u64,

    /// Optional message to include with the tip.
    #[serde(default)]
    pub message: Option<String>,
}

/// Response for tipping with credits.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct CreditTipResponse {
    /// The transaction ID of the tip transaction.
    #[serde(rename = "txid")]
    txid: String,

    /// The amount of credits spent.
    #[serde(rename = "creditSpent")]
    #[getter(copy)]
    credit_spent: u64,

    /// The remaining credit balance after the tip.
    #[serde(rename = "remainingCredits")]
    #[getter(copy)]
    remaining_credits: u64,
}

/// Request for getting credit balance.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct CreditBalanceRequest {
    /// The address to check the credit balance for.
    pub address: String,
}

/// A single credit grant with expiration info.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct CreditGrant {
    /// The amount of credits granted.
    #[getter(copy)]
    amount: u64,

    /// The block height when these credits were granted.
    #[serde(rename = "grantedBlock")]
    #[getter(copy)]
    granted_block: u32,

    /// The block height when these credits expire.
    #[serde(rename = "expiresBlock")]
    #[getter(copy)]
    expires_block: u32,

    /// The amount already spent from this grant.
    #[getter(copy)]
    spent: u64,
}

/// Response for getting credit balance.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct CreditBalanceResponse {
    /// The total available credit balance (in zatoshis).
    #[getter(copy)]
    balance: u64,

    /// Credits expiring within the next day (1440 blocks).
    #[serde(rename = "expiringSoon")]
    #[getter(copy)]
    expiring_soon: u64,

    /// Individual credit grants with their expiration info.
    grants: Vec<CreditGrant>,
}

/// Request for getting market feed.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct MarketFeedRequest {
    /// The type of feed to retrieve: "hot", "top", "new", or "boosted".
    #[serde(rename = "feedType", default = "default_feed_type")]
    pub feed_type: String,

    /// Optional category filter (0-255).
    #[serde(default)]
    pub category: Option<u8>,

    /// Maximum number of items to return.
    #[serde(default = "default_market_limit")]
    pub limit: u32,

    /// Offset for pagination.
    #[serde(default)]
    pub offset: u32,
}

fn default_feed_type() -> String {
    "hot".to_string()
}

fn default_market_limit() -> u32 {
    50
}

/// A content item in the market feed.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct MarketContent {
    /// The transaction ID of the content.
    #[serde(rename = "txid")]
    txid: String,

    /// The content text (if available).
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,

    /// The author's address (if known).
    #[serde(skip_serializing_if = "Option::is_none")]
    author: Option<String>,

    /// The calculated Attention Units (AU) score.
    #[getter(copy)]
    au: f64,

    /// The total BCASH paid for boosts.
    #[serde(rename = "bcashPaid")]
    #[getter(copy)]
    bcash_paid: u64,

    /// The total tips received.
    #[serde(rename = "tipsReceived")]
    #[getter(copy)]
    tips_received: u64,

    /// The block height of the content.
    #[getter(copy)]
    height: u32,

    /// The Unix timestamp.
    #[getter(copy)]
    timestamp: i64,

    /// Whether the content is currently boosted.
    #[serde(rename = "isBoosted")]
    #[getter(copy)]
    is_boosted: bool,

    /// The block when the boost expires (if boosted).
    #[serde(rename = "boostExpires", skip_serializing_if = "Option::is_none")]
    boost_expires: Option<u32>,

    /// The category code (if set).
    #[serde(skip_serializing_if = "Option::is_none")]
    category: Option<u8>,
}

impl Eq for MarketContent {}

/// Response for getting market feed.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct MarketFeedResponse {
    /// The content items in the feed.
    items: Vec<MarketContent>,

    /// The total number of items matching the query.
    #[serde(rename = "totalCount")]
    #[getter(copy)]
    total_count: u32,

    /// The feed type that was queried.
    #[serde(rename = "feedType")]
    feed_type: String,
}

impl Eq for MarketFeedResponse {}

/// Request for getting epoch statistics.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct EpochStatsRequest {
    /// The epoch number to query. If not specified, returns current epoch.
    #[serde(rename = "epochNumber", default)]
    pub epoch_number: Option<u32>,
}

/// Response for getting epoch statistics.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct EpochStatsResponse {
    /// The epoch number.
    #[serde(rename = "epochNumber")]
    #[getter(copy)]
    epoch_number: u32,

    /// The starting block height of the epoch.
    #[serde(rename = "startBlock")]
    #[getter(copy)]
    start_block: u32,

    /// The ending block height of the epoch.
    #[serde(rename = "endBlock")]
    #[getter(copy)]
    end_block: u32,

    /// Total BCASH paid into the attention market this epoch.
    #[serde(rename = "totalPaid")]
    #[getter(copy)]
    total_paid: u64,

    /// Number of unique participants (payers) this epoch.
    #[getter(copy)]
    participants: u32,

    /// Total credits distributed this epoch.
    #[getter(copy)]
    distributed: u64,

    /// Whether this epoch is complete (ended).
    #[serde(rename = "isComplete")]
    #[getter(copy)]
    is_complete: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn social_post_request_deserialize() {
        let json = r#"{"from":"bs1test","content":"Hello Botcash!","tags":["test","social"]}"#;
        let req: SocialPostRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.from, "bs1test");
        assert_eq!(req.content, "Hello Botcash!");
        assert_eq!(req.tags, vec!["test", "social"]);
    }

    #[test]
    fn social_post_request_deserialize_no_tags() {
        let json = r#"{"from":"bs1test","content":"Hello!"}"#;
        let req: SocialPostRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.from, "bs1test");
        assert!(req.tags.is_empty());
    }

    #[test]
    fn social_post_response_serialize() {
        let resp = SocialPostResponse::new("abcd1234".to_string(), "Post".to_string());
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"txid\":\"abcd1234\""));
        assert!(json.contains("\"messageType\":\"Post\""));
    }

    #[test]
    fn social_dm_request_deserialize() {
        let json = r#"{"from":"bs1sender","to":"bs1receiver","content":"Private message"}"#;
        let req: SocialDmRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.from, "bs1sender");
        assert_eq!(req.to, "bs1receiver");
        assert_eq!(req.content, "Private message");
    }

    #[test]
    fn social_dm_response_serialize() {
        let resp = SocialDmResponse::new("txid123".to_string(), true);
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"txid\":\"txid123\""));
        assert!(json.contains("\"success\":true"));
    }

    #[test]
    fn social_follow_request_deserialize() {
        let json = r#"{"from":"bs1follower","target":"bs1target"}"#;
        let req: SocialFollowRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.from, "bs1follower");
        assert_eq!(req.target, "bs1target");
    }

    #[test]
    fn social_follow_response_serialize() {
        let resp = SocialFollowResponse::new("txid456".to_string(), "bs1target".to_string());
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"txid\":\"txid456\""));
        assert!(json.contains("\"target\":\"bs1target\""));
    }

    #[test]
    fn social_feed_request_deserialize() {
        let json = r#"{"ivks":["ivk1","ivk2"],"limit":10,"startHeight":100}"#;
        let req: SocialFeedRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.ivks, vec!["ivk1", "ivk2"]);
        assert_eq!(req.limit, 10);
        assert_eq!(req.start_height, Some(100));
    }

    #[test]
    fn social_feed_request_default_limit() {
        let json = r#"{"ivks":["ivk1"]}"#;
        let req: SocialFeedRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.limit, 50); // default
    }

    #[test]
    fn social_feed_post_serialize() {
        let post = SocialFeedPost::new(
            "txid789".to_string(),
            "Post".to_string(),
            1000,
            Some("Hello world!".to_string()),
            Some("bs1author".to_string()),
            1234567890,
            vec!["hello".to_string()],
        );
        let json = serde_json::to_string(&post).unwrap();
        assert!(json.contains("\"txid\":\"txid789\""));
        assert!(json.contains("\"messageType\":\"Post\""));
        assert!(json.contains("\"height\":1000"));
        assert!(json.contains("\"content\":\"Hello world!\""));
        assert!(json.contains("\"from\":\"bs1author\""));
    }

    #[test]
    fn social_feed_post_skip_none() {
        let post = SocialFeedPost::new(
            "txid".to_string(),
            "Post".to_string(),
            100,
            None,
            None,
            0,
            vec![],
        );
        let json = serde_json::to_string(&post).unwrap();
        assert!(!json.contains("\"content\""));
        assert!(!json.contains("\"from\""));
    }

    #[test]
    fn social_feed_response_serialize() {
        let resp = SocialFeedResponse::new(vec![], 0, ScannedRange::new(100, 200));
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"posts\":[]"));
        assert!(json.contains("\"totalCount\":0"));
        assert!(json.contains("\"scannedRange\""));
    }

    #[test]
    fn scanned_range_serialize() {
        let range = ScannedRange::new(100, 200);
        let json = serde_json::to_string(&range).unwrap();
        assert!(json.contains("\"start\":100"));
        assert!(json.contains("\"end\":200"));
    }

    // ==================== Attention Market Tests ====================

    #[test]
    fn z_attention_boost_request_deserialize() {
        let json = r#"{"from":"bs1sender","targetTxid":"abc123","amount":100000,"durationBlocks":2880,"category":1}"#;
        let req: AttentionBoostRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.from, "bs1sender");
        assert_eq!(req.target_txid, "abc123");
        assert_eq!(req.amount, 100000);
        assert_eq!(req.duration_blocks, 2880);
        assert_eq!(req.category, Some(1));
    }

    #[test]
    fn z_attention_boost_request_defaults() {
        let json = r#"{"from":"bs1sender","targetTxid":"abc123","amount":100000}"#;
        let req: AttentionBoostRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.duration_blocks, 1440); // default
        assert_eq!(req.category, None);
    }

    #[test]
    fn z_attention_boost_response_serialize() {
        let resp = AttentionBoostResponse::new(
            "txid789".to_string(),
            "target123".to_string(),
            100000,
            12345,
        );
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"txid\":\"txid789\""));
        assert!(json.contains("\"targetTxid\":\"target123\""));
        assert!(json.contains("\"amount\":100000"));
        assert!(json.contains("\"expiresAtBlock\":12345"));
    }

    #[test]
    fn z_attention_credit_tip_request_deserialize() {
        let json = r#"{"from":"bs1sender","targetTxid":"abc123","creditAmount":50000,"message":"Great post!"}"#;
        let req: CreditTipRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.from, "bs1sender");
        assert_eq!(req.target_txid, "abc123");
        assert_eq!(req.credit_amount, 50000);
        assert_eq!(req.message, Some("Great post!".to_string()));
    }

    #[test]
    fn z_attention_credit_tip_request_no_message() {
        let json = r#"{"from":"bs1sender","targetTxid":"abc123","creditAmount":50000}"#;
        let req: CreditTipRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.message, None);
    }

    #[test]
    fn z_attention_credit_tip_response_serialize() {
        let resp = CreditTipResponse::new("txid456".to_string(), 50000, 100000);
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"txid\":\"txid456\""));
        assert!(json.contains("\"creditSpent\":50000"));
        assert!(json.contains("\"remainingCredits\":100000"));
    }

    #[test]
    fn z_attention_credit_balance_request_deserialize() {
        let json = r#"{"address":"bs1myaddress"}"#;
        let req: CreditBalanceRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.address, "bs1myaddress");
    }

    #[test]
    fn z_attention_credit_grant_serialize() {
        let grant = CreditGrant::new(100000, 1000, 11080, 20000);
        let json = serde_json::to_string(&grant).unwrap();
        assert!(json.contains("\"amount\":100000"));
        assert!(json.contains("\"grantedBlock\":1000"));
        assert!(json.contains("\"expiresBlock\":11080"));
        assert!(json.contains("\"spent\":20000"));
    }

    #[test]
    fn z_attention_credit_balance_response_serialize() {
        let grant = CreditGrant::new(100000, 1000, 11080, 20000);
        let resp = CreditBalanceResponse::new(80000, 30000, vec![grant]);
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"balance\":80000"));
        assert!(json.contains("\"expiringSoon\":30000"));
        assert!(json.contains("\"grants\":["));
    }

    #[test]
    fn z_attention_market_feed_request_deserialize() {
        let json = r#"{"feedType":"hot","category":1,"limit":25,"offset":10}"#;
        let req: MarketFeedRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.feed_type, "hot");
        assert_eq!(req.category, Some(1));
        assert_eq!(req.limit, 25);
        assert_eq!(req.offset, 10);
    }

    #[test]
    fn z_attention_market_feed_request_defaults() {
        let json = r#"{}"#;
        let req: MarketFeedRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.feed_type, "hot"); // default
        assert_eq!(req.limit, 50); // default
        assert_eq!(req.offset, 0);
        assert_eq!(req.category, None);
    }

    #[test]
    fn z_attention_market_content_serialize() {
        let content = MarketContent::new(
            "txid123".to_string(),
            Some("Hello world".to_string()),
            Some("bs1author".to_string()),
            12.5,
            100000,
            50000,
            1000,
            1234567890,
            true,
            Some(2000),
            Some(1),
        );
        let json = serde_json::to_string(&content).unwrap();
        assert!(json.contains("\"txid\":\"txid123\""));
        assert!(json.contains("\"content\":\"Hello world\""));
        assert!(json.contains("\"author\":\"bs1author\""));
        assert!(json.contains("\"au\":12.5"));
        assert!(json.contains("\"bcashPaid\":100000"));
        assert!(json.contains("\"tipsReceived\":50000"));
        assert!(json.contains("\"isBoosted\":true"));
        assert!(json.contains("\"boostExpires\":2000"));
        assert!(json.contains("\"category\":1"));
    }

    #[test]
    fn z_attention_market_content_skip_none() {
        let content = MarketContent::new(
            "txid123".to_string(),
            None,
            None,
            5.0,
            50000,
            0,
            1000,
            1234567890,
            false,
            None,
            None,
        );
        let json = serde_json::to_string(&content).unwrap();
        assert!(!json.contains("\"content\""));
        assert!(!json.contains("\"author\""));
        assert!(!json.contains("\"boostExpires\""));
        assert!(!json.contains("\"category\""));
    }

    #[test]
    fn z_attention_market_feed_response_serialize() {
        let resp = MarketFeedResponse::new(vec![], 0, "hot".to_string());
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"items\":[]"));
        assert!(json.contains("\"totalCount\":0"));
        assert!(json.contains("\"feedType\":\"hot\""));
    }

    #[test]
    fn z_attention_epoch_stats_request_deserialize() {
        let json = r#"{"epochNumber":5}"#;
        let req: EpochStatsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.epoch_number, Some(5));
    }

    #[test]
    fn z_attention_epoch_stats_request_no_epoch() {
        let json = r#"{}"#;
        let req: EpochStatsRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.epoch_number, None);
    }

    #[test]
    fn z_attention_epoch_stats_response_serialize() {
        let resp = EpochStatsResponse::new(5, 7200, 8639, 1000000, 50, 800000, true);
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"epochNumber\":5"));
        assert!(json.contains("\"startBlock\":7200"));
        assert!(json.contains("\"endBlock\":8639"));
        assert!(json.contains("\"totalPaid\":1000000"));
        assert!(json.contains("\"participants\":50"));
        assert!(json.contains("\"distributed\":800000"));
        assert!(json.contains("\"isComplete\":true"));
    }
}
