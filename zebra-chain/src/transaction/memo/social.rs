//! Botcash Social Protocol (BSP) message types.
//!
//! This module defines the social message types that can be encoded in
//! transaction memo fields, enabling social interactions on the Botcash
//! blockchain.
//!
//! # Message Format
//!
//! All social messages follow a common header format:
//!
//! ```text
//! ┌────────┬─────────┬──────────────────────┐
//! │ Type   │ Version │ Payload              │
//! │ 1 byte │ 1 byte  │ Variable (≤510 bytes)│
//! └────────┴─────────┴──────────────────────┘
//! ```
//!
//! # Message Types
//!
//! The protocol defines 16 core message types organized by category:
//!
//! - **Profile (0x10)**: Agent/user metadata
//! - **Content (0x20-0x22)**: Posts, comments, and upvotes
//! - **Social (0x30-0x31)**: Follow/unfollow actions
//! - **Messaging (0x40-0x41)**: Private and group DMs
//! - **Value (0x50-0x54)**: Tips, bounties, attention boosts, and credits
//! - **Media (0x60)**: Media attachments
//! - **Polls (0x70-0x71)**: Poll creation and voting
//! - **Governance (0xE0-0xE1)**: On-chain voting and proposals
//! - **Channels (0xC0-0xC2)**: Layer-2 social channels for high-frequency messaging
//! - **Moderation (0xD0-0xD1)**: Trust/reputation and content reports
//! - **Recovery (0xF0-0xF6)**: Key recovery, social recovery, and multi-sig identity mechanisms

use std::fmt;

use super::Memo;

/// The current version of the social protocol.
pub const SOCIAL_PROTOCOL_VERSION: u8 = 1;

/// Minimum size of a valid social message (type + version).
pub const MIN_SOCIAL_MESSAGE_SIZE: usize = 2;

/// Maximum number of actions allowed in a batch.
///
/// Limited by memo size (~510 bytes). With average action size of ~100 bytes,
/// 5 actions is a practical limit that leaves room for batch overhead.
pub const MAX_BATCH_ACTIONS: usize = 5;

/// Minimum size of a valid batch message (type + version + count).
pub const MIN_BATCH_MESSAGE_SIZE: usize = 3;

/// Size of the length prefix for each action in a batch (2 bytes = u16).
pub const BATCH_ACTION_LENGTH_SIZE: usize = 2;

/// Social message type identifiers.
///
/// Each type is represented as a single byte in the memo field.
/// Types are organized into logical groups by their high nibble:
///
/// - `0x1_`: Profile/identity
/// - `0x2_`: Content (posts, comments, votes)
/// - `0x3_`: Social graph (follow/unfollow)
/// - `0x4_`: Messaging (DM, group DM)
/// - `0x5_`: Value transfer (tips, bounties, attention)
/// - `0x6_`: Media
/// - `0x7_`: Polls
/// - `0x8_`: Batching
/// - `0xB_`: Bridges (cross-platform identity linking)
/// - `0xC_`: Channels (layer-2 social channels)
/// - `0xD_`: Moderation (trust/reputation, content reports)
/// - `0xE_`: Governance (voting, proposals)
/// - `0xF_`: Recovery (social recovery, key rotation)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SocialMessageType {
    /// Agent/user profile metadata (0x10).
    ///
    /// Contains display name, bio, avatar URL, and other profile fields.
    Profile = 0x10,

    /// Original content post (0x20).
    ///
    /// A standalone post visible on the author's timeline.
    Post = 0x20,

    /// Comment/reply to existing content (0x21).
    ///
    /// References a parent transaction ID.
    Comment = 0x21,

    /// Upvote/endorsement with optional payment (0x22).
    ///
    /// The transaction value serves as the upvote weight.
    Upvote = 0x22,

    /// Subscribe to a user's content (0x30).
    ///
    /// Creates a following relationship.
    Follow = 0x30,

    /// Unsubscribe from a user's content (0x31).
    ///
    /// Removes an existing following relationship.
    Unfollow = 0x31,

    /// Private direct message (0x40).
    ///
    /// Encrypted message to a single recipient.
    Dm = 0x40,

    /// Group direct message (0x41).
    ///
    /// Encrypted message to multiple recipients.
    DmGroup = 0x41,

    /// Tip with optional message (0x50).
    ///
    /// Value transfer with social context.
    Tip = 0x50,

    /// Bounty for completing a task (0x51).
    ///
    /// Includes task description and reward conditions.
    Bounty = 0x51,

    /// Paid visibility boost (0x52).
    ///
    /// Increases content ranking in attention market feeds.
    AttentionBoost = 0x52,

    /// Tip using earned credits (0x53).
    ///
    /// Spends credits from attention market redistribution.
    CreditTip = 0x53,

    /// Claim earned credits from pool (0x54).
    ///
    /// Withdraws available credits from an epoch.
    CreditClaim = 0x54,

    /// Media attachment reference (0x60).
    ///
    /// Contains hash/URL of off-chain media content.
    Media = 0x60,

    /// Poll creation (0x70).
    ///
    /// Defines poll options and voting parameters.
    Poll = 0x70,

    /// Poll vote (0x71).
    ///
    /// Casts a vote on an existing poll.
    Vote = 0x71,

    /// Batched actions (0x80).
    ///
    /// Multiple social actions bundled into a single transaction.
    /// Reduces fees and chain bloat by combining up to 5 actions.
    /// Format: [0x80][version][count][action1_len(2)][action1]...[actionN_len(2)][actionN]
    Batch = 0x80,

    /// Bridge identity link (0xB0).
    ///
    /// Links an external platform identity (Telegram, Discord, Nostr, Mastodon)
    /// to a Botcash address using a signed challenge-response proof.
    /// Format: [platform(1)][platform_id_len(1)][platform_id][challenge(32)][signature_len(1)][signature]
    BridgeLink = 0xB0,

    /// Bridge identity unlink (0xB1).
    ///
    /// Removes an existing identity link from a platform.
    /// Only the linked Botcash address owner can unlink.
    /// Format: [platform(1)][platform_id_len(1)][platform_id]
    BridgeUnlink = 0xB1,

    /// Bridge cross-post (0xB2).
    ///
    /// Posts content from an external platform to Botcash via a bridge.
    /// Includes attribution to the original platform and user.
    /// Format: [platform(1)][original_id_len(1)][original_id][content_len(2)][content]
    BridgePost = 0xB2,

    /// Bridge verification request (0xB3).
    ///
    /// Requests verification of a bridge identity link.
    /// Used by bridges to prove ownership before relaying messages.
    /// Format: [platform(1)][platform_id_len(1)][platform_id][nonce(8)]
    BridgeVerify = 0xB3,

    /// Channel open (0xC0).
    ///
    /// Opens a new Layer-2 social channel between parties for high-frequency
    /// off-chain messaging (chat, group DM, thread replies).
    /// Format: [parties_count(1)][party1_addr_len(1)][party1_addr]...[deposit(8)][timeout_blocks(4)]
    ChannelOpen = 0xC0,

    /// Channel close (0xC1).
    ///
    /// Closes an existing channel cooperatively. All parties must agree.
    /// Format: [channel_id(32)][final_seq(4)]
    ChannelClose = 0xC1,

    /// Channel settlement (0xC2).
    ///
    /// Settles a channel with final state. Can be unilateral after timeout.
    /// Format: [channel_id(32)][final_seq(4)][message_hash(32)]
    ChannelSettle = 0xC2,

    /// Governance vote on a proposal (0xE0).
    ///
    /// Cast a vote (yes/no/abstain) on an existing governance proposal.
    /// Voting power is calculated based on karma and BCASH balance.
    /// Format: [proposal_id(32)][vote(1)][weight(8)]
    GovernanceVote = 0xE0,

    /// Governance proposal creation (0xE1).
    ///
    /// Create a new governance proposal for community voting.
    /// Requires a minimum deposit that is returned if the proposal
    /// receives sufficient support (>10%).
    /// Format: [proposal_type(1)][title_len(1)][title][description_len(2)][description][params...]
    GovernanceProposal = 0xE1,

    /// Recovery configuration setup (0xF0).
    ///
    /// Registers a social recovery configuration with guardians who can
    /// collectively help recover the account if keys are lost.
    /// Format: [guardian_count(1)][guardian_hash(32)]...[threshold(1)][timelock_blocks(4)]
    RecoveryConfig = 0xF0,

    /// Recovery request initiation (0xF1).
    ///
    /// Initiates an account recovery process from a new device or key.
    /// Starts the timelock period during which the original owner can cancel.
    /// Format: [target_address_len(1)][target_address][new_pubkey(33)][proof_len(1)][proof]
    RecoveryRequest = 0xF1,

    /// Recovery approval from guardian (0xF2).
    ///
    /// A guardian approves a recovery request by providing their encrypted
    /// Shamir share. M-of-N guardians must approve for recovery to succeed.
    /// Format: [request_txid(32)][encrypted_share_len(1)][encrypted_share]
    RecoveryApprove = 0xF2,

    /// Recovery cancellation by owner (0xF3).
    ///
    /// The original account owner cancels a pending recovery request.
    /// Must be submitted before the timelock expires to prevent unauthorized recovery.
    /// Format: [request_txid(32)][owner_sig_len(1)][owner_sig]
    RecoveryCancel = 0xF3,

    /// Key rotation to migrate identity to new address (0xF4).
    ///
    /// Allows users to rotate their keys by migrating their social identity
    /// (followers, following, karma) to a new address. Must be signed by both
    /// the old key and the new key to prove ownership of both.
    /// Format: [old_addr_len(1)][old_addr][new_addr_len(1)][new_addr][old_sig_len(1)][old_sig][new_sig_len(1)][new_sig]
    /// After rotation: followers auto-follow new address, karma transfers, old address marked "migrated".
    KeyRotation = 0xF4,

    /// Multi-sig identity setup (0xF5).
    ///
    /// Registers a multi-sig identity with M-of-N signature requirements.
    /// For high-value accounts (influencers, businesses, agents with significant stake).
    /// All subsequent posts from this address require M signatures from the N keys.
    /// Format: [key_count(1)][pubkey1(33)]...[pubkeyN(33)][threshold(1)]
    /// - key_count: Number of keys (2-15)
    /// - pubkeyN: Compressed public keys (33 bytes each)
    /// - threshold: M value (1 to key_count)
    MultisigSetup = 0xF5,

    /// Multi-sig action with signatures (0xF6).
    ///
    /// A social action (post, follow, etc.) signed by multiple keys.
    /// Used by multi-sig identities to authorize actions.
    /// Format: [action_type(1)][action_len(2)][action][sig_count(1)][sig1_idx(1)][sig1(64)]...[sigM_idx(1)][sigM(64)]
    /// - action_type: The wrapped social message type
    /// - action_len: Length of the wrapped action (2 bytes, little-endian)
    /// - action: The serialized social message being authorized
    /// - sig_count: Number of signatures (must meet threshold)
    /// - sigN_idx: Index of the key that made this signature (0-based)
    /// - sigN: Schnorr signature (64 bytes)
    MultisigAction = 0xF6,

    /// Trust/vouch for another user (0xD0).
    ///
    /// Explicitly express trust in another user, contributing to the web of trust.
    /// Trust propagates through the social graph with decay.
    /// Format: [target_addr_len(1)][target_addr][level(1)][reason_len(1)][reason]
    /// Level: 0 = distrust, 1 = neutral, 2 = trusted
    Trust = 0xD0,

    /// Stake-weighted content report (0xD1).
    ///
    /// Report content for moderation. Requires a small BCASH stake that is
    /// forfeited for false reports or returned (with small reward) for valid ones.
    /// Format: [target_txid(32)][category(1)][stake(8)][evidence_len(1)][evidence]
    /// Categories: 0 = spam, 1 = scam, 2 = harassment, 3 = illegal, 4 = other
    Report = 0xD1,
}

impl SocialMessageType {
    /// Returns the byte value of this message type.
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    /// Returns a human-readable name for this message type.
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Profile => "Profile",
            Self::Post => "Post",
            Self::Comment => "Comment",
            Self::Upvote => "Upvote",
            Self::Follow => "Follow",
            Self::Unfollow => "Unfollow",
            Self::Dm => "DM",
            Self::DmGroup => "GroupDM",
            Self::Tip => "Tip",
            Self::Bounty => "Bounty",
            Self::AttentionBoost => "AttentionBoost",
            Self::CreditTip => "CreditTip",
            Self::CreditClaim => "CreditClaim",
            Self::Media => "Media",
            Self::Poll => "Poll",
            Self::Vote => "Vote",
            Self::Batch => "Batch",
            Self::BridgeLink => "BridgeLink",
            Self::BridgeUnlink => "BridgeUnlink",
            Self::BridgePost => "BridgePost",
            Self::BridgeVerify => "BridgeVerify",
            Self::ChannelOpen => "ChannelOpen",
            Self::ChannelClose => "ChannelClose",
            Self::ChannelSettle => "ChannelSettle",
            Self::GovernanceVote => "GovernanceVote",
            Self::GovernanceProposal => "GovernanceProposal",
            Self::RecoveryConfig => "RecoveryConfig",
            Self::RecoveryRequest => "RecoveryRequest",
            Self::RecoveryApprove => "RecoveryApprove",
            Self::RecoveryCancel => "RecoveryCancel",
            Self::KeyRotation => "KeyRotation",
            Self::MultisigSetup => "MultisigSetup",
            Self::MultisigAction => "MultisigAction",
            Self::Trust => "Trust",
            Self::Report => "Report",
        }
    }

    /// Returns true if this is a batch message type.
    pub const fn is_batch(&self) -> bool {
        matches!(self, Self::Batch)
    }

    /// Returns true if this is a channel message type.
    ///
    /// Channel messages are used for Layer-2 social channels that enable
    /// high-frequency off-chain messaging (chat, group DM, thread replies).
    pub const fn is_channel(&self) -> bool {
        matches!(self, Self::ChannelOpen | Self::ChannelClose | Self::ChannelSettle)
    }

    /// Returns true if this message type involves value transfer.
    pub const fn is_value_transfer(&self) -> bool {
        matches!(
            self,
            Self::Tip
                | Self::Bounty
                | Self::AttentionBoost
                | Self::CreditTip
                | Self::CreditClaim
                | Self::Upvote
        )
    }

    /// Returns true if this is an attention market message type.
    pub const fn is_attention_market(&self) -> bool {
        matches!(
            self,
            Self::AttentionBoost | Self::CreditTip | Self::CreditClaim
        )
    }

    /// Returns true if this is a governance message type.
    pub const fn is_governance(&self) -> bool {
        matches!(self, Self::GovernanceVote | Self::GovernanceProposal)
    }

    /// Returns true if this is a recovery message type.
    ///
    /// Recovery messages are used for social recovery mechanisms that allow
    /// users to recover access to their accounts using trusted guardians.
    /// Also includes key rotation and multi-sig identity setup.
    pub const fn is_recovery(&self) -> bool {
        matches!(
            self,
            Self::RecoveryConfig
                | Self::RecoveryRequest
                | Self::RecoveryApprove
                | Self::RecoveryCancel
                | Self::KeyRotation
                | Self::MultisigSetup
                | Self::MultisigAction
        )
    }

    /// Returns true if this is a multi-sig message type.
    ///
    /// Multi-sig messages are used for identities that require M-of-N
    /// signatures to authorize actions, suitable for high-value accounts.
    pub const fn is_multisig(&self) -> bool {
        matches!(self, Self::MultisigSetup | Self::MultisigAction)
    }

    /// Returns true if this is a bridge message type.
    ///
    /// Bridge messages are used for cross-platform identity linking,
    /// allowing users to connect their Botcash addresses to external
    /// platforms like Telegram, Discord, Nostr, and Mastodon.
    pub const fn is_bridge(&self) -> bool {
        matches!(
            self,
            Self::BridgeLink | Self::BridgeUnlink | Self::BridgePost | Self::BridgeVerify
        )
    }

    /// Returns true if this is a moderation message type.
    ///
    /// Moderation messages are used for the reputation system (trust/vouch)
    /// and stake-weighted content reports. They enable community-driven
    /// moderation without central authority.
    pub const fn is_moderation(&self) -> bool {
        matches!(self, Self::Trust | Self::Report)
    }
}

impl TryFrom<u8> for SocialMessageType {
    type Error = SocialParseError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x10 => Ok(Self::Profile),
            0x20 => Ok(Self::Post),
            0x21 => Ok(Self::Comment),
            0x22 => Ok(Self::Upvote),
            0x30 => Ok(Self::Follow),
            0x31 => Ok(Self::Unfollow),
            0x40 => Ok(Self::Dm),
            0x41 => Ok(Self::DmGroup),
            0x50 => Ok(Self::Tip),
            0x51 => Ok(Self::Bounty),
            0x52 => Ok(Self::AttentionBoost),
            0x53 => Ok(Self::CreditTip),
            0x54 => Ok(Self::CreditClaim),
            0x60 => Ok(Self::Media),
            0x70 => Ok(Self::Poll),
            0x71 => Ok(Self::Vote),
            0x80 => Ok(Self::Batch),
            0xB0 => Ok(Self::BridgeLink),
            0xB1 => Ok(Self::BridgeUnlink),
            0xB2 => Ok(Self::BridgePost),
            0xB3 => Ok(Self::BridgeVerify),
            0xC0 => Ok(Self::ChannelOpen),
            0xC1 => Ok(Self::ChannelClose),
            0xC2 => Ok(Self::ChannelSettle),
            0xE0 => Ok(Self::GovernanceVote),
            0xE1 => Ok(Self::GovernanceProposal),
            0xF0 => Ok(Self::RecoveryConfig),
            0xF1 => Ok(Self::RecoveryRequest),
            0xF2 => Ok(Self::RecoveryApprove),
            0xF3 => Ok(Self::RecoveryCancel),
            0xF4 => Ok(Self::KeyRotation),
            0xF5 => Ok(Self::MultisigSetup),
            0xF6 => Ok(Self::MultisigAction),
            0xD0 => Ok(Self::Trust),
            0xD1 => Ok(Self::Report),
            _ => Err(SocialParseError::UnknownMessageType(value)),
        }
    }
}

impl fmt::Display for SocialMessageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Errors that can occur when parsing a social message from a memo.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SocialParseError {
    /// The memo is empty.
    Empty,

    /// The memo is too short to contain a valid social message.
    TooShort {
        /// The actual length of the memo content.
        actual: usize,
        /// The minimum required length.
        minimum: usize,
    },

    /// Unknown or unsupported message type.
    UnknownMessageType(u8),

    /// Unsupported protocol version.
    UnsupportedVersion {
        /// The version found in the message.
        found: u8,
        /// The maximum supported version.
        max_supported: u8,
    },

    /// The payload is malformed for the given message type.
    MalformedPayload {
        /// The message type that failed to parse.
        msg_type: SocialMessageType,
        /// Description of what went wrong.
        reason: &'static str,
    },

    /// The memo does not contain a social message.
    ///
    /// This is indicated by the first byte not matching any known message type
    /// and not being in the social protocol range (0x10-0x7F).
    NotSocialMessage,
}

