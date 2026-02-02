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

// ==================== Governance Types ====================

/// The type of governance proposal.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GovernanceProposalType {
    /// Change a protocol parameter (fees, block size, etc.).
    Parameter,
    /// Protocol upgrade (soft fork).
    Upgrade,
    /// Treasury spending (if enabled).
    Spending,
    /// Other/general proposal.
    Other,
}

impl GovernanceProposalType {
    /// Returns the byte value for encoding.
    pub fn as_u8(&self) -> u8 {
        match self {
            GovernanceProposalType::Parameter => 0x01,
            GovernanceProposalType::Upgrade => 0x02,
            GovernanceProposalType::Spending => 0x03,
            GovernanceProposalType::Other => 0x00,
        }
    }
}

impl Default for GovernanceProposalType {
    fn default() -> Self {
        Self::Other
    }
}

/// The vote choice for a governance proposal.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GovernanceVoteChoice {
    /// Vote against the proposal.
    No,
    /// Vote in favor of the proposal.
    Yes,
    /// Abstain from voting (counts towards quorum but not threshold).
    Abstain,
}

impl GovernanceVoteChoice {
    /// Returns the byte value for encoding.
    pub fn as_u8(&self) -> u8 {
        match self {
            GovernanceVoteChoice::No => 0x00,
            GovernanceVoteChoice::Yes => 0x01,
            GovernanceVoteChoice::Abstain => 0x02,
        }
    }
}

/// Request for creating a governance proposal.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct GovernanceProposalRequest {
    /// The proposer's address (unified or shielded).
    pub from: String,

    /// The type of proposal.
    #[serde(rename = "proposalType", default)]
    pub proposal_type: GovernanceProposalType,

    /// The title of the proposal (max 255 chars).
    pub title: String,

    /// The description of the proposal.
    pub description: String,

    /// Optional parameter changes (for Parameter proposals).
    /// Format: [{"param": "name", "value": "new_value"}, ...]
    #[serde(default)]
    pub parameters: Vec<ParameterChange>,

    /// The deposit amount in zatoshis (returned if >10% support).
    /// Default is minimum required: 10 BCASH = 1,000,000,000 zatoshis.
    #[serde(default = "default_proposal_deposit")]
    pub deposit: u64,
}

fn default_proposal_deposit() -> u64 {
    1_000_000_000 // 10 BCASH in zatoshis
}

/// A parameter change within a governance proposal.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct ParameterChange {
    /// The parameter name to change.
    pub param: String,

    /// The new value for the parameter.
    pub value: String,
}

/// Response for creating a governance proposal.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct GovernanceProposalResponse {
    /// The transaction ID of the proposal.
    #[serde(rename = "txid")]
    txid: String,

    /// The unique proposal ID (hex-encoded).
    #[serde(rename = "proposalId")]
    proposal_id: String,

    /// The block height when the proposal was created.
    #[getter(copy)]
    height: u32,

    /// The block height when voting begins.
    #[serde(rename = "votingStartsBlock")]
    #[getter(copy)]
    voting_starts_block: u32,

    /// The block height when voting ends.
    #[serde(rename = "votingEndsBlock")]
    #[getter(copy)]
    voting_ends_block: u32,

    /// The deposit amount locked (in zatoshis).
    #[getter(copy)]
    deposit: u64,
}

/// Request for voting on a governance proposal.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct GovernanceVoteRequest {
    /// The voter's address (unified or shielded).
    pub from: String,

    /// The proposal ID to vote on (hex-encoded, 32 bytes).
    #[serde(rename = "proposalId")]
    pub proposal_id: String,

    /// The vote choice.
    pub vote: GovernanceVoteChoice,
}

/// Response for voting on a governance proposal.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct GovernanceVoteResponse {
    /// The transaction ID of the vote.
    #[serde(rename = "txid")]
    txid: String,

    /// The proposal ID that was voted on.
    #[serde(rename = "proposalId")]
    proposal_id: String,

    /// The vote that was cast.
    vote: GovernanceVoteChoice,

    /// The voter's calculated voting power.
    #[serde(rename = "votingPower")]
    #[getter(copy)]
    voting_power: f64,
}

impl Eq for GovernanceVoteResponse {}

/// Request for getting proposal status.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct GovernanceProposalStatusRequest {
    /// The proposal ID to query (hex-encoded, 32 bytes).
    #[serde(rename = "proposalId")]
    pub proposal_id: String,
}

/// Response for getting proposal status.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct GovernanceProposalStatusResponse {
    /// The proposal ID.
    #[serde(rename = "proposalId")]
    proposal_id: String,

    /// The proposal title.
    title: String,

    /// The proposal type.
    #[serde(rename = "proposalType")]
    proposal_type: GovernanceProposalType,

    /// Current status: "pending", "voting", "passed", "rejected", "executed".
    status: String,

    /// Total "yes" voting power.
    #[serde(rename = "yesVotes")]
    #[getter(copy)]
    yes_votes: f64,

    /// Total "no" voting power.
    #[serde(rename = "noVotes")]
    #[getter(copy)]
    no_votes: f64,

    /// Total "abstain" voting power.
    #[serde(rename = "abstainVotes")]
    #[getter(copy)]
    abstain_votes: f64,

    /// Current quorum percentage (votes / circulating supply).
    #[serde(rename = "quorumPercent")]
    #[getter(copy)]
    quorum_percent: f64,

    /// Required quorum percentage (default 20%).
    #[serde(rename = "quorumRequired")]
    #[getter(copy)]
    quorum_required: f64,

    /// Current approval percentage (yes / (yes + no)).
    #[serde(rename = "approvalPercent")]
    #[getter(copy)]
    approval_percent: f64,

    /// Required approval percentage (default 66%).
    #[serde(rename = "approvalRequired")]
    #[getter(copy)]
    approval_required: f64,

    /// Block height when voting ends.
    #[serde(rename = "votingEndsBlock")]
    #[getter(copy)]
    voting_ends_block: u32,

    /// Block height when the proposal executes (if passed).
    #[serde(rename = "executionBlock", skip_serializing_if = "Option::is_none")]
    execution_block: Option<u32>,
}

impl Eq for GovernanceProposalStatusResponse {}

/// Request for listing governance proposals.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct GovernanceListRequest {
    /// Filter by status: "all", "pending", "voting", "passed", "rejected", "executed".
    #[serde(default = "default_governance_list_status")]
    pub status: String,

    /// Maximum number of proposals to return.
    #[serde(default = "default_governance_list_limit")]
    pub limit: u32,

    /// Offset for pagination.
    #[serde(default)]
    pub offset: u32,
}

fn default_governance_list_status() -> String {
    "all".to_string()
}

fn default_governance_list_limit() -> u32 {
    50
}

/// Summary of a governance proposal for listing.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct GovernanceProposalSummary {
    /// The proposal ID.
    #[serde(rename = "proposalId")]
    proposal_id: String,

    /// The proposal title.
    title: String,

    /// The proposal type.
    #[serde(rename = "proposalType")]
    proposal_type: GovernanceProposalType,

    /// Current status.
    status: String,

    /// Current approval percentage.
    #[serde(rename = "approvalPercent")]
    #[getter(copy)]
    approval_percent: f64,

    /// Block height when voting ends.
    #[serde(rename = "votingEndsBlock")]
    #[getter(copy)]
    voting_ends_block: u32,
}

impl Eq for GovernanceProposalSummary {}

/// Response for listing governance proposals.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct GovernanceListResponse {
    /// The proposals matching the query.
    proposals: Vec<GovernanceProposalSummary>,

    /// Total number of proposals matching the filter.
    #[serde(rename = "totalCount")]
    #[getter(copy)]
    total_count: u32,
}

// ==================== Batch Queue Types ====================

/// Maximum number of actions that can be queued for batching.
pub const MAX_BATCH_QUEUE_SIZE: usize = 5;

/// A single action to be queued for batching.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum BatchAction {
    /// Create a post.
    Post {
        /// The content of the post.
        content: String,
        /// Optional tags for the post.
        #[serde(default)]
        tags: Vec<String>,
    },
    /// Send a direct message.
    Dm {
        /// The recipient's address.
        to: String,
        /// The message content.
        content: String,
    },
    /// Follow a user.
    Follow {
        /// The address to follow.
        target: String,
    },
    /// Unfollow a user.
    Unfollow {
        /// The address to unfollow.
        target: String,
    },
    /// Upvote content.
    Upvote {
        /// The transaction ID of content to upvote.
        #[serde(rename = "targetTxid")]
        target_txid: String,
    },
    /// Create a comment.
    Comment {
        /// The transaction ID of content to comment on.
        #[serde(rename = "targetTxid")]
        target_txid: String,
        /// The comment content.
        content: String,
    },
    /// Tip content.
    Tip {
        /// The transaction ID of content to tip.
        #[serde(rename = "targetTxid")]
        target_txid: String,
        /// The amount to tip in zatoshis.
        amount: u64,
    },
}

impl BatchAction {
    /// Returns the action type as a string for display.
    pub fn action_type(&self) -> &'static str {
        match self {
            BatchAction::Post { .. } => "Post",
            BatchAction::Dm { .. } => "Dm",
            BatchAction::Follow { .. } => "Follow",
            BatchAction::Unfollow { .. } => "Unfollow",
            BatchAction::Upvote { .. } => "Upvote",
            BatchAction::Comment { .. } => "Comment",
            BatchAction::Tip { .. } => "Tip",
        }
    }
}

/// Request for queuing actions to be batched.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct BatchQueueRequest {
    /// The sender's address (unified or shielded).
    pub from: String,

    /// The actions to queue for batching.
    pub actions: Vec<BatchAction>,

    /// Whether to send immediately if queue reaches max size.
    /// If false, returns an error when queue is full.
    #[serde(rename = "autoSend", default)]
    pub auto_send: bool,
}

/// Response for queuing actions.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct BatchQueueResponse {
    /// Number of actions successfully queued.
    #[getter(copy)]
    queued: usize,

    /// Current queue size after adding actions.
    #[serde(rename = "queueSize")]
    #[getter(copy)]
    queue_size: usize,

    /// If auto_send was true and queue was sent, this is the transaction ID.
    #[serde(rename = "txid", skip_serializing_if = "Option::is_none")]
    txid: Option<String>,

    /// Actions that were queued (type names).
    #[serde(rename = "actionTypes")]
    action_types: Vec<String>,
}

/// Request for sending the current batch queue.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct BatchSendRequest {
    /// The sender's address (must match the queued actions).
    pub from: String,
}

/// Response for sending the batch.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct BatchSendResponse {
    /// The transaction ID of the batched transaction.
    #[serde(rename = "txid")]
    txid: String,

    /// Number of actions that were batched.
    #[serde(rename = "actionCount")]
    #[getter(copy)]
    action_count: usize,

    /// The types of actions that were batched.
    #[serde(rename = "actionTypes")]
    action_types: Vec<String>,

    /// Estimated fee saved compared to individual transactions (in zatoshis).
    #[serde(rename = "feeSaved")]
    #[getter(copy)]
    fee_saved: u64,
}

/// Request for getting the current batch queue status.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct BatchStatusRequest {
    /// The address to check queue status for.
    pub from: String,
}

/// Response for batch queue status.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct BatchStatusResponse {
    /// Current number of actions in the queue.
    #[serde(rename = "queueSize")]
    #[getter(copy)]
    queue_size: usize,

    /// Maximum queue size before auto-send or error.
    #[serde(rename = "maxSize")]
    #[getter(copy)]
    max_size: usize,

    /// The types of actions currently queued.
    #[serde(rename = "actionTypes")]
    action_types: Vec<String>,

    /// Estimated encoded size of the batch (bytes).
    #[serde(rename = "estimatedSize")]
    #[getter(copy)]
    estimated_size: usize,
}

/// Request for clearing the batch queue.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct BatchClearRequest {
    /// The address whose queue should be cleared.
    pub from: String,
}

/// Response for clearing the batch queue.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct BatchClearResponse {
    /// Number of actions that were cleared.
    #[getter(copy)]
    cleared: usize,

    /// Whether the operation was successful.
    #[getter(copy)]
    success: bool,
}

// ==================== Channel Types (Layer-2 Social Channels) ====================

/// Default channel timeout in blocks (~1 day at 60s blocks).
pub const DEFAULT_CHANNEL_TIMEOUT_BLOCKS: u32 = 1440;

/// Maximum number of parties in a channel.
pub const MAX_CHANNEL_PARTIES: usize = 10;

/// Minimum deposit required for a channel (in zatoshis).
pub const MIN_CHANNEL_DEPOSIT: u64 = 100_000; // 0.001 BCASH

/// Request for opening a new Layer-2 social channel.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct ChannelOpenRequest {
    /// The initiator's address (unified or shielded).
    pub from: String,

    /// The list of party addresses for the channel.
    pub parties: Vec<String>,

    /// The deposit amount in zatoshis.
    /// This is the total deposit for the channel, split among parties.
    pub deposit: u64,

    /// Timeout in blocks before unilateral settlement is allowed.
    /// Default is 1440 (~1 day at 60s blocks).
    #[serde(rename = "timeoutBlocks", default = "default_channel_timeout")]
    pub timeout_blocks: u32,
}

fn default_channel_timeout() -> u32 {
    DEFAULT_CHANNEL_TIMEOUT_BLOCKS
}

