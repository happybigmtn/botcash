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
}
