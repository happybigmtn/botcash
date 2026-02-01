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
        }
    }

    /// Returns true if this is a batch message type.
    pub const fn is_batch(&self) -> bool {
        matches!(self, Self::Batch)
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
        // Social messages use 0x10-0x7F range
        if type_byte < 0x10 || type_byte > 0x7F {
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

        // Test various invalid type bytes
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
    fn all_17_message_types_exist() {
        let _init_guard = zebra_test::init();

        // Verify we have exactly 17 message types (16 core + 1 batch)
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
        ];

        assert_eq!(all_types.len(), 17, "Should have exactly 17 message types");

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
}