/// Response for opening a channel.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct ChannelOpenResponse {
    /// The unique channel ID (32 bytes hex-encoded).
    #[serde(rename = "channelId")]
    channel_id: String,

    /// The transaction ID that opened the channel.
    #[serde(rename = "txid")]
    txid: String,

    /// The block height at which the channel was opened.
    #[serde(rename = "openedAtBlock")]
    #[getter(copy)]
    opened_at_block: u32,

    /// The block height at which unilateral settlement becomes available.
    #[serde(rename = "timeoutBlock")]
    #[getter(copy)]
    timeout_block: u32,
}

/// Request for closing a channel cooperatively.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct ChannelCloseRequest {
    /// The closer's address (must be a party to the channel).
    pub from: String,

    /// The channel ID to close (32 bytes hex-encoded).
    #[serde(rename = "channelId")]
    pub channel_id: String,

    /// The final sequence number of the last off-chain message.
    #[serde(rename = "finalSeq")]
    pub final_seq: u32,
}

/// Response for closing a channel.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct ChannelCloseResponse {
    /// The transaction ID of the close transaction.
    #[serde(rename = "txid")]
    txid: String,

    /// The channel ID that was closed.
    #[serde(rename = "channelId")]
    channel_id: String,

    /// The final sequence number.
    #[serde(rename = "finalSeq")]
    #[getter(copy)]
    final_seq: u32,

    /// Whether all parties have agreed to the close.
    #[serde(rename = "cooperative")]
    #[getter(copy)]
    cooperative: bool,
}

/// Request for settling a channel (may be unilateral).
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct ChannelSettleRequest {
    /// The settler's address (must be a party to the channel).
    pub from: String,

    /// The channel ID to settle (32 bytes hex-encoded).
    #[serde(rename = "channelId")]
    pub channel_id: String,

    /// The final sequence number of off-chain messages.
    #[serde(rename = "finalSeq")]
    pub final_seq: u32,

    /// Merkle root hash of all off-chain messages (32 bytes hex-encoded).
    /// Used for dispute resolution.
    #[serde(rename = "messageHash")]
    pub message_hash: String,
}

/// Response for settling a channel.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct ChannelSettleResponse {
    /// The transaction ID of the settlement.
    #[serde(rename = "txid")]
    txid: String,

    /// The channel ID that was settled.
    #[serde(rename = "channelId")]
    channel_id: String,

    /// The final sequence number.
    #[serde(rename = "finalSeq")]
    #[getter(copy)]
    final_seq: u32,

    /// The final balance distribution (address -> amount in zatoshis).
    /// In the simplest case, the original deposit is returned to parties.
    #[serde(rename = "finalBalances")]
    final_balances: std::collections::HashMap<String, u64>,
}

/// The state of a channel.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ChannelState {
    /// Channel is open and active.
    Open,
    /// Channel close has been initiated but not finalized.
    Closing,
    /// Channel has been settled.
    Settled,
    /// Channel settlement is disputed.
    Disputed,
}

impl Default for ChannelState {
    fn default() -> Self {
        Self::Open
    }
}

impl std::fmt::Display for ChannelState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Open => write!(f, "open"),
            Self::Closing => write!(f, "closing"),
            Self::Settled => write!(f, "settled"),
            Self::Disputed => write!(f, "disputed"),
        }
    }
}

/// Request for getting the status of a channel.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct ChannelStatusRequest {
    /// The channel ID to query (32 bytes hex-encoded).
    #[serde(rename = "channelId")]
    pub channel_id: String,
}

/// Response for channel status.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct ChannelStatusResponse {
    /// The channel ID.
    #[serde(rename = "channelId")]
    channel_id: String,

    /// The current state of the channel.
    state: ChannelState,

    /// The parties in the channel.
    parties: Vec<String>,

    /// The total deposit in the channel (zatoshis).
    #[getter(copy)]
    deposit: u64,

    /// The current sequence number of off-chain messages.
    #[serde(rename = "currentSeq")]
    #[getter(copy)]
    current_seq: u32,

    /// Block height when the channel was opened.
    #[serde(rename = "openedAtBlock")]
    #[getter(copy)]
    opened_at_block: u32,

    /// Block height when unilateral settlement becomes available.
    #[serde(rename = "timeoutBlock")]
    #[getter(copy)]
    timeout_block: u32,

    /// Latest message hash (if any off-chain messages exist).
    #[serde(rename = "latestMessageHash", skip_serializing_if = "Option::is_none")]
    latest_message_hash: Option<String>,
}

/// Request for listing channels for an address.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct ChannelListRequest {
    /// The address to list channels for.
    pub address: String,

    /// Filter by channel state (optional).
    #[serde(default)]
    pub state: Option<ChannelState>,

    /// Maximum number of channels to return.
    #[serde(default = "default_channel_list_limit")]
    pub limit: u32,
}

fn default_channel_list_limit() -> u32 {
    50
}

/// Summary of a channel for listing.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct ChannelSummary {
    /// The channel ID.
    #[serde(rename = "channelId")]
    channel_id: String,

    /// The current state.
    state: ChannelState,

    /// Number of parties in the channel.
    #[serde(rename = "partyCount")]
    #[getter(copy)]
    party_count: usize,

    /// Total deposit in zatoshis.
    #[getter(copy)]
    deposit: u64,

    /// Current sequence number.
    #[serde(rename = "currentSeq")]
    #[getter(copy)]
    current_seq: u32,

    /// Block height when opened.
    #[serde(rename = "openedAtBlock")]
    #[getter(copy)]
    opened_at_block: u32,
}

/// Response for listing channels.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct ChannelListResponse {
    /// The channels matching the query.
    channels: Vec<ChannelSummary>,

    /// Total number of channels matching the filter.
    #[serde(rename = "totalCount")]
    #[getter(copy)]
    total_count: u32,
}

// ==================== Recovery Types ====================

/// Default timelock in blocks (~7 days at 60s blocks).
pub const DEFAULT_RECOVERY_TIMELOCK_BLOCKS: u32 = 10080;

/// Minimum number of guardians required for recovery.
pub const MIN_RECOVERY_GUARDIANS: usize = 1;

/// Maximum number of guardians allowed.
pub const MAX_RECOVERY_GUARDIANS: usize = 15;

/// Default threshold for M-of-N recovery (3-of-5).
pub const DEFAULT_RECOVERY_THRESHOLD: usize = 3;

/// Status of a recovery configuration.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RecoveryStatus {
    /// Recovery is configured and active.
    Active,
    /// Recovery request is pending guardian approval.
    Pending,
    /// Sufficient guardians have approved, waiting for timelock.
    Approved,
    /// Recovery is in the timelock waiting period.
    Timelocked,
    /// Recovery was successfully executed.
    Executed,
    /// Recovery was cancelled by the owner.
    Cancelled,
    /// Recovery request expired (timelock exceeded without execution).
    Expired,
}

impl std::fmt::Display for RecoveryStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Pending => write!(f, "pending"),
            Self::Approved => write!(f, "approved"),
            Self::Timelocked => write!(f, "timelocked"),
            Self::Executed => write!(f, "executed"),
            Self::Cancelled => write!(f, "cancelled"),
            Self::Expired => write!(f, "expired"),
        }
    }
}

/// Request for setting up recovery configuration.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct RecoveryConfigRequest {
    /// The address to configure recovery for.
    pub from: String,

    /// List of guardian addresses (their hashes will be stored on-chain).
    pub guardians: Vec<String>,

    /// Number of guardians required to approve recovery (M in M-of-N).
    pub threshold: u8,

    /// Timelock period in blocks before recovery can be executed.
    #[serde(rename = "timelockBlocks", default = "default_recovery_timelock")]
    pub timelock_blocks: u32,
}

fn default_recovery_timelock() -> u32 {
    DEFAULT_RECOVERY_TIMELOCK_BLOCKS
}

/// Response for setting up recovery configuration.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct RecoveryConfigResponse {
    /// The transaction ID of the recovery config.
    #[serde(rename = "txid")]
    txid: String,

    /// Unique identifier for this recovery configuration.
    #[serde(rename = "recoveryId")]
    recovery_id: String,

    /// Number of guardians registered.
    #[serde(rename = "guardianCount")]
    #[getter(copy)]
    guardian_count: u8,

    /// Threshold required for recovery.
    #[getter(copy)]
    threshold: u8,

    /// Timelock period in blocks.
    #[serde(rename = "timelockBlocks")]
    #[getter(copy)]
    timelock_blocks: u32,
}

/// Request for initiating account recovery.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct RecoveryRequestRequest {
    /// The new address initiating recovery (from the new device/key).
    pub from: String,

    /// The target address to recover (the lost account).
    #[serde(rename = "targetAddress")]
    pub target_address: String,

    /// The new public key to transfer control to (33 bytes hex-encoded).
    #[serde(rename = "newPubkey")]
    pub new_pubkey: String,

    /// Proof of authorization (signed challenge).
    pub proof: String,
}

/// Response for initiating account recovery.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct RecoveryRequestResponse {
    /// The transaction ID of the recovery request.
    #[serde(rename = "txid")]
    txid: String,

    /// The recovery configuration ID.
    #[serde(rename = "recoveryId")]
    recovery_id: String,

    /// Unique identifier for this recovery request.
    #[serde(rename = "requestId")]
    request_id: String,

    /// Block height when the timelock expires.
    #[serde(rename = "timelockExpiresBlock")]
    #[getter(copy)]
    timelock_expires_block: u32,

    /// Number of guardian approvals needed.
    #[serde(rename = "approvalsNeeded")]
    #[getter(copy)]
    approvals_needed: u8,
}

/// Request for guardian approval of recovery.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct RecoveryApproveRequest {
    /// The guardian's address.
    pub from: String,

    /// The recovery request transaction ID to approve.
    #[serde(rename = "requestId")]
    pub request_id: String,

    /// The encrypted Shamir share for reconstruction.
    #[serde(rename = "encryptedShare")]
    pub encrypted_share: String,
}

/// Response for guardian approval.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct RecoveryApproveResponse {
    /// The transaction ID of the approval.
    #[serde(rename = "txid")]
    txid: String,

    /// Number of approvals received so far.
    #[serde(rename = "approvalsCount")]
    #[getter(copy)]
    approvals_count: u8,

    /// Number of approvals still needed.
    #[serde(rename = "approvalsNeeded")]
    #[getter(copy)]
    approvals_needed: u8,

    /// Whether the threshold has been met.
    #[serde(rename = "thresholdMet")]
    #[getter(copy)]
    threshold_met: bool,
}

/// Request for cancelling a recovery request.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct RecoveryCancelRequest {
    /// The original owner's address.
    pub from: String,

    /// The recovery request transaction ID to cancel.
    #[serde(rename = "requestId")]
    pub request_id: String,
}

/// Response for cancelling a recovery request.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct RecoveryCancelResponse {
    /// The transaction ID of the cancellation.
    #[serde(rename = "txid")]
    txid: String,

    /// The cancelled request ID.
    #[serde(rename = "requestId")]
    request_id: String,

    /// Whether the cancellation was successful.
    #[getter(copy)]
    success: bool,
}

/// Request for getting recovery status.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct RecoveryStatusRequest {
    /// The address to check recovery status for.
    pub address: String,
}

/// Response for recovery status.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct RecoveryStatusResponse {
    /// The address queried.
    address: String,

    /// Whether recovery is configured for this address.
    #[serde(rename = "hasRecovery")]
    #[getter(copy)]
    has_recovery: bool,

    /// The recovery configuration ID (if configured).
    #[serde(rename = "recoveryId", skip_serializing_if = "Option::is_none")]
    recovery_id: Option<String>,

    /// Number of guardians configured.
    #[serde(rename = "guardianCount", skip_serializing_if = "Option::is_none")]
    #[getter(copy)]
    guardian_count: Option<u8>,

    /// Recovery threshold (M of N).
    #[serde(skip_serializing_if = "Option::is_none")]
    #[getter(copy)]
    threshold: Option<u8>,

    /// Timelock period in blocks.
    #[serde(rename = "timelockBlocks", skip_serializing_if = "Option::is_none")]
    #[getter(copy)]
    timelock_blocks: Option<u32>,

    /// Current recovery status.
    status: RecoveryStatus,

    /// Pending request information (if any).
    #[serde(rename = "pendingRequest", skip_serializing_if = "Option::is_none")]
    pending_request: Option<PendingRecoveryInfo>,
}

/// Information about a pending recovery request.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct PendingRecoveryInfo {
    /// The request ID.
    #[serde(rename = "requestId")]
    request_id: String,

    /// Block height when the request was made.
    #[serde(rename = "requestedAtBlock")]
    #[getter(copy)]
    requested_at_block: u32,

    /// Block height when the timelock expires.
    #[serde(rename = "timelockExpiresBlock")]
    #[getter(copy)]
    timelock_expires_block: u32,

    /// Number of approvals received.
    #[serde(rename = "approvalsCount")]
    #[getter(copy)]
    approvals_count: u8,

    /// Number of approvals needed.
    #[serde(rename = "approvalsNeeded")]
    #[getter(copy)]
    approvals_needed: u8,

    /// Addresses of guardians who have approved.
    #[serde(rename = "approvedGuardians")]
    approved_guardians: Vec<String>,
}

/// Request for listing guardians for an address.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct GuardianListRequest {
    /// The address to list guardians for.
    pub address: String,
}

/// Summary of a guardian.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct GuardianSummary {
    /// The guardian's address hash (for privacy).
    #[serde(rename = "addressHash")]
    address_hash: String,

    /// Index in the guardian list (0-based).
    #[getter(copy)]
    index: u8,

    /// Whether this guardian is active (not revoked).
    #[serde(rename = "isActive")]
    #[getter(copy)]
    is_active: bool,

    /// Block height when the guardian was added.
    #[serde(rename = "addedAtBlock")]
    #[getter(copy)]
    added_at_block: u32,
}