impl fmt::Display for SocialParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "memo is empty"),
            Self::TooShort { actual, minimum } => {
                write!(
                    f,
                    "memo too short: {} bytes, minimum {} required",
                    actual, minimum
                )
            }
            Self::UnknownMessageType(byte) => {
                write!(f, "unknown social message type: 0x{:02X}", byte)
            }
            Self::UnsupportedVersion {
                found,
                max_supported,
            } => {
                write!(
                    f,
                    "unsupported protocol version: {}, max supported: {}",
                    found, max_supported
                )
            }
            Self::MalformedPayload { msg_type, reason } => {
                write!(f, "malformed {} payload: {}", msg_type, reason)
            }
            Self::NotSocialMessage => {
                write!(f, "memo does not contain a social message")
            }
        }
    }
}

impl std::error::Error for SocialParseError {}

// ==================== Bridge Types ====================

/// Supported bridge platforms for cross-platform identity linking.
///
/// Each platform has a unique identifier used in bridge messages to specify
/// which external platform is being linked or verified.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum BridgePlatform {
    /// Telegram messaging platform (0x01).
    Telegram = 0x01,

    /// Discord chat platform (0x02).
    Discord = 0x02,

    /// Nostr decentralized protocol (0x03).
    Nostr = 0x03,

    /// Mastodon/ActivityPub (0x04).
    Mastodon = 0x04,

    /// X/Twitter (0x05) - primarily read-only bridging.
    Twitter = 0x05,
}

impl BridgePlatform {
    /// Returns the byte value of this platform.
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

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
    ///
    /// Some platforms (like Twitter/X) have API restrictions that make
    /// bidirectional bridging difficult or impractical.
    pub const fn is_bidirectional(&self) -> bool {
        !matches!(self, Self::Twitter)
    }
}

impl TryFrom<u8> for BridgePlatform {
    type Error = BridgeParseError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(Self::Telegram),
            0x02 => Ok(Self::Discord),
            0x03 => Ok(Self::Nostr),
            0x04 => Ok(Self::Mastodon),
            0x05 => Ok(Self::Twitter),
            _ => Err(BridgeParseError::UnknownPlatform(value)),
        }
    }
}

impl fmt::Display for BridgePlatform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Errors that can occur when parsing bridge messages.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BridgeParseError {
    /// Unknown or unsupported platform.
    UnknownPlatform(u8),

    /// The platform ID is too long.
    PlatformIdTooLong {
        /// The maximum allowed length.
        max_len: usize,
        /// The actual length.
        actual_len: usize,
    },

    /// The challenge is invalid or missing.
    InvalidChallenge,

    /// The signature is invalid or missing.
    InvalidSignature,

    /// The payload is too short.
    PayloadTooShort {
        /// The minimum required length.
        minimum: usize,
        /// The actual length.
        actual: usize,
    },
}

impl fmt::Display for BridgeParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownPlatform(byte) => {
                write!(f, "unknown bridge platform: 0x{:02X}", byte)
            }
            Self::PlatformIdTooLong { max_len, actual_len } => {
                write!(
                    f,
                    "platform ID too long: {} bytes, maximum {} allowed",
                    actual_len, max_len
                )
            }
            Self::InvalidChallenge => write!(f, "invalid or missing challenge"),
            Self::InvalidSignature => write!(f, "invalid or missing signature"),
            Self::PayloadTooShort { minimum, actual } => {
                write!(
                    f,
                    "bridge payload too short: {} bytes, minimum {} required",
                    actual, minimum
                )
            }
        }
    }
}

impl std::error::Error for BridgeParseError {}

/// Maximum length for a platform user ID.
pub const MAX_PLATFORM_ID_LENGTH: usize = 64;

/// Size of the challenge in bridge link messages (32 bytes).
pub const BRIDGE_CHALLENGE_SIZE: usize = 32;

/// Maximum size of a bridge signature (varies by platform).
pub const MAX_BRIDGE_SIGNATURE_SIZE: usize = 128;

/// A parsed bridge message from a transaction memo.
///
/// This struct provides access to the bridge-specific fields for identity
/// linking and cross-platform messaging operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BridgeMessage {
    /// The target platform for this bridge operation.
    platform: BridgePlatform,

    /// The platform-specific user identifier.
    platform_id: String,

    /// Optional challenge for link verification (32 bytes).
    challenge: Option<[u8; BRIDGE_CHALLENGE_SIZE]>,

    /// Optional signature proving ownership.
    signature: Option<Vec<u8>>,

    /// Optional content for cross-posts.
    content: Option<String>,

    /// Optional original post ID on the source platform.
    original_id: Option<String>,

    /// Optional nonce for verification requests.
    nonce: Option<u64>,
}

impl BridgeMessage {
    /// Creates a new bridge link message.
    pub fn new_link(
        platform: BridgePlatform,
        platform_id: String,
        challenge: [u8; BRIDGE_CHALLENGE_SIZE],
        signature: Vec<u8>,
    ) -> Self {
        Self {
            platform,
            platform_id,
            challenge: Some(challenge),
            signature: Some(signature),
            content: None,
            original_id: None,
            nonce: None,
        }
    }

    /// Creates a new bridge unlink message.
    pub fn new_unlink(platform: BridgePlatform, platform_id: String) -> Self {
        Self {
            platform,
            platform_id,
            challenge: None,
            signature: None,
            content: None,
            original_id: None,
            nonce: None,
        }
    }

    /// Creates a new bridge cross-post message.
    pub fn new_post(
        platform: BridgePlatform,
        original_id: String,
        content: String,
    ) -> Self {
        Self {
            platform,
            platform_id: String::new(),
            challenge: None,
            signature: None,
            content: Some(content),
            original_id: Some(original_id),
            nonce: None,
        }
    }

    /// Creates a new bridge verification request message.
    pub fn new_verify(platform: BridgePlatform, platform_id: String, nonce: u64) -> Self {
        Self {
            platform,
            platform_id,
            challenge: None,
            signature: None,
            content: None,
            original_id: None,
            nonce: Some(nonce),
        }
    }

    /// Returns the platform for this bridge message.
    #[inline]
    pub fn platform(&self) -> BridgePlatform {
        self.platform
    }

    /// Returns the platform-specific user identifier.
    #[inline]
    pub fn platform_id(&self) -> &str {
        &self.platform_id
    }

    /// Returns the challenge bytes if present.
    #[inline]
    pub fn challenge(&self) -> Option<&[u8; BRIDGE_CHALLENGE_SIZE]> {
        self.challenge.as_ref()
    }

    /// Returns the signature bytes if present.
    #[inline]
    pub fn signature(&self) -> Option<&[u8]> {
        self.signature.as_deref()
    }

    /// Returns the content for cross-posts.
    #[inline]
    pub fn content(&self) -> Option<&str> {
        self.content.as_deref()
    }

    /// Returns the original post ID for cross-posts.
    #[inline]
    pub fn original_id(&self) -> Option<&str> {
        self.original_id.as_deref()
    }

    /// Returns the nonce for verification requests.
    #[inline]
    pub fn nonce(&self) -> Option<u64> {
        self.nonce
    }

    /// Encodes this bridge message into bytes.
    ///
    /// The encoding varies by message type (link, unlink, post, verify).
    pub fn encode(&self, msg_type: SocialMessageType) -> Vec<u8> {
        let mut bytes = Vec::new();

        match msg_type {
            SocialMessageType::BridgeLink => {
                // Format: [platform(1)][platform_id_len(1)][platform_id][challenge(32)][sig_len(1)][sig]
                bytes.push(self.platform.as_u8());
                let id_bytes = self.platform_id.as_bytes();
                bytes.push(id_bytes.len() as u8);
                bytes.extend_from_slice(id_bytes);
                if let Some(challenge) = &self.challenge {
                    bytes.extend_from_slice(challenge);
                }
                if let Some(sig) = &self.signature {
                    bytes.push(sig.len() as u8);
                    bytes.extend_from_slice(sig);
                }
            }
            SocialMessageType::BridgeUnlink => {
                // Format: [platform(1)][platform_id_len(1)][platform_id]
                bytes.push(self.platform.as_u8());
                let id_bytes = self.platform_id.as_bytes();
                bytes.push(id_bytes.len() as u8);
                bytes.extend_from_slice(id_bytes);
            }
            SocialMessageType::BridgePost => {
                // Format: [platform(1)][original_id_len(1)][original_id][content_len(2)][content]
                bytes.push(self.platform.as_u8());
                if let Some(orig_id) = &self.original_id {
                    let orig_bytes = orig_id.as_bytes();
                    bytes.push(orig_bytes.len() as u8);
                    bytes.extend_from_slice(orig_bytes);
                } else {
                    bytes.push(0);
                }
                if let Some(content) = &self.content {
                    let content_bytes = content.as_bytes();
                    let len = content_bytes.len() as u16;
                    bytes.push((len & 0xFF) as u8);
                    bytes.push((len >> 8) as u8);
                    bytes.extend_from_slice(content_bytes);
                } else {
                    bytes.push(0);
                    bytes.push(0);
                }
            }
            SocialMessageType::BridgeVerify => {
                // Format: [platform(1)][platform_id_len(1)][platform_id][nonce(8)]
                bytes.push(self.platform.as_u8());
                let id_bytes = self.platform_id.as_bytes();
                bytes.push(id_bytes.len() as u8);
                bytes.extend_from_slice(id_bytes);
                if let Some(nonce) = self.nonce {
                    bytes.extend_from_slice(&nonce.to_le_bytes());
                }
            }
            _ => {}
        }

        bytes
    }

    /// Parses a bridge message from payload bytes.
    pub fn parse(msg_type: SocialMessageType, payload: &[u8]) -> Result<Self, BridgeParseError> {
        if payload.is_empty() {
            return Err(BridgeParseError::PayloadTooShort {
                minimum: 2,
                actual: 0,
            });
        }

        let platform = BridgePlatform::try_from(payload[0])?;

        match msg_type {
            SocialMessageType::BridgeLink => {
                // Format: [platform(1)][platform_id_len(1)][platform_id][challenge(32)][sig_len(1)][sig]
                if payload.len() < 2 {
                    return Err(BridgeParseError::PayloadTooShort {
                        minimum: 2,
                        actual: payload.len(),
                    });
                }

                let id_len = payload[1] as usize;
                if id_len > MAX_PLATFORM_ID_LENGTH {
                    return Err(BridgeParseError::PlatformIdTooLong {
                        max_len: MAX_PLATFORM_ID_LENGTH,
                        actual_len: id_len,
                    });
                }

                let id_end = 2 + id_len;
                if payload.len() < id_end {
                    return Err(BridgeParseError::PayloadTooShort {
                        minimum: id_end,
                        actual: payload.len(),
                    });
                }

                let platform_id = String::from_utf8_lossy(&payload[2..id_end]).to_string();

                let challenge_end = id_end + BRIDGE_CHALLENGE_SIZE;
                if payload.len() < challenge_end {
                    return Err(BridgeParseError::InvalidChallenge);
                }

                let mut challenge = [0u8; BRIDGE_CHALLENGE_SIZE];
                challenge.copy_from_slice(&payload[id_end..challenge_end]);

                if payload.len() < challenge_end + 1 {
                    return Err(BridgeParseError::InvalidSignature);
                }

                let sig_len = payload[challenge_end] as usize;
                let sig_end = challenge_end + 1 + sig_len;
                if payload.len() < sig_end {
                    return Err(BridgeParseError::InvalidSignature);
                }

                let signature = payload[challenge_end + 1..sig_end].to_vec();

                Ok(Self::new_link(platform, platform_id, challenge, signature))
            }
            SocialMessageType::BridgeUnlink => {
                // Format: [platform(1)][platform_id_len(1)][platform_id]
                if payload.len() < 2 {
                    return Err(BridgeParseError::PayloadTooShort {
                        minimum: 2,
                        actual: payload.len(),
                    });
                }

                let id_len = payload[1] as usize;
                if id_len > MAX_PLATFORM_ID_LENGTH {
                    return Err(BridgeParseError::PlatformIdTooLong {
                        max_len: MAX_PLATFORM_ID_LENGTH,
                        actual_len: id_len,
                    });
                }

                let id_end = 2 + id_len;
                if payload.len() < id_end {
                    return Err(BridgeParseError::PayloadTooShort {
                        minimum: id_end,
                        actual: payload.len(),
                    });
                }

                let platform_id = String::from_utf8_lossy(&payload[2..id_end]).to_string();

                Ok(Self::new_unlink(platform, platform_id))
            }
            SocialMessageType::BridgePost => {
                // Format: [platform(1)][original_id_len(1)][original_id][content_len(2)][content]
                if payload.len() < 2 {
                    return Err(BridgeParseError::PayloadTooShort {
                        minimum: 2,
                        actual: payload.len(),
                    });
                }

                let orig_len = payload[1] as usize;
                let orig_end = 2 + orig_len;
                if payload.len() < orig_end + 2 {
                    return Err(BridgeParseError::PayloadTooShort {
                        minimum: orig_end + 2,
                        actual: payload.len(),
                    });
                }

                let original_id = String::from_utf8_lossy(&payload[2..orig_end]).to_string();

                let content_len = u16::from_le_bytes([payload[orig_end], payload[orig_end + 1]]) as usize;
                let content_end = orig_end + 2 + content_len;
                if payload.len() < content_end {
                    return Err(BridgeParseError::PayloadTooShort {
                        minimum: content_end,
                        actual: payload.len(),
                    });
                }

                let content = String::from_utf8_lossy(&payload[orig_end + 2..content_end]).to_string();

                Ok(Self::new_post(platform, original_id, content))
            }
            SocialMessageType::BridgeVerify => {
                // Format: [platform(1)][platform_id_len(1)][platform_id][nonce(8)]
                if payload.len() < 2 {
                    return Err(BridgeParseError::PayloadTooShort {
                        minimum: 2,
                        actual: payload.len(),
                    });
                }

                let id_len = payload[1] as usize;
                if id_len > MAX_PLATFORM_ID_LENGTH {
                    return Err(BridgeParseError::PlatformIdTooLong {
                        max_len: MAX_PLATFORM_ID_LENGTH,
                        actual_len: id_len,
                    });
                }

                let id_end = 2 + id_len;
                if payload.len() < id_end + 8 {
                    return Err(BridgeParseError::PayloadTooShort {
                        minimum: id_end + 8,
                        actual: payload.len(),
                    });
                }

                let platform_id = String::from_utf8_lossy(&payload[2..id_end]).to_string();
                let nonce = u64::from_le_bytes([
                    payload[id_end],
                    payload[id_end + 1],
                    payload[id_end + 2],
                    payload[id_end + 3],
                    payload[id_end + 4],
                    payload[id_end + 5],
                    payload[id_end + 6],
                    payload[id_end + 7],
                ]);

                Ok(Self::new_verify(platform, platform_id, nonce))
            }
            _ => Err(BridgeParseError::PayloadTooShort {
                minimum: 1,
                actual: 0,
            }),
        }
    }
}

// ==================== Moderation Types ====================

/// Trust level for the web of trust reputation system.
///
/// Users can explicitly express trust levels for other users, which propagates
/// through the social graph with decay. This enables reputation-based filtering
/// without centralized authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TrustLevel {
    /// Explicit distrust - negative endorsement (0x00).
    Distrust = 0x00,

    /// Neutral - removes any previous trust/distrust (0x01).
    Neutral = 0x01,

    /// Trusted - positive endorsement (0x02).
    Trusted = 0x02,
}

impl TrustLevel {
    /// Returns the byte value of this trust level.
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    /// Returns a human-readable name for this trust level.
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Distrust => "Distrust",
            Self::Neutral => "Neutral",
            Self::Trusted => "Trusted",
        }
    }
}

impl TryFrom<u8> for TrustLevel {
    type Error = ModerationParseError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(Self::Distrust),
            0x01 => Ok(Self::Neutral),
            0x02 => Ok(Self::Trusted),
            _ => Err(ModerationParseError::InvalidTrustLevel(value)),
        }
    }
}

impl fmt::Display for TrustLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Report categories for stake-weighted content reports.
///
/// Each category has different handling in moderation systems.
/// Some categories (like illegal content) may trigger indexer-level filtering,
/// while others (like spam) only affect ranking and user-level filtering.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ReportCategory {
    /// Spam - unsolicited bulk content (0x00).
    Spam = 0x00,

    /// Scam - fraudulent schemes (0x01).
    Scam = 0x01,

    /// Harassment - targeted abuse (0x02).
    Harassment = 0x02,

    /// Illegal - potentially illegal content (0x03).
    Illegal = 0x03,

    /// Other - miscellaneous reports (0x04).
    Other = 0x04,
}

impl ReportCategory {
    /// Returns the byte value of this report category.
    #[inline]
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    /// Returns a human-readable name for this report category.
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Spam => "Spam",
            Self::Scam => "Scam",
            Self::Harassment => "Harassment",
            Self::Illegal => "Illegal",
            Self::Other => "Other",
        }
    }

    /// Returns true if this report category should trigger immediate indexer filtering.
    ///
    /// Some categories (like illegal content) are too sensitive to wait for
    /// stake resolution and should be filtered pending review.
    pub const fn requires_immediate_filtering(&self) -> bool {
        matches!(self, Self::Illegal)
    }
}

impl TryFrom<u8> for ReportCategory {
    type Error = ModerationParseError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(Self::Spam),
            0x01 => Ok(Self::Scam),
            0x02 => Ok(Self::Harassment),
            0x03 => Ok(Self::Illegal),
            0x04 => Ok(Self::Other),
            _ => Err(ModerationParseError::InvalidReportCategory(value)),
        }
    }
}

impl fmt::Display for ReportCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Errors that can occur when parsing moderation messages.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModerationParseError {
    /// Invalid trust level value.
    InvalidTrustLevel(u8),

    /// Invalid report category value.
    InvalidReportCategory(u8),

    /// The target address is too long.
    TargetAddressTooLong {
        /// The maximum allowed length.
        max_len: usize,
        /// The actual length.
        actual_len: usize,
    },

    /// The evidence text is too long.
    EvidenceTooLong {
        /// The maximum allowed length.
        max_len: usize,
        /// The actual length.
        actual_len: usize,
    },

    /// The reason text is too long.
    ReasonTooLong {
        /// The maximum allowed length.
        max_len: usize,
        /// The actual length.
        actual_len: usize,
    },

    /// The payload is too short.
    PayloadTooShort {
        /// The minimum required length.
        minimum: usize,
        /// The actual length.
        actual: usize,
    },

    /// Invalid stake amount (below minimum).
    StakeTooLow {
        /// The minimum required stake.
        minimum: u64,
        /// The actual stake.
        actual: u64,
    },
}

impl fmt::Display for ModerationParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTrustLevel(byte) => {
                write!(f, "invalid trust level: 0x{:02X}", byte)
            }
            Self::InvalidReportCategory(byte) => {
                write!(f, "invalid report category: 0x{:02X}", byte)
            }
            Self::TargetAddressTooLong { max_len, actual_len } => {
                write!(
                    f,
                    "target address too long: {} bytes, maximum {} allowed",
                    actual_len, max_len
                )
            }
            Self::EvidenceTooLong { max_len, actual_len } => {
                write!(
                    f,
                    "evidence too long: {} bytes, maximum {} allowed",
                    actual_len, max_len
                )
            }
            Self::ReasonTooLong { max_len, actual_len } => {
                write!(
                    f,
                    "reason too long: {} bytes, maximum {} allowed",
                    actual_len, max_len
                )
            }
            Self::PayloadTooShort { minimum, actual } => {
                write!(
                    f,
                    "moderation payload too short: {} bytes, minimum {} required",
                    actual, minimum
                )
            }
            Self::StakeTooLow { minimum, actual } => {
                write!(
                    f,
                    "report stake too low: {} zatoshis, minimum {} required",
                    actual, minimum
                )
            }
        }
    }
}