/// Response for listing guardians.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct GuardianListResponse {
    /// The address queried.
    address: String,

    /// List of guardians.
    guardians: Vec<GuardianSummary>,

    /// Recovery threshold (M of N).
    #[getter(copy)]
    threshold: u8,

    /// Total number of active guardians.
    #[serde(rename = "activeCount")]
    #[getter(copy)]
    active_count: u8,
}

// ==================== Key Rotation Types ====================

/// Request for initiating a key rotation to migrate identity to a new address.
///
/// Key rotation allows users to migrate their social identity (followers, following,
/// karma) to a new address. This can be used after social recovery or proactively
/// for security reasons.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct KeyRotationRequest {
    /// The current (old) address initiating the rotation.
    #[serde(rename = "oldAddress")]
    pub old_address: String,

    /// The new address to rotate to.
    #[serde(rename = "newAddress")]
    pub new_address: String,

    /// Optional reason for the rotation (for on-chain record).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,

    /// Whether to transfer karma to the new address.
    /// Default: true
    #[serde(rename = "transferKarma", default = "default_true")]
    pub transfer_karma: bool,

    /// Whether followers should auto-follow the new address.
    /// Default: true
    #[serde(rename = "notifyFollowers", default = "default_true")]
    pub notify_followers: bool,
}

impl KeyRotationRequest {
    /// Create a new key rotation request.
    pub fn new(old_address: String, new_address: String) -> Self {
        Self {
            old_address,
            new_address,
            reason: None,
            transfer_karma: true,
            notify_followers: true,
        }
    }

    /// Set the reason for rotation.
    pub fn with_reason(mut self, reason: String) -> Self {
        self.reason = Some(reason);
        self
    }
}

/// Response for a key rotation request.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct KeyRotationResponse {
    /// The transaction ID of the key rotation.
    #[serde(rename = "txid")]
    txid: String,

    /// The old address being rotated from.
    #[serde(rename = "oldAddress")]
    old_address: String,

    /// The new address being rotated to.
    #[serde(rename = "newAddress")]
    new_address: String,

    /// Block height at which the rotation was submitted.
    #[serde(rename = "rotationBlock")]
    #[getter(copy)]
    rotation_block: u32,

    /// Number of followers that will be notified.
    #[serde(rename = "followerCount")]
    #[getter(copy)]
    follower_count: u32,

    /// Amount of karma being transferred.
    #[serde(rename = "karmaTransferred")]
    #[getter(copy)]
    karma_transferred: i64,
}

/// Request for checking key rotation history for an address.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct KeyRotationHistoryRequest {
    /// The address to check rotation history for.
    /// This can be either the current address or any previous address in the chain.
    pub address: String,
}

/// Response for key rotation history.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct KeyRotationHistoryResponse {
    /// The queried address.
    address: String,

    /// The current active address (if different from queried).
    #[serde(rename = "currentAddress")]
    current_address: String,

    /// List of all addresses in this identity's rotation history.
    #[serde(rename = "rotationHistory")]
    rotation_history: Vec<KeyRotationRecord>,

    /// Total number of rotations this identity has undergone.
    #[serde(rename = "totalRotations")]
    #[getter(copy)]
    total_rotations: u32,
}

/// A single key rotation event in the history.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct KeyRotationRecord {
    /// The transaction ID of the rotation.
    #[serde(rename = "txid")]
    txid: String,

    /// The old address.
    #[serde(rename = "oldAddress")]
    old_address: String,

    /// The new address.
    #[serde(rename = "newAddress")]
    new_address: String,

    /// Block height of the rotation.
    #[serde(rename = "rotationBlock")]
    #[getter(copy)]
    rotation_block: u32,

    /// Reason for the rotation (if provided).
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,

    /// Whether this was via social recovery.
    #[serde(rename = "viaRecovery")]
    #[getter(copy)]
    via_recovery: bool,
}

/// Status of a key rotation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum KeyRotationStatus {
    /// Rotation is active and the new address is the current identity.
    Active,
    /// Rotation is pending confirmation.
    Pending,
    /// Rotation was cancelled (by owner or failed).
    Cancelled,
    /// The old address has been migrated (superseded by new address).
    Migrated,
}

impl std::fmt::Display for KeyRotationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Pending => write!(f, "pending"),
            Self::Cancelled => write!(f, "cancelled"),
            Self::Migrated => write!(f, "migrated"),
        }
    }
}

// ==================== Bridge Types ====================

/// Supported bridge platforms for cross-platform identity linking.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BridgePlatform {
    /// Telegram messaging platform.
    Telegram,
    /// Discord chat platform.
    Discord,
    /// Nostr decentralized protocol.
    Nostr,
    /// Mastodon/ActivityPub.
    Mastodon,
    /// X/Twitter (primarily read-only bridging).
    Twitter,
}

impl BridgePlatform {
    /// Returns a human-readable name for this platform.
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Telegram => "Telegram",
            Self::Discord => "Discord",
            Self::Nostr => "Nostr",
            Self::Mastodon => "Mastodon",
            Self::Twitter => "Twitter",
        }
    }

    /// Returns true if this platform supports bidirectional bridging.
    pub const fn is_bidirectional(&self) -> bool {
        !matches!(self, Self::Twitter)
    }
}

impl std::fmt::Display for BridgePlatform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Privacy mode for bridge message relaying.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BridgePrivacyMode {
    /// Full mirror: relay all messages both directions.
    Full,
    /// Selective: only relay explicit /post commands.
    Selective,
    /// Read only: only receive Botcash posts, don't relay from platform.
    ReadOnly,
    /// Private: only relay DMs, no public posts.
    Private,
}

impl std::fmt::Display for BridgePrivacyMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Full => write!(f, "full"),
            Self::Selective => write!(f, "selective"),
            Self::ReadOnly => write!(f, "readonly"),
            Self::Private => write!(f, "private"),
        }
    }
}

/// Status of a bridge identity link.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BridgeLinkStatus {
    /// Link is active and verified.
    Active,
    /// Link is pending verification.
    Pending,
    /// Link has been unlinked.
    Unlinked,
    /// Link verification failed.
    Failed,
    /// Link is temporarily suspended.
    Suspended,
}

impl std::fmt::Display for BridgeLinkStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Pending => write!(f, "pending"),
            Self::Unlinked => write!(f, "unlinked"),
            Self::Failed => write!(f, "failed"),
            Self::Suspended => write!(f, "suspended"),
        }
    }
}

/// Request for linking an external platform identity.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct BridgeLinkRequest {
    /// The Botcash address to link.
    pub from: String,

    /// The platform to link (telegram, discord, nostr, mastodon, twitter).
    pub platform: BridgePlatform,

    /// The platform-specific user identifier.
    #[serde(rename = "platformId")]
    pub platform_id: String,

    /// The signed challenge proving ownership (hex-encoded).
    pub proof: String,

    /// Privacy mode for this link.
    #[serde(rename = "privacyMode", default = "default_bridge_privacy_mode")]
    pub privacy_mode: BridgePrivacyMode,
}

fn default_bridge_privacy_mode() -> BridgePrivacyMode {
    BridgePrivacyMode::Selective
}

/// Response for linking an external platform identity.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct BridgeLinkResponse {
    /// The transaction ID of the link transaction.
    #[serde(rename = "txid")]
    txid: String,

    /// The platform that was linked.
    platform: BridgePlatform,

    /// The platform user ID that was linked.
    #[serde(rename = "platformId")]
    platform_id: String,

    /// The Botcash address that was linked.
    address: String,

    /// Current status of the link.
    status: BridgeLinkStatus,

    /// Block height when the link was created.
    #[serde(rename = "linkedAtBlock")]
    #[getter(copy)]
    linked_at_block: u32,
}

/// Request for unlinking an external platform identity.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct BridgeUnlinkRequest {
    /// The Botcash address that owns the link.
    pub from: String,

    /// The platform to unlink.
    pub platform: BridgePlatform,

    /// The platform-specific user identifier to unlink.
    #[serde(rename = "platformId")]
    pub platform_id: String,
}

/// Response for unlinking an external platform identity.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct BridgeUnlinkResponse {
    /// The transaction ID of the unlink transaction.
    #[serde(rename = "txid")]
    txid: String,

    /// The platform that was unlinked.
    platform: BridgePlatform,

    /// The platform user ID that was unlinked.
    #[serde(rename = "platformId")]
    platform_id: String,

    /// Whether the unlink was successful.
    #[getter(copy)]
    success: bool,
}

/// Request for posting content from an external platform via bridge.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct BridgePostRequest {
    /// The Botcash address to post from.
    pub from: String,

    /// The source platform.
    pub platform: BridgePlatform,

    /// The original post ID on the source platform.
    #[serde(rename = "originalId")]
    pub original_id: String,

    /// The content to post.
    pub content: String,

    /// Whether this is a reply to another post.
    #[serde(rename = "inReplyTo", skip_serializing_if = "Option::is_none")]
    pub in_reply_to: Option<String>,
}

/// Response for posting content from an external platform via bridge.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct BridgePostResponse {
    /// The transaction ID of the post.
    #[serde(rename = "txid")]
    txid: String,

    /// The source platform.
    platform: BridgePlatform,

    /// The original post ID on the source platform.
    #[serde(rename = "originalId")]
    original_id: String,

    /// Block height when the post was created.
    #[serde(rename = "postedAtBlock")]
    #[getter(copy)]
    posted_at_block: u32,
}

/// Request for querying bridge link status.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct BridgeStatusRequest {
    /// The Botcash address to query.
    pub address: String,

    /// Optional platform filter.
    #[serde(default)]
    pub platform: Option<BridgePlatform>,
}

/// Information about a single bridge link.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct BridgeLinkInfo {
    /// The platform.
    platform: BridgePlatform,

    /// The platform user ID.
    #[serde(rename = "platformId")]
    platform_id: String,

    /// Current status of the link.
    status: BridgeLinkStatus,

    /// Privacy mode for this link.
    #[serde(rename = "privacyMode")]
    privacy_mode: BridgePrivacyMode,

    /// Block height when the link was created.
    #[serde(rename = "linkedAtBlock")]
    #[getter(copy)]
    linked_at_block: u32,

    /// Number of messages relayed via this bridge.
    #[serde(rename = "messagesRelayed")]
    #[getter(copy)]
    messages_relayed: u64,

    /// Block height when last message was relayed.
    #[serde(rename = "lastActiveBlock", skip_serializing_if = "Option::is_none")]
    last_active_block: Option<u32>,
}

/// Response for querying bridge link status.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct BridgeStatusResponse {
    /// The Botcash address queried.
    address: String,

    /// List of active bridge links.
    links: Vec<BridgeLinkInfo>,

    /// Total number of active links.
    #[serde(rename = "activeLinksCount")]
    #[getter(copy)]
    active_links_count: u32,
}

/// Request for listing all bridge links (admin/indexer use).
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct BridgeListRequest {
    /// Optional platform filter.
    #[serde(default)]
    pub platform: Option<BridgePlatform>,

    /// Optional status filter.
    #[serde(default)]
    pub status: Option<BridgeLinkStatus>,

    /// Maximum number of results to return.
    #[serde(default = "default_bridge_list_limit")]
    pub limit: u32,

    /// Offset for pagination.
    #[serde(default)]
    pub offset: u32,
}

fn default_bridge_list_limit() -> u32 {
    100
}

/// Summary of a bridge link for listing.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct BridgeLinkSummary {
    /// The Botcash address.
    address: String,

    /// The platform.
    platform: BridgePlatform,

    /// The platform user ID.
    #[serde(rename = "platformId")]
    platform_id: String,

    /// Current status.
    status: BridgeLinkStatus,

    /// Block height when linked.
    #[serde(rename = "linkedAtBlock")]
    #[getter(copy)]
    linked_at_block: u32,
}

/// Response for listing bridge links.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct BridgeListResponse {
    /// List of bridge links matching the query.
    links: Vec<BridgeLinkSummary>,

    /// Total count matching the filter (may be more than returned).
    #[serde(rename = "totalCount")]
    #[getter(copy)]
    total_count: u32,
}

/// Request for getting a challenge to verify bridge ownership.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct BridgeVerifyRequest {
    /// The Botcash address requesting verification.
    pub address: String,

    /// The platform to verify.
    pub platform: BridgePlatform,

    /// The platform user ID to verify.
    #[serde(rename = "platformId")]
    pub platform_id: String,
}

/// Response for getting a bridge verification challenge.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct BridgeVerifyResponse {
    /// The challenge to sign (hex-encoded).
    challenge: String,

    /// Unix timestamp when the challenge expires.
    #[serde(rename = "expiresAt")]
    #[getter(copy)]
    expires_at: i64,

    /// Instructions for how to sign the challenge on this platform.
    instructions: String,
}

/// Maximum length for a platform user ID.
pub const MAX_PLATFORM_ID_LENGTH: usize = 64;

/// Size of the challenge in bridge verification (32 bytes).
pub const BRIDGE_CHALLENGE_SIZE: usize = 32;

/// Challenge expiration time in seconds (10 minutes).
pub const BRIDGE_CHALLENGE_EXPIRY_SECS: i64 = 600;

// ====================================================================================
//                                   MODERATION TYPES
// ====================================================================================

/// Trust level for the web of trust reputation system.
///
/// Users can express explicit trust in other users, building a decentralized
/// reputation system. Trust propagates through the social graph with decay.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TrustLevel {
    /// Negative endorsement - warns others about this user.
    Distrust,

    /// Neutral - removes any previous trust/distrust.
    Neutral,

    /// Positive endorsement - vouches for this user.
    Trusted,
}

impl std::fmt::Display for TrustLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Distrust => write!(f, "distrust"),
            Self::Neutral => write!(f, "neutral"),
            Self::Trusted => write!(f, "trusted"),
        }
    }
}

impl TrustLevel {
    /// Returns the byte value for encoding this trust level.
    pub const fn as_u8(&self) -> u8 {
        match self {
            Self::Distrust => 0,
            Self::Neutral => 1,
            Self::Trusted => 2,
        }
    }
}

/// Report categories for stake-weighted content moderation.
///
/// Different categories have different handling - some trigger immediate
/// filtering while others only affect ranking and reputation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportCategory {
    /// Unsolicited bulk content.
    Spam,

    /// Fraudulent schemes.
    Scam,

    /// Targeted abuse.
    Harassment,

    /// Potentially illegal content (triggers immediate filtering).
    Illegal,

    /// Other (miscellaneous).
    Other,
}

impl std::fmt::Display for ReportCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spam => write!(f, "spam"),
            Self::Scam => write!(f, "scam"),
            Self::Harassment => write!(f, "harassment"),
            Self::Illegal => write!(f, "illegal"),
            Self::Other => write!(f, "other"),
        }
    }
}

impl ReportCategory {
    /// Returns the byte value for encoding this report category.
    pub const fn as_u8(&self) -> u8 {
        match self {
            Self::Spam => 0,
            Self::Scam => 1,
            Self::Harassment => 2,
            Self::Illegal => 3,
            Self::Other => 4,
        }
    }

    /// Returns true if this category requires immediate indexer filtering.
    ///
    /// Some categories are too sensitive to wait for stake resolution.
    pub const fn requires_immediate_filtering(&self) -> bool {
        matches!(self, Self::Illegal)
    }
}

/// Status of a content report.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ReportStatus {
    /// Report is pending review.
    Pending,

    /// Report was validated (stake returned with reward).
    Validated,

    /// Report was rejected (stake forfeited).
    Rejected,

    /// Report expired (no resolution reached).
    Expired,
}

impl std::fmt::Display for ReportStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Validated => write!(f, "validated"),
            Self::Rejected => write!(f, "rejected"),
            Self::Expired => write!(f, "expired"),
        }
    }
}

/// Request for expressing trust in another user.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct TrustRequest {
    /// The address expressing trust.
    pub from: String,

    /// The address being trusted/distrusted.
    pub target: String,

    /// The level of trust.
    pub level: TrustLevel,

    /// Optional reason for this trust assignment.
    #[serde(default)]
    pub reason: Option<String>,
}

/// Response for expressing trust in another user.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct TrustResponse {
    /// The transaction ID of the trust action.
    #[serde(rename = "txid")]
    txid: String,

    /// The target address.
    target: String,

    /// The trust level assigned.
    level: TrustLevel,
}

/// Request for querying trust relationships.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct TrustQueryRequest {
    /// The address to query trust for.
    pub address: String,

    /// Whether to include incoming trust (others trusting this address).
    #[serde(rename = "includeIncoming", default = "default_true")]
    pub include_incoming: bool,

    /// Whether to include outgoing trust (this address trusting others).
    #[serde(rename = "includeOutgoing", default = "default_true")]
    pub include_outgoing: bool,

    /// Maximum number of entries to return.
    #[serde(default = "default_trust_limit")]
    pub limit: u32,
}

fn default_true() -> bool {
    true
}

fn default_trust_limit() -> u32 {
    100
}

/// Summary of a trust relationship.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct TrustSummary {
    /// The address that expressed trust.
    #[serde(rename = "fromAddress")]
    from_address: String,

    /// The address that received trust.
    #[serde(rename = "toAddress")]
    to_address: String,

    /// The trust level.
    level: TrustLevel,

    /// Optional reason for the trust.
    reason: Option<String>,

    /// Block height when trust was expressed.
    #[serde(rename = "blockHeight")]
    #[getter(copy)]
    block_height: u32,

    /// Transaction ID of the trust action.
    #[serde(rename = "txid")]
    txid: String,
}

/// Response for querying trust relationships.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct TrustQueryResponse {
    /// The queried address.
    address: String,

    /// Trust score (incoming trusted count - incoming distrusted count).
    #[serde(rename = "trustScore")]
    #[getter(copy)]
    trust_score: i32,

    /// Number of users trusting this address.
    #[serde(rename = "trustedByCount")]
    #[getter(copy)]
    trusted_by_count: u32,

    /// Number of users distrusting this address.
    #[serde(rename = "distrustedByCount")]
    #[getter(copy)]
    distrusted_by_count: u32,

    /// Trust relationships (both incoming and outgoing based on request).
    relationships: Vec<TrustSummary>,
}

/// Request for reporting content.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct ReportRequest {
    /// The reporter's address.
    pub from: String,

    /// The transaction ID of the content being reported.
    #[serde(rename = "targetTxid")]
    pub target_txid: String,

    /// The category of the report.
    pub category: ReportCategory,

    /// The stake amount in zatoshis (minimum: 1_000_000 = 0.01 BCASH).
    pub stake: u64,

    /// Optional evidence/description for the report.
    #[serde(default)]
    pub evidence: Option<String>,
}

/// Response for reporting content.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct ReportResponse {
    /// The transaction ID of the report.
    #[serde(rename = "txid")]
    txid: String,

    /// The target content's transaction ID.
    #[serde(rename = "targetTxid")]
    target_txid: String,

    /// The report category.
    category: ReportCategory,

    /// The stake amount in zatoshis.
    #[getter(copy)]
    stake: u64,
}

/// Request for querying report status.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct ReportStatusRequest {
    /// The transaction ID of the report to query.
    #[serde(rename = "reportTxid")]
    pub report_txid: String,
}

/// Response for querying report status.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct ReportStatusResponse {
    /// The report transaction ID.
    #[serde(rename = "reportTxid")]
    report_txid: String,

    /// The target content's transaction ID.
    #[serde(rename = "targetTxid")]
    target_txid: String,

    /// The report category.
    category: ReportCategory,

    /// The stake amount in zatoshis.
    #[getter(copy)]
    stake: u64,

    /// Current status of the report.
    status: ReportStatus,

    /// Block height when the report was submitted.
    #[serde(rename = "blockHeight")]
    #[getter(copy)]
    block_height: u32,

    /// Block height when the report was resolved (if resolved).
    #[serde(rename = "resolvedAtHeight")]
    resolved_at_height: Option<u32>,
}

/// Request for listing reports against content.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct ReportListRequest {
    /// Optional content transaction ID to filter reports for.
    #[serde(rename = "targetTxid")]
    pub target_txid: Option<String>,

    /// Optional reporter address to filter by.
    #[serde(rename = "reporterAddress")]
    pub reporter_address: Option<String>,

    /// Optional category filter.
    pub category: Option<ReportCategory>,

    /// Optional status filter.
    pub status: Option<ReportStatus>,

    /// Maximum number of reports to return.
    #[serde(default = "default_report_limit")]
    pub limit: u32,
}

fn default_report_limit() -> u32 {
    50
}

/// Summary of a report for list responses.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct ReportSummary {
    /// The report transaction ID.
    #[serde(rename = "reportTxid")]
    report_txid: String,

    /// The target content's transaction ID.
    #[serde(rename = "targetTxid")]
    target_txid: String,

    /// The reporter's address.
    #[serde(rename = "reporterAddress")]
    reporter_address: String,

    /// The report category.
    category: ReportCategory,

    /// The stake amount in zatoshis.
    #[getter(copy)]
    stake: u64,

    /// Current status of the report.
    status: ReportStatus,

    /// Block height when the report was submitted.
    #[serde(rename = "blockHeight")]
    #[getter(copy)]
    block_height: u32,
}

/// Response for listing reports.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize, Getters, new)]
pub struct ReportListResponse {
    /// List of reports matching the query.
    reports: Vec<ReportSummary>,

    /// Total number of matching reports (may be more than returned).
    #[serde(rename = "totalCount")]
    #[getter(copy)]
    total_count: u32,
}

/// Minimum stake required for a report (0.01 BCASH = 1_000_000 zatoshis).
pub const MIN_REPORT_STAKE: u64 = 1_000_000;

/// Maximum length for trust reason text.
pub const MAX_TRUST_REASON_LENGTH: usize = 200;

/// Maximum length for report evidence text.
pub const MAX_REPORT_EVIDENCE_LENGTH: usize = 300;

/// Maximum trust query limit.
pub const MAX_TRUST_LIMIT: u32 = 1000;