impl std::error::Error for ModerationParseError {}

/// Maximum length for a target address in trust messages.
pub const MAX_TRUST_ADDRESS_LENGTH: usize = 128;

/// Maximum length for the reason field in trust messages.
pub const MAX_TRUST_REASON_LENGTH: usize = 200;

/// Maximum length for evidence in report messages.
pub const MAX_REPORT_EVIDENCE_LENGTH: usize = 300;

/// Minimum stake required for a report (0.01 BCASH = 1_000_000 zatoshis).
pub const MIN_REPORT_STAKE: u64 = 1_000_000;

/// A parsed trust message from a transaction memo.
///
/// Trust messages allow users to explicitly vouch for or warn against other users,
/// building a decentralized web of trust for reputation-based filtering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrustMessage {
    /// The address being trusted/distrusted.
    target_address: String,

    /// The trust level being assigned.
    level: TrustLevel,

    /// Optional reason for the trust assignment.
    reason: Option<String>,
}

impl TrustMessage {
    /// Creates a new trust message.
    pub fn new(target_address: String, level: TrustLevel, reason: Option<String>) -> Self {
        Self {
            target_address,
            level,
            reason,
        }
    }

    /// Returns the target address.
    pub fn target_address(&self) -> &str {
        &self.target_address
    }

    /// Returns the trust level.
    pub fn level(&self) -> TrustLevel {
        self.level
    }

    /// Returns the optional reason.
    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }

    /// Encodes this trust message to bytes for inclusion in a memo.
    ///
    /// Format: [target_addr_len(1)][target_addr][level(1)][reason_len(1)][reason]
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Target address length and bytes
        bytes.push(self.target_address.len() as u8);
        bytes.extend_from_slice(self.target_address.as_bytes());

        // Trust level
        bytes.push(self.level.as_u8());

        // Reason (optional)
        if let Some(ref reason) = self.reason {
            bytes.push(reason.len() as u8);
            bytes.extend_from_slice(reason.as_bytes());
        } else {
            bytes.push(0);
        }

        bytes
    }

    /// Parses a trust message from a payload.
    ///
    /// The payload should NOT include the message type and version bytes.
    pub fn parse(payload: &[u8]) -> Result<Self, ModerationParseError> {
        if payload.is_empty() {
            return Err(ModerationParseError::PayloadTooShort {
                minimum: 3, // addr_len + level + reason_len
                actual: 0,
            });
        }

        let mut pos = 0;

        // Target address length
        let addr_len = payload[pos] as usize;
        pos += 1;

        if addr_len > MAX_TRUST_ADDRESS_LENGTH {
            return Err(ModerationParseError::TargetAddressTooLong {
                max_len: MAX_TRUST_ADDRESS_LENGTH,
                actual_len: addr_len,
            });
        }

        if pos + addr_len >= payload.len() {
            return Err(ModerationParseError::PayloadTooShort {
                minimum: pos + addr_len + 2, // + level + reason_len
                actual: payload.len(),
            });
        }

        let target_address =
            String::from_utf8_lossy(&payload[pos..pos + addr_len]).to_string();
        pos += addr_len;

        // Trust level
        if pos >= payload.len() {
            return Err(ModerationParseError::PayloadTooShort {
                minimum: pos + 2,
                actual: payload.len(),
            });
        }
        let level = TrustLevel::try_from(payload[pos])?;
        pos += 1;

        // Reason length
        if pos >= payload.len() {
            return Err(ModerationParseError::PayloadTooShort {
                minimum: pos + 1,
                actual: payload.len(),
            });
        }
        let reason_len = payload[pos] as usize;
        pos += 1;

        let reason = if reason_len > 0 {
            if reason_len > MAX_TRUST_REASON_LENGTH {
                return Err(ModerationParseError::ReasonTooLong {
                    max_len: MAX_TRUST_REASON_LENGTH,
                    actual_len: reason_len,
                });
            }

            if pos + reason_len > payload.len() {
                return Err(ModerationParseError::PayloadTooShort {
                    minimum: pos + reason_len,
                    actual: payload.len(),
                });
            }

            Some(String::from_utf8_lossy(&payload[pos..pos + reason_len]).to_string())
        } else {
            None
        };

        Ok(Self {
            target_address,
            level,
            reason,
        })
    }
}

/// A parsed report message from a transaction memo.
///
/// Report messages enable stake-weighted content moderation. Reports require
/// a BCASH stake that is forfeited for false reports or returned with a small
/// reward for valid ones.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReportMessage {
    /// The transaction ID of the content being reported (32 bytes).
    target_txid: [u8; 32],

    /// The category of the report.
    category: ReportCategory,

    /// The stake amount in zatoshis.
    stake: u64,

    /// Optional evidence/description.
    evidence: Option<String>,
}

impl ReportMessage {
    /// Creates a new report message.
    pub fn new(
        target_txid: [u8; 32],
        category: ReportCategory,
        stake: u64,
        evidence: Option<String>,
    ) -> Self {
        Self {
            target_txid,
            category,
            stake,
            evidence,
        }
    }

    /// Returns the target transaction ID.
    pub fn target_txid(&self) -> &[u8; 32] {
        &self.target_txid
    }

    /// Returns the report category.
    pub fn category(&self) -> ReportCategory {
        self.category
    }

    /// Returns the stake amount in zatoshis.
    pub fn stake(&self) -> u64 {
        self.stake
    }

    /// Returns the optional evidence.
    pub fn evidence(&self) -> Option<&str> {
        self.evidence.as_deref()
    }

    /// Encodes this report message to bytes for inclusion in a memo.
    ///
    /// Format: [target_txid(32)][category(1)][stake(8)][evidence_len(1)][evidence]
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Target txid (32 bytes)
        bytes.extend_from_slice(&self.target_txid);

        // Category
        bytes.push(self.category.as_u8());

        // Stake (8 bytes, little-endian)
        bytes.extend_from_slice(&self.stake.to_le_bytes());

        // Evidence (optional)
        if let Some(ref evidence) = self.evidence {
            bytes.push(evidence.len() as u8);
            bytes.extend_from_slice(evidence.as_bytes());
        } else {
            bytes.push(0);
        }

        bytes
    }

    /// Parses a report message from a payload.
    ///
    /// The payload should NOT include the message type and version bytes.
    pub fn parse(payload: &[u8]) -> Result<Self, ModerationParseError> {
        // Minimum: txid(32) + category(1) + stake(8) + evidence_len(1) = 42
        const MIN_LEN: usize = 42;

        if payload.len() < MIN_LEN {
            return Err(ModerationParseError::PayloadTooShort {
                minimum: MIN_LEN,
                actual: payload.len(),
            });
        }

        let mut pos = 0;

        // Target txid (32 bytes)
        let mut target_txid = [0u8; 32];
        target_txid.copy_from_slice(&payload[pos..pos + 32]);
        pos += 32;

        // Category
        let category = ReportCategory::try_from(payload[pos])?;
        pos += 1;

        // Stake (8 bytes, little-endian)
        let stake = u64::from_le_bytes(payload[pos..pos + 8].try_into().unwrap());
        pos += 8;

        if stake < MIN_REPORT_STAKE {
            return Err(ModerationParseError::StakeTooLow {
                minimum: MIN_REPORT_STAKE,
                actual: stake,
            });
        }

        // Evidence length
        let evidence_len = payload[pos] as usize;
        pos += 1;

        let evidence = if evidence_len > 0 {
            if evidence_len > MAX_REPORT_EVIDENCE_LENGTH {
                return Err(ModerationParseError::EvidenceTooLong {
                    max_len: MAX_REPORT_EVIDENCE_LENGTH,
                    actual_len: evidence_len,
                });
            }

            if pos + evidence_len > payload.len() {
                return Err(ModerationParseError::PayloadTooShort {
                    minimum: pos + evidence_len,
                    actual: payload.len(),
                });
            }

            Some(String::from_utf8_lossy(&payload[pos..pos + evidence_len]).to_string())
        } else {
            None
        };

        Ok(Self {
            target_txid,
            category,
            stake,
            evidence,
        })
    }
}

/// A parsed social message from a transaction memo.
///
/// This struct provides access to the message type, version, and raw payload
/// bytes. Higher-level parsing of specific message types can be done using
/// the payload data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SocialMessage {
    /// The type of social message.
    msg_type: SocialMessageType,

    /// The protocol version of this message.
    version: u8,

    /// The raw payload bytes (everything after type and version).
    payload: Vec<u8>,
}

impl SocialMessage {
    /// Creates a new social message with the given components.
    ///
    /// # Arguments
    ///
    /// * `msg_type` - The type of social message
    /// * `version` - The protocol version (should be `SOCIAL_PROTOCOL_VERSION`)
    /// * `payload` - The raw payload bytes
    pub fn new(msg_type: SocialMessageType, version: u8, payload: Vec<u8>) -> Self {
        Self {
            msg_type,
            version,
            payload,
        }
    }

    /// Returns the message type.
    #[inline]
    pub fn msg_type(&self) -> SocialMessageType {
        self.msg_type
    }

    /// Returns the protocol version.
    #[inline]
    pub fn version(&self) -> u8 {
        self.version
    }

    /// Returns the raw payload bytes.
    #[inline]
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    /// Returns true if this is a value transfer message.
    #[inline]
    pub fn is_value_transfer(&self) -> bool {
        self.msg_type.is_value_transfer()
    }

    /// Returns true if this is an attention market message.
    #[inline]
    pub fn is_attention_market(&self) -> bool {
        self.msg_type.is_attention_market()
    }

    /// Encodes this message into bytes suitable for a memo field.
    ///
    /// The encoding is: `[type_byte, version_byte, payload...]`
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(2 + self.payload.len());
        bytes.push(self.msg_type.as_u8());
        bytes.push(self.version);
        bytes.extend_from_slice(&self.payload);
        bytes
    }
}

impl TryFrom<&Memo> for SocialMessage {
    type Error = SocialParseError;

    fn try_from(memo: &Memo) -> Result<Self, Self::Error> {
        let bytes = &memo.0[..];

        // Find the actual content length (trim trailing zeros)
        let content_len = bytes
            .iter()
            .rposition(|&b| b != 0)
            .map(|pos| pos + 1)
            .unwrap_or(0);

        if content_len == 0 {
            return Err(SocialParseError::Empty);
        }

        if content_len < MIN_SOCIAL_MESSAGE_SIZE {
            return Err(SocialParseError::TooShort {
                actual: content_len,
                minimum: MIN_SOCIAL_MESSAGE_SIZE,
            });
        }

        let type_byte = bytes[0];
        let version = bytes[1];

        // Check if this looks like a social message (type in valid range)
        // Social messages use 0x10-0xFE range:
        // - 0x10-0x7F: Standard message types
        // - 0x80-0xEF: Experimental/extended types (batching, governance, channels)
        // - 0xF0-0xFE: Recovery and advanced features
        if type_byte < 0x10 || type_byte > 0xFE {
            return Err(SocialParseError::NotSocialMessage);
        }

        // Parse the message type
        let msg_type = SocialMessageType::try_from(type_byte)?;

        // Check version compatibility
        if version > SOCIAL_PROTOCOL_VERSION {
            return Err(SocialParseError::UnsupportedVersion {
                found: version,
                max_supported: SOCIAL_PROTOCOL_VERSION,
            });
        }

        // Extract payload (everything after type and version, excluding trailing zeros)
        let payload = bytes[2..content_len].to_vec();

        Ok(SocialMessage {
            msg_type,
            version,
            payload,
        })
    }
}

impl fmt::Display for SocialMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SocialMessage {{ type: {}, version: {}, payload_len: {} }}",
            self.msg_type,
            self.version,
            self.payload.len()
        )
    }
}

/// Errors that can occur when parsing a batch message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BatchParseError {
    /// The batch is empty (no actions).
    EmptyBatch,

    /// Too many actions in the batch.
    TooManyActions {
        /// The number of actions found.
        count: usize,
        /// The maximum allowed.
        max: usize,
    },

    /// The batch payload is truncated.
    TruncatedPayload {
        /// Expected bytes remaining.
        expected: usize,
        /// Actual bytes available.
        available: usize,
    },

    /// An action within the batch failed to parse.
    ActionParseError {
        /// Zero-based index of the failing action.
        index: usize,
        /// The underlying parse error.
        error: SocialParseError,
    },

    /// Nested batches are not allowed.
    NestedBatch {
        /// Index of the nested batch action.
        index: usize,
    },
}

impl fmt::Display for BatchParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyBatch => write!(f, "batch contains no actions"),
            Self::TooManyActions { count, max } => {
                write!(f, "batch has {} actions, maximum is {}", count, max)
            }
            Self::TruncatedPayload {
                expected,
                available,
            } => {
                write!(
                    f,
                    "batch payload truncated: expected {} bytes, only {} available",
                    expected, available
                )
            }
            Self::ActionParseError { index, error } => {
                write!(f, "failed to parse action {}: {}", index, error)
            }
            Self::NestedBatch { index } => {
                write!(f, "nested batch at action {} is not allowed", index)
            }
        }
    }
}

impl std::error::Error for BatchParseError {}

/// A batch of multiple social actions in a single transaction.
///
/// Batching reduces transaction fees and chain bloat by combining multiple
/// social actions (posts, follows, upvotes, etc.) into a single transaction.
///
/// # Format
///
/// ```text
/// ┌────────┬─────────┬───────┬─────────────────────────────────────────┐
/// │ Type   │ Version │ Count │ Actions                                  │
/// │ 0x80   │ 1 byte  │ 1 byte│ [len(2)][action]...[len(2)][action]     │
/// └────────┴─────────┴───────┴─────────────────────────────────────────┘
/// ```
///
/// Each action is prefixed with a 2-byte length (little-endian u16).
///
/// # Constraints
///
/// - Maximum 5 actions per batch
/// - Nested batches are not allowed
/// - Total encoded size must fit in memo field (~510 bytes)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BatchMessage {
    /// The protocol version of this batch.
    version: u8,

    /// The actions contained in this batch.
    actions: Vec<SocialMessage>,
}

impl BatchMessage {
    /// Creates a new batch message with the given actions.
    ///
    /// # Arguments
    ///
    /// * `actions` - The social actions to batch (1 to MAX_BATCH_ACTIONS)
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The actions list is empty
    /// - There are more than MAX_BATCH_ACTIONS actions
    /// - Any action is itself a batch (nested batches not allowed)
    pub fn new(actions: Vec<SocialMessage>) -> Result<Self, BatchParseError> {
        if actions.is_empty() {
            return Err(BatchParseError::EmptyBatch);
        }

        if actions.len() > MAX_BATCH_ACTIONS {
            return Err(BatchParseError::TooManyActions {
                count: actions.len(),
                max: MAX_BATCH_ACTIONS,
            });
        }

        // Check for nested batches
        for (index, action) in actions.iter().enumerate() {
            if action.msg_type().is_batch() {
                return Err(BatchParseError::NestedBatch { index });
            }
        }

        Ok(Self {
            version: SOCIAL_PROTOCOL_VERSION,
            actions,
        })
    }

    /// Returns the protocol version of this batch.
    #[inline]
    pub fn version(&self) -> u8 {
        self.version
    }

    /// Returns the actions contained in this batch.
    #[inline]
    pub fn actions(&self) -> &[SocialMessage] {
        &self.actions
    }

    /// Returns the number of actions in this batch.
    #[inline]
    pub fn len(&self) -> usize {
        self.actions.len()
    }

    /// Returns true if this batch is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    /// Encodes this batch message into bytes suitable for a memo field.
    ///
    /// The encoding is:
    /// `[0x80][version][count][len1_lo][len1_hi][action1]...[lenN_lo][lenN_hi][actionN]`
    pub fn encode(&self) -> Vec<u8> {
        // Calculate total size
        let actions_size: usize = self
            .actions
            .iter()
            .map(|a| BATCH_ACTION_LENGTH_SIZE + a.encode().len())
            .sum();
        let total_size = 3 + actions_size; // type + version + count + actions

        let mut bytes = Vec::with_capacity(total_size);
        bytes.push(SocialMessageType::Batch.as_u8());
        bytes.push(self.version);
        bytes.push(self.actions.len() as u8);

        for action in &self.actions {
            let encoded = action.encode();
            let len = encoded.len() as u16;
            bytes.extend_from_slice(&len.to_le_bytes());
            bytes.extend_from_slice(&encoded);
        }

        bytes
    }

    /// Parses a batch message from a memo.
    ///
    /// # Arguments
    ///
    /// * `memo` - The memo to parse
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The memo is not a batch message (wrong type byte)
    /// - The batch is malformed
    /// - Any contained action fails to parse
    pub fn try_from_memo(memo: &Memo) -> Result<Self, BatchParseError> {
        let bytes = &memo.0[..];

        // Find actual content length (trim trailing zeros)
        let content_len = bytes
            .iter()
            .rposition(|&b| b != 0)
            .map(|pos| pos + 1)
            .unwrap_or(0);

        if content_len < MIN_BATCH_MESSAGE_SIZE {
            return Err(BatchParseError::TruncatedPayload {
                expected: MIN_BATCH_MESSAGE_SIZE,
                available: content_len,
            });
        }

        // Verify this is a batch message
        if bytes[0] != SocialMessageType::Batch.as_u8() {
            return Err(BatchParseError::ActionParseError {
                index: 0,
                error: SocialParseError::NotSocialMessage,
            });
        }

        let version = bytes[1];
        let count = bytes[2] as usize;

        if count == 0 {
            return Err(BatchParseError::EmptyBatch);
        }

        if count > MAX_BATCH_ACTIONS {
            return Err(BatchParseError::TooManyActions {
                count,
                max: MAX_BATCH_ACTIONS,
            });
        }

        // Parse each action
        let mut actions = Vec::with_capacity(count);
        let mut offset = 3; // After type, version, count

        for index in 0..count {
            // Read action length (2 bytes, little-endian)
            if offset + BATCH_ACTION_LENGTH_SIZE > content_len {
                return Err(BatchParseError::TruncatedPayload {
                    expected: BATCH_ACTION_LENGTH_SIZE,
                    available: content_len - offset,
                });
            }

            let action_len =
                u16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as usize;
            offset += BATCH_ACTION_LENGTH_SIZE;

            // Read action bytes
            if offset + action_len > content_len {
                return Err(BatchParseError::TruncatedPayload {
                    expected: action_len,
                    available: content_len - offset,
                });
            }

            let action_bytes = &bytes[offset..offset + action_len];
            offset += action_len;

            // Parse the action - create a temporary memo for parsing
            if action_bytes.len() < MIN_SOCIAL_MESSAGE_SIZE {
                return Err(BatchParseError::ActionParseError {
                    index,
                    error: SocialParseError::TooShort {
                        actual: action_bytes.len(),
                        minimum: MIN_SOCIAL_MESSAGE_SIZE,
                    },
                });
            }

            let type_byte = action_bytes[0];
            let action_version = action_bytes[1];

            // Check for nested batch
            if type_byte == SocialMessageType::Batch.as_u8() {
                return Err(BatchParseError::NestedBatch { index });
            }

            // Parse message type
            let msg_type = SocialMessageType::try_from(type_byte).map_err(|e| {
                BatchParseError::ActionParseError { index, error: e }
            })?;

            // Check version
            if action_version > SOCIAL_PROTOCOL_VERSION {
                return Err(BatchParseError::ActionParseError {
                    index,
                    error: SocialParseError::UnsupportedVersion {
                        found: action_version,
                        max_supported: SOCIAL_PROTOCOL_VERSION,
                    },
                });
            }

            let payload = action_bytes[2..].to_vec();
            actions.push(SocialMessage::new(msg_type, action_version, payload));
        }

        Ok(Self { version, actions })
    }
}