/// Maximum report list limit.
pub const MAX_REPORT_LIMIT: u32 = 1000;

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

    // ==================== Batch Queue Tests ====================

    #[test]
    fn batch_action_post_deserialize() {
        let json = r#"{"type":"post","content":"Hello Botcash!","tags":["test"]}"#;
        let action: BatchAction = serde_json::from_str(json).unwrap();
        match &action {
            BatchAction::Post { content, tags } => {
                assert_eq!(content, "Hello Botcash!");
                assert_eq!(tags, &vec!["test"]);
            }
            _ => panic!("Expected Post action"),
        }
        assert_eq!(action.action_type(), "Post");
    }

    #[test]
    fn batch_action_dm_deserialize() {
        let json = r#"{"type":"dm","to":"bs1recipient","content":"Private message"}"#;
        let action: BatchAction = serde_json::from_str(json).unwrap();
        match &action {
            BatchAction::Dm { to, content } => {
                assert_eq!(to, "bs1recipient");
                assert_eq!(content, "Private message");
            }
            _ => panic!("Expected Dm action"),
        }
        assert_eq!(action.action_type(), "Dm");
    }

    #[test]
    fn batch_action_follow_deserialize() {
        let json = r#"{"type":"follow","target":"bs1target"}"#;
        let action: BatchAction = serde_json::from_str(json).unwrap();
        match &action {
            BatchAction::Follow { target } => {
                assert_eq!(target, "bs1target");
            }
            _ => panic!("Expected Follow action"),
        }
        assert_eq!(action.action_type(), "Follow");
    }

    #[test]
    fn batch_action_unfollow_deserialize() {
        let json = r#"{"type":"unfollow","target":"bs1target"}"#;
        let action: BatchAction = serde_json::from_str(json).unwrap();
        match &action {
            BatchAction::Unfollow { target } => {
                assert_eq!(target, "bs1target");
            }
            _ => panic!("Expected Unfollow action"),
        }
        assert_eq!(action.action_type(), "Unfollow");
    }

    #[test]
    fn batch_action_upvote_deserialize() {
        let json = r#"{"type":"upvote","targetTxid":"abc123"}"#;
        let action: BatchAction = serde_json::from_str(json).unwrap();
        match &action {
            BatchAction::Upvote { target_txid } => {
                assert_eq!(target_txid, "abc123");
            }
            _ => panic!("Expected Upvote action"),
        }
        assert_eq!(action.action_type(), "Upvote");
    }

    #[test]
    fn batch_action_comment_deserialize() {
        let json = r#"{"type":"comment","targetTxid":"abc123","content":"Great post!"}"#;
        let action: BatchAction = serde_json::from_str(json).unwrap();
        match &action {
            BatchAction::Comment {
                target_txid,
                content,
            } => {
                assert_eq!(target_txid, "abc123");
                assert_eq!(content, "Great post!");
            }
            _ => panic!("Expected Comment action"),
        }
        assert_eq!(action.action_type(), "Comment");
    }

    #[test]
    fn batch_action_tip_deserialize() {
        let json = r#"{"type":"tip","targetTxid":"abc123","amount":100000}"#;
        let action: BatchAction = serde_json::from_str(json).unwrap();
        match &action {
            BatchAction::Tip {
                target_txid,
                amount,
            } => {
                assert_eq!(target_txid, "abc123");
                assert_eq!(*amount, 100000);
            }
            _ => panic!("Expected Tip action"),
        }
        assert_eq!(action.action_type(), "Tip");
    }

    #[test]
    fn batch_queue_request_deserialize() {
        let json = r#"{
            "from": "bs1sender",
            "actions": [
                {"type":"post","content":"Hello!","tags":[]},
                {"type":"follow","target":"bs1friend"}
            ],
            "autoSend": true
        }"#;
        let req: BatchQueueRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.from, "bs1sender");
        assert_eq!(req.actions.len(), 2);
        assert!(req.auto_send);
    }

    #[test]
    fn batch_queue_request_defaults() {
        let json = r#"{"from":"bs1sender","actions":[]}"#;
        let req: BatchQueueRequest = serde_json::from_str(json).unwrap();
        assert!(!req.auto_send); // default is false
    }

    #[test]
    fn batch_queue_response_serialize() {
        let resp =
            BatchQueueResponse::new(2, 3, None, vec!["Post".to_string(), "Follow".to_string()]);
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"queued\":2"));
        assert!(json.contains("\"queueSize\":3"));
        assert!(!json.contains("\"txid\"")); // skipped when None
        assert!(json.contains("\"actionTypes\":[\"Post\",\"Follow\"]"));
    }

    #[test]
    fn batch_queue_response_with_txid() {
        let resp = BatchQueueResponse::new(
            5,
            0, // queue was sent
            Some("txid123".to_string()),
            vec!["Post".to_string()],
        );
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"txid\":\"txid123\""));
        assert!(json.contains("\"queueSize\":0"));
    }

    #[test]
    fn batch_send_request_deserialize() {
        let json = r#"{"from":"bs1sender"}"#;
        let req: BatchSendRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.from, "bs1sender");
    }

    #[test]
    fn batch_send_response_serialize() {
        let resp = BatchSendResponse::new(
            "txid456".to_string(),
            3,
            vec![
                "Post".to_string(),
                "Follow".to_string(),
                "Upvote".to_string(),
            ],
            5000,
        );
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"txid\":\"txid456\""));
        assert!(json.contains("\"actionCount\":3"));
        assert!(json.contains("\"feeSaved\":5000"));
    }

    #[test]
    fn batch_status_request_deserialize() {
        let json = r#"{"from":"bs1sender"}"#;
        let req: BatchStatusRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.from, "bs1sender");
    }

    #[test]
    fn batch_status_response_serialize() {
        let resp = BatchStatusResponse::new(2, 5, vec!["Post".to_string(), "Dm".to_string()], 128);
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"queueSize\":2"));
        assert!(json.contains("\"maxSize\":5"));
        assert!(json.contains("\"estimatedSize\":128"));
    }

    #[test]
    fn batch_clear_request_deserialize() {
        let json = r#"{"from":"bs1sender"}"#;
        let req: BatchClearRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.from, "bs1sender");
    }

    #[test]
    fn batch_clear_response_serialize() {
        let resp = BatchClearResponse::new(3, true);
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"cleared\":3"));
        assert!(json.contains("\"success\":true"));
    }

    #[test]
    fn max_batch_queue_size_constant() {
        assert_eq!(MAX_BATCH_QUEUE_SIZE, 5);
    }

    // ==================== Governance Tests ====================

    #[test]
    fn governance_proposal_type_as_u8() {
        assert_eq!(GovernanceProposalType::Other.as_u8(), 0x00);
        assert_eq!(GovernanceProposalType::Parameter.as_u8(), 0x01);
        assert_eq!(GovernanceProposalType::Upgrade.as_u8(), 0x02);
        assert_eq!(GovernanceProposalType::Spending.as_u8(), 0x03);
    }

    #[test]
    fn governance_vote_choice_as_u8() {
        assert_eq!(GovernanceVoteChoice::No.as_u8(), 0x00);
        assert_eq!(GovernanceVoteChoice::Yes.as_u8(), 0x01);
        assert_eq!(GovernanceVoteChoice::Abstain.as_u8(), 0x02);
    }

    #[test]
    fn governance_proposal_request_deserialize() {
        let json = r#"{
            "from": "bs1proposer",
            "proposalType": "parameter",
            "title": "Increase block size",
            "description": "Proposal to increase max block size from 2MB to 4MB",
            "parameters": [{"param": "max_block_size", "value": "4194304"}],
            "deposit": 1000000000
        }"#;
        let req: GovernanceProposalRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.from, "bs1proposer");
        assert_eq!(req.proposal_type, GovernanceProposalType::Parameter);
        assert_eq!(req.title, "Increase block size");
        assert_eq!(req.parameters.len(), 1);
        assert_eq!(req.parameters[0].param, "max_block_size");
        assert_eq!(req.deposit, 1_000_000_000);
    }

    #[test]
    fn governance_proposal_request_defaults() {
        let json = r#"{"from":"bs1proposer","title":"Test","description":"Test description"}"#;
        let req: GovernanceProposalRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.proposal_type, GovernanceProposalType::Other); // default
        assert!(req.parameters.is_empty()); // default
        assert_eq!(req.deposit, 1_000_000_000); // default 10 BCASH
    }

    #[test]
    fn governance_proposal_response_serialize() {
        let resp = GovernanceProposalResponse::new(
            "txid123".to_string(),
            "abcd1234".to_string(),
            1000,
            1000 + 10080, // voting starts after 7 days
            1000 + 10080 + 20160, // voting lasts 14 days
            1_000_000_000,
        );
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"txid\":\"txid123\""));
        assert!(json.contains("\"proposalId\":\"abcd1234\""));
        assert!(json.contains("\"height\":1000"));
        assert!(json.contains("\"votingStartsBlock\":11080"));
        assert!(json.contains("\"votingEndsBlock\":31240"));
        assert!(json.contains("\"deposit\":1000000000"));
    }

    #[test]
    fn governance_vote_request_deserialize() {
        let json = r#"{"from":"bs1voter","proposalId":"abcd1234567890","vote":"yes"}"#;
        let req: GovernanceVoteRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.from, "bs1voter");
        assert_eq!(req.proposal_id, "abcd1234567890");
        assert_eq!(req.vote, GovernanceVoteChoice::Yes);
    }

    #[test]
    fn governance_vote_request_all_choices() {
        // Test all vote choices deserialize correctly
        let choices = [("no", GovernanceVoteChoice::No), ("yes", GovernanceVoteChoice::Yes), ("abstain", GovernanceVoteChoice::Abstain)];
        for (str_choice, enum_choice) in choices {
            let json = format!(r#"{{"from":"bs1voter","proposalId":"abc","vote":"{}"}}"#, str_choice);
            let req: GovernanceVoteRequest = serde_json::from_str(&json).unwrap();
            assert_eq!(req.vote, enum_choice);
        }
    }

    #[test]
    fn governance_vote_response_serialize() {
        let resp = GovernanceVoteResponse::new(
            "txid456".to_string(),
            "proposal123".to_string(),
            GovernanceVoteChoice::Yes,
            150.5,
        );
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"txid\":\"txid456\""));
        assert!(json.contains("\"proposalId\":\"proposal123\""));
        assert!(json.contains("\"vote\":\"yes\""));
        assert!(json.contains("\"votingPower\":150.5"));
    }

    #[test]
    fn governance_proposal_status_request_deserialize() {
        let json = r#"{"proposalId":"abcd1234"}"#;
        let req: GovernanceProposalStatusRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.proposal_id, "abcd1234");
    }

    #[test]
    fn governance_proposal_status_response_serialize() {
        let resp = GovernanceProposalStatusResponse::new(
            "proposal123".to_string(),
            "Test Proposal".to_string(),
            GovernanceProposalType::Parameter,
            "voting".to_string(),
            1000.0, // yes votes
            500.0,  // no votes
            100.0,  // abstain votes
            15.5,   // quorum percent
            20.0,   // quorum required
            66.7,   // approval percent
            66.0,   // approval required
            50000,  // voting ends block
            Some(80000), // execution block
        );
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"proposalId\":\"proposal123\""));
        assert!(json.contains("\"title\":\"Test Proposal\""));
        assert!(json.contains("\"proposalType\":\"parameter\""));
        assert!(json.contains("\"status\":\"voting\""));
        assert!(json.contains("\"yesVotes\":1000.0"));
        assert!(json.contains("\"noVotes\":500.0"));
        assert!(json.contains("\"abstainVotes\":100.0"));
        assert!(json.contains("\"quorumPercent\":15.5"));
        assert!(json.contains("\"approvalPercent\":66.7"));
        assert!(json.contains("\"executionBlock\":80000"));
    }

    #[test]
    fn governance_proposal_status_response_no_execution() {
        let resp = GovernanceProposalStatusResponse::new(
            "proposal123".to_string(),
            "Test".to_string(),
            GovernanceProposalType::Other,
            "rejected".to_string(),
            100.0, 900.0, 0.0,
            10.0, 20.0, 10.0, 66.0,
            50000,
            None, // no execution block for rejected
        );
        let json = serde_json::to_string(&resp).unwrap();
        assert!(!json.contains("executionBlock"));
    }

    #[test]
    fn governance_list_request_deserialize() {
        let json = r#"{"status":"voting","limit":25,"offset":10}"#;
        let req: GovernanceListRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.status, "voting");
        assert_eq!(req.limit, 25);
        assert_eq!(req.offset, 10);
    }

    #[test]
    fn governance_list_request_defaults() {
        let json = r#"{}"#;
        let req: GovernanceListRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.status, "all"); // default
        assert_eq!(req.limit, 50); // default
        assert_eq!(req.offset, 0); // default
    }

    #[test]
    fn governance_proposal_summary_serialize() {
        let summary = GovernanceProposalSummary::new(
            "proposal123".to_string(),
            "Test Proposal".to_string(),
            GovernanceProposalType::Upgrade,
            "voting".to_string(),
            75.5,
            50000,
        );
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("\"proposalId\":\"proposal123\""));
        assert!(json.contains("\"title\":\"Test Proposal\""));
        assert!(json.contains("\"proposalType\":\"upgrade\""));
        assert!(json.contains("\"approvalPercent\":75.5"));
    }

    #[test]
    fn governance_list_response_serialize() {
        let summary = GovernanceProposalSummary::new(
            "p1".to_string(),
            "Test".to_string(),
            GovernanceProposalType::Other,
            "pending".to_string(),
            0.0,
            1000,
        );
        let resp = GovernanceListResponse::new(vec![summary], 1);
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"proposals\":["));
        assert!(json.contains("\"totalCount\":1"));
    }

    // ==================== Channel Tests ====================

    #[test]
    fn channel_open_request_deserialize() {
        let json = r#"{
            "from": "bs1initiator",
            "parties": ["bs1alice", "bs1bob"],
            "deposit": 100000000,
            "timeoutBlocks": 2880
        }"#;
        let req: ChannelOpenRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.from, "bs1initiator");
        assert_eq!(req.parties, vec!["bs1alice", "bs1bob"]);
        assert_eq!(req.deposit, 100_000_000);
        assert_eq!(req.timeout_blocks, 2880);
    }

    #[test]
    fn channel_open_request_defaults() {
        let json = r#"{"from":"bs1sender","parties":["bs1alice"],"deposit":1000000}"#;
        let req: ChannelOpenRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.timeout_blocks, DEFAULT_CHANNEL_TIMEOUT_BLOCKS); // default 1440
    }

    #[test]
    fn channel_open_response_serialize() {
        let resp = ChannelOpenResponse::new(
            "ch123abc".repeat(4), // 32-byte hex
            "txid456".to_string(),
            1000,
            1000 + 1440,
        );
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"channelId\""));
        assert!(json.contains("\"txid\":\"txid456\""));
        assert!(json.contains("\"openedAtBlock\":1000"));
        assert!(json.contains("\"timeoutBlock\":2440"));
    }

    #[test]
    fn channel_close_request_deserialize() {
        let json = r#"{"from":"bs1alice","channelId":"abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234","finalSeq":42}"#;
        let req: ChannelCloseRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.from, "bs1alice");
        assert_eq!(req.channel_id.len(), 64); // 32 bytes hex
        assert_eq!(req.final_seq, 42);
    }

    #[test]
    fn channel_close_response_serialize() {
        let resp = ChannelCloseResponse::new(
            "txid789".to_string(),
            "ch123".to_string(),
            50,
            true,
        );
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"txid\":\"txid789\""));
        assert!(json.contains("\"channelId\":\"ch123\""));
        assert!(json.contains("\"finalSeq\":50"));
        assert!(json.contains("\"cooperative\":true"));
    }

    #[test]
    fn channel_settle_request_deserialize() {
        let json = r#"{
            "from": "bs1alice",
            "channelId": "abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234",
            "finalSeq": 100,
            "messageHash": "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef"
        }"#;
        let req: ChannelSettleRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.from, "bs1alice");
        assert_eq!(req.channel_id.len(), 64);
        assert_eq!(req.final_seq, 100);
        assert_eq!(req.message_hash.len(), 64);
    }

    #[test]
    fn channel_settle_response_serialize() {
        let mut balances = std::collections::HashMap::new();
        balances.insert("bs1alice".to_string(), 50_000_000u64);
        balances.insert("bs1bob".to_string(), 50_000_000u64);

        let resp = ChannelSettleResponse::new(
            "txid000".to_string(),
            "ch456".to_string(),
            200,
            balances,
        );
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"txid\":\"txid000\""));
        assert!(json.contains("\"channelId\":\"ch456\""));
        assert!(json.contains("\"finalSeq\":200"));
        assert!(json.contains("\"finalBalances\""));
    }

    #[test]
    fn channel_state_serialize() {
        let state = ChannelState::Open;
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, "\"open\"");

        let state = ChannelState::Closing;
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, "\"closing\"");

        let state = ChannelState::Settled;
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, "\"settled\"");

        let state = ChannelState::Disputed;
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(json, "\"disputed\"");
    }

    #[test]
    fn channel_state_deserialize() {
        let state: ChannelState = serde_json::from_str("\"open\"").unwrap();
        assert_eq!(state, ChannelState::Open);

        let state: ChannelState = serde_json::from_str("\"closing\"").unwrap();
        assert_eq!(state, ChannelState::Closing);

        let state: ChannelState = serde_json::from_str("\"settled\"").unwrap();
        assert_eq!(state, ChannelState::Settled);

        let state: ChannelState = serde_json::from_str("\"disputed\"").unwrap();
        assert_eq!(state, ChannelState::Disputed);
    }

    #[test]
    fn channel_state_default() {
        let state = ChannelState::default();
        assert_eq!(state, ChannelState::Open);
    }

    #[test]
    fn channel_state_display() {
        assert_eq!(format!("{}", ChannelState::Open), "open");
        assert_eq!(format!("{}", ChannelState::Closing), "closing");
        assert_eq!(format!("{}", ChannelState::Settled), "settled");
        assert_eq!(format!("{}", ChannelState::Disputed), "disputed");
    }

    #[test]
    fn channel_status_request_deserialize() {
        let json = r#"{"channelId":"abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234"}"#;
        let req: ChannelStatusRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.channel_id.len(), 64);
    }

    #[test]
    fn channel_status_response_serialize() {
        let resp = ChannelStatusResponse::new(
            "ch789".to_string(),
            ChannelState::Open,
            vec!["bs1alice".to_string(), "bs1bob".to_string()],
            100_000_000,
            50,
            1000,
            2440,
            Some("hash123".to_string()),
        );
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"channelId\":\"ch789\""));
        assert!(json.contains("\"state\":\"open\""));
        assert!(json.contains("\"parties\":[\"bs1alice\",\"bs1bob\"]"));
        assert!(json.contains("\"deposit\":100000000"));
        assert!(json.contains("\"currentSeq\":50"));
        assert!(json.contains("\"latestMessageHash\":\"hash123\""));
    }

    #[test]
    fn channel_status_response_no_message_hash() {
        let resp = ChannelStatusResponse::new(
            "ch789".to_string(),
            ChannelState::Open,
            vec!["bs1alice".to_string()],
            100_000_000,
            0,
            1000,
            2440,
            None,
        );
        let json = serde_json::to_string(&resp).unwrap();
        // latestMessageHash should be skipped when None
        assert!(!json.contains("latestMessageHash"));
    }

    #[test]
    fn channel_list_request_deserialize() {
        let json = r#"{"address":"bs1alice","state":"open","limit":10}"#;
        let req: ChannelListRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.address, "bs1alice");
        assert_eq!(req.state, Some(ChannelState::Open));
        assert_eq!(req.limit, 10);
    }

    #[test]
    fn channel_list_request_defaults() {
        let json = r#"{"address":"bs1alice"}"#;
        let req: ChannelListRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.state, None);
        assert_eq!(req.limit, 50); // default
    }

    #[test]
    fn channel_summary_serialize() {
        let summary = ChannelSummary::new(
            "ch001".to_string(),
            ChannelState::Open,
            2,
            50_000_000,
            25,
            1000,
        );
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("\"channelId\":\"ch001\""));
        assert!(json.contains("\"state\":\"open\""));
        assert!(json.contains("\"partyCount\":2"));
        assert!(json.contains("\"deposit\":50000000"));
        assert!(json.contains("\"currentSeq\":25"));
    }

    #[test]
    fn channel_list_response_serialize() {
        let summary = ChannelSummary::new(
            "ch001".to_string(),
            ChannelState::Open,
            2,
            100_000_000,
            10,
            500,
        );
        let resp = ChannelListResponse::new(vec![summary], 1);
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"channels\":["));
        assert!(json.contains("\"totalCount\":1"));
    }

    #[test]
    fn channel_constants() {
        assert_eq!(DEFAULT_CHANNEL_TIMEOUT_BLOCKS, 1440);
        assert_eq!(MAX_CHANNEL_PARTIES, 10);
        assert_eq!(MIN_CHANNEL_DEPOSIT, 100_000);
    }

    // ==================== Recovery Tests ====================

    #[test]
    fn recovery_config_request_deserialize() {
        let json = r#"{
            "from": "bs1owner",
            "guardians": ["bs1guardian1", "bs1guardian2", "bs1guardian3"],
            "threshold": 2,
            "timelockBlocks": 10080
        }"#;
        let req: RecoveryConfigRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.from, "bs1owner");
        assert_eq!(req.guardians.len(), 3);
        assert_eq!(req.threshold, 2);
        assert_eq!(req.timelock_blocks, 10080);
    }

    #[test]
    fn recovery_config_request_defaults() {
        let json = r#"{"from":"bs1owner","guardians":["bs1g1","bs1g2","bs1g3"],"threshold":2}"#;
        let req: RecoveryConfigRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.timelock_blocks, DEFAULT_RECOVERY_TIMELOCK_BLOCKS); // default 10080
    }

    #[test]
    fn recovery_config_response_serialize() {
        let resp = RecoveryConfigResponse::new(
            "txid123".to_string(),
            "recovery_id_456".to_string(),
            3,
            2,
            10080,
        );
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"txid\":\"txid123\""));
        assert!(json.contains("\"recoveryId\":\"recovery_id_456\""));
        assert!(json.contains("\"guardianCount\":3"));
        assert!(json.contains("\"threshold\":2"));
        assert!(json.contains("\"timelockBlocks\":10080"));
    }

    #[test]
    fn recovery_request_request_deserialize() {
        let json = r#"{
            "from": "bs1newdevice",
            "targetAddress": "bs1oldaddress",
            "newPubkey": "02abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890ab",
            "proof": "signed_challenge"
        }"#;
        let req: RecoveryRequestRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.from, "bs1newdevice");
        assert_eq!(req.target_address, "bs1oldaddress");
        assert_eq!(req.new_pubkey.len(), 66); // 33 bytes hex
        assert_eq!(req.proof, "signed_challenge");
    }

    #[test]
    fn recovery_request_response_serialize() {
        let resp = RecoveryRequestResponse::new(
            "txid789".to_string(),
            "recovery_id".to_string(),
            "request_id".to_string(),
            20160, // timelock expires in ~14 days
            2,
        );
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"txid\":\"txid789\""));
        assert!(json.contains("\"recoveryId\":\"recovery_id\""));
        assert!(json.contains("\"requestId\":\"request_id\""));
        assert!(json.contains("\"timelockExpiresBlock\":20160"));
        assert!(json.contains("\"approvalsNeeded\":2"));
    }

    #[test]
    fn recovery_approve_request_deserialize() {
        let json = r#"{
            "from": "bs1guardian1",
            "requestId": "request_abc123",
            "encryptedShare": "encrypted_shamir_share_hex"
        }"#;
        let req: RecoveryApproveRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.from, "bs1guardian1");
        assert_eq!(req.request_id, "request_abc123");
        assert_eq!(req.encrypted_share, "encrypted_shamir_share_hex");
    }

    #[test]
    fn recovery_approve_response_serialize() {
        let resp = RecoveryApproveResponse::new(
            "txid_approve".to_string(),
            2, // approvals count
            1, // approvals needed
            true, // threshold met
        );
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"txid\":\"txid_approve\""));
        assert!(json.contains("\"approvalsCount\":2"));
        assert!(json.contains("\"approvalsNeeded\":1"));
        assert!(json.contains("\"thresholdMet\":true"));
    }

    #[test]
    fn recovery_cancel_request_deserialize() {
        let json = r#"{"from":"bs1owner","requestId":"request_to_cancel"}"#;
        let req: RecoveryCancelRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.from, "bs1owner");
        assert_eq!(req.request_id, "request_to_cancel");
    }

    #[test]
    fn recovery_cancel_response_serialize() {
        let resp = RecoveryCancelResponse::new(
            "txid_cancel".to_string(),
            "request_cancelled".to_string(),
            true,
        );
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"txid\":\"txid_cancel\""));
        assert!(json.contains("\"requestId\":\"request_cancelled\""));
        assert!(json.contains("\"success\":true"));
    }

    #[test]
    fn recovery_status_request_deserialize() {
        let json = r#"{"address":"bs1myaddress"}"#;
        let req: RecoveryStatusRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.address, "bs1myaddress");
    }

    #[test]
    fn recovery_status_response_with_pending_request() {
        let pending = PendingRecoveryInfo::new(
            "request123".to_string(),
            1000,
            11080,
            1,
            2,
            vec!["bs1guardian1".to_string()],
        );
        let resp = RecoveryStatusResponse::new(
            "bs1myaddress".to_string(),
            true,
            Some("recovery_id".to_string()),
            Some(3),
            Some(2),
            Some(10080),
            RecoveryStatus::Pending,
            Some(pending),
        );
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"address\":\"bs1myaddress\""));
        assert!(json.contains("\"hasRecovery\":true"));
        assert!(json.contains("\"recoveryId\":\"recovery_id\""));
        assert!(json.contains("\"guardianCount\":3"));
        assert!(json.contains("\"threshold\":2"));
        assert!(json.contains("\"status\":\"pending\""));
        assert!(json.contains("\"pendingRequest\""));
        assert!(json.contains("\"requestId\":\"request123\""));
    }

    #[test]
    fn recovery_status_response_no_recovery() {
        let resp = RecoveryStatusResponse::new(
            "bs1newaddress".to_string(),
            false,
            None,
            None,
            None,
            None,
            RecoveryStatus::Active, // Would be Active even for "no recovery configured"
            None,
        );
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"hasRecovery\":false"));
        assert!(!json.contains("recoveryId")); // skip_serializing_if = None
        assert!(!json.contains("pendingRequest"));
    }

    #[test]
    fn recovery_status_enum_values() {
        let statuses = [
            (RecoveryStatus::Active, "active"),
            (RecoveryStatus::Pending, "pending"),
            (RecoveryStatus::Approved, "approved"),
            (RecoveryStatus::Timelocked, "timelocked"),
            (RecoveryStatus::Executed, "executed"),
            (RecoveryStatus::Cancelled, "cancelled"),
            (RecoveryStatus::Expired, "expired"),
        ];
        for (status, expected) in statuses {
            assert_eq!(format!("{}", status), expected);
        }
    }

    #[test]
    fn guardian_list_request_deserialize() {
        let json = r#"{"address":"bs1protected"}"#;
        let req: GuardianListRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.address, "bs1protected");
    }

    #[test]
    fn guardian_summary_serialize() {
        let summary = GuardianSummary::new(
            "sha256_hash_of_address".to_string(),
            0,
            true,
            500,
        );
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("\"addressHash\":\"sha256_hash_of_address\""));
        assert!(json.contains("\"index\":0"));
        assert!(json.contains("\"isActive\":true"));
        assert!(json.contains("\"addedAtBlock\":500"));
    }

    #[test]
    fn guardian_list_response_serialize() {
        let g1 = GuardianSummary::new("hash1".to_string(), 0, true, 100);
        let g2 = GuardianSummary::new("hash2".to_string(), 1, true, 100);
        let g3 = GuardianSummary::new("hash3".to_string(), 2, false, 100); // revoked

        let resp = GuardianListResponse::new(
            "bs1protected".to_string(),
            vec![g1, g2, g3],
            2,
            2, // 2 active out of 3
        );
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"address\":\"bs1protected\""));
        assert!(json.contains("\"guardians\":["));
        assert!(json.contains("\"threshold\":2"));
        assert!(json.contains("\"activeCount\":2"));
    }

    #[test]
    fn recovery_constants() {
        assert_eq!(DEFAULT_RECOVERY_TIMELOCK_BLOCKS, 10080);
        assert_eq!(MIN_RECOVERY_GUARDIANS, 1);
        assert_eq!(MAX_RECOVERY_GUARDIANS, 15);
        assert_eq!(DEFAULT_RECOVERY_THRESHOLD, 3);
    }

    #[test]
    fn pending_recovery_info_serialize() {
        let info = PendingRecoveryInfo::new(
            "req123".to_string(),
            1000,
            11080,
            2,
            3,
            vec!["g1".to_string(), "g2".to_string()],
        );
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"requestId\":\"req123\""));
        assert!(json.contains("\"requestedAtBlock\":1000"));
        assert!(json.contains("\"timelockExpiresBlock\":11080"));
        assert!(json.contains("\"approvalsCount\":2"));
        assert!(json.contains("\"approvalsNeeded\":3"));
        assert!(json.contains("\"approvedGuardians\":[\"g1\",\"g2\"]"));
    }

    // ==================== Key Rotation Types Tests ====================

    #[test]
    fn key_rotation_request_deserialize() {
        let json = r#"{
            "oldAddress": "bs1oldaddress...",
            "newAddress": "bs1newaddress...",
            "reason": "Device compromised",
            "transferKarma": true,
            "notifyFollowers": true
        }"#;
        let req: KeyRotationRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.old_address, "bs1oldaddress...");
        assert_eq!(req.new_address, "bs1newaddress...");
        assert_eq!(req.reason, Some("Device compromised".to_string()));
        assert!(req.transfer_karma);
        assert!(req.notify_followers);
    }

    #[test]
    fn key_rotation_request_defaults() {
        let json = r#"{
            "oldAddress": "bs1old...",
            "newAddress": "bs1new..."
        }"#;
        let req: KeyRotationRequest = serde_json::from_str(json).unwrap();
        assert!(req.transfer_karma); // default true
        assert!(req.notify_followers); // default true
        assert!(req.reason.is_none());
    }

    #[test]
    fn key_rotation_request_builder() {
        let req = KeyRotationRequest::new(
            "bs1old...".to_string(),
            "bs1new...".to_string(),
        )
        .with_reason("Routine rotation".to_string());

        assert_eq!(req.old_address, "bs1old...");
        assert_eq!(req.new_address, "bs1new...");
        assert_eq!(req.reason, Some("Routine rotation".to_string()));
    }

    #[test]
    fn key_rotation_response_serialize() {
        let resp = KeyRotationResponse::new(
            "txid123".to_string(),
            "bs1oldaddr...".to_string(),
            "bs1newaddr...".to_string(),
            100000,
            150,
            2500,
        );
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"txid\":\"txid123\""));
        assert!(json.contains("\"oldAddress\":\"bs1oldaddr...\""));
        assert!(json.contains("\"newAddress\":\"bs1newaddr...\""));
        assert!(json.contains("\"rotationBlock\":100000"));
        assert!(json.contains("\"followerCount\":150"));
        assert!(json.contains("\"karmaTransferred\":2500"));
    }

    #[test]
    fn key_rotation_history_request_deserialize() {
        let json = r#"{"address": "bs1someaddress..."}"#;
        let req: KeyRotationHistoryRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.address, "bs1someaddress...");
    }

    #[test]
    fn key_rotation_history_response_serialize() {
        let record = KeyRotationRecord::new(
            "tx001".to_string(),
            "bs1old1...".to_string(),
            "bs1old2...".to_string(),
            50000,
            Some("Key compromised".to_string()),
            false,
        );
        let resp = KeyRotationHistoryResponse::new(
            "bs1old1...".to_string(),
            "bs1current...".to_string(),
            vec![record],
            1,
        );
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"currentAddress\":\"bs1current...\""));
        assert!(json.contains("\"totalRotations\":1"));
        assert!(json.contains("\"rotationHistory\""));
    }

    #[test]
    fn key_rotation_record_via_recovery() {
        let record = KeyRotationRecord::new(
            "tx002".to_string(),
            "bs1old...".to_string(),
            "bs1new...".to_string(),
            75000,
            None,
            true, // via recovery
        );
        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("\"viaRecovery\":true"));
        // Reason should be omitted when None
        assert!(!json.contains("\"reason\""));
    }

    #[test]
    fn key_rotation_status_serialize() {
        assert_eq!(serde_json::to_string(&KeyRotationStatus::Active).unwrap(), "\"active\"");
        assert_eq!(serde_json::to_string(&KeyRotationStatus::Pending).unwrap(), "\"pending\"");
        assert_eq!(serde_json::to_string(&KeyRotationStatus::Cancelled).unwrap(), "\"cancelled\"");
        assert_eq!(serde_json::to_string(&KeyRotationStatus::Migrated).unwrap(), "\"migrated\"");
    }

    #[test]
    fn key_rotation_status_deserialize() {
        assert_eq!(
            serde_json::from_str::<KeyRotationStatus>("\"active\"").unwrap(),
            KeyRotationStatus::Active
        );
        assert_eq!(
            serde_json::from_str::<KeyRotationStatus>("\"pending\"").unwrap(),
            KeyRotationStatus::Pending
        );
        assert_eq!(
            serde_json::from_str::<KeyRotationStatus>("\"cancelled\"").unwrap(),
            KeyRotationStatus::Cancelled
        );
        assert_eq!(
            serde_json::from_str::<KeyRotationStatus>("\"migrated\"").unwrap(),
            KeyRotationStatus::Migrated
        );
    }

    #[test]
    fn key_rotation_status_display() {
        assert_eq!(format!("{}", KeyRotationStatus::Active), "active");
        assert_eq!(format!("{}", KeyRotationStatus::Pending), "pending");
        assert_eq!(format!("{}", KeyRotationStatus::Cancelled), "cancelled");
        assert_eq!(format!("{}", KeyRotationStatus::Migrated), "migrated");
    }

    // ==================== Bridge Types Tests ====================

    #[test]
    fn bridge_platform_serialize() {
        assert_eq!(serde_json::to_string(&BridgePlatform::Telegram).unwrap(), "\"telegram\"");
        assert_eq!(serde_json::to_string(&BridgePlatform::Discord).unwrap(), "\"discord\"");
        assert_eq!(serde_json::to_string(&BridgePlatform::Nostr).unwrap(), "\"nostr\"");
        assert_eq!(serde_json::to_string(&BridgePlatform::Mastodon).unwrap(), "\"mastodon\"");
        assert_eq!(serde_json::to_string(&BridgePlatform::Twitter).unwrap(), "\"twitter\"");
    }

    #[test]
    fn bridge_platform_deserialize() {
        assert_eq!(
            serde_json::from_str::<BridgePlatform>("\"telegram\"").unwrap(),
            BridgePlatform::Telegram
        );
        assert_eq!(
            serde_json::from_str::<BridgePlatform>("\"discord\"").unwrap(),
            BridgePlatform::Discord
        );
        assert_eq!(
            serde_json::from_str::<BridgePlatform>("\"nostr\"").unwrap(),
            BridgePlatform::Nostr
        );
        assert_eq!(
            serde_json::from_str::<BridgePlatform>("\"mastodon\"").unwrap(),
            BridgePlatform::Mastodon
        );
        assert_eq!(
            serde_json::from_str::<BridgePlatform>("\"twitter\"").unwrap(),
            BridgePlatform::Twitter
        );
    }

    #[test]
    fn bridge_platform_bidirectional() {
        assert!(BridgePlatform::Telegram.is_bidirectional());
        assert!(BridgePlatform::Discord.is_bidirectional());
        assert!(BridgePlatform::Nostr.is_bidirectional());
        assert!(BridgePlatform::Mastodon.is_bidirectional());
        assert!(!BridgePlatform::Twitter.is_bidirectional());
    }

    #[test]
    fn bridge_platform_display() {
        assert_eq!(format!("{}", BridgePlatform::Telegram), "Telegram");
        assert_eq!(format!("{}", BridgePlatform::Discord), "Discord");
        assert_eq!(format!("{}", BridgePlatform::Nostr), "Nostr");
        assert_eq!(format!("{}", BridgePlatform::Mastodon), "Mastodon");
        assert_eq!(format!("{}", BridgePlatform::Twitter), "Twitter");
    }

    #[test]
    fn bridge_privacy_mode_serialize() {
        assert_eq!(serde_json::to_string(&BridgePrivacyMode::Full).unwrap(), "\"full\"");
        assert_eq!(serde_json::to_string(&BridgePrivacyMode::Selective).unwrap(), "\"selective\"");
        assert_eq!(serde_json::to_string(&BridgePrivacyMode::ReadOnly).unwrap(), "\"readonly\"");
        assert_eq!(serde_json::to_string(&BridgePrivacyMode::Private).unwrap(), "\"private\"");
    }

    #[test]
    fn bridge_link_status_serialize() {
        assert_eq!(serde_json::to_string(&BridgeLinkStatus::Active).unwrap(), "\"active\"");
        assert_eq!(serde_json::to_string(&BridgeLinkStatus::Pending).unwrap(), "\"pending\"");
        assert_eq!(serde_json::to_string(&BridgeLinkStatus::Unlinked).unwrap(), "\"unlinked\"");
        assert_eq!(serde_json::to_string(&BridgeLinkStatus::Failed).unwrap(), "\"failed\"");
        assert_eq!(serde_json::to_string(&BridgeLinkStatus::Suspended).unwrap(), "\"suspended\"");
    }

    #[test]
    fn bridge_link_request_deserialize() {
        let json = r#"{
            "from": "bs1test",
            "platform": "telegram",
            "platformId": "123456789",
            "proof": "abcd1234",
            "privacyMode": "selective"
        }"#;
        let req: BridgeLinkRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.from, "bs1test");
        assert_eq!(req.platform, BridgePlatform::Telegram);
        assert_eq!(req.platform_id, "123456789");
        assert_eq!(req.proof, "abcd1234");
        assert_eq!(req.privacy_mode, BridgePrivacyMode::Selective);
    }

    #[test]
    fn bridge_link_request_default_privacy_mode() {
        let json = r#"{
            "from": "bs1test",
            "platform": "discord",
            "platformId": "987654321",
            "proof": "efgh5678"
        }"#;
        let req: BridgeLinkRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.privacy_mode, BridgePrivacyMode::Selective);
    }

    #[test]
    fn bridge_link_response_serialize() {
        let resp = BridgeLinkResponse::new(
            "tx123".to_string(),
            BridgePlatform::Nostr,
            "npub1abc".to_string(),
            "bs1test".to_string(),
            BridgeLinkStatus::Active,
            1000,
        );
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"txid\":\"tx123\""));
        assert!(json.contains("\"platform\":\"nostr\""));
        assert!(json.contains("\"platformId\":\"npub1abc\""));
        assert!(json.contains("\"address\":\"bs1test\""));
        assert!(json.contains("\"status\":\"active\""));
        assert!(json.contains("\"linkedAtBlock\":1000"));
    }

    #[test]
    fn bridge_unlink_request_deserialize() {
        let json = r#"{
            "from": "bs1test",
            "platform": "mastodon",
            "platformId": "@alice@mastodon.social"
        }"#;
        let req: BridgeUnlinkRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.from, "bs1test");
        assert_eq!(req.platform, BridgePlatform::Mastodon);
        assert_eq!(req.platform_id, "@alice@mastodon.social");
    }

    #[test]
    fn bridge_unlink_response_serialize() {
        let resp = BridgeUnlinkResponse::new(
            "tx456".to_string(),
            BridgePlatform::Discord,
            "123456789".to_string(),
            true,
        );
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"txid\":\"tx456\""));
        assert!(json.contains("\"platform\":\"discord\""));
        assert!(json.contains("\"platformId\":\"123456789\""));
        assert!(json.contains("\"success\":true"));
    }

    #[test]
    fn bridge_post_request_deserialize() {
        let json = r#"{
            "from": "bs1test",
            "platform": "twitter",
            "originalId": "1234567890123456789",
            "content": "Hello from Twitter!"
        }"#;
        let req: BridgePostRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.from, "bs1test");
        assert_eq!(req.platform, BridgePlatform::Twitter);
        assert_eq!(req.original_id, "1234567890123456789");
        assert_eq!(req.content, "Hello from Twitter!");
        assert!(req.in_reply_to.is_none());
    }

    #[test]
    fn bridge_post_request_with_reply() {
        let json = r#"{
            "from": "bs1test",
            "platform": "nostr",
            "originalId": "note1abc",
            "content": "This is a reply!",
            "inReplyTo": "tx789"
        }"#;
        let req: BridgePostRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.in_reply_to, Some("tx789".to_string()));
    }

    #[test]
    fn bridge_post_response_serialize() {
        let resp = BridgePostResponse::new(
            "tx789".to_string(),
            BridgePlatform::Telegram,
            "msg123".to_string(),
            2000,
        );
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"txid\":\"tx789\""));
        assert!(json.contains("\"platform\":\"telegram\""));
        assert!(json.contains("\"originalId\":\"msg123\""));
        assert!(json.contains("\"postedAtBlock\":2000"));
    }

    #[test]
    fn bridge_status_request_deserialize() {
        let json = r#"{
            "address": "bs1test",
            "platform": "discord"
        }"#;
        let req: BridgeStatusRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.address, "bs1test");
        assert_eq!(req.platform, Some(BridgePlatform::Discord));
    }

    #[test]
    fn bridge_status_request_no_platform() {
        let json = r#"{"address": "bs1test"}"#;
        let req: BridgeStatusRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.address, "bs1test");
        assert!(req.platform.is_none());
    }

    #[test]
    fn bridge_link_info_serialize() {
        let info = BridgeLinkInfo::new(
            BridgePlatform::Nostr,
            "npub1test".to_string(),
            BridgeLinkStatus::Active,
            BridgePrivacyMode::Full,
            1000,
            50,
            Some(2000),
        );
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"platform\":\"nostr\""));
        assert!(json.contains("\"platformId\":\"npub1test\""));
        assert!(json.contains("\"status\":\"active\""));
        assert!(json.contains("\"privacyMode\":\"full\""));
        assert!(json.contains("\"linkedAtBlock\":1000"));
        assert!(json.contains("\"messagesRelayed\":50"));
        assert!(json.contains("\"lastActiveBlock\":2000"));
    }

    #[test]
    fn bridge_status_response_serialize() {
        let info = BridgeLinkInfo::new(
            BridgePlatform::Telegram,
            "12345".to_string(),
            BridgeLinkStatus::Active,
            BridgePrivacyMode::Selective,
            500,
            10,
            None,
        );
        let resp = BridgeStatusResponse::new(
            "bs1test".to_string(),
            vec![info],
            1,
        );
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"address\":\"bs1test\""));
        assert!(json.contains("\"links\":["));
        assert!(json.contains("\"activeLinksCount\":1"));
    }

    #[test]
    fn bridge_list_request_deserialize() {
        let json = r#"{
            "platform": "mastodon",
            "status": "active",
            "limit": 50,
            "offset": 10
        }"#;
        let req: BridgeListRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.platform, Some(BridgePlatform::Mastodon));
        assert_eq!(req.status, Some(BridgeLinkStatus::Active));
        assert_eq!(req.limit, 50);
        assert_eq!(req.offset, 10);
    }

    #[test]
    fn bridge_list_request_defaults() {
        let json = r#"{}"#;
        let req: BridgeListRequest = serde_json::from_str(json).unwrap();
        assert!(req.platform.is_none());
        assert!(req.status.is_none());
        assert_eq!(req.limit, 100); // default
        assert_eq!(req.offset, 0);
    }

    #[test]
    fn bridge_link_summary_serialize() {
        let summary = BridgeLinkSummary::new(
            "bs1test".to_string(),
            BridgePlatform::Discord,
            "123456789".to_string(),
            BridgeLinkStatus::Active,
            1500,
        );
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("\"address\":\"bs1test\""));
        assert!(json.contains("\"platform\":\"discord\""));
        assert!(json.contains("\"platformId\":\"123456789\""));
        assert!(json.contains("\"status\":\"active\""));
        assert!(json.contains("\"linkedAtBlock\":1500"));
    }

    #[test]
    fn bridge_list_response_serialize() {
        let summary = BridgeLinkSummary::new(
            "bs1user".to_string(),
            BridgePlatform::Telegram,
            "987654".to_string(),
            BridgeLinkStatus::Active,
            2000,
        );
        let resp = BridgeListResponse::new(vec![summary], 1);
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"links\":["));
        assert!(json.contains("\"totalCount\":1"));
    }

    #[test]
    fn bridge_verify_request_deserialize() {
        let json = r#"{
            "address": "bs1test",
            "platform": "nostr",
            "platformId": "npub1abc"
        }"#;
        let req: BridgeVerifyRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.address, "bs1test");
        assert_eq!(req.platform, BridgePlatform::Nostr);
        assert_eq!(req.platform_id, "npub1abc");
    }

    #[test]
    fn bridge_verify_response_serialize() {
        let resp = BridgeVerifyResponse::new(
            "0123456789abcdef".to_string(),
            1700000000,
            "Sign this challenge with your Nostr key".to_string(),
        );
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"challenge\":\"0123456789abcdef\""));
        assert!(json.contains("\"expiresAt\":1700000000"));
        assert!(json.contains("\"instructions\":\"Sign this challenge"));
    }

    #[test]
    fn bridge_constants() {
        assert_eq!(MAX_PLATFORM_ID_LENGTH, 64);
        assert_eq!(BRIDGE_CHALLENGE_SIZE, 32);
        assert_eq!(BRIDGE_CHALLENGE_EXPIRY_SECS, 600);
    }

    // ====================================================================================
    //                               MODERATION TYPE TESTS
    // ====================================================================================

    #[test]
    fn trust_level_serialize() {
        assert_eq!(serde_json::to_string(&TrustLevel::Distrust).unwrap(), "\"distrust\"");
        assert_eq!(serde_json::to_string(&TrustLevel::Neutral).unwrap(), "\"neutral\"");
        assert_eq!(serde_json::to_string(&TrustLevel::Trusted).unwrap(), "\"trusted\"");
    }

    #[test]
    fn trust_level_deserialize() {
        let distrust: TrustLevel = serde_json::from_str("\"distrust\"").unwrap();
        let neutral: TrustLevel = serde_json::from_str("\"neutral\"").unwrap();
        let trusted: TrustLevel = serde_json::from_str("\"trusted\"").unwrap();

        assert_eq!(distrust, TrustLevel::Distrust);
        assert_eq!(neutral, TrustLevel::Neutral);
        assert_eq!(trusted, TrustLevel::Trusted);
    }

    #[test]
    fn trust_level_display() {
        assert_eq!(format!("{}", TrustLevel::Distrust), "distrust");
        assert_eq!(format!("{}", TrustLevel::Neutral), "neutral");
        assert_eq!(format!("{}", TrustLevel::Trusted), "trusted");
    }

    #[test]
    fn trust_level_as_u8() {
        assert_eq!(TrustLevel::Distrust.as_u8(), 0);
        assert_eq!(TrustLevel::Neutral.as_u8(), 1);
        assert_eq!(TrustLevel::Trusted.as_u8(), 2);
    }

    #[test]
    fn report_category_serialize() {
        assert_eq!(serde_json::to_string(&ReportCategory::Spam).unwrap(), "\"spam\"");
        assert_eq!(serde_json::to_string(&ReportCategory::Scam).unwrap(), "\"scam\"");
        assert_eq!(serde_json::to_string(&ReportCategory::Harassment).unwrap(), "\"harassment\"");
        assert_eq!(serde_json::to_string(&ReportCategory::Illegal).unwrap(), "\"illegal\"");
        assert_eq!(serde_json::to_string(&ReportCategory::Other).unwrap(), "\"other\"");
    }

    #[test]
    fn report_category_deserialize() {
        let spam: ReportCategory = serde_json::from_str("\"spam\"").unwrap();
        let scam: ReportCategory = serde_json::from_str("\"scam\"").unwrap();
        let harassment: ReportCategory = serde_json::from_str("\"harassment\"").unwrap();
        let illegal: ReportCategory = serde_json::from_str("\"illegal\"").unwrap();
        let other: ReportCategory = serde_json::from_str("\"other\"").unwrap();

        assert_eq!(spam, ReportCategory::Spam);
        assert_eq!(scam, ReportCategory::Scam);
        assert_eq!(harassment, ReportCategory::Harassment);
        assert_eq!(illegal, ReportCategory::Illegal);
        assert_eq!(other, ReportCategory::Other);
    }

    #[test]
    fn report_category_as_u8() {
        assert_eq!(ReportCategory::Spam.as_u8(), 0);
        assert_eq!(ReportCategory::Scam.as_u8(), 1);
        assert_eq!(ReportCategory::Harassment.as_u8(), 2);
        assert_eq!(ReportCategory::Illegal.as_u8(), 3);
        assert_eq!(ReportCategory::Other.as_u8(), 4);
    }

    #[test]
    fn report_category_immediate_filtering() {
        assert!(ReportCategory::Illegal.requires_immediate_filtering());
        assert!(!ReportCategory::Spam.requires_immediate_filtering());
        assert!(!ReportCategory::Scam.requires_immediate_filtering());
        assert!(!ReportCategory::Harassment.requires_immediate_filtering());
        assert!(!ReportCategory::Other.requires_immediate_filtering());
    }

    #[test]
    fn report_status_serialize() {
        assert_eq!(serde_json::to_string(&ReportStatus::Pending).unwrap(), "\"pending\"");
        assert_eq!(serde_json::to_string(&ReportStatus::Validated).unwrap(), "\"validated\"");
        assert_eq!(serde_json::to_string(&ReportStatus::Rejected).unwrap(), "\"rejected\"");
        assert_eq!(serde_json::to_string(&ReportStatus::Expired).unwrap(), "\"expired\"");
    }

    #[test]
    fn report_status_deserialize() {
        let pending: ReportStatus = serde_json::from_str("\"pending\"").unwrap();
        let validated: ReportStatus = serde_json::from_str("\"validated\"").unwrap();
        let rejected: ReportStatus = serde_json::from_str("\"rejected\"").unwrap();
        let expired: ReportStatus = serde_json::from_str("\"expired\"").unwrap();

        assert_eq!(pending, ReportStatus::Pending);
        assert_eq!(validated, ReportStatus::Validated);
        assert_eq!(rejected, ReportStatus::Rejected);
        assert_eq!(expired, ReportStatus::Expired);
    }

    #[test]
    fn trust_request_deserialize() {
        let json = r#"{"from":"bs1alice","target":"bs1bob","level":"trusted","reason":"Great contributor"}"#;
        let req: TrustRequest = serde_json::from_str(json).unwrap();

        assert_eq!(req.from, "bs1alice");
        assert_eq!(req.target, "bs1bob");
        assert_eq!(req.level, TrustLevel::Trusted);
        assert_eq!(req.reason, Some("Great contributor".to_string()));
    }

    #[test]
    fn trust_request_deserialize_no_reason() {
        let json = r#"{"from":"bs1alice","target":"bs1bob","level":"distrust"}"#;
        let req: TrustRequest = serde_json::from_str(json).unwrap();

        assert_eq!(req.from, "bs1alice");
        assert_eq!(req.target, "bs1bob");
        assert_eq!(req.level, TrustLevel::Distrust);
        assert_eq!(req.reason, None);
    }

    #[test]
    fn trust_response_serialize() {
        let resp = TrustResponse::new(
            "abc123".to_string(),
            "bs1bob".to_string(),
            TrustLevel::Trusted,
        );
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"txid\":\"abc123\""));
        assert!(json.contains("\"target\":\"bs1bob\""));
        assert!(json.contains("\"level\":\"trusted\""));
    }

    #[test]
    fn trust_query_request_deserialize() {
        let json = r#"{"address":"bs1alice","includeIncoming":true,"includeOutgoing":false,"limit":50}"#;
        let req: TrustQueryRequest = serde_json::from_str(json).unwrap();

        assert_eq!(req.address, "bs1alice");
        assert!(req.include_incoming);
        assert!(!req.include_outgoing);
        assert_eq!(req.limit, 50);
    }

    #[test]
    fn trust_query_request_defaults() {
        let json = r#"{"address":"bs1alice"}"#;
        let req: TrustQueryRequest = serde_json::from_str(json).unwrap();

        assert_eq!(req.address, "bs1alice");
        assert!(req.include_incoming); // default true
        assert!(req.include_outgoing); // default true
        assert_eq!(req.limit, 100); // default
    }

    #[test]
    fn trust_summary_serialize() {
        let summary = TrustSummary::new(
            "bs1alice".to_string(),
            "bs1bob".to_string(),
            TrustLevel::Trusted,
            Some("Great dev".to_string()),
            100,
            "txid123".to_string(),
        );
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("\"fromAddress\":\"bs1alice\""));
        assert!(json.contains("\"toAddress\":\"bs1bob\""));
        assert!(json.contains("\"level\":\"trusted\""));
        assert!(json.contains("\"blockHeight\":100"));
    }

    #[test]
    fn trust_query_response_serialize() {
        let resp = TrustQueryResponse::new(
            "bs1alice".to_string(),
            5,
            10,
            2,
            vec![],
        );
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"address\":\"bs1alice\""));
        assert!(json.contains("\"trustScore\":5"));
        assert!(json.contains("\"trustedByCount\":10"));
        assert!(json.contains("\"distrustedByCount\":2"));
    }

    #[test]
    fn report_request_deserialize() {
        let json = r#"{"from":"bs1reporter","targetTxid":"abc123","category":"spam","stake":1000000,"evidence":"Repeated spam posts"}"#;
        let req: ReportRequest = serde_json::from_str(json).unwrap();

        assert_eq!(req.from, "bs1reporter");
        assert_eq!(req.target_txid, "abc123");
        assert_eq!(req.category, ReportCategory::Spam);
        assert_eq!(req.stake, 1_000_000);
        assert_eq!(req.evidence, Some("Repeated spam posts".to_string()));
    }

    #[test]
    fn report_request_deserialize_no_evidence() {
        let json = r#"{"from":"bs1reporter","targetTxid":"abc123","category":"harassment","stake":2000000}"#;
        let req: ReportRequest = serde_json::from_str(json).unwrap();

        assert_eq!(req.from, "bs1reporter");
        assert_eq!(req.target_txid, "abc123");
        assert_eq!(req.category, ReportCategory::Harassment);
        assert_eq!(req.stake, 2_000_000);
        assert_eq!(req.evidence, None);
    }

    #[test]
    fn report_response_serialize() {
        let resp = ReportResponse::new(
            "reporttxid".to_string(),
            "contenttxid".to_string(),
            ReportCategory::Scam,
            MIN_REPORT_STAKE,
        );
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"txid\":\"reporttxid\""));
        assert!(json.contains("\"targetTxid\":\"contenttxid\""));
        assert!(json.contains("\"category\":\"scam\""));
        assert!(json.contains("\"stake\":1000000"));
    }

    #[test]
    fn report_status_request_deserialize() {
        let json = r#"{"reportTxid":"abc123"}"#;
        let req: ReportStatusRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.report_txid, "abc123");
    }

    #[test]
    fn report_status_response_serialize() {
        let resp = ReportStatusResponse::new(
            "reporttxid".to_string(),
            "contenttxid".to_string(),
            ReportCategory::Illegal,
            MIN_REPORT_STAKE,
            ReportStatus::Pending,
            500,
            None,
        );
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"reportTxid\":\"reporttxid\""));
        assert!(json.contains("\"targetTxid\":\"contenttxid\""));
        assert!(json.contains("\"category\":\"illegal\""));
        assert!(json.contains("\"status\":\"pending\""));
        assert!(json.contains("\"blockHeight\":500"));
    }

    #[test]
    fn report_list_request_deserialize_full() {
        let json = r#"{"targetTxid":"abc123","reporterAddress":"bs1alice","category":"spam","status":"pending","limit":25}"#;
        let req: ReportListRequest = serde_json::from_str(json).unwrap();

        assert_eq!(req.target_txid, Some("abc123".to_string()));
        assert_eq!(req.reporter_address, Some("bs1alice".to_string()));
        assert_eq!(req.category, Some(ReportCategory::Spam));
        assert_eq!(req.status, Some(ReportStatus::Pending));
        assert_eq!(req.limit, 25);
    }

    #[test]
    fn report_list_request_deserialize_minimal() {
        let json = r#"{}"#;
        let req: ReportListRequest = serde_json::from_str(json).unwrap();

        assert_eq!(req.target_txid, None);
        assert_eq!(req.reporter_address, None);
        assert_eq!(req.category, None);
        assert_eq!(req.status, None);
        assert_eq!(req.limit, 50); // default
    }

    #[test]
    fn report_summary_serialize() {
        let summary = ReportSummary::new(
            "reporttxid".to_string(),
            "contenttxid".to_string(),
            "bs1reporter".to_string(),
            ReportCategory::Harassment,
            MIN_REPORT_STAKE,
            ReportStatus::Validated,
            1000,
        );
        let json = serde_json::to_string(&summary).unwrap();
        assert!(json.contains("\"reportTxid\":\"reporttxid\""));
        assert!(json.contains("\"targetTxid\":\"contenttxid\""));
        assert!(json.contains("\"reporterAddress\":\"bs1reporter\""));
        assert!(json.contains("\"category\":\"harassment\""));
        assert!(json.contains("\"status\":\"validated\""));
    }

    #[test]
    fn report_list_response_serialize() {
        let resp = ReportListResponse::new(vec![], 0);
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"reports\":[]"));
        assert!(json.contains("\"totalCount\":0"));
    }

    #[test]
    fn moderation_constants() {
        assert_eq!(MIN_REPORT_STAKE, 1_000_000);
        assert_eq!(MAX_TRUST_REASON_LENGTH, 200);
        assert_eq!(MAX_REPORT_EVIDENCE_LENGTH, 300);
        assert_eq!(MAX_TRUST_LIMIT, 1000);
        assert_eq!(MAX_REPORT_LIMIT, 1000);
    }
}