impl fmt::Display for BatchMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BatchMessage {{ version: {}, actions: {} }}",
            self.version,
            self.actions.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_memo(bytes: &[u8]) -> Memo {
        Memo::try_from(bytes).expect("valid memo bytes")
    }

    // Required test: social_message_type tests
    #[test]
    fn social_message_type_values() {
        let _init_guard = zebra_test::init();

        // Verify all message type byte values match spec
        assert_eq!(SocialMessageType::Profile.as_u8(), 0x10);
        assert_eq!(SocialMessageType::Post.as_u8(), 0x20);
        assert_eq!(SocialMessageType::Comment.as_u8(), 0x21);
        assert_eq!(SocialMessageType::Upvote.as_u8(), 0x22);
        assert_eq!(SocialMessageType::Follow.as_u8(), 0x30);
        assert_eq!(SocialMessageType::Unfollow.as_u8(), 0x31);
        assert_eq!(SocialMessageType::Dm.as_u8(), 0x40);
        assert_eq!(SocialMessageType::DmGroup.as_u8(), 0x41);
        assert_eq!(SocialMessageType::Tip.as_u8(), 0x50);
        assert_eq!(SocialMessageType::Bounty.as_u8(), 0x51);
        assert_eq!(SocialMessageType::AttentionBoost.as_u8(), 0x52);
        assert_eq!(SocialMessageType::CreditTip.as_u8(), 0x53);
        assert_eq!(SocialMessageType::CreditClaim.as_u8(), 0x54);
        assert_eq!(SocialMessageType::Media.as_u8(), 0x60);
        assert_eq!(SocialMessageType::Poll.as_u8(), 0x70);
        assert_eq!(SocialMessageType::Vote.as_u8(), 0x71);
        assert_eq!(SocialMessageType::GovernanceVote.as_u8(), 0xE0);
        assert_eq!(SocialMessageType::GovernanceProposal.as_u8(), 0xE1);
    }

    #[test]
    fn social_message_type_roundtrip() {
        let _init_guard = zebra_test::init();

        let types = [
            SocialMessageType::Profile,
            SocialMessageType::Post,
            SocialMessageType::Comment,
            SocialMessageType::Upvote,
            SocialMessageType::Follow,
            SocialMessageType::Unfollow,
            SocialMessageType::Dm,
            SocialMessageType::DmGroup,
            SocialMessageType::Tip,
            SocialMessageType::Bounty,
            SocialMessageType::AttentionBoost,
            SocialMessageType::CreditTip,
            SocialMessageType::CreditClaim,
            SocialMessageType::Media,
            SocialMessageType::Poll,
            SocialMessageType::Vote,
            SocialMessageType::Batch,
            SocialMessageType::ChannelOpen,
            SocialMessageType::ChannelClose,
            SocialMessageType::ChannelSettle,
            SocialMessageType::GovernanceVote,
            SocialMessageType::GovernanceProposal,
            SocialMessageType::RecoveryConfig,
            SocialMessageType::RecoveryRequest,
            SocialMessageType::RecoveryApprove,
            SocialMessageType::RecoveryCancel,
            SocialMessageType::KeyRotation,
            SocialMessageType::Trust,
            SocialMessageType::Report,
        ];

        for msg_type in types {
            let byte = msg_type.as_u8();
            let parsed = SocialMessageType::try_from(byte).expect("should parse");
            assert_eq!(parsed, msg_type);
        }
    }

    #[test]
    fn social_message_type_unknown() {
        let _init_guard = zebra_test::init();

        // Test various invalid type bytes (0xF4 is now KeyRotation, so remove it from invalid)
        let invalid_types: &[u8] = &[0x00, 0x0F, 0x11, 0x23, 0x32, 0x42, 0x55, 0x61, 0x72, 0xFF];

        for &byte in invalid_types {
            let result = SocialMessageType::try_from(byte);
            assert!(
                result.is_err(),
                "byte 0x{:02X} should not parse as valid type",
                byte
            );
            if let Err(SocialParseError::UnknownMessageType(b)) = result {
                assert_eq!(b, byte);
            }
        }
    }

    #[test]
    fn social_message_type_categories() {
        let _init_guard = zebra_test::init();

        // Value transfer types
        assert!(SocialMessageType::Tip.is_value_transfer());
        assert!(SocialMessageType::Bounty.is_value_transfer());
        assert!(SocialMessageType::AttentionBoost.is_value_transfer());
        assert!(SocialMessageType::CreditTip.is_value_transfer());
        assert!(SocialMessageType::CreditClaim.is_value_transfer());
        assert!(SocialMessageType::Upvote.is_value_transfer());

        // Non-value transfer types
        assert!(!SocialMessageType::Profile.is_value_transfer());
        assert!(!SocialMessageType::Post.is_value_transfer());
        assert!(!SocialMessageType::Follow.is_value_transfer());

        // Attention market types
        assert!(SocialMessageType::AttentionBoost.is_attention_market());
        assert!(SocialMessageType::CreditTip.is_attention_market());
        assert!(SocialMessageType::CreditClaim.is_attention_market());

        // Non-attention market types
        assert!(!SocialMessageType::Tip.is_attention_market());
        assert!(!SocialMessageType::Post.is_attention_market());

        // Governance types
        assert!(SocialMessageType::GovernanceVote.is_governance());
        assert!(SocialMessageType::GovernanceProposal.is_governance());

        // Non-governance types
        assert!(!SocialMessageType::Post.is_governance());
        assert!(!SocialMessageType::Vote.is_governance()); // Poll vote, not governance vote
    }

    // Required test: parse post message
    #[test]
    fn parse_post_message() {
        let _init_guard = zebra_test::init();

        let content = b"Hello Botcash!";
        let mut memo_bytes = vec![0x20, 0x01]; // Post type, version 1
        memo_bytes.extend_from_slice(content);

        let memo = create_memo(&memo_bytes);
        let msg = SocialMessage::try_from(&memo).expect("should parse post");

        assert_eq!(msg.msg_type(), SocialMessageType::Post);
        assert_eq!(msg.version(), 1);
        assert_eq!(msg.payload(), content);
    }

    // Required test: parse dm message
    #[test]
    fn parse_dm_message() {
        let _init_guard = zebra_test::init();

        let content = b"Private message";
        let mut memo_bytes = vec![0x40, 0x01]; // DM type, version 1
        memo_bytes.extend_from_slice(content);

        let memo = create_memo(&memo_bytes);
        let msg = SocialMessage::try_from(&memo).expect("should parse DM");

        assert_eq!(msg.msg_type(), SocialMessageType::Dm);
        assert_eq!(msg.version(), 1);
        assert_eq!(msg.payload(), content);
    }

    #[test]
    fn parse_attention_boost() {
        let _init_guard = zebra_test::init();

        // Attention boost: target txid (32 bytes) + duration (4 bytes) + category (1 byte)
        let mut memo_bytes = vec![0x52, 0x01]; // AttentionBoost type, version 1
        memo_bytes.extend_from_slice(&[0xAB; 32]); // target txid
        memo_bytes.extend_from_slice(&[0x00, 0x01, 0x00, 0x00]); // duration: 256 blocks
        memo_bytes.push(0x01); // category: Services

        let memo = create_memo(&memo_bytes);
        let msg = SocialMessage::try_from(&memo).expect("should parse attention boost");

        assert_eq!(msg.msg_type(), SocialMessageType::AttentionBoost);
        assert!(msg.is_attention_market());
        assert!(msg.is_value_transfer());
        assert_eq!(msg.payload().len(), 37); // 32 + 4 + 1
    }

    #[test]
    fn parse_empty_memo() {
        let _init_guard = zebra_test::init();

        let memo = create_memo(&[]);
        let result = SocialMessage::try_from(&memo);

        assert!(matches!(result, Err(SocialParseError::Empty)));
    }

    #[test]
    fn parse_too_short_memo() {
        let _init_guard = zebra_test::init();

        // Only 1 byte - needs at least 2 (type + version)
        let memo = create_memo(&[0x20]);
        let result = SocialMessage::try_from(&memo);

        assert!(matches!(
            result,
            Err(SocialParseError::TooShort {
                actual: 1,
                minimum: 2
            })
        ));
    }

    #[test]
    fn parse_non_social_memo() {
        let _init_guard = zebra_test::init();

        // First byte 0x00 is not in social range
        let memo = create_memo(&[0x00, 0x01, 0x02, 0x03]);
        let result = SocialMessage::try_from(&memo);

        assert!(matches!(result, Err(SocialParseError::NotSocialMessage)));
    }

    #[test]
    fn parse_unsupported_version() {
        let _init_guard = zebra_test::init();

        // Version 99 is not supported
        let memo = create_memo(&[0x20, 99, b'H', b'i']);
        let result = SocialMessage::try_from(&memo);

        assert!(matches!(
            result,
            Err(SocialParseError::UnsupportedVersion {
                found: 99,
                max_supported: 1
            })
        ));
    }

    #[test]
    fn social_message_encode_roundtrip() {
        let _init_guard = zebra_test::init();

        let original = SocialMessage::new(
            SocialMessageType::Post,
            SOCIAL_PROTOCOL_VERSION,
            b"Test post content".to_vec(),
        );

        let encoded = original.encode();
        assert_eq!(encoded[0], 0x20); // Post type
        assert_eq!(encoded[1], SOCIAL_PROTOCOL_VERSION);
        assert_eq!(&encoded[2..], b"Test post content");

        let memo = create_memo(&encoded);
        let decoded = SocialMessage::try_from(&memo).expect("should decode");

        assert_eq!(decoded.msg_type(), original.msg_type());
        assert_eq!(decoded.version(), original.version());
        assert_eq!(decoded.payload(), original.payload());
    }

    #[test]
    fn social_message_type_display() {
        let _init_guard = zebra_test::init();

        assert_eq!(format!("{}", SocialMessageType::Profile), "Profile");
        assert_eq!(format!("{}", SocialMessageType::Post), "Post");
        assert_eq!(format!("{}", SocialMessageType::Dm), "DM");
        assert_eq!(
            format!("{}", SocialMessageType::AttentionBoost),
            "AttentionBoost"
        );
    }

    #[test]
    fn social_parse_error_display() {
        let _init_guard = zebra_test::init();

        let err = SocialParseError::Empty;
        assert_eq!(format!("{}", err), "memo is empty");

        let err = SocialParseError::UnknownMessageType(0xFF);
        assert_eq!(format!("{}", err), "unknown social message type: 0xFF");

        let err = SocialParseError::TooShort {
            actual: 1,
            minimum: 2,
        };
        assert_eq!(
            format!("{}", err),
            "memo too short: 1 bytes, minimum 2 required"
        );
    }

    #[test]
    fn all_message_types_exist_pre_recovery() {
        let _init_guard = zebra_test::init();

        // Verify the 22 pre-recovery message types (16 core + 1 batch + 3 channels + 2 governance)
        // Note: Recovery adds 4 more types, tested in all_26_message_types_exist
        let pre_recovery_types = [
            SocialMessageType::Profile,
            SocialMessageType::Post,
            SocialMessageType::Comment,
            SocialMessageType::Upvote,
            SocialMessageType::Follow,
            SocialMessageType::Unfollow,
            SocialMessageType::Dm,
            SocialMessageType::DmGroup,
            SocialMessageType::Tip,
            SocialMessageType::Bounty,
            SocialMessageType::AttentionBoost,
            SocialMessageType::CreditTip,
            SocialMessageType::CreditClaim,
            SocialMessageType::Media,
            SocialMessageType::Poll,
            SocialMessageType::Vote,
            SocialMessageType::Batch,
            SocialMessageType::ChannelOpen,
            SocialMessageType::ChannelClose,
            SocialMessageType::ChannelSettle,
            SocialMessageType::GovernanceVote,
            SocialMessageType::GovernanceProposal,
        ];

        assert_eq!(pre_recovery_types.len(), 22, "Should have exactly 22 pre-recovery message types");

        // Verify each has a unique byte value
        let mut seen_bytes = std::collections::HashSet::new();
        for msg_type in pre_recovery_types {
            let byte = msg_type.as_u8();
            assert!(
                seen_bytes.insert(byte),
                "Duplicate byte value: 0x{:02X}",
                byte
            );
        }
    }

    // ========================================================================
    // Batch Message Tests (Required for P6.1 Transaction Batching)
    // ========================================================================

    #[test]
    fn batch_message_type_value() {
        let _init_guard = zebra_test::init();

        assert_eq!(SocialMessageType::Batch.as_u8(), 0x80);
        assert_eq!(SocialMessageType::Batch.name(), "Batch");
        assert!(SocialMessageType::Batch.is_batch());
        assert!(!SocialMessageType::Post.is_batch());
    }

    #[test]
    fn batch_message_encode_roundtrip() {
        let _init_guard = zebra_test::init();

        // Create a batch with 3 different action types
        let actions = vec![
            SocialMessage::new(
                SocialMessageType::Post,
                SOCIAL_PROTOCOL_VERSION,
                b"Hello!".to_vec(),
            ),
            SocialMessage::new(
                SocialMessageType::Follow,
                SOCIAL_PROTOCOL_VERSION,
                b"bs1target...".to_vec(),
            ),
            SocialMessage::new(
                SocialMessageType::Upvote,
                SOCIAL_PROTOCOL_VERSION,
                vec![0xAB; 32], // txid
            ),
        ];

        let batch = BatchMessage::new(actions).expect("valid batch");
        assert_eq!(batch.len(), 3);

        // Encode to bytes
        let encoded = batch.encode();
        assert_eq!(encoded[0], 0x80); // Batch type
        assert_eq!(encoded[1], SOCIAL_PROTOCOL_VERSION);
        assert_eq!(encoded[2], 3); // 3 actions

        // Decode from memo
        let memo = create_memo(&encoded);
        let decoded = BatchMessage::try_from_memo(&memo).expect("should decode");

        assert_eq!(decoded.len(), batch.len());
        assert_eq!(decoded.version(), batch.version());

        // Verify each action
        for (orig, decoded) in batch.actions().iter().zip(decoded.actions().iter()) {
            assert_eq!(decoded.msg_type(), orig.msg_type());
            assert_eq!(decoded.version(), orig.version());
            assert_eq!(decoded.payload(), orig.payload());
        }
    }

    #[test]
    fn batch_message_max_actions() {
        let _init_guard = zebra_test::init();

        // Create exactly MAX_BATCH_ACTIONS actions (should succeed)
        let actions: Vec<_> = (0..MAX_BATCH_ACTIONS)
            .map(|i| {
                SocialMessage::new(
                    SocialMessageType::Post,
                    SOCIAL_PROTOCOL_VERSION,
                    format!("Post {}", i).into_bytes(),
                )
            })
            .collect();

        let batch = BatchMessage::new(actions);
        assert!(batch.is_ok());
        assert_eq!(batch.unwrap().len(), MAX_BATCH_ACTIONS);
    }

    #[test]
    fn batch_message_too_many_actions() {
        let _init_guard = zebra_test::init();

        // Create MAX_BATCH_ACTIONS + 1 actions (should fail)
        let actions: Vec<_> = (0..=MAX_BATCH_ACTIONS)
            .map(|i| {
                SocialMessage::new(
                    SocialMessageType::Post,
                    SOCIAL_PROTOCOL_VERSION,
                    format!("Post {}", i).into_bytes(),
                )
            })
            .collect();

        let result = BatchMessage::new(actions);
        assert!(matches!(
            result,
            Err(BatchParseError::TooManyActions {
                count: 6,
                max: 5
            })
        ));
    }

    #[test]
    fn batch_message_empty() {
        let _init_guard = zebra_test::init();

        let result = BatchMessage::new(vec![]);
        assert!(matches!(result, Err(BatchParseError::EmptyBatch)));
    }

    #[test]
    fn batch_message_nested_not_allowed() {
        let _init_guard = zebra_test::init();

        // Try to create a batch containing another batch type
        let inner_batch_msg = SocialMessage::new(
            SocialMessageType::Batch,
            SOCIAL_PROTOCOL_VERSION,
            vec![0x01, 0x02, 0x03],
        );

        let actions = vec![
            SocialMessage::new(
                SocialMessageType::Post,
                SOCIAL_PROTOCOL_VERSION,
                b"Normal post".to_vec(),
            ),
            inner_batch_msg,
        ];

        let result = BatchMessage::new(actions);
        assert!(matches!(result, Err(BatchParseError::NestedBatch { index: 1 })));
    }

    #[test]
    fn batch_message_mixed_types() {
        let _init_guard = zebra_test::init();

        // Create a batch with all different non-value types
        let actions = vec![
            SocialMessage::new(
                SocialMessageType::Profile,
                SOCIAL_PROTOCOL_VERSION,
                b"display_name=Agent".to_vec(),
            ),
            SocialMessage::new(
                SocialMessageType::Post,
                SOCIAL_PROTOCOL_VERSION,
                b"My first post!".to_vec(),
            ),
            SocialMessage::new(
                SocialMessageType::Follow,
                SOCIAL_PROTOCOL_VERSION,
                b"bs1xyz...".to_vec(),
            ),
            SocialMessage::new(
                SocialMessageType::Dm,
                SOCIAL_PROTOCOL_VERSION,
                b"encrypted_content".to_vec(),
            ),
        ];

        let batch = BatchMessage::new(actions).expect("valid batch");
        let encoded = batch.encode();
        let memo = create_memo(&encoded);
        let decoded = BatchMessage::try_from_memo(&memo).expect("should decode");

        // Verify all types are preserved
        assert_eq!(decoded.actions()[0].msg_type(), SocialMessageType::Profile);
        assert_eq!(decoded.actions()[1].msg_type(), SocialMessageType::Post);
        assert_eq!(decoded.actions()[2].msg_type(), SocialMessageType::Follow);
        assert_eq!(decoded.actions()[3].msg_type(), SocialMessageType::Dm);
    }

    #[test]
    fn batch_message_with_value_transfers() {
        let _init_guard = zebra_test::init();

        // Create a batch combining social actions with value transfers
        let actions = vec![
            SocialMessage::new(
                SocialMessageType::Post,
                SOCIAL_PROTOCOL_VERSION,
                b"Check out this content!".to_vec(),
            ),
            SocialMessage::new(
                SocialMessageType::Tip,
                SOCIAL_PROTOCOL_VERSION,
                vec![0xAB; 32], // target txid
            ),
            SocialMessage::new(
                SocialMessageType::AttentionBoost,
                SOCIAL_PROTOCOL_VERSION,
                {
                    let mut payload = vec![0xCD; 32]; // target txid
                    payload.extend_from_slice(&1440u32.to_le_bytes()); // duration
                    payload.push(0x01); // category
                    payload
                },
            ),
        ];

        let batch = BatchMessage::new(actions).expect("valid batch");
        let encoded = batch.encode();
        let memo = create_memo(&encoded);
        let decoded = BatchMessage::try_from_memo(&memo).expect("should decode");

        // Verify value transfer flags
        assert!(!decoded.actions()[0].is_value_transfer()); // Post
        assert!(decoded.actions()[1].is_value_transfer()); // Tip
        assert!(decoded.actions()[2].is_value_transfer()); // AttentionBoost
        assert!(decoded.actions()[2].is_attention_market());
    }

    #[test]
    fn batch_parse_truncated_payload() {
        let _init_guard = zebra_test::init();

        // Create a valid batch
        let actions = vec![SocialMessage::new(
            SocialMessageType::Post,
            SOCIAL_PROTOCOL_VERSION,
            b"Hello".to_vec(),
        )];
        let batch = BatchMessage::new(actions).expect("valid batch");
        let encoded = batch.encode();

        // Truncate the encoded bytes (remove the action data)
        let truncated = &encoded[..5]; // Only type, version, count, and partial length
        let memo = create_memo(truncated);
        let result = BatchMessage::try_from_memo(&memo);

        assert!(matches!(
            result,
            Err(BatchParseError::TruncatedPayload { .. })
        ));
    }

    #[test]
    fn batch_parse_error_display() {
        let _init_guard = zebra_test::init();

        let err = BatchParseError::EmptyBatch;
        assert_eq!(format!("{}", err), "batch contains no actions");

        let err = BatchParseError::TooManyActions { count: 10, max: 5 };
        assert_eq!(
            format!("{}", err),
            "batch has 10 actions, maximum is 5"
        );

        let err = BatchParseError::NestedBatch { index: 2 };
        assert_eq!(
            format!("{}", err),
            "nested batch at action 2 is not allowed"
        );
    }

    #[test]
    fn batch_message_single_action() {
        let _init_guard = zebra_test::init();

        // Even a single action batch should work
        let actions = vec![SocialMessage::new(
            SocialMessageType::Follow,
            SOCIAL_PROTOCOL_VERSION,
            b"bs1target".to_vec(),
        )];

        let batch = BatchMessage::new(actions).expect("valid batch");
        assert_eq!(batch.len(), 1);
        assert!(!batch.is_empty());

        let encoded = batch.encode();
        let memo = create_memo(&encoded);
        let decoded = BatchMessage::try_from_memo(&memo).expect("should decode");

        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded.actions()[0].msg_type(), SocialMessageType::Follow);
    }

    #[test]
    fn batch_message_display() {
        let _init_guard = zebra_test::init();

        let actions = vec![
            SocialMessage::new(
                SocialMessageType::Post,
                SOCIAL_PROTOCOL_VERSION,
                b"Hi".to_vec(),
            ),
            SocialMessage::new(
                SocialMessageType::Follow,
                SOCIAL_PROTOCOL_VERSION,
                b"target".to_vec(),
            ),
        ];

        let batch = BatchMessage::new(actions).expect("valid batch");
        let display = format!("{}", batch);
        assert!(display.contains("BatchMessage"));
        assert!(display.contains("version: 1"));
        assert!(display.contains("actions: 2"));
    }

    // ========================================================================
    // Governance Message Tests (Required for P6.2 On-Chain Voting)
    // ========================================================================

    #[test]
    fn governance_message_type_values() {
        let _init_guard = zebra_test::init();

        // Verify governance type byte values match spec
        assert_eq!(SocialMessageType::GovernanceVote.as_u8(), 0xE0);
        assert_eq!(SocialMessageType::GovernanceProposal.as_u8(), 0xE1);

        // Verify names
        assert_eq!(SocialMessageType::GovernanceVote.name(), "GovernanceVote");
        assert_eq!(
            SocialMessageType::GovernanceProposal.name(),
            "GovernanceProposal"
        );
    }

    #[test]
    fn governance_vote_message_roundtrip() {
        let _init_guard = zebra_test::init();

        // GovernanceVote format: [proposal_id(32)][vote(1)][weight(8)]
        // Use a weight with non-zero high bytes to avoid trailing zero trimming
        let weight: u64 = 0x0102030405060708;
        let mut payload = Vec::new();
        payload.extend_from_slice(&[0xAB; 32]); // proposal_id (32 bytes)
        payload.push(0x01); // vote: 1 = yes
        payload.extend_from_slice(&weight.to_le_bytes()); // weight with all non-zero bytes

        let msg = SocialMessage::new(
            SocialMessageType::GovernanceVote,
            SOCIAL_PROTOCOL_VERSION,
            payload.clone(),
        );

        let encoded = msg.encode();
        assert_eq!(encoded[0], 0xE0); // GovernanceVote type
        assert_eq!(encoded[1], SOCIAL_PROTOCOL_VERSION);

        let memo = create_memo(&encoded);
        let decoded = SocialMessage::try_from(&memo).expect("should decode");

        assert_eq!(decoded.msg_type(), SocialMessageType::GovernanceVote);
        assert!(decoded.msg_type().is_governance());
        assert_eq!(decoded.payload(), payload.as_slice());
    }

    #[test]
    fn governance_proposal_message_roundtrip() {
        let _init_guard = zebra_test::init();

        // GovernanceProposal format: [proposal_type(1)][title_len(1)][title][desc_len(2)][description]
        let mut payload = Vec::new();
        payload.push(0x01); // proposal_type: 1 = parameter change

        let title = b"Increase block size";
        payload.push(title.len() as u8);
        payload.extend_from_slice(title);

        let description = b"Proposal to increase max block size from 2MB to 4MB";
        payload.extend_from_slice(&(description.len() as u16).to_le_bytes());
        payload.extend_from_slice(description);

        let msg = SocialMessage::new(
            SocialMessageType::GovernanceProposal,
            SOCIAL_PROTOCOL_VERSION,
            payload.clone(),
        );

        let encoded = msg.encode();
        assert_eq!(encoded[0], 0xE1); // GovernanceProposal type
        assert_eq!(encoded[1], SOCIAL_PROTOCOL_VERSION);

        let memo = create_memo(&encoded);
        let decoded = SocialMessage::try_from(&memo).expect("should decode");

        assert_eq!(decoded.msg_type(), SocialMessageType::GovernanceProposal);
        assert!(decoded.msg_type().is_governance());
        assert_eq!(decoded.payload(), payload.as_slice());
    }

    #[test]
    fn governance_types_in_batch() {
        let _init_guard = zebra_test::init();

        // Governance messages can be batched with other actions
        let actions = vec![
            SocialMessage::new(
                SocialMessageType::GovernanceVote,
                SOCIAL_PROTOCOL_VERSION,
                {
                    let mut payload = vec![0xAB; 32]; // proposal_id
                    payload.push(0x01); // vote: yes
                    payload.extend_from_slice(&50u64.to_le_bytes()); // weight
                    payload
                },
            ),
            SocialMessage::new(
                SocialMessageType::Post,
                SOCIAL_PROTOCOL_VERSION,
                b"Voted yes on BIP-001!".to_vec(),
            ),
        ];

        let batch = BatchMessage::new(actions).expect("valid batch");
        let encoded = batch.encode();
        let memo = create_memo(&encoded);
        let decoded = BatchMessage::try_from_memo(&memo).expect("should decode");

        assert_eq!(decoded.len(), 2);
        assert_eq!(
            decoded.actions()[0].msg_type(),
            SocialMessageType::GovernanceVote
        );
        assert!(decoded.actions()[0].msg_type().is_governance());
        assert_eq!(decoded.actions()[1].msg_type(), SocialMessageType::Post);
    }

    #[test]
    fn governance_types_not_value_transfer() {
        let _init_guard = zebra_test::init();

        // Governance types are not value transfers (deposit is separate)
        assert!(!SocialMessageType::GovernanceVote.is_value_transfer());
        assert!(!SocialMessageType::GovernanceProposal.is_value_transfer());

        // Governance types are not attention market
        assert!(!SocialMessageType::GovernanceVote.is_attention_market());
        assert!(!SocialMessageType::GovernanceProposal.is_attention_market());
    }

    #[test]
    fn governance_vote_choices() {
        let _init_guard = zebra_test::init();

        // Test all vote choices: 0 = no, 1 = yes, 2 = abstain
        for vote_choice in [0u8, 1u8, 2u8] {
            let mut payload = vec![0xCD; 32]; // proposal_id
            payload.push(vote_choice);
            payload.extend_from_slice(&1000u64.to_le_bytes()); // weight

            let msg = SocialMessage::new(
                SocialMessageType::GovernanceVote,
                SOCIAL_PROTOCOL_VERSION,
                payload,
            );

            let encoded = msg.encode();
            let memo = create_memo(&encoded);
            let decoded = SocialMessage::try_from(&memo).expect("should decode");

            assert_eq!(decoded.msg_type(), SocialMessageType::GovernanceVote);
            // Verify vote choice is preserved
            assert_eq!(decoded.payload()[32], vote_choice);
        }
    }

    #[test]
    fn governance_type_display() {
        let _init_guard = zebra_test::init();

        assert_eq!(format!("{}", SocialMessageType::GovernanceVote), "GovernanceVote");
        assert_eq!(
            format!("{}", SocialMessageType::GovernanceProposal),
            "GovernanceProposal"
        );
    }

    // ========================================================================
    // Channel Message Tests (Required for P6.2 Layer-2 Social Channels)
    // ========================================================================

    #[test]
    fn channel_message_type_values() {
        let _init_guard = zebra_test::init();

        // Verify channel type byte values match spec
        assert_eq!(SocialMessageType::ChannelOpen.as_u8(), 0xC0);
        assert_eq!(SocialMessageType::ChannelClose.as_u8(), 0xC1);
        assert_eq!(SocialMessageType::ChannelSettle.as_u8(), 0xC2);

        // Verify names
        assert_eq!(SocialMessageType::ChannelOpen.name(), "ChannelOpen");
        assert_eq!(SocialMessageType::ChannelClose.name(), "ChannelClose");
        assert_eq!(SocialMessageType::ChannelSettle.name(), "ChannelSettle");
    }

    #[test]
    fn channel_open_message_roundtrip() {
        let _init_guard = zebra_test::init();

        // ChannelOpen format: [parties_count(1)][party1_addr_len(1)][party1_addr]...[deposit(8)][timeout_blocks(4)]
        let mut payload = Vec::new();
        payload.push(2); // 2 parties

        let alice = b"bs1alice...";
        payload.push(alice.len() as u8);
        payload.extend_from_slice(alice);

        let bob = b"bs1bob...";
        payload.push(bob.len() as u8);
        payload.extend_from_slice(bob);

        // Use values that don't have trailing zeros to avoid memo trimming issues
        let deposit: u64 = 0x0102030405060708; // Non-zero in all bytes
        payload.extend_from_slice(&deposit.to_le_bytes());

        let timeout_blocks: u32 = 0x01020304; // Non-zero in all bytes
        payload.extend_from_slice(&timeout_blocks.to_le_bytes());

        let msg = SocialMessage::new(
            SocialMessageType::ChannelOpen,
            SOCIAL_PROTOCOL_VERSION,
            payload.clone(),
        );

        let encoded = msg.encode();
        assert_eq!(encoded[0], 0xC0); // ChannelOpen type
        assert_eq!(encoded[1], SOCIAL_PROTOCOL_VERSION);

        let memo = create_memo(&encoded);
        let decoded = SocialMessage::try_from(&memo).expect("should decode");

        assert_eq!(decoded.msg_type(), SocialMessageType::ChannelOpen);
        assert!(decoded.msg_type().is_channel());
        assert_eq!(decoded.payload(), payload.as_slice());
    }

    #[test]
    fn channel_close_message_roundtrip() {
        let _init_guard = zebra_test::init();

        // ChannelClose format: [channel_id(32)][final_seq(4)]
        let mut payload = Vec::new();
        payload.extend_from_slice(&[0xAB; 32]); // channel_id (32 bytes)

        // Use a value that doesn't have trailing zeros to avoid memo trimming issues
        let final_seq: u32 = 0x01020304;
        payload.extend_from_slice(&final_seq.to_le_bytes());

        let msg = SocialMessage::new(
            SocialMessageType::ChannelClose,
            SOCIAL_PROTOCOL_VERSION,
            payload.clone(),
        );

        let encoded = msg.encode();
        assert_eq!(encoded[0], 0xC1); // ChannelClose type
        assert_eq!(encoded[1], SOCIAL_PROTOCOL_VERSION);

        let memo = create_memo(&encoded);
        let decoded = SocialMessage::try_from(&memo).expect("should decode");

        assert_eq!(decoded.msg_type(), SocialMessageType::ChannelClose);
        assert!(decoded.msg_type().is_channel());
        assert_eq!(decoded.payload(), payload.as_slice());
    }

    #[test]
    fn channel_settle_message_roundtrip() {
        let _init_guard = zebra_test::init();

        // ChannelSettle format: [channel_id(32)][final_seq(4)][message_hash(32)]
        let mut payload = Vec::new();
        payload.extend_from_slice(&[0xCD; 32]); // channel_id

        let final_seq: u32 = 100;
        payload.extend_from_slice(&final_seq.to_le_bytes());

        payload.extend_from_slice(&[0xEF; 32]); // merkle root of messages

        let msg = SocialMessage::new(
            SocialMessageType::ChannelSettle,
            SOCIAL_PROTOCOL_VERSION,
            payload.clone(),
        );

        let encoded = msg.encode();
        assert_eq!(encoded[0], 0xC2); // ChannelSettle type
        assert_eq!(encoded[1], SOCIAL_PROTOCOL_VERSION);

        let memo = create_memo(&encoded);
        let decoded = SocialMessage::try_from(&memo).expect("should decode");

        assert_eq!(decoded.msg_type(), SocialMessageType::ChannelSettle);
        assert!(decoded.msg_type().is_channel());
        assert_eq!(decoded.payload(), payload.as_slice());
    }

    #[test]
    fn channel_types_in_batch() {
        let _init_guard = zebra_test::init();

        // Channel messages can be batched with other actions
        let actions = vec![
            SocialMessage::new(
                SocialMessageType::ChannelOpen,
                SOCIAL_PROTOCOL_VERSION,
                {
                    let mut payload = vec![2u8]; // 2 parties
                    payload.push(10);
                    payload.extend_from_slice(b"bs1alice..");
                    payload.push(8);
                    payload.extend_from_slice(b"bs1bob..");
                    payload.extend_from_slice(&100u64.to_le_bytes());
                    payload.extend_from_slice(&1440u32.to_le_bytes());
                    payload
                },
            ),
            SocialMessage::new(
                SocialMessageType::Post,
                SOCIAL_PROTOCOL_VERSION,
                b"Opened a new chat channel!".to_vec(),
            ),
        ];

        let batch = BatchMessage::new(actions).expect("valid batch");
        let encoded = batch.encode();
        let memo = create_memo(&encoded);
        let decoded = BatchMessage::try_from_memo(&memo).expect("should decode");

        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded.actions()[0].msg_type(), SocialMessageType::ChannelOpen);
        assert!(decoded.actions()[0].msg_type().is_channel());
        assert_eq!(decoded.actions()[1].msg_type(), SocialMessageType::Post);
    }

    #[test]
    fn channel_types_not_value_transfer() {
        let _init_guard = zebra_test::init();

        // Channel types are not value transfers (deposits are handled separately)
        assert!(!SocialMessageType::ChannelOpen.is_value_transfer());
        assert!(!SocialMessageType::ChannelClose.is_value_transfer());
        assert!(!SocialMessageType::ChannelSettle.is_value_transfer());

        // Channel types are not attention market
        assert!(!SocialMessageType::ChannelOpen.is_attention_market());
        assert!(!SocialMessageType::ChannelClose.is_attention_market());
        assert!(!SocialMessageType::ChannelSettle.is_attention_market());

        // Channel types are not governance
        assert!(!SocialMessageType::ChannelOpen.is_governance());
        assert!(!SocialMessageType::ChannelClose.is_governance());
        assert!(!SocialMessageType::ChannelSettle.is_governance());

        // Channel types are not batch
        assert!(!SocialMessageType::ChannelOpen.is_batch());
        assert!(!SocialMessageType::ChannelClose.is_batch());
        assert!(!SocialMessageType::ChannelSettle.is_batch());
    }

    #[test]
    fn channel_is_channel_helper() {
        let _init_guard = zebra_test::init();

        // Channel types should return true
        assert!(SocialMessageType::ChannelOpen.is_channel());
        assert!(SocialMessageType::ChannelClose.is_channel());
        assert!(SocialMessageType::ChannelSettle.is_channel());

        // Non-channel types should return false
        assert!(!SocialMessageType::Post.is_channel());
        assert!(!SocialMessageType::Dm.is_channel());
        assert!(!SocialMessageType::Batch.is_channel());
        assert!(!SocialMessageType::GovernanceVote.is_channel());
    }

    #[test]
    fn channel_type_display() {
        let _init_guard = zebra_test::init();

        assert_eq!(format!("{}", SocialMessageType::ChannelOpen), "ChannelOpen");
        assert_eq!(format!("{}", SocialMessageType::ChannelClose), "ChannelClose");
        assert_eq!(format!("{}", SocialMessageType::ChannelSettle), "ChannelSettle");
    }

    #[test]
    fn channel_open_with_group() {
        let _init_guard = zebra_test::init();

        // Test channel with multiple parties (group chat scenario)
        let mut payload = Vec::new();
        payload.push(5); // 5 parties

        let parties = [
            b"bs1alice....".as_slice(),
            b"bs1bob......",
            b"bs1charlie..",
            b"bs1dave.....",
            b"bs1eve......",
        ];

        for party in parties {
            payload.push(party.len() as u8);
            payload.extend_from_slice(party);
        }

        let deposit: u64 = 500_000_000; // 5 BCASH total deposit
        payload.extend_from_slice(&deposit.to_le_bytes());

        let timeout_blocks: u32 = 10080; // ~7 days
        payload.extend_from_slice(&timeout_blocks.to_le_bytes());

        let msg = SocialMessage::new(
            SocialMessageType::ChannelOpen,
            SOCIAL_PROTOCOL_VERSION,
            payload.clone(),
        );

        let encoded = msg.encode();
        let memo = create_memo(&encoded);
        let decoded = SocialMessage::try_from(&memo).expect("should decode");

        assert_eq!(decoded.msg_type(), SocialMessageType::ChannelOpen);
        assert!(decoded.msg_type().is_channel());

        // Verify first byte is party count
        assert_eq!(decoded.payload()[0], 5);
    }

    // ========================================================================
    // Recovery Message Tests (Required for P6.4 Social Recovery)
    // ========================================================================

    #[test]
    fn recovery_message_type_values() {
        let _init_guard = zebra_test::init();

        // Verify recovery type byte values match spec
        assert_eq!(SocialMessageType::RecoveryConfig.as_u8(), 0xF0);
        assert_eq!(SocialMessageType::RecoveryRequest.as_u8(), 0xF1);
        assert_eq!(SocialMessageType::RecoveryApprove.as_u8(), 0xF2);
        assert_eq!(SocialMessageType::RecoveryCancel.as_u8(), 0xF3);

        // Verify names
        assert_eq!(SocialMessageType::RecoveryConfig.name(), "RecoveryConfig");
        assert_eq!(SocialMessageType::RecoveryRequest.name(), "RecoveryRequest");
        assert_eq!(SocialMessageType::RecoveryApprove.name(), "RecoveryApprove");
        assert_eq!(SocialMessageType::RecoveryCancel.name(), "RecoveryCancel");
    }

    #[test]
    fn recovery_config_message_roundtrip() {
        let _init_guard = zebra_test::init();

        // RecoveryConfig format: [guardian_count(1)][guardian_hash(32)]...[threshold(1)][timelock_blocks(4)]
        let mut payload = Vec::new();
        payload.push(3); // 3 guardians

        // Add 3 guardian hashes (SHA256 of addresses)
        for i in 0..3 {
            let mut hash = [0u8; 32];
            hash[0] = i + 1;
            hash[31] = 0xFF; // Ensure non-zero ending for memo parsing
            payload.extend_from_slice(&hash);
        }

        payload.push(2); // threshold: 2-of-3
        let timelock: u32 = 10080; // ~7 days at 60s blocks
        payload.extend_from_slice(&timelock.to_le_bytes());

        let msg = SocialMessage::new(
            SocialMessageType::RecoveryConfig,
            SOCIAL_PROTOCOL_VERSION,
            payload.clone(),
        );

        let encoded = msg.encode();
        assert_eq!(encoded[0], 0xF0); // RecoveryConfig type
        assert_eq!(encoded[1], SOCIAL_PROTOCOL_VERSION);

        let memo = create_memo(&encoded);
        let decoded = SocialMessage::try_from(&memo).expect("should decode");

        assert_eq!(decoded.msg_type(), SocialMessageType::RecoveryConfig);
        assert!(decoded.msg_type().is_recovery());
        assert_eq!(decoded.payload()[0], 3); // 3 guardians
    }

    #[test]
    fn recovery_request_message_roundtrip() {
        let _init_guard = zebra_test::init();

        // RecoveryRequest format: [target_address_len(1)][target_address][new_pubkey(33)][proof_len(1)][proof]
        let mut payload = Vec::new();

        let target = b"bs1oldaddress...";
        payload.push(target.len() as u8);
        payload.extend_from_slice(target);

        // 33-byte compressed public key
        let new_pubkey = [0xAB; 33];
        payload.extend_from_slice(&new_pubkey);

        let proof = b"signed_challenge_proof";
        payload.push(proof.len() as u8);
        payload.extend_from_slice(proof);

        let msg = SocialMessage::new(
            SocialMessageType::RecoveryRequest,
            SOCIAL_PROTOCOL_VERSION,
            payload.clone(),
        );

        let encoded = msg.encode();
        assert_eq!(encoded[0], 0xF1); // RecoveryRequest type
        assert_eq!(encoded[1], SOCIAL_PROTOCOL_VERSION);

        let memo = create_memo(&encoded);
        let decoded = SocialMessage::try_from(&memo).expect("should decode");

        assert_eq!(decoded.msg_type(), SocialMessageType::RecoveryRequest);
        assert!(decoded.msg_type().is_recovery());
    }

    #[test]
    fn recovery_approve_message_roundtrip() {
        let _init_guard = zebra_test::init();

        // RecoveryApprove format: [request_txid(32)][encrypted_share_len(1)][encrypted_share]
        let mut payload = Vec::new();

        // Request transaction ID (32 bytes)
        payload.extend_from_slice(&[0xCD; 32]);

        // Encrypted Shamir share
        let encrypted_share = b"encrypted_shamir_share_data_here";
        payload.push(encrypted_share.len() as u8);
        payload.extend_from_slice(encrypted_share);

        let msg = SocialMessage::new(
            SocialMessageType::RecoveryApprove,
            SOCIAL_PROTOCOL_VERSION,
            payload.clone(),
        );

        let encoded = msg.encode();
        assert_eq!(encoded[0], 0xF2); // RecoveryApprove type
        assert_eq!(encoded[1], SOCIAL_PROTOCOL_VERSION);

        let memo = create_memo(&encoded);
        let decoded = SocialMessage::try_from(&memo).expect("should decode");

        assert_eq!(decoded.msg_type(), SocialMessageType::RecoveryApprove);
        assert!(decoded.msg_type().is_recovery());
    }

    #[test]
    fn recovery_cancel_message_roundtrip() {
        let _init_guard = zebra_test::init();

        // RecoveryCancel format: [request_txid(32)][owner_sig_len(1)][owner_sig]
        let mut payload = Vec::new();

        // Request transaction ID (32 bytes)
        payload.extend_from_slice(&[0xEF; 32]);

        // Owner signature to prove authorization
        let owner_sig = b"owner_signature_bytes";
        payload.push(owner_sig.len() as u8);
        payload.extend_from_slice(owner_sig);

        let msg = SocialMessage::new(
            SocialMessageType::RecoveryCancel,
            SOCIAL_PROTOCOL_VERSION,
            payload.clone(),
        );

        let encoded = msg.encode();
        assert_eq!(encoded[0], 0xF3); // RecoveryCancel type
        assert_eq!(encoded[1], SOCIAL_PROTOCOL_VERSION);

        let memo = create_memo(&encoded);
        let decoded = SocialMessage::try_from(&memo).expect("should decode");

        assert_eq!(decoded.msg_type(), SocialMessageType::RecoveryCancel);
        assert!(decoded.msg_type().is_recovery());
    }

    #[test]
    fn key_rotation_message_roundtrip() {
        let _init_guard = zebra_test::init();

        // KeyRotation format: [old_addr_len(1)][old_addr][new_addr_len(1)][new_addr][old_sig_len(1)][old_sig][new_sig_len(1)][new_sig]
        let mut payload = Vec::new();

        // Old address (simulated bech32 z-address hash, 43 bytes)
        let old_addr = b"bs1oldaddress1234567890abcdefghijklmnop";
        payload.push(old_addr.len() as u8);
        payload.extend_from_slice(old_addr);

        // New address (simulated bech32 z-address hash, 43 bytes)
        let new_addr = b"bs1newaddress0987654321zyxwvutsrqponml";
        payload.push(new_addr.len() as u8);
        payload.extend_from_slice(new_addr);

        // Old key signature (64 bytes for Ed25519)
        let old_sig = [0xAA; 64];
        payload.push(old_sig.len() as u8);
        payload.extend_from_slice(&old_sig);

        // New key signature (64 bytes for Ed25519)
        let new_sig = [0xBB; 64];
        payload.push(new_sig.len() as u8);
        payload.extend_from_slice(&new_sig);

        let msg = SocialMessage::new(
            SocialMessageType::KeyRotation,
            SOCIAL_PROTOCOL_VERSION,
            payload.clone(),
        );

        let encoded = msg.encode();
        assert_eq!(encoded[0], 0xF4); // KeyRotation type
        assert_eq!(encoded[1], SOCIAL_PROTOCOL_VERSION);

        let memo = create_memo(&encoded);
        let decoded = SocialMessage::try_from(&memo).expect("should decode");

        assert_eq!(decoded.msg_type(), SocialMessageType::KeyRotation);
        assert!(decoded.msg_type().is_recovery());
    }

    #[test]
    fn key_rotation_message_type_values() {
        let _init_guard = zebra_test::init();

        assert_eq!(SocialMessageType::KeyRotation.as_u8(), 0xF4);
        assert_eq!(SocialMessageType::KeyRotation.name(), "KeyRotation");

        // Verify roundtrip from u8
        let parsed = SocialMessageType::try_from(0xF4).expect("should parse");
        assert_eq!(parsed, SocialMessageType::KeyRotation);
    }

    #[test]
    fn recovery_types_in_batch() {
        let _init_guard = zebra_test::init();

        // Recovery messages can be batched with other actions
        let actions = vec![
            SocialMessage::new(
                SocialMessageType::RecoveryApprove,
                SOCIAL_PROTOCOL_VERSION,
                {
                    let mut payload = vec![0xAB; 32]; // request_txid
                    payload.push(16);
                    payload.extend_from_slice(b"encrypted_share_");
                    payload
                },
            ),
            SocialMessage::new(
                SocialMessageType::Post,
                SOCIAL_PROTOCOL_VERSION,
                b"Approved recovery for @friend!".to_vec(),
            ),
        ];

        let batch = BatchMessage::new(actions).expect("valid batch");
        let encoded = batch.encode();
        let memo = create_memo(&encoded);
        let decoded = BatchMessage::try_from_memo(&memo).expect("should decode");

        assert_eq!(decoded.len(), 2);
        assert_eq!(
            decoded.actions()[0].msg_type(),
            SocialMessageType::RecoveryApprove
        );
        assert!(decoded.actions()[0].msg_type().is_recovery());
        assert_eq!(decoded.actions()[1].msg_type(), SocialMessageType::Post);
    }

    #[test]
    fn recovery_types_not_value_transfer() {
        let _init_guard = zebra_test::init();

        // Recovery types are not value transfers
        assert!(!SocialMessageType::RecoveryConfig.is_value_transfer());
        assert!(!SocialMessageType::RecoveryRequest.is_value_transfer());
        assert!(!SocialMessageType::RecoveryApprove.is_value_transfer());
        assert!(!SocialMessageType::RecoveryCancel.is_value_transfer());
        assert!(!SocialMessageType::KeyRotation.is_value_transfer());

        // Recovery types are not attention market
        assert!(!SocialMessageType::RecoveryConfig.is_attention_market());
        assert!(!SocialMessageType::RecoveryRequest.is_attention_market());
        assert!(!SocialMessageType::RecoveryApprove.is_attention_market());
        assert!(!SocialMessageType::RecoveryCancel.is_attention_market());
        assert!(!SocialMessageType::KeyRotation.is_attention_market());

        // Recovery types are not governance
        assert!(!SocialMessageType::RecoveryConfig.is_governance());
        assert!(!SocialMessageType::RecoveryRequest.is_governance());
        assert!(!SocialMessageType::RecoveryApprove.is_governance());
        assert!(!SocialMessageType::RecoveryCancel.is_governance());
        assert!(!SocialMessageType::KeyRotation.is_governance());

        // Recovery types are not channels
        assert!(!SocialMessageType::RecoveryConfig.is_channel());
        assert!(!SocialMessageType::RecoveryRequest.is_channel());
        assert!(!SocialMessageType::RecoveryApprove.is_channel());
        assert!(!SocialMessageType::RecoveryCancel.is_channel());
        assert!(!SocialMessageType::KeyRotation.is_channel());

        // Recovery types are not batch
        assert!(!SocialMessageType::RecoveryConfig.is_batch());
        assert!(!SocialMessageType::RecoveryRequest.is_batch());
        assert!(!SocialMessageType::RecoveryApprove.is_batch());
        assert!(!SocialMessageType::RecoveryCancel.is_batch());
        assert!(!SocialMessageType::KeyRotation.is_batch());
    }

    #[test]
    fn recovery_is_recovery_helper() {
        let _init_guard = zebra_test::init();

        // Recovery types should return true (includes key rotation)
        assert!(SocialMessageType::RecoveryConfig.is_recovery());
        assert!(SocialMessageType::RecoveryRequest.is_recovery());
        assert!(SocialMessageType::RecoveryApprove.is_recovery());
        assert!(SocialMessageType::RecoveryCancel.is_recovery());
        assert!(SocialMessageType::KeyRotation.is_recovery());

        // Non-recovery types should return false
        assert!(!SocialMessageType::Post.is_recovery());
        assert!(!SocialMessageType::Dm.is_recovery());
        assert!(!SocialMessageType::Batch.is_recovery());
        assert!(!SocialMessageType::GovernanceVote.is_recovery());
        assert!(!SocialMessageType::ChannelOpen.is_recovery());
    }

    #[test]
    fn recovery_type_display() {
        let _init_guard = zebra_test::init();

        assert_eq!(format!("{}", SocialMessageType::RecoveryConfig), "RecoveryConfig");
        assert_eq!(format!("{}", SocialMessageType::RecoveryRequest), "RecoveryRequest");
        assert_eq!(format!("{}", SocialMessageType::RecoveryApprove), "RecoveryApprove");
        assert_eq!(format!("{}", SocialMessageType::RecoveryCancel), "RecoveryCancel");
        assert_eq!(format!("{}", SocialMessageType::KeyRotation), "KeyRotation");
    }

    #[test]
    fn recovery_type_roundtrip_from_u8() {
        let _init_guard = zebra_test::init();

        // Test all recovery types can roundtrip through u8
        let recovery_types = [
            SocialMessageType::RecoveryConfig,
            SocialMessageType::RecoveryRequest,
            SocialMessageType::RecoveryApprove,
            SocialMessageType::RecoveryCancel,
            SocialMessageType::KeyRotation,
        ];

        for msg_type in recovery_types {
            let byte = msg_type.as_u8();
            let parsed = SocialMessageType::try_from(byte).expect("should parse");
            assert_eq!(parsed, msg_type);
        }
    }

    #[test]
    fn all_27_message_types_exist() {
        let _init_guard = zebra_test::init();

        // Verify we have exactly 27 message types (22 pre-recovery + 5 recovery/key-rotation)
        let all_types = [
            SocialMessageType::Profile,
            SocialMessageType::Post,
            SocialMessageType::Comment,
            SocialMessageType::Upvote,
            SocialMessageType::Follow,
            SocialMessageType::Unfollow,
            SocialMessageType::Dm,
            SocialMessageType::DmGroup,
            SocialMessageType::Tip,
            SocialMessageType::Bounty,
            SocialMessageType::AttentionBoost,
            SocialMessageType::CreditTip,
            SocialMessageType::CreditClaim,
            SocialMessageType::Media,
            SocialMessageType::Poll,
            SocialMessageType::Vote,
            SocialMessageType::Batch,
            SocialMessageType::ChannelOpen,
            SocialMessageType::ChannelClose,
            SocialMessageType::ChannelSettle,
            SocialMessageType::GovernanceVote,
            SocialMessageType::GovernanceProposal,
            SocialMessageType::RecoveryConfig,
            SocialMessageType::RecoveryRequest,
            SocialMessageType::RecoveryApprove,
            SocialMessageType::RecoveryCancel,
            SocialMessageType::KeyRotation,
        ];

        assert_eq!(all_types.len(), 27, "Should have exactly 27 message types");

        // Verify each has a unique byte value
        let mut seen_bytes = std::collections::HashSet::new();
        for msg_type in all_types {
            let byte = msg_type.as_u8();
            assert!(
                seen_bytes.insert(byte),
                "Duplicate byte value: 0x{:02X}",
                byte
            );
        }
    }

    #[test]
    fn recovery_config_with_many_guardians() {
        let _init_guard = zebra_test::init();

        // Test recovery config with maximum reasonable guardians (e.g., 10)
        let mut payload = Vec::new();
        let guardian_count = 10u8;
        payload.push(guardian_count);

        // Add 10 guardian hashes
        for i in 0..guardian_count {
            let mut hash = [0u8; 32];
            hash[0] = i + 1;
            hash[31] = 0xFF;
            payload.extend_from_slice(&hash);
        }

        payload.push(6); // threshold: 6-of-10
        let timelock: u32 = 0x01020304; // Non-zero bytes to avoid trimming
        payload.extend_from_slice(&timelock.to_le_bytes());

        let msg = SocialMessage::new(
            SocialMessageType::RecoveryConfig,
            SOCIAL_PROTOCOL_VERSION,
            payload.clone(),
        );

        let encoded = msg.encode();
        let memo = create_memo(&encoded);
        let decoded = SocialMessage::try_from(&memo).expect("should decode");

        assert_eq!(decoded.msg_type(), SocialMessageType::RecoveryConfig);
        assert_eq!(decoded.payload()[0], 10); // 10 guardians
    }

    // ==================== Bridge Tests ====================

    #[test]
    fn bridge_message_type_values() {
        let _init_guard = zebra_test::init();

        assert_eq!(SocialMessageType::BridgeLink.as_u8(), 0xB0);
        assert_eq!(SocialMessageType::BridgeUnlink.as_u8(), 0xB1);
        assert_eq!(SocialMessageType::BridgePost.as_u8(), 0xB2);
        assert_eq!(SocialMessageType::BridgeVerify.as_u8(), 0xB3);

        assert_eq!(SocialMessageType::BridgeLink.name(), "BridgeLink");
        assert_eq!(SocialMessageType::BridgeUnlink.name(), "BridgeUnlink");
        assert_eq!(SocialMessageType::BridgePost.name(), "BridgePost");
        assert_eq!(SocialMessageType::BridgeVerify.name(), "BridgeVerify");
    }

    #[test]
    fn bridge_platform_values() {
        let _init_guard = zebra_test::init();

        assert_eq!(BridgePlatform::Telegram.as_u8(), 0x01);
        assert_eq!(BridgePlatform::Discord.as_u8(), 0x02);
        assert_eq!(BridgePlatform::Nostr.as_u8(), 0x03);
        assert_eq!(BridgePlatform::Mastodon.as_u8(), 0x04);
        assert_eq!(BridgePlatform::Twitter.as_u8(), 0x05);

        assert_eq!(BridgePlatform::Telegram.name(), "Telegram");
        assert_eq!(BridgePlatform::Discord.name(), "Discord");
        assert_eq!(BridgePlatform::Nostr.name(), "Nostr");
        assert_eq!(BridgePlatform::Mastodon.name(), "Mastodon");
        assert_eq!(BridgePlatform::Twitter.name(), "Twitter");
    }

    #[test]
    fn bridge_platform_bidirectional() {
        let _init_guard = zebra_test::init();

        // Most platforms support bidirectional bridging
        assert!(BridgePlatform::Telegram.is_bidirectional());
        assert!(BridgePlatform::Discord.is_bidirectional());
        assert!(BridgePlatform::Nostr.is_bidirectional());
        assert!(BridgePlatform::Mastodon.is_bidirectional());

        // Twitter is read-only due to API restrictions
        assert!(!BridgePlatform::Twitter.is_bidirectional());
    }

    #[test]
    fn bridge_platform_try_from() {
        let _init_guard = zebra_test::init();

        assert_eq!(BridgePlatform::try_from(0x01).unwrap(), BridgePlatform::Telegram);
        assert_eq!(BridgePlatform::try_from(0x02).unwrap(), BridgePlatform::Discord);
        assert_eq!(BridgePlatform::try_from(0x03).unwrap(), BridgePlatform::Nostr);
        assert_eq!(BridgePlatform::try_from(0x04).unwrap(), BridgePlatform::Mastodon);
        assert_eq!(BridgePlatform::try_from(0x05).unwrap(), BridgePlatform::Twitter);

        // Unknown platforms should error
        assert!(BridgePlatform::try_from(0x00).is_err());
        assert!(BridgePlatform::try_from(0x06).is_err());
        assert!(BridgePlatform::try_from(0xFF).is_err());
    }

    #[test]
    fn bridge_link_message_roundtrip() {
        let _init_guard = zebra_test::init();

        let platform = BridgePlatform::Telegram;
        let platform_id = "123456789".to_string();
        let challenge = [0xAB; BRIDGE_CHALLENGE_SIZE];
        let signature = vec![0xCD; 64];

        let msg = BridgeMessage::new_link(platform, platform_id.clone(), challenge, signature.clone());

        assert_eq!(msg.platform(), BridgePlatform::Telegram);
        assert_eq!(msg.platform_id(), "123456789");
        assert_eq!(msg.challenge(), Some(&challenge));
        assert_eq!(msg.signature(), Some(signature.as_slice()));

        // Encode and parse back
        let payload = msg.encode(SocialMessageType::BridgeLink);
        let parsed = BridgeMessage::parse(SocialMessageType::BridgeLink, &payload).unwrap();

        assert_eq!(parsed.platform(), platform);
        assert_eq!(parsed.platform_id(), platform_id);
        assert_eq!(parsed.challenge(), Some(&challenge));
        assert_eq!(parsed.signature(), Some(signature.as_slice()));
    }

    #[test]
    fn bridge_unlink_message_roundtrip() {
        let _init_guard = zebra_test::init();

        let platform = BridgePlatform::Discord;
        let platform_id = "987654321012345678".to_string(); // Discord snowflake ID

        let msg = BridgeMessage::new_unlink(platform, platform_id.clone());

        assert_eq!(msg.platform(), BridgePlatform::Discord);
        assert_eq!(msg.platform_id(), "987654321012345678");
        assert!(msg.challenge().is_none());
        assert!(msg.signature().is_none());

        // Encode and parse back
        let payload = msg.encode(SocialMessageType::BridgeUnlink);
        let parsed = BridgeMessage::parse(SocialMessageType::BridgeUnlink, &payload).unwrap();

        assert_eq!(parsed.platform(), platform);
        assert_eq!(parsed.platform_id(), platform_id);
    }

    #[test]
    fn bridge_post_message_roundtrip() {
        let _init_guard = zebra_test::init();

        let platform = BridgePlatform::Nostr;
        let original_id = "note1abc123xyz".to_string();
        let content = "Hello from Nostr! This is a cross-posted message.".to_string();

        let msg = BridgeMessage::new_post(platform, original_id.clone(), content.clone());

        assert_eq!(msg.platform(), BridgePlatform::Nostr);
        assert_eq!(msg.original_id(), Some(original_id.as_str()));
        assert_eq!(msg.content(), Some(content.as_str()));

        // Encode and parse back
        let payload = msg.encode(SocialMessageType::BridgePost);
        let parsed = BridgeMessage::parse(SocialMessageType::BridgePost, &payload).unwrap();

        assert_eq!(parsed.platform(), platform);
        assert_eq!(parsed.original_id(), Some(original_id.as_str()));
        assert_eq!(parsed.content(), Some(content.as_str()));
    }

    #[test]
    fn bridge_verify_message_roundtrip() {
        let _init_guard = zebra_test::init();

        let platform = BridgePlatform::Mastodon;
        let platform_id = "@alice@mastodon.social".to_string();
        let nonce = 0x123456789ABCDEF0u64;

        let msg = BridgeMessage::new_verify(platform, platform_id.clone(), nonce);

        assert_eq!(msg.platform(), BridgePlatform::Mastodon);
        assert_eq!(msg.platform_id(), "@alice@mastodon.social");
        assert_eq!(msg.nonce(), Some(nonce));

        // Encode and parse back
        let payload = msg.encode(SocialMessageType::BridgeVerify);
        let parsed = BridgeMessage::parse(SocialMessageType::BridgeVerify, &payload).unwrap();

        assert_eq!(parsed.platform(), platform);
        assert_eq!(parsed.platform_id(), platform_id);
        assert_eq!(parsed.nonce(), Some(nonce));
    }

    #[test]
    fn bridge_is_bridge_helper() {
        let _init_guard = zebra_test::init();

        // Bridge types should return true
        assert!(SocialMessageType::BridgeLink.is_bridge());
        assert!(SocialMessageType::BridgeUnlink.is_bridge());
        assert!(SocialMessageType::BridgePost.is_bridge());
        assert!(SocialMessageType::BridgeVerify.is_bridge());

        // Non-bridge types should return false
        assert!(!SocialMessageType::Post.is_bridge());
        assert!(!SocialMessageType::Dm.is_bridge());
        assert!(!SocialMessageType::Batch.is_bridge());
        assert!(!SocialMessageType::GovernanceVote.is_bridge());
        assert!(!SocialMessageType::ChannelOpen.is_bridge());
        assert!(!SocialMessageType::RecoveryConfig.is_bridge());
    }

    #[test]
    fn bridge_types_not_other_categories() {
        let _init_guard = zebra_test::init();

        // Bridge types are not value transfers
        assert!(!SocialMessageType::BridgeLink.is_value_transfer());
        assert!(!SocialMessageType::BridgeUnlink.is_value_transfer());
        assert!(!SocialMessageType::BridgePost.is_value_transfer());
        assert!(!SocialMessageType::BridgeVerify.is_value_transfer());

        // Bridge types are not attention market
        assert!(!SocialMessageType::BridgeLink.is_attention_market());
        assert!(!SocialMessageType::BridgeUnlink.is_attention_market());
        assert!(!SocialMessageType::BridgePost.is_attention_market());
        assert!(!SocialMessageType::BridgeVerify.is_attention_market());

        // Bridge types are not governance
        assert!(!SocialMessageType::BridgeLink.is_governance());
        assert!(!SocialMessageType::BridgeUnlink.is_governance());
        assert!(!SocialMessageType::BridgePost.is_governance());
        assert!(!SocialMessageType::BridgeVerify.is_governance());

        // Bridge types are not channels
        assert!(!SocialMessageType::BridgeLink.is_channel());
        assert!(!SocialMessageType::BridgeUnlink.is_channel());
        assert!(!SocialMessageType::BridgePost.is_channel());
        assert!(!SocialMessageType::BridgeVerify.is_channel());

        // Bridge types are not recovery
        assert!(!SocialMessageType::BridgeLink.is_recovery());
        assert!(!SocialMessageType::BridgeUnlink.is_recovery());
        assert!(!SocialMessageType::BridgePost.is_recovery());
        assert!(!SocialMessageType::BridgeVerify.is_recovery());

        // Bridge types are not batch
        assert!(!SocialMessageType::BridgeLink.is_batch());
        assert!(!SocialMessageType::BridgeUnlink.is_batch());
        assert!(!SocialMessageType::BridgePost.is_batch());
        assert!(!SocialMessageType::BridgeVerify.is_batch());

        // Bridge types are not moderation
        assert!(!SocialMessageType::BridgeLink.is_moderation());
        assert!(!SocialMessageType::BridgeUnlink.is_moderation());
        assert!(!SocialMessageType::BridgePost.is_moderation());
        assert!(!SocialMessageType::BridgeVerify.is_moderation());
    }

    #[test]
    fn bridge_type_display() {
        let _init_guard = zebra_test::init();

        assert_eq!(format!("{}", SocialMessageType::BridgeLink), "BridgeLink");
        assert_eq!(format!("{}", SocialMessageType::BridgeUnlink), "BridgeUnlink");
        assert_eq!(format!("{}", SocialMessageType::BridgePost), "BridgePost");
        assert_eq!(format!("{}", SocialMessageType::BridgeVerify), "BridgeVerify");

        assert_eq!(format!("{}", BridgePlatform::Telegram), "Telegram");
        assert_eq!(format!("{}", BridgePlatform::Discord), "Discord");
        assert_eq!(format!("{}", BridgePlatform::Nostr), "Nostr");
        assert_eq!(format!("{}", BridgePlatform::Mastodon), "Mastodon");
        assert_eq!(format!("{}", BridgePlatform::Twitter), "Twitter");
    }

    #[test]
    fn bridge_message_try_from_bytes() {
        let _init_guard = zebra_test::init();

        let bridge_types = vec![
            SocialMessageType::BridgeLink,
            SocialMessageType::BridgeUnlink,
            SocialMessageType::BridgePost,
            SocialMessageType::BridgeVerify,
        ];

        for msg_type in &bridge_types {
            let byte = msg_type.as_u8();
            let parsed = SocialMessageType::try_from(byte).expect("should parse");
            assert_eq!(&parsed, msg_type);
        }
    }

    #[test]
    fn bridge_social_message_roundtrip() {
        let _init_guard = zebra_test::init();

        // BridgeLink format: [platform(1)][platform_id_len(1)][platform_id][challenge(32)][sig_len(1)][sig]
        let mut payload = Vec::new();
        payload.push(BridgePlatform::Telegram.as_u8());
        let platform_id = b"12345678901";
        payload.push(platform_id.len() as u8);
        payload.extend_from_slice(platform_id);
        payload.extend_from_slice(&[0xAB; BRIDGE_CHALLENGE_SIZE]); // challenge
        payload.push(64);
        payload.extend_from_slice(&[0xCD; 64]); // signature

        let msg = SocialMessage::new(
            SocialMessageType::BridgeLink,
            SOCIAL_PROTOCOL_VERSION,
            payload.clone(),
        );

        let encoded = msg.encode();
        assert_eq!(encoded[0], 0xB0); // BridgeLink type
        assert_eq!(encoded[1], SOCIAL_PROTOCOL_VERSION);

        let memo = create_memo(&encoded);
        let decoded = SocialMessage::try_from(&memo).expect("should decode");

        assert_eq!(decoded.msg_type(), SocialMessageType::BridgeLink);
        assert!(decoded.msg_type().is_bridge());
        assert_eq!(decoded.payload(), payload.as_slice());
    }

    #[test]
    fn bridge_types_in_batch() {
        let _init_guard = zebra_test::init();

        // Bridge messages can be batched with other actions
        let actions = vec![
            SocialMessage::new(
                SocialMessageType::BridgeLink,
                SOCIAL_PROTOCOL_VERSION,
                {
                    let mut payload = vec![BridgePlatform::Discord.as_u8()];
                    let id = b"123456789012345678";
                    payload.push(id.len() as u8);
                    payload.extend_from_slice(id);
                    payload.extend_from_slice(&[0xAB; 32]); // challenge
                    payload.push(64);
                    payload.extend_from_slice(&[0xCD; 64]); // sig
                    payload
                },
            ),
            SocialMessage::new(
                SocialMessageType::Post,
                SOCIAL_PROTOCOL_VERSION,
                b"Linked my Discord account!".to_vec(),
            ),
        ];

        let batch = BatchMessage::new(actions).expect("valid batch");
        let encoded = batch.encode();
        let memo = create_memo(&encoded);
        let decoded = BatchMessage::try_from_memo(&memo).expect("should decode");

        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded.actions()[0].msg_type(), SocialMessageType::BridgeLink);
        assert!(decoded.actions()[0].msg_type().is_bridge());
        assert_eq!(decoded.actions()[1].msg_type(), SocialMessageType::Post);
    }

    #[test]
    fn bridge_parse_error_unknown_platform() {
        let _init_guard = zebra_test::init();

        let payload = vec![0xFF, 5, b'h', b'e', b'l', b'l', b'o'];
        let result = BridgeMessage::parse(SocialMessageType::BridgeUnlink, &payload);
        assert!(result.is_err());

        if let Err(BridgeParseError::UnknownPlatform(byte)) = result {
            assert_eq!(byte, 0xFF);
        } else {
            panic!("Expected UnknownPlatform error");
        }
    }

    #[test]
    fn bridge_parse_error_platform_id_too_long() {
        let _init_guard = zebra_test::init();

        // Create a payload with platform ID length exceeding MAX_PLATFORM_ID_LENGTH
        let mut payload = vec![BridgePlatform::Telegram.as_u8()];
        payload.push(100); // Length > MAX_PLATFORM_ID_LENGTH (64)
        payload.extend_from_slice(&[b'x'; 100]);

        let result = BridgeMessage::parse(SocialMessageType::BridgeUnlink, &payload);
        assert!(result.is_err());

        if let Err(BridgeParseError::PlatformIdTooLong { max_len, actual_len }) = result {
            assert_eq!(max_len, MAX_PLATFORM_ID_LENGTH);
            assert_eq!(actual_len, 100);
        } else {
            panic!("Expected PlatformIdTooLong error");
        }
    }

    #[test]
    fn bridge_parse_error_payload_too_short() {
        let _init_guard = zebra_test::init();

        // Empty payload
        let result = BridgeMessage::parse(SocialMessageType::BridgeLink, &[]);
        assert!(matches!(result, Err(BridgeParseError::PayloadTooShort { .. })));

        // Only platform byte
        let result = BridgeMessage::parse(SocialMessageType::BridgeLink, &[0x01]);
        assert!(matches!(result, Err(BridgeParseError::PayloadTooShort { .. })));
    }

    #[test]
    fn bridge_all_types_count() {
        let _init_guard = zebra_test::init();

        // Verify SocialMessageType now has 30 variants (26 + 4 bridge types)
        let all_types = vec![
            SocialMessageType::Profile,
            SocialMessageType::Post,
            SocialMessageType::Comment,
            SocialMessageType::Upvote,
            SocialMessageType::Follow,
            SocialMessageType::Unfollow,
            SocialMessageType::Dm,
            SocialMessageType::DmGroup,
            SocialMessageType::Tip,
            SocialMessageType::Bounty,
            SocialMessageType::AttentionBoost,
            SocialMessageType::CreditTip,
            SocialMessageType::CreditClaim,
            SocialMessageType::Media,
            SocialMessageType::Poll,
            SocialMessageType::Vote,
            SocialMessageType::Batch,
            SocialMessageType::BridgeLink,
            SocialMessageType::BridgeUnlink,
            SocialMessageType::BridgePost,
            SocialMessageType::BridgeVerify,
            SocialMessageType::ChannelOpen,
            SocialMessageType::ChannelClose,
            SocialMessageType::ChannelSettle,
            SocialMessageType::GovernanceVote,
            SocialMessageType::GovernanceProposal,
            SocialMessageType::RecoveryConfig,
            SocialMessageType::RecoveryRequest,
            SocialMessageType::RecoveryApprove,
            SocialMessageType::RecoveryCancel,
            SocialMessageType::KeyRotation,
            SocialMessageType::Trust,
            SocialMessageType::Report,
        ];

        assert_eq!(all_types.len(), 33);

        // Verify all can be parsed from their byte values
        for msg_type in &all_types {
            let byte = msg_type.as_u8();
            let parsed = SocialMessageType::try_from(byte).expect("should parse");
            assert_eq!(&parsed, msg_type);
        }
    }

    // ==================== Moderation Message Tests (Required for P6.6 Moderation) ====================

    #[test]
    fn trust_level_values() {
        let _init_guard = zebra_test::init();

        assert_eq!(TrustLevel::Distrust.as_u8(), 0x00);
        assert_eq!(TrustLevel::Neutral.as_u8(), 0x01);
        assert_eq!(TrustLevel::Trusted.as_u8(), 0x02);
    }

    #[test]
    fn trust_level_roundtrip() {
        let _init_guard = zebra_test::init();

        let levels = [TrustLevel::Distrust, TrustLevel::Neutral, TrustLevel::Trusted];

        for level in levels {
            let byte = level.as_u8();
            let parsed = TrustLevel::try_from(byte).expect("should parse");
            assert_eq!(parsed, level);
        }
    }

    #[test]
    fn trust_level_invalid() {
        let _init_guard = zebra_test::init();

        let invalid_levels = [0x03, 0x04, 0xFF];
        for byte in invalid_levels {
            let result = TrustLevel::try_from(byte);
            assert!(result.is_err());
            if let Err(ModerationParseError::InvalidTrustLevel(b)) = result {
                assert_eq!(b, byte);
            }
        }
    }

    #[test]
    fn trust_level_display() {
        let _init_guard = zebra_test::init();

        assert_eq!(format!("{}", TrustLevel::Distrust), "Distrust");
        assert_eq!(format!("{}", TrustLevel::Neutral), "Neutral");
        assert_eq!(format!("{}", TrustLevel::Trusted), "Trusted");
    }

    #[test]
    fn report_category_values() {
        let _init_guard = zebra_test::init();

        assert_eq!(ReportCategory::Spam.as_u8(), 0x00);
        assert_eq!(ReportCategory::Scam.as_u8(), 0x01);
        assert_eq!(ReportCategory::Harassment.as_u8(), 0x02);
        assert_eq!(ReportCategory::Illegal.as_u8(), 0x03);
        assert_eq!(ReportCategory::Other.as_u8(), 0x04);
    }

    #[test]
    fn report_category_roundtrip() {
        let _init_guard = zebra_test::init();

        let categories = [
            ReportCategory::Spam,
            ReportCategory::Scam,
            ReportCategory::Harassment,
            ReportCategory::Illegal,
            ReportCategory::Other,
        ];

        for category in categories {
            let byte = category.as_u8();
            let parsed = ReportCategory::try_from(byte).expect("should parse");
            assert_eq!(parsed, category);
        }
    }

    #[test]
    fn report_category_invalid() {
        let _init_guard = zebra_test::init();

        let invalid_categories = [0x05, 0x10, 0xFF];
        for byte in invalid_categories {
            let result = ReportCategory::try_from(byte);
            assert!(result.is_err());
            if let Err(ModerationParseError::InvalidReportCategory(b)) = result {
                assert_eq!(b, byte);
            }
        }
    }

    #[test]
    fn report_category_immediate_filtering() {
        let _init_guard = zebra_test::init();

        // Only illegal content requires immediate filtering
        assert!(ReportCategory::Illegal.requires_immediate_filtering());

        // Other categories don't require immediate filtering
        assert!(!ReportCategory::Spam.requires_immediate_filtering());
        assert!(!ReportCategory::Scam.requires_immediate_filtering());
        assert!(!ReportCategory::Harassment.requires_immediate_filtering());
        assert!(!ReportCategory::Other.requires_immediate_filtering());
    }

    #[test]
    fn trust_message_roundtrip() {
        let _init_guard = zebra_test::init();

        let msg = TrustMessage::new(
            "bs1test12345".to_string(),
            TrustLevel::Trusted,
            Some("Helpful in m/botcash".to_string()),
        );

        assert_eq!(msg.target_address(), "bs1test12345");
        assert_eq!(msg.level(), TrustLevel::Trusted);
        assert_eq!(msg.reason(), Some("Helpful in m/botcash"));

        // Encode and parse back
        let payload = msg.encode();
        let parsed = TrustMessage::parse(&payload).expect("should parse");

        assert_eq!(parsed.target_address(), msg.target_address());
        assert_eq!(parsed.level(), msg.level());
        assert_eq!(parsed.reason(), msg.reason());
    }

    #[test]
    fn trust_message_no_reason() {
        let _init_guard = zebra_test::init();

        let msg = TrustMessage::new(
            "bs1anon67890".to_string(),
            TrustLevel::Distrust,
            None,
        );

        assert_eq!(msg.target_address(), "bs1anon67890");
        assert_eq!(msg.level(), TrustLevel::Distrust);
        assert_eq!(msg.reason(), None);

        // Encode and parse back
        let payload = msg.encode();
        let parsed = TrustMessage::parse(&payload).expect("should parse");

        assert_eq!(parsed.target_address(), msg.target_address());
        assert_eq!(parsed.level(), msg.level());
        assert_eq!(parsed.reason(), None);
    }

    #[test]
    fn trust_message_in_social_message() {
        let _init_guard = zebra_test::init();

        let trust_msg = TrustMessage::new(
            "bs1target".to_string(),
            TrustLevel::Neutral,
            Some("Changed my mind".to_string()),
        );

        // Build full social message
        let mut memo_bytes = vec![0xD0, 0x01]; // Trust type, version 1
        memo_bytes.extend_from_slice(&trust_msg.encode());

        let memo = create_memo(&memo_bytes);
        let social_msg = SocialMessage::try_from(&memo).expect("should parse");

        assert_eq!(social_msg.msg_type(), SocialMessageType::Trust);
        assert!(social_msg.msg_type().is_moderation());

        // Parse the payload as TrustMessage
        let decoded = TrustMessage::parse(social_msg.payload()).expect("should parse");
        assert_eq!(decoded.target_address(), "bs1target");
        assert_eq!(decoded.level(), TrustLevel::Neutral);
    }

    #[test]
    fn report_message_roundtrip() {
        let _init_guard = zebra_test::init();

        let target_txid = [0xAB; 32];
        let msg = ReportMessage::new(
            target_txid,
            ReportCategory::Spam,
            MIN_REPORT_STAKE,
            Some("Identical to 50 other posts".to_string()),
        );

        assert_eq!(msg.target_txid(), &target_txid);
        assert_eq!(msg.category(), ReportCategory::Spam);
        assert_eq!(msg.stake(), MIN_REPORT_STAKE);
        assert_eq!(msg.evidence(), Some("Identical to 50 other posts"));

        // Encode and parse back
        let payload = msg.encode();
        let parsed = ReportMessage::parse(&payload).expect("should parse");

        assert_eq!(parsed.target_txid(), msg.target_txid());
        assert_eq!(parsed.category(), msg.category());
        assert_eq!(parsed.stake(), msg.stake());
        assert_eq!(parsed.evidence(), msg.evidence());
    }

    #[test]
    fn report_message_no_evidence() {
        let _init_guard = zebra_test::init();

        let target_txid = [0xCD; 32];
        let msg = ReportMessage::new(
            target_txid,
            ReportCategory::Scam,
            MIN_REPORT_STAKE * 2,
            None,
        );

        assert_eq!(msg.evidence(), None);

        // Encode and parse back
        let payload = msg.encode();
        let parsed = ReportMessage::parse(&payload).expect("should parse");

        assert_eq!(parsed.target_txid(), msg.target_txid());
        assert_eq!(parsed.evidence(), None);
    }

    #[test]
    fn report_message_stake_too_low() {
        let _init_guard = zebra_test::init();

        let target_txid = [0xEF; 32];

        // Build payload with stake below minimum
        let mut payload = Vec::new();
        payload.extend_from_slice(&target_txid);
        payload.push(ReportCategory::Spam.as_u8());
        payload.extend_from_slice(&(MIN_REPORT_STAKE - 1).to_le_bytes());
        payload.push(0); // no evidence

        let result = ReportMessage::parse(&payload);
        assert!(result.is_err());

        if let Err(ModerationParseError::StakeTooLow { minimum, actual }) = result {
            assert_eq!(minimum, MIN_REPORT_STAKE);
            assert_eq!(actual, MIN_REPORT_STAKE - 1);
        } else {
            panic!("Expected StakeTooLow error");
        }
    }

    #[test]
    fn report_message_in_social_message() {
        let _init_guard = zebra_test::init();

        let target_txid = [0x12; 32];
        let report_msg = ReportMessage::new(
            target_txid,
            ReportCategory::Harassment,
            MIN_REPORT_STAKE,
            Some("Targeted abuse".to_string()),
        );

        // Build full social message
        let mut memo_bytes = vec![0xD1, 0x01]; // Report type, version 1
        memo_bytes.extend_from_slice(&report_msg.encode());

        let memo = create_memo(&memo_bytes);
        let social_msg = SocialMessage::try_from(&memo).expect("should parse");

        assert_eq!(social_msg.msg_type(), SocialMessageType::Report);
        assert!(social_msg.msg_type().is_moderation());

        // Parse the payload as ReportMessage
        let decoded = ReportMessage::parse(social_msg.payload()).expect("should parse");
        assert_eq!(decoded.target_txid(), &target_txid);
        assert_eq!(decoded.category(), ReportCategory::Harassment);
    }

    #[test]
    fn moderation_message_type_values() {
        let _init_guard = zebra_test::init();

        assert_eq!(SocialMessageType::Trust.as_u8(), 0xD0);
        assert_eq!(SocialMessageType::Report.as_u8(), 0xD1);
    }

    #[test]
    fn moderation_is_moderation_helper() {
        let _init_guard = zebra_test::init();

        // Moderation types should return true
        assert!(SocialMessageType::Trust.is_moderation());
        assert!(SocialMessageType::Report.is_moderation());

        // Non-moderation types should return false
        assert!(!SocialMessageType::Post.is_moderation());
        assert!(!SocialMessageType::Dm.is_moderation());
        assert!(!SocialMessageType::Batch.is_moderation());
        assert!(!SocialMessageType::GovernanceVote.is_moderation());
        assert!(!SocialMessageType::ChannelOpen.is_moderation());
        assert!(!SocialMessageType::RecoveryConfig.is_moderation());
        assert!(!SocialMessageType::BridgeLink.is_moderation());
    }

    #[test]
    fn moderation_types_not_other_categories() {
        let _init_guard = zebra_test::init();

        // Moderation types are not value transfers
        assert!(!SocialMessageType::Trust.is_value_transfer());
        assert!(!SocialMessageType::Report.is_value_transfer());

        // Moderation types are not attention market
        assert!(!SocialMessageType::Trust.is_attention_market());
        assert!(!SocialMessageType::Report.is_attention_market());

        // Moderation types are not governance
        assert!(!SocialMessageType::Trust.is_governance());
        assert!(!SocialMessageType::Report.is_governance());

        // Moderation types are not channels
        assert!(!SocialMessageType::Trust.is_channel());
        assert!(!SocialMessageType::Report.is_channel());

        // Moderation types are not recovery
        assert!(!SocialMessageType::Trust.is_recovery());
        assert!(!SocialMessageType::Report.is_recovery());

        // Moderation types are not batch
        assert!(!SocialMessageType::Trust.is_batch());
        assert!(!SocialMessageType::Report.is_batch());

        // Moderation types are not bridge
        assert!(!SocialMessageType::Trust.is_bridge());
        assert!(!SocialMessageType::Report.is_bridge());
    }

    #[test]
    fn moderation_type_display() {
        let _init_guard = zebra_test::init();

        assert_eq!(format!("{}", SocialMessageType::Trust), "Trust");
        assert_eq!(format!("{}", SocialMessageType::Report), "Report");
    }

    #[test]
    fn moderation_batch_roundtrip() {
        let _init_guard = zebra_test::init();

        // Create a trust message
        let trust_msg = TrustMessage::new(
            "bs1friend".to_string(),
            TrustLevel::Trusted,
            Some("Good contributor".to_string()),
        );

        // Create a batch containing trust + post
        let actions = vec![
            SocialMessage::new(SocialMessageType::Trust, 1, trust_msg.encode()),
            SocialMessage::new(SocialMessageType::Post, 1, b"Hello!".to_vec()),
        ];

        let batch = BatchMessage::new(actions).expect("batch should be valid");
        let encoded = batch.encode();
        let memo = create_memo(&encoded);
        let decoded = BatchMessage::try_from_memo(&memo).expect("should parse batch");

        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded.actions()[0].msg_type(), SocialMessageType::Trust);
        assert!(decoded.actions()[0].msg_type().is_moderation());
        assert_eq!(decoded.actions()[1].msg_type(), SocialMessageType::Post);
    }

    #[test]
    fn trust_parse_error_payload_too_short() {
        let _init_guard = zebra_test::init();

        // Empty payload
        let result = TrustMessage::parse(&[]);
        assert!(matches!(result, Err(ModerationParseError::PayloadTooShort { .. })));

        // Only address length
        let result = TrustMessage::parse(&[5]);
        assert!(matches!(result, Err(ModerationParseError::PayloadTooShort { .. })));
    }

    #[test]
    fn trust_parse_error_address_too_long() {
        let _init_guard = zebra_test::init();

        // Address length exceeding max
        let mut payload = vec![200]; // len > MAX_TRUST_ADDRESS_LENGTH
        payload.extend_from_slice(&[b'x'; 200]);
        payload.push(TrustLevel::Trusted.as_u8());
        payload.push(0);

        let result = TrustMessage::parse(&payload);
        assert!(matches!(result, Err(ModerationParseError::TargetAddressTooLong { .. })));
    }

    #[test]
    fn report_parse_error_payload_too_short() {
        let _init_guard = zebra_test::init();

        // Empty payload
        let result = ReportMessage::parse(&[]);
        assert!(matches!(result, Err(ModerationParseError::PayloadTooShort { .. })));

        // Partial txid
        let result = ReportMessage::parse(&[0xAB; 20]);
        assert!(matches!(result, Err(ModerationParseError::PayloadTooShort { .. })));
    }

    // ========================================================================
    // Multi-Sig Identity Tests (0xF5, 0xF6)
    // ========================================================================

    #[test]
    fn multisig_message_type_values() {
        let _init_guard = zebra_test::init();

        // Verify multi-sig type byte values match spec
        assert_eq!(SocialMessageType::MultisigSetup.as_u8(), 0xF5);
        assert_eq!(SocialMessageType::MultisigAction.as_u8(), 0xF6);

        // Verify names
        assert_eq!(SocialMessageType::MultisigSetup.name(), "MultisigSetup");
        assert_eq!(SocialMessageType::MultisigAction.name(), "MultisigAction");
    }

    #[test]
    fn multisig_setup_message_roundtrip() {
        let _init_guard = zebra_test::init();

        // MultisigSetup format: [key_count(1)][pubkey1(33)]...[pubkeyN(33)][threshold(1)]
        // Example: 2-of-3 multi-sig setup with 3 compressed public keys

        let mut payload = Vec::new();
        payload.push(3u8); // key_count = 3

        // Add 3 fake compressed public keys (33 bytes each)
        // First byte is 0x02 or 0x03 for compressed keys
        let pubkey1: [u8; 33] = {
            let mut key = [0u8; 33];
            key[0] = 0x02;
            key[1..].copy_from_slice(&[0xAA; 32]);
            key
        };
        let pubkey2: [u8; 33] = {
            let mut key = [0u8; 33];
            key[0] = 0x03;
            key[1..].copy_from_slice(&[0xBB; 32]);
            key
        };
        let pubkey3: [u8; 33] = {
            let mut key = [0u8; 33];
            key[0] = 0x02;
            key[1..].copy_from_slice(&[0xCC; 32]);
            key
        };

        payload.extend_from_slice(&pubkey1);
        payload.extend_from_slice(&pubkey2);
        payload.extend_from_slice(&pubkey3);
        payload.push(2u8); // threshold = 2 (2-of-3)

        let _msg = SocialMessage::new(SocialMessageType::MultisigSetup, SOCIAL_PROTOCOL_VERSION, payload.clone());

        // Encode to memo and parse back
        let mut memo_bytes = vec![SocialMessageType::MultisigSetup.as_u8(), SOCIAL_PROTOCOL_VERSION];
        memo_bytes.extend_from_slice(&payload);
        let memo = create_memo(&memo_bytes);
        let decoded = SocialMessage::try_from(&memo).expect("should parse");

        assert_eq!(decoded.msg_type(), SocialMessageType::MultisigSetup);
        assert_eq!(decoded.version(), SOCIAL_PROTOCOL_VERSION);
        assert_eq!(decoded.payload(), &payload);
        assert!(decoded.msg_type().is_recovery());
        assert!(decoded.msg_type().is_multisig());
    }

    #[test]
    fn multisig_action_message_roundtrip() {
        let _init_guard = zebra_test::init();

        // MultisigAction format: [action_type(1)][action_len(2)][action][sig_count(1)][sig1_idx(1)][sig1(64)]...
        // Example: A multi-sig post with 2 signatures

        let mut payload = Vec::new();

        // The wrapped action: a simple Post
        let inner_action = b"Hello from multisig!";
        payload.push(SocialMessageType::Post.as_u8()); // action_type
        payload.extend_from_slice(&(inner_action.len() as u16).to_le_bytes()); // action_len (2 bytes LE)
        payload.extend_from_slice(inner_action); // action content

        // 2 signatures
        payload.push(2u8); // sig_count

        // Signature 1 from key index 0
        payload.push(0u8); // sig1_idx
        payload.extend_from_slice(&[0x11; 64]); // sig1 (Schnorr signature placeholder)

        // Signature 2 from key index 2
        payload.push(2u8); // sig2_idx
        payload.extend_from_slice(&[0x22; 64]); // sig2 (Schnorr signature placeholder)

        let _msg = SocialMessage::new(SocialMessageType::MultisigAction, SOCIAL_PROTOCOL_VERSION, payload.clone());

        // Encode to memo and parse back
        let mut memo_bytes = vec![SocialMessageType::MultisigAction.as_u8(), SOCIAL_PROTOCOL_VERSION];
        memo_bytes.extend_from_slice(&payload);
        let memo = create_memo(&memo_bytes);
        let decoded = SocialMessage::try_from(&memo).expect("should parse");

        assert_eq!(decoded.msg_type(), SocialMessageType::MultisigAction);
        assert_eq!(decoded.version(), SOCIAL_PROTOCOL_VERSION);
        assert_eq!(decoded.payload(), &payload);
        assert!(decoded.msg_type().is_recovery());
        assert!(decoded.msg_type().is_multisig());
    }

    #[test]
    fn multisig_type_roundtrip_from_u8() {
        let _init_guard = zebra_test::init();

        // Test multisig types can roundtrip through u8
        let setup = SocialMessageType::try_from(0xF5).expect("should parse");
        assert_eq!(setup, SocialMessageType::MultisigSetup);
        assert_eq!(setup.as_u8(), 0xF5);

        let action = SocialMessageType::try_from(0xF6).expect("should parse");
        assert_eq!(action, SocialMessageType::MultisigAction);
        assert_eq!(action.as_u8(), 0xF6);
    }

    #[test]
    fn multisig_is_recovery_helper() {
        let _init_guard = zebra_test::init();

        // Multi-sig types should return true for is_recovery (grouped with recovery types)
        assert!(SocialMessageType::MultisigSetup.is_recovery());
        assert!(SocialMessageType::MultisigAction.is_recovery());

        // And also true for is_multisig
        assert!(SocialMessageType::MultisigSetup.is_multisig());
        assert!(SocialMessageType::MultisigAction.is_multisig());
    }

    #[test]
    fn multisig_not_value_transfer() {
        let _init_guard = zebra_test::init();

        // Multi-sig types are not value transfers
        assert!(!SocialMessageType::MultisigSetup.is_value_transfer());
        assert!(!SocialMessageType::MultisigAction.is_value_transfer());
    }

    #[test]
    fn multisig_not_other_categories() {
        let _init_guard = zebra_test::init();

        // Multi-sig is not governance
        assert!(!SocialMessageType::MultisigSetup.is_governance());
        assert!(!SocialMessageType::MultisigAction.is_governance());

        // Multi-sig is not bridge
        assert!(!SocialMessageType::MultisigSetup.is_bridge());
        assert!(!SocialMessageType::MultisigAction.is_bridge());

        // Multi-sig is not channel
        assert!(!SocialMessageType::MultisigSetup.is_channel());
        assert!(!SocialMessageType::MultisigAction.is_channel());

        // Multi-sig is not moderation
        assert!(!SocialMessageType::MultisigSetup.is_moderation());
        assert!(!SocialMessageType::MultisigAction.is_moderation());

        // Multi-sig is not batch
        assert!(!SocialMessageType::MultisigSetup.is_batch());
        assert!(!SocialMessageType::MultisigAction.is_batch());

        // Multi-sig is not attention market
        assert!(!SocialMessageType::MultisigSetup.is_attention_market());
        assert!(!SocialMessageType::MultisigAction.is_attention_market());
    }

    #[test]
    fn multisig_type_display() {
        let _init_guard = zebra_test::init();

        assert_eq!(format!("{}", SocialMessageType::MultisigSetup), "MultisigSetup");
        assert_eq!(format!("{}", SocialMessageType::MultisigAction), "MultisigAction");
    }

    #[test]
    fn multisig_in_batch() {
        let _init_guard = zebra_test::init();

        // Multi-sig setup can be batched (though unusual)
        let setup_payload = {
            let mut p = Vec::new();
            p.push(2u8); // 2 keys
            p.extend_from_slice(&[0x02; 33]); // pubkey1
            p.extend_from_slice(&[0x03; 33]); // pubkey2
            p.push(2u8); // threshold = 2
            p
        };

        let actions = vec![
            SocialMessage::new(SocialMessageType::MultisigSetup, SOCIAL_PROTOCOL_VERSION, setup_payload),
            SocialMessage::new(SocialMessageType::Post, SOCIAL_PROTOCOL_VERSION, b"After setup!".to_vec()),
        ];

        let batch = BatchMessage::new(actions).expect("batch should be valid");
        let encoded = batch.encode();
        let memo = create_memo(&encoded);
        let decoded = BatchMessage::try_from_memo(&memo).expect("should parse batch");

        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded.actions()[0].msg_type(), SocialMessageType::MultisigSetup);
        assert!(decoded.actions()[0].msg_type().is_multisig());
        assert_eq!(decoded.actions()[1].msg_type(), SocialMessageType::Post);
    }

    #[test]
    fn multisig_setup_with_max_keys() {
        let _init_guard = zebra_test::init();

        // Test setup with 15 keys (maximum supported)
        let mut payload = Vec::new();
        payload.push(15u8); // 15 keys

        for i in 0..15 {
            let mut key = [0u8; 33];
            key[0] = if i % 2 == 0 { 0x02 } else { 0x03 };
            key[1..].copy_from_slice(&[i as u8; 32]);
            payload.extend_from_slice(&key);
        }

        payload.push(10u8); // threshold = 10-of-15

        let mut memo_bytes = vec![SocialMessageType::MultisigSetup.as_u8(), SOCIAL_PROTOCOL_VERSION];
        memo_bytes.extend_from_slice(&payload);
        let memo = create_memo(&memo_bytes);
        let decoded = SocialMessage::try_from(&memo).expect("should parse");

        assert_eq!(decoded.msg_type(), SocialMessageType::MultisigSetup);
        assert!(decoded.msg_type().is_multisig());
        // 1 (key_count) + 15*33 (keys) + 1 (threshold) = 497 bytes
        assert_eq!(decoded.payload().len(), 1 + 15 * 33 + 1);
    }

    #[test]
    fn multisig_setup_with_min_keys() {
        let _init_guard = zebra_test::init();

        // Test setup with 2 keys (minimum for multi-sig)
        let mut payload = Vec::new();
        payload.push(2u8); // 2 keys

        // Add 2 compressed public keys
        payload.extend_from_slice(&[0x02; 33]); // pubkey1
        payload.extend_from_slice(&[0x03; 33]); // pubkey2
        payload.push(1u8); // threshold = 1-of-2

        let mut memo_bytes = vec![SocialMessageType::MultisigSetup.as_u8(), SOCIAL_PROTOCOL_VERSION];
        memo_bytes.extend_from_slice(&payload);
        let memo = create_memo(&memo_bytes);
        let decoded = SocialMessage::try_from(&memo).expect("should parse");

        assert_eq!(decoded.msg_type(), SocialMessageType::MultisigSetup);
        // 1 (key_count) + 2*33 (keys) + 1 (threshold) = 68 bytes
        assert_eq!(decoded.payload().len(), 68);
    }
}
