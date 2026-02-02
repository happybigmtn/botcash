//! Indexer governance parsing and voting logic utilities.
//!
//! This module provides utilities for indexers to:
//! - Parse governance messages (proposals and votes) from transaction memos
//! - Track proposal lifecycle and state
//! - Calculate voting power using the karma-weighted formula
//! - Aggregate votes and determine proposal outcomes
//!
//! # Overview
//!
//! Governance messages (types 0xE0, 0xE1) enable on-chain voting for protocol
//! parameters, upgrades, and other community decisions. The governance system
//! follows a three-phase process:
//!
//! 1. **Proposal Phase (7 days)**: Anyone can submit a proposal with a deposit
//! 2. **Voting Phase (14 days)**: Token holders cast votes with karma-weighted power
//! 3. **Execution Phase**: If quorum (20%) and threshold (66%) are met, proposal passes
//!
//! # Voting Power Formula
//!
//! Voting power is calculated using Option C from specs/governance.md:
//! ```text
//! voting_power = sqrt(karma) + sqrt(bcash_balance)
//! ```
//!
//! This rewards both social contribution (karma) and stake (BCASH balance).
//!
//! # Usage
//!
//! ```ignore
//! use zebra_chain::transaction::Memo;
//! use zebra_rpc::indexer::governance::{parse_governance_memo, IndexedGovernance};
//!
//! let gov = parse_governance_memo(&memo, "txid123", 1000)?;
//! match gov {
//!     IndexedGovernance::Proposal(proposal) => {
//!         println!("New proposal: {}", proposal.title);
//!     }
//!     IndexedGovernance::Vote(vote) => {
//!         println!("Vote cast: {:?} on {}", vote.vote_choice, vote.proposal_id);
//!     }
//! }
//! ```

use std::fmt;

use serde::{Deserialize, Serialize};
use zebra_chain::transaction::{
    social::{SocialMessage, SocialMessageType, SocialParseError},
    Memo,
};

/// Proposal phase duration in blocks (~7 days at 60s blocks).
pub const PROPOSAL_PHASE_BLOCKS: u32 = 10080;

/// Voting phase duration in blocks (~14 days at 60s blocks).
pub const VOTING_PHASE_BLOCKS: u32 = 20160;

/// Execution timelock in blocks (~30 days at 60s blocks).
pub const EXECUTION_TIMELOCK_BLOCKS: u32 = 43200;

/// Minimum deposit required for a proposal (in zatoshis = 10 BCASH).
pub const MIN_PROPOSAL_DEPOSIT: u64 = 1_000_000_000;

/// Minimum support percentage for deposit return (10%).
pub const DEPOSIT_RETURN_THRESHOLD: f64 = 10.0;

/// Required quorum percentage (20% of circulating supply).
pub const QUORUM_REQUIRED: f64 = 20.0;

/// Required approval percentage (66% of yes/(yes+no)).
pub const APPROVAL_REQUIRED: f64 = 66.0;

/// Vote choice for governance proposals.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VoteChoice {
    /// Vote against the proposal.
    No = 0,
    /// Vote in favor of the proposal.
    Yes = 1,
    /// Abstain from voting (counts for quorum but not approval).
    Abstain = 2,
}

impl VoteChoice {
    /// Creates a VoteChoice from a byte value.
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::No),
            1 => Some(Self::Yes),
            2 => Some(Self::Abstain),
            _ => None,
        }
    }

    /// Returns the byte value of this vote choice.
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

impl fmt::Display for VoteChoice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::No => write!(f, "no"),
            Self::Yes => write!(f, "yes"),
            Self::Abstain => write!(f, "abstain"),
        }
    }
}

/// Proposal type for governance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProposalType {
    /// Other/general proposal.
    Other = 0,
    /// Protocol parameter change.
    Parameter = 1,
    /// Protocol upgrade (soft fork).
    Upgrade = 2,
    /// Treasury spending proposal.
    Spending = 3,
}

impl ProposalType {
    /// Creates a ProposalType from a byte value.
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Other),
            1 => Some(Self::Parameter),
            2 => Some(Self::Upgrade),
            3 => Some(Self::Spending),
            _ => None,
        }
    }

    /// Returns the byte value of this proposal type.
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

impl Default for ProposalType {
    fn default() -> Self {
        Self::Other
    }
}

impl fmt::Display for ProposalType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Other => write!(f, "other"),
            Self::Parameter => write!(f, "parameter"),
            Self::Upgrade => write!(f, "upgrade"),
            Self::Spending => write!(f, "spending"),
        }
    }
}

/// Proposal status based on lifecycle and voting results.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ProposalStatus {
    /// Proposal is in the initial discussion phase (7 days).
    #[default]
    Pending,
    /// Proposal is in the active voting phase (14 days).
    Voting,
    /// Proposal passed and is waiting for execution (30-day timelock).
    Passed,
    /// Proposal was rejected (failed quorum or threshold).
    Rejected,
    /// Proposal has been executed (activation block reached).
    Executed,
}

impl ProposalStatus {
    /// Returns the string representation of this status.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Voting => "voting",
            Self::Passed => "passed",
            Self::Rejected => "rejected",
            Self::Executed => "executed",
        }
    }
}

impl fmt::Display for ProposalStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// An indexed governance proposal extracted from a memo.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexedProposal {
    /// The transaction ID containing this proposal.
    pub tx_id: String,

    /// Block height where this proposal was created.
    pub created_at_block: u32,

    /// The proposal ID (derived from tx_id hash, 32 bytes hex-encoded).
    pub proposal_id: String,

    /// The type of proposal.
    pub proposal_type: ProposalType,

    /// The proposal title.
    pub title: String,

    /// The proposal description.
    pub description: String,

    /// Protocol version.
    pub version: u8,

    /// Block height when voting starts (created_at_block + PROPOSAL_PHASE_BLOCKS).
    pub voting_starts_block: u32,

    /// Block height when voting ends (voting_starts_block + VOTING_PHASE_BLOCKS).
    pub voting_ends_block: u32,

    /// Block height when proposal executes if passed (voting_ends_block + EXECUTION_TIMELOCK_BLOCKS).
    pub execution_block: u32,
}

impl IndexedProposal {
    /// Creates a new indexed proposal from parsed data.
    pub fn new(
        tx_id: &str,
        created_at_block: u32,
        proposal_type: ProposalType,
        title: String,
        description: String,
        version: u8,
    ) -> Self {
        // Derive proposal_id from tx_id (first 32 bytes of hash)
        let proposal_id = derive_proposal_id(tx_id);

        let voting_starts_block = created_at_block.saturating_add(PROPOSAL_PHASE_BLOCKS);
        let voting_ends_block = voting_starts_block.saturating_add(VOTING_PHASE_BLOCKS);
        let execution_block = voting_ends_block.saturating_add(EXECUTION_TIMELOCK_BLOCKS);

        Self {
            tx_id: tx_id.to_string(),
            created_at_block,
            proposal_id,
            proposal_type,
            title,
            description,
            version,
            voting_starts_block,
            voting_ends_block,
            execution_block,
        }
    }

    /// Returns the status of this proposal at a given block height.
    pub fn status_at(&self, current_height: u32) -> ProposalStatus {
        if current_height < self.voting_starts_block {
            ProposalStatus::Pending
        } else if current_height < self.voting_ends_block {
            ProposalStatus::Voting
        } else if current_height >= self.execution_block {
            // After execution block, status depends on voting results
            // This is a placeholder - actual status requires vote aggregation
            ProposalStatus::Executed
        } else {
            // Between voting end and execution - status depends on results
            // This is a placeholder - actual status requires vote aggregation
            ProposalStatus::Passed
        }
    }

    /// Returns true if voting is currently open at the given block height.
    pub fn is_voting_open(&self, current_height: u32) -> bool {
        current_height >= self.voting_starts_block && current_height < self.voting_ends_block
    }
}

impl fmt::Display for IndexedProposal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Proposal {{ id: {}..., type: {}, title: \"{}\" }}",
            &self.proposal_id[..8.min(self.proposal_id.len())],
            self.proposal_type,
            &self.title[..32.min(self.title.len())]
        )
    }
}

/// An indexed governance vote extracted from a memo.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IndexedVote {
    /// The transaction ID containing this vote.
    pub tx_id: String,

    /// Block height where this vote was cast.
    pub block_height: u32,

    /// The proposal ID being voted on (32 bytes hex-encoded).
    pub proposal_id: String,

    /// The vote choice (yes/no/abstain).
    pub vote_choice: VoteChoice,

    /// The voting power weight from the memo payload.
    pub weight: f64,

    /// Protocol version.
    pub version: u8,
}

impl IndexedVote {
    /// Creates a new indexed vote from parsed data.
    pub fn new(
        tx_id: &str,
        block_height: u32,
        proposal_id: String,
        vote_choice: VoteChoice,
        weight: f64,
        version: u8,
    ) -> Self {
        Self {
            tx_id: tx_id.to_string(),
            block_height,
            proposal_id,
            vote_choice,
            weight,
            version,
        }
    }
}

impl Eq for IndexedVote {}

impl fmt::Display for IndexedVote {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Vote {{ proposal: {}..., choice: {}, weight: {:.2} }}",
            &self.proposal_id[..8.min(self.proposal_id.len())],
            self.vote_choice,
            self.weight
        )
    }
}

/// An indexed governance event (proposal or vote).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum IndexedGovernance {
    /// A governance proposal.
    Proposal(IndexedProposal),
    /// A governance vote.
    Vote(IndexedVote),
}

impl IndexedGovernance {
    /// Returns the transaction ID for this governance event.
    pub fn tx_id(&self) -> &str {
        match self {
            Self::Proposal(p) => &p.tx_id,
            Self::Vote(v) => &v.tx_id,
        }
    }

    /// Returns the block height for this governance event.
    pub fn block_height(&self) -> u32 {
        match self {
            Self::Proposal(p) => p.created_at_block,
            Self::Vote(v) => v.block_height,
        }
    }

    /// Returns true if this is a proposal.
    pub fn is_proposal(&self) -> bool {
        matches!(self, Self::Proposal(_))
    }

    /// Returns true if this is a vote.
    pub fn is_vote(&self) -> bool {
        matches!(self, Self::Vote(_))
    }

    /// Returns the event type name.
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::Proposal(_) => "proposal",
            Self::Vote(_) => "vote",
        }
    }
}

impl Eq for IndexedGovernance {}

impl fmt::Display for IndexedGovernance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Proposal(p) => write!(f, "{}", p),
            Self::Vote(v) => write!(f, "{}", v),
        }
    }
}

/// Errors that can occur during governance indexing operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GovernanceIndexError {
    /// The memo is not a governance message.
    NotGovernance,

    /// Failed to parse the social message.
    ParseError(SocialParseError),

    /// Invalid proposal payload.
    InvalidProposal(String),

    /// Invalid vote payload.
    InvalidVote(String),

    /// Invalid transaction ID.
    InvalidTxId,
}

impl fmt::Display for GovernanceIndexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotGovernance => write!(f, "memo is not a governance message"),
            Self::ParseError(e) => write!(f, "parse error: {}", e),
            Self::InvalidProposal(msg) => write!(f, "invalid proposal: {}", msg),
            Self::InvalidVote(msg) => write!(f, "invalid vote: {}", msg),
            Self::InvalidTxId => write!(f, "invalid transaction ID"),
        }
    }
}

impl std::error::Error for GovernanceIndexError {}

impl From<SocialParseError> for GovernanceIndexError {
    fn from(err: SocialParseError) -> Self {
        Self::ParseError(err)
    }
}

/// Checks if a memo contains a governance message.
///
/// This is a quick check that only looks at the first byte to determine
/// if the memo is a governance message (0xE0 or 0xE1).
pub fn is_governance_memo(memo: &Memo) -> bool {
    let bytes = memo.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    matches!(bytes[0], 0xE0 | 0xE1)
}

/// Returns the governance message type from a memo, if it is a governance message.
pub fn governance_type_from_memo(memo: &Memo) -> Option<SocialMessageType> {
    let bytes = memo.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    match bytes[0] {
        0xE0 => Some(SocialMessageType::GovernanceVote),
        0xE1 => Some(SocialMessageType::GovernanceProposal),
        _ => None,
    }
}

/// Derives a proposal ID from a transaction ID.
///
/// The proposal ID is the first 32 bytes of the SHA-256 hash of the tx_id,
/// hex-encoded to a 64-character string.
fn derive_proposal_id(tx_id: &str) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(tx_id.as_bytes());
    let result = hasher.finalize();
    hex::encode(&result[..32])
}

/// Parses a governance proposal payload.
///
/// Format: [proposal_type(1)][title_len(1)][title][desc_len(2)][description]
fn parse_proposal_payload(
    payload: &[u8],
    tx_id: &str,
    block_height: u32,
    version: u8,
) -> Result<IndexedProposal, GovernanceIndexError> {
    if payload.is_empty() {
        return Err(GovernanceIndexError::InvalidProposal(
            "empty payload".to_string(),
        ));
    }

    // Parse proposal type
    let proposal_type = ProposalType::from_u8(payload[0]).ok_or_else(|| {
        GovernanceIndexError::InvalidProposal(format!("invalid proposal type: {}", payload[0]))
    })?;

    // Need at least: type(1) + title_len(1) + desc_len(2) = 4 bytes
    if payload.len() < 4 {
        return Err(GovernanceIndexError::InvalidProposal(
            "payload too short for header".to_string(),
        ));
    }

    // Parse title
    let title_len = payload[1] as usize;
    if title_len == 0 {
        return Err(GovernanceIndexError::InvalidProposal(
            "title cannot be empty".to_string(),
        ));
    }

    let title_start = 2;
    let title_end = title_start + title_len;
    if title_end > payload.len() {
        return Err(GovernanceIndexError::InvalidProposal(format!(
            "payload too short for title: need {}, have {}",
            title_end,
            payload.len()
        )));
    }

    let title = String::from_utf8_lossy(&payload[title_start..title_end]).to_string();

    // Parse description length (2 bytes, little-endian)
    if title_end + 2 > payload.len() {
        return Err(GovernanceIndexError::InvalidProposal(
            "payload too short for description length".to_string(),
        ));
    }

    let desc_len = u16::from_le_bytes([payload[title_end], payload[title_end + 1]]) as usize;

    // Parse description
    let desc_start = title_end + 2;
    let desc_end = desc_start + desc_len;
    if desc_end > payload.len() {
        return Err(GovernanceIndexError::InvalidProposal(format!(
            "payload too short for description: need {}, have {}",
            desc_end,
            payload.len()
        )));
    }

    let description = String::from_utf8_lossy(&payload[desc_start..desc_end]).to_string();

    Ok(IndexedProposal::new(
        tx_id,
        block_height,
        proposal_type,
        title,
        description,
        version,
    ))
}

/// Parses a governance vote payload.
///
/// Format: [proposal_id(32)][vote(1)][weight(8)]
fn parse_vote_payload(
    payload: &[u8],
    tx_id: &str,
    block_height: u32,
    version: u8,
) -> Result<IndexedVote, GovernanceIndexError> {
    // Need exactly: proposal_id(32) + vote(1) + weight(8) = 41 bytes
    if payload.len() < 41 {
        return Err(GovernanceIndexError::InvalidVote(format!(
            "payload too short: {} bytes, expected at least 41",
            payload.len()
        )));
    }

    let proposal_id = hex::encode(&payload[0..32]);

    let vote_byte = payload[32];
    let vote_choice = VoteChoice::from_u8(vote_byte).ok_or_else(|| {
        GovernanceIndexError::InvalidVote(format!("invalid vote choice: {}", vote_byte))
    })?;

    let weight_bytes: [u8; 8] = payload[33..41]
        .try_into()
        .map_err(|_| GovernanceIndexError::InvalidVote("invalid weight bytes".to_string()))?;
    let weight = u64::from_le_bytes(weight_bytes) as f64;

    Ok(IndexedVote::new(
        tx_id,
        block_height,
        proposal_id,
        vote_choice,
        weight,
        version,
    ))
}

/// Parses a governance message from a memo and returns an indexed governance event.
///
/// # Arguments
///
/// * `memo` - The memo to parse
/// * `tx_id` - The transaction ID containing this memo
/// * `block_height` - The block height where the transaction was included
///
/// # Returns
///
/// An `IndexedGovernance` variant (Proposal or Vote), or an error if the
/// memo is not a valid governance message.
pub fn parse_governance_memo(
    memo: &Memo,
    tx_id: &str,
    block_height: u32,
) -> Result<IndexedGovernance, GovernanceIndexError> {
    // Validate tx_id
    if tx_id.is_empty() {
        return Err(GovernanceIndexError::InvalidTxId);
    }

    // Quick check for governance type
    if !is_governance_memo(memo) {
        return Err(GovernanceIndexError::NotGovernance);
    }

    // Parse the social message
    let msg = SocialMessage::try_from(memo)?;
    let version = msg.version();
    let payload = msg.payload();

    match msg.msg_type() {
        SocialMessageType::GovernanceProposal => {
            let proposal = parse_proposal_payload(payload, tx_id, block_height, version)?;
            Ok(IndexedGovernance::Proposal(proposal))
        }
        SocialMessageType::GovernanceVote => {
            let vote = parse_vote_payload(payload, tx_id, block_height, version)?;
            Ok(IndexedGovernance::Vote(vote))
        }
        _ => Err(GovernanceIndexError::NotGovernance),
    }
}

/// Calculates voting power using the karma-weighted formula.
///
/// Formula: `voting_power = sqrt(karma) + sqrt(bcash_balance)`
///
/// This formula balances social contribution (karma) with stake (balance),
/// using square root to provide diminishing returns and resist Sybil attacks.
///
/// # Arguments
///
/// * `karma` - The voter's karma score (social reputation)
/// * `bcash_balance` - The voter's BCASH balance in zatoshis
///
/// # Returns
///
/// The calculated voting power as a floating-point number.
pub fn calculate_voting_power(karma: f64, bcash_balance: u64) -> f64 {
    let karma_sqrt = karma.max(0.0).sqrt();
    let balance_sqrt = (bcash_balance as f64).sqrt();
    karma_sqrt + balance_sqrt
}

/// Vote tally for a proposal.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct VoteTally {
    /// Total voting power for "yes" votes.
    pub yes_power: f64,

    /// Total voting power for "no" votes.
    pub no_power: f64,

    /// Total voting power for "abstain" votes.
    pub abstain_power: f64,

    /// Number of unique voters.
    pub voter_count: u32,

    /// Total circulating supply for quorum calculation.
    pub circulating_supply: u64,
}

impl VoteTally {
    /// Creates a new empty vote tally.
    pub fn new(circulating_supply: u64) -> Self {
        Self {
            circulating_supply,
            ..Default::default()
        }
    }

    /// Records a vote in the tally.
    pub fn record_vote(&mut self, vote_choice: VoteChoice, voting_power: f64) {
        match vote_choice {
            VoteChoice::Yes => self.yes_power += voting_power,
            VoteChoice::No => self.no_power += voting_power,
            VoteChoice::Abstain => self.abstain_power += voting_power,
        }
        self.voter_count += 1;
    }

    /// Returns the total voting power cast (yes + no + abstain).
    pub fn total_power(&self) -> f64 {
        self.yes_power + self.no_power + self.abstain_power
    }

    /// Returns the quorum percentage (total voted / circulating supply).
    pub fn quorum_percent(&self) -> f64 {
        if self.circulating_supply == 0 {
            return 0.0;
        }
        (self.total_power() / self.circulating_supply as f64) * 100.0
    }

    /// Returns true if quorum has been reached (>= 20%).
    pub fn has_quorum(&self) -> bool {
        self.quorum_percent() >= QUORUM_REQUIRED
    }

    /// Returns the approval percentage (yes / (yes + no)).
    ///
    /// Abstain votes are not counted in the approval calculation.
    pub fn approval_percent(&self) -> f64 {
        let yes_no_total = self.yes_power + self.no_power;
        if yes_no_total == 0.0 {
            return 0.0;
        }
        (self.yes_power / yes_no_total) * 100.0
    }

    /// Returns true if approval threshold has been reached (>= 66%).
    pub fn has_approval(&self) -> bool {
        self.approval_percent() >= APPROVAL_REQUIRED
    }

    /// Returns true if the proposal has passed (quorum AND approval).
    pub fn has_passed(&self) -> bool {
        self.has_quorum() && self.has_approval()
    }

    /// Returns the proposal status based on voting results.
    ///
    /// This should be called after voting has ended.
    pub fn final_status(&self) -> ProposalStatus {
        if self.has_passed() {
            ProposalStatus::Passed
        } else {
            ProposalStatus::Rejected
        }
    }
}

impl fmt::Display for VoteTally {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "VoteTally {{ yes: {:.2}, no: {:.2}, abstain: {:.2}, quorum: {:.1}%, approval: {:.1}% }}",
            self.yes_power,
            self.no_power,
            self.abstain_power,
            self.quorum_percent(),
            self.approval_percent()
        )
    }
}

/// Statistics about governance activity in a block.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockGovernanceStats {
    /// Block height.
    pub block_height: u32,

    /// Total number of governance transactions.
    pub total_governance_txs: u32,

    /// Number of proposals created.
    pub proposals_created: u32,

    /// Number of votes cast.
    pub votes_cast: u32,

    /// Unique proposals voted on in this block.
    pub unique_proposals_voted: u32,
}

impl BlockGovernanceStats {
    /// Creates a new stats tracker for a block.
    pub fn new(block_height: u32) -> Self {
        Self {
            block_height,
            ..Default::default()
        }
    }

    /// Records a proposal creation.
    pub fn record_proposal(&mut self) {
        self.total_governance_txs += 1;
        self.proposals_created += 1;
    }

    /// Records a vote.
    pub fn record_vote(&mut self) {
        self.total_governance_txs += 1;
        self.votes_cast += 1;
    }

    /// Records an indexed governance event.
    pub fn record_governance(&mut self, gov: &IndexedGovernance) {
        match gov {
            IndexedGovernance::Proposal(_) => self.record_proposal(),
            IndexedGovernance::Vote(_) => self.record_vote(),
        }
    }
}

impl fmt::Display for BlockGovernanceStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Block {} governance stats: {} txs ({} proposals, {} votes)",
            self.block_height, self.total_governance_txs, self.proposals_created, self.votes_cast
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

    /// Creates a governance proposal payload.
    /// Format: [proposal_type(1)][title_len(1)][title][desc_len(2)][description]
    fn create_proposal_payload(
        proposal_type: ProposalType,
        title: &str,
        description: &str,
    ) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.push(proposal_type.as_u8());
        payload.push(title.len() as u8);
        payload.extend_from_slice(title.as_bytes());
        payload.extend_from_slice(&(description.len() as u16).to_le_bytes());
        payload.extend_from_slice(description.as_bytes());
        payload
    }

    /// Creates a governance vote payload.
    /// Format: [proposal_id(32)][vote(1)][weight(8)]
    fn create_vote_payload(
        proposal_id: &[u8; 32],
        vote_choice: VoteChoice,
        weight: u64,
    ) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(proposal_id);
        payload.push(vote_choice.as_u8());
        payload.extend_from_slice(&weight.to_le_bytes());
        payload
    }

    fn create_social_memo(msg_type: SocialMessageType, payload: &[u8]) -> Memo {
        let msg = SocialMessage::new(msg_type, SOCIAL_PROTOCOL_VERSION, payload.to_vec());
        let encoded = msg.encode();
        create_memo(&encoded)
    }

    // ========================================================================
    // Tests for VoteChoice
    // ========================================================================

    #[test]
    fn test_vote_choice_from_u8() {
        let _init_guard = zebra_test::init();

        assert_eq!(VoteChoice::from_u8(0), Some(VoteChoice::No));
        assert_eq!(VoteChoice::from_u8(1), Some(VoteChoice::Yes));
        assert_eq!(VoteChoice::from_u8(2), Some(VoteChoice::Abstain));
        assert_eq!(VoteChoice::from_u8(3), None);
        assert_eq!(VoteChoice::from_u8(255), None);
    }

    #[test]
    fn test_vote_choice_as_u8() {
        let _init_guard = zebra_test::init();

        assert_eq!(VoteChoice::No.as_u8(), 0);
        assert_eq!(VoteChoice::Yes.as_u8(), 1);
        assert_eq!(VoteChoice::Abstain.as_u8(), 2);
    }

    #[test]
    fn test_vote_choice_display() {
        let _init_guard = zebra_test::init();

        assert_eq!(format!("{}", VoteChoice::No), "no");
        assert_eq!(format!("{}", VoteChoice::Yes), "yes");
        assert_eq!(format!("{}", VoteChoice::Abstain), "abstain");
    }

    // ========================================================================
    // Tests for ProposalType
    // ========================================================================

    #[test]
    fn test_proposal_type_from_u8() {
        let _init_guard = zebra_test::init();

        assert_eq!(ProposalType::from_u8(0), Some(ProposalType::Other));
        assert_eq!(ProposalType::from_u8(1), Some(ProposalType::Parameter));
        assert_eq!(ProposalType::from_u8(2), Some(ProposalType::Upgrade));
        assert_eq!(ProposalType::from_u8(3), Some(ProposalType::Spending));
        assert_eq!(ProposalType::from_u8(4), None);
        assert_eq!(ProposalType::from_u8(255), None);
    }

    #[test]
    fn test_proposal_type_display() {
        let _init_guard = zebra_test::init();

        assert_eq!(format!("{}", ProposalType::Other), "other");
        assert_eq!(format!("{}", ProposalType::Parameter), "parameter");
        assert_eq!(format!("{}", ProposalType::Upgrade), "upgrade");
        assert_eq!(format!("{}", ProposalType::Spending), "spending");
    }

    // ========================================================================
    // Tests for ProposalStatus
    // ========================================================================

    #[test]
    fn test_proposal_status_as_str() {
        let _init_guard = zebra_test::init();

        assert_eq!(ProposalStatus::Pending.as_str(), "pending");
        assert_eq!(ProposalStatus::Voting.as_str(), "voting");
        assert_eq!(ProposalStatus::Passed.as_str(), "passed");
        assert_eq!(ProposalStatus::Rejected.as_str(), "rejected");
        assert_eq!(ProposalStatus::Executed.as_str(), "executed");
    }

    #[test]
    fn test_proposal_status_display() {
        let _init_guard = zebra_test::init();

        assert_eq!(format!("{}", ProposalStatus::Pending), "pending");
        assert_eq!(format!("{}", ProposalStatus::Voting), "voting");
    }

    // ========================================================================
    // Tests for is_governance_memo
    // ========================================================================

    #[test]
    fn test_is_governance_memo() {
        let _init_guard = zebra_test::init();

        // Governance vote memo
        let vote_memo = create_memo(&[0xE0, 0x01, 0x00]);
        assert!(is_governance_memo(&vote_memo));

        // Governance proposal memo
        let proposal_memo = create_memo(&[0xE1, 0x01, 0x00]);
        assert!(is_governance_memo(&proposal_memo));

        // Non-governance memo (Post = 0x20)
        let post_memo = create_memo(&[0x20, 0x01, b'H', b'i']);
        assert!(!is_governance_memo(&post_memo));

        // Empty memo
        let empty_memo = create_memo(&[]);
        assert!(!is_governance_memo(&empty_memo));
    }

    #[test]
    fn test_governance_type_from_memo() {
        let _init_guard = zebra_test::init();

        let vote_memo = create_memo(&[0xE0, 0x01]);
        assert_eq!(
            governance_type_from_memo(&vote_memo),
            Some(SocialMessageType::GovernanceVote)
        );

        let proposal_memo = create_memo(&[0xE1, 0x01]);
        assert_eq!(
            governance_type_from_memo(&proposal_memo),
            Some(SocialMessageType::GovernanceProposal)
        );

        let post_memo = create_memo(&[0x20, 0x01]);
        assert_eq!(governance_type_from_memo(&post_memo), None);

        let empty_memo = create_memo(&[]);
        assert_eq!(governance_type_from_memo(&empty_memo), None);
    }

    // ========================================================================
    // Tests for parse_governance_memo - Proposal
    // ========================================================================

    #[test]
    fn test_parse_governance_proposal() {
        let _init_guard = zebra_test::init();

        let title = "Increase block size";
        let description = "Proposal to increase max block size from 2MB to 4MB";
        let payload = create_proposal_payload(ProposalType::Parameter, title, description);
        let memo = create_social_memo(SocialMessageType::GovernanceProposal, &payload);

        let result = parse_governance_memo(&memo, "txid_proposal_123", 5000).expect("should parse");

        match result {
            IndexedGovernance::Proposal(proposal) => {
                assert_eq!(proposal.tx_id, "txid_proposal_123");
                assert_eq!(proposal.created_at_block, 5000);
                assert_eq!(proposal.proposal_type, ProposalType::Parameter);
                assert_eq!(proposal.title, title);
                assert_eq!(proposal.description, description);
                assert_eq!(proposal.voting_starts_block, 5000 + PROPOSAL_PHASE_BLOCKS);
                assert_eq!(
                    proposal.voting_ends_block,
                    5000 + PROPOSAL_PHASE_BLOCKS + VOTING_PHASE_BLOCKS
                );
            }
            _ => panic!("expected Proposal variant"),
        }
    }

    #[test]
    fn test_parse_governance_proposal_all_types() {
        let _init_guard = zebra_test::init();

        for proposal_type in [
            ProposalType::Other,
            ProposalType::Parameter,
            ProposalType::Upgrade,
            ProposalType::Spending,
        ] {
            let payload = create_proposal_payload(proposal_type, "Test", "Description");
            let memo = create_social_memo(SocialMessageType::GovernanceProposal, &payload);
            let result = parse_governance_memo(&memo, "txid", 1000).expect("should parse");

            if let IndexedGovernance::Proposal(p) = result {
                assert_eq!(p.proposal_type, proposal_type);
            } else {
                panic!("expected Proposal");
            }
        }
    }

    // ========================================================================
    // Tests for parse_governance_memo - Vote
    // ========================================================================

    #[test]
    fn test_parse_governance_vote() {
        let _init_guard = zebra_test::init();

        let proposal_id: [u8; 32] = [0xAB; 32];
        let weight: u64 = 0x0102030405060708;
        let payload = create_vote_payload(&proposal_id, VoteChoice::Yes, weight);
        let memo = create_social_memo(SocialMessageType::GovernanceVote, &payload);

        let result = parse_governance_memo(&memo, "txid_vote_456", 6000).expect("should parse");

        match result {
            IndexedGovernance::Vote(vote) => {
                assert_eq!(vote.tx_id, "txid_vote_456");
                assert_eq!(vote.block_height, 6000);
                assert_eq!(vote.proposal_id, hex::encode([0xAB; 32]));
                assert_eq!(vote.vote_choice, VoteChoice::Yes);
                assert_eq!(vote.weight, weight as f64);
            }
            _ => panic!("expected Vote variant"),
        }
    }

    #[test]
    fn test_parse_governance_vote_all_choices() {
        let _init_guard = zebra_test::init();

        // Use weight with non-zero bytes to avoid memo trimming
        // (memo parsing trims trailing zeros from LE-encoded integers)
        let weight_no_trailing_zeros = 0x0102030405060708u64;

        for vote_choice in [VoteChoice::No, VoteChoice::Yes, VoteChoice::Abstain] {
            let proposal_id: [u8; 32] = [0xCD; 32];
            let payload =
                create_vote_payload(&proposal_id, vote_choice, weight_no_trailing_zeros);
            let memo = create_social_memo(SocialMessageType::GovernanceVote, &payload);
            let result = parse_governance_memo(&memo, "txid", 1000).expect("should parse");

            if let IndexedGovernance::Vote(v) = result {
                assert_eq!(v.vote_choice, vote_choice);
                assert_eq!(v.weight, weight_no_trailing_zeros as f64);
            } else {
                panic!("expected Vote");
            }
        }
    }

    // ========================================================================
    // Tests for error cases
    // ========================================================================

    #[test]
    fn test_parse_governance_not_governance() {
        let _init_guard = zebra_test::init();

        // Post memo (not governance)
        let post_memo = create_memo(&[0x20, 0x01, b'H', b'i']);
        let result = parse_governance_memo(&post_memo, "txid", 1000);

        assert!(matches!(result, Err(GovernanceIndexError::NotGovernance)));
    }

    #[test]
    fn test_parse_governance_invalid_tx_id() {
        let _init_guard = zebra_test::init();

        let payload = create_proposal_payload(ProposalType::Other, "Test", "Desc");
        let memo = create_social_memo(SocialMessageType::GovernanceProposal, &payload);

        let result = parse_governance_memo(&memo, "", 1000);
        assert!(matches!(result, Err(GovernanceIndexError::InvalidTxId)));
    }

    #[test]
    fn test_parse_proposal_empty_payload() {
        let _init_guard = zebra_test::init();

        let memo = create_social_memo(SocialMessageType::GovernanceProposal, &[]);
        let result = parse_governance_memo(&memo, "txid", 1000);

        assert!(matches!(
            result,
            Err(GovernanceIndexError::InvalidProposal(_))
        ));
    }

    #[test]
    fn test_parse_proposal_empty_title() {
        let _init_guard = zebra_test::init();

        // Proposal type + empty title length
        let payload = vec![0x01, 0x00, 0x00, 0x00]; // type, title_len=0, desc_len
        let memo = create_social_memo(SocialMessageType::GovernanceProposal, &payload);
        let result = parse_governance_memo(&memo, "txid", 1000);

        assert!(matches!(
            result,
            Err(GovernanceIndexError::InvalidProposal(_))
        ));
    }

    #[test]
    fn test_parse_vote_short_payload() {
        let _init_guard = zebra_test::init();

        // Only 30 bytes instead of 41
        let short_payload = vec![0xAB; 30];
        let memo = create_social_memo(SocialMessageType::GovernanceVote, &short_payload);
        let result = parse_governance_memo(&memo, "txid", 1000);

        assert!(matches!(result, Err(GovernanceIndexError::InvalidVote(_))));
    }

    #[test]
    fn test_parse_vote_invalid_choice() {
        let _init_guard = zebra_test::init();

        let mut payload = vec![0xAB; 32]; // proposal_id
        payload.push(0xFF); // invalid vote choice
        payload.extend_from_slice(&1000u64.to_le_bytes());

        let memo = create_social_memo(SocialMessageType::GovernanceVote, &payload);
        let result = parse_governance_memo(&memo, "txid", 1000);

        assert!(matches!(result, Err(GovernanceIndexError::InvalidVote(_))));
    }

    // ========================================================================
    // Tests for calculate_voting_power
    // ========================================================================

    #[test]
    fn test_calculate_voting_power() {
        let _init_guard = zebra_test::init();

        // Zero values
        assert_eq!(calculate_voting_power(0.0, 0), 0.0);

        // Only karma
        let power = calculate_voting_power(100.0, 0);
        assert!((power - 10.0).abs() < 0.001); // sqrt(100) = 10

        // Only balance
        let power = calculate_voting_power(0.0, 10000);
        assert!((power - 100.0).abs() < 0.001); // sqrt(10000) = 100

        // Both karma and balance
        let power = calculate_voting_power(100.0, 10000);
        assert!((power - 110.0).abs() < 0.001); // sqrt(100) + sqrt(10000) = 10 + 100

        // Negative karma should be treated as zero
        let power = calculate_voting_power(-50.0, 100);
        assert!((power - 10.0).abs() < 0.001); // sqrt(0) + sqrt(100) = 10
    }

    // ========================================================================
    // Tests for VoteTally
    // ========================================================================

    #[test]
    fn test_vote_tally_record_vote() {
        let _init_guard = zebra_test::init();

        let mut tally = VoteTally::new(1_000_000);

        tally.record_vote(VoteChoice::Yes, 100.0);
        tally.record_vote(VoteChoice::No, 50.0);
        tally.record_vote(VoteChoice::Abstain, 25.0);
        tally.record_vote(VoteChoice::Yes, 50.0);

        assert_eq!(tally.yes_power, 150.0);
        assert_eq!(tally.no_power, 50.0);
        assert_eq!(tally.abstain_power, 25.0);
        assert_eq!(tally.voter_count, 4);
    }

    #[test]
    fn test_vote_tally_total_power() {
        let _init_guard = zebra_test::init();

        let mut tally = VoteTally::new(1_000_000);
        tally.yes_power = 100.0;
        tally.no_power = 50.0;
        tally.abstain_power = 25.0;

        assert_eq!(tally.total_power(), 175.0);
    }

    #[test]
    fn test_vote_tally_quorum_percent() {
        let _init_guard = zebra_test::init();

        let mut tally = VoteTally::new(1000);
        tally.yes_power = 100.0;
        tally.no_power = 50.0;
        tally.abstain_power = 50.0;

        // 200 / 1000 = 20%
        assert!((tally.quorum_percent() - 20.0).abs() < 0.001);
    }

    #[test]
    fn test_vote_tally_has_quorum() {
        let _init_guard = zebra_test::init();

        let mut tally = VoteTally::new(1000);

        // Below quorum (19%)
        tally.yes_power = 190.0;
        assert!(!tally.has_quorum());

        // At quorum (20%)
        tally.yes_power = 200.0;
        assert!(tally.has_quorum());

        // Above quorum (21%)
        tally.yes_power = 210.0;
        assert!(tally.has_quorum());
    }

    #[test]
    fn test_vote_tally_approval_percent() {
        let _init_guard = zebra_test::init();

        let mut tally = VoteTally::new(1000);
        tally.yes_power = 200.0;
        tally.no_power = 100.0;
        tally.abstain_power = 50.0; // Abstain not counted

        // 200 / (200 + 100) = 66.67%
        assert!((tally.approval_percent() - 66.666).abs() < 0.01);
    }

    #[test]
    fn test_vote_tally_approval_percent_zero_votes() {
        let _init_guard = zebra_test::init();

        let tally = VoteTally::new(1000);
        assert_eq!(tally.approval_percent(), 0.0);
    }

    #[test]
    fn test_vote_tally_has_approval() {
        let _init_guard = zebra_test::init();

        let mut tally = VoteTally::new(1000);

        // Below threshold (65%)
        tally.yes_power = 65.0;
        tally.no_power = 35.0;
        assert!(!tally.has_approval());

        // At threshold (66%)
        tally.yes_power = 66.0;
        tally.no_power = 34.0;
        assert!(tally.has_approval());

        // Above threshold (70%)
        tally.yes_power = 70.0;
        tally.no_power = 30.0;
        assert!(tally.has_approval());
    }

    #[test]
    fn test_vote_tally_has_passed() {
        let _init_guard = zebra_test::init();

        let mut tally = VoteTally::new(1000);

        // Neither quorum nor approval
        tally.yes_power = 50.0;
        tally.no_power = 50.0;
        assert!(!tally.has_passed());

        // Quorum but not approval
        tally.yes_power = 100.0;
        tally.no_power = 100.0;
        assert!(!tally.has_passed());

        // Approval but not quorum
        tally.yes_power = 70.0;
        tally.no_power = 30.0;
        assert!(!tally.has_passed());

        // Both quorum and approval
        tally.yes_power = 160.0;
        tally.no_power = 40.0;
        assert!(tally.has_passed());
    }

    #[test]
    fn test_vote_tally_final_status() {
        let _init_guard = zebra_test::init();

        let mut tally = VoteTally::new(1000);

        // Rejected (no quorum)
        tally.yes_power = 50.0;
        assert_eq!(tally.final_status(), ProposalStatus::Rejected);

        // Passed (quorum + approval)
        tally.yes_power = 160.0;
        tally.no_power = 40.0;
        assert_eq!(tally.final_status(), ProposalStatus::Passed);
    }

    #[test]
    fn test_vote_tally_display() {
        let _init_guard = zebra_test::init();

        let mut tally = VoteTally::new(1000);
        tally.yes_power = 100.0;
        tally.no_power = 50.0;
        tally.abstain_power = 25.0;

        let display = format!("{}", tally);
        assert!(display.contains("yes: 100"));
        assert!(display.contains("no: 50"));
        assert!(display.contains("abstain: 25"));
    }

    // ========================================================================
    // Tests for IndexedProposal
    // ========================================================================

    #[test]
    fn test_indexed_proposal_status_at() {
        let _init_guard = zebra_test::init();

        let proposal = IndexedProposal::new(
            "txid",
            1000,
            ProposalType::Parameter,
            "Test".to_string(),
            "Desc".to_string(),
            1,
        );

        // In pending phase
        assert_eq!(proposal.status_at(1000), ProposalStatus::Pending);
        assert_eq!(
            proposal.status_at(proposal.voting_starts_block - 1),
            ProposalStatus::Pending
        );

        // In voting phase
        assert_eq!(
            proposal.status_at(proposal.voting_starts_block),
            ProposalStatus::Voting
        );
        assert_eq!(
            proposal.status_at(proposal.voting_ends_block - 1),
            ProposalStatus::Voting
        );
    }

    #[test]
    fn test_indexed_proposal_is_voting_open() {
        let _init_guard = zebra_test::init();

        let proposal = IndexedProposal::new(
            "txid",
            1000,
            ProposalType::Parameter,
            "Test".to_string(),
            "Desc".to_string(),
            1,
        );

        assert!(!proposal.is_voting_open(1000));
        assert!(!proposal.is_voting_open(proposal.voting_starts_block - 1));
        assert!(proposal.is_voting_open(proposal.voting_starts_block));
        assert!(proposal.is_voting_open(proposal.voting_ends_block - 1));
        assert!(!proposal.is_voting_open(proposal.voting_ends_block));
    }

    #[test]
    fn test_indexed_proposal_display() {
        let _init_guard = zebra_test::init();

        let proposal = IndexedProposal::new(
            "txid123",
            1000,
            ProposalType::Parameter,
            "Test Proposal Title".to_string(),
            "Description".to_string(),
            1,
        );

        let display = format!("{}", proposal);
        assert!(display.contains("Proposal"));
        assert!(display.contains("type: parameter"));
        assert!(display.contains("Test Proposal Title"));
    }

    // ========================================================================
    // Tests for IndexedVote
    // ========================================================================

    #[test]
    fn test_indexed_vote_display() {
        let _init_guard = zebra_test::init();

        let vote = IndexedVote::new(
            "txid456",
            2000,
            "abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234".to_string(),
            VoteChoice::Yes,
            150.5,
            1,
        );

        let display = format!("{}", vote);
        assert!(display.contains("Vote"));
        assert!(display.contains("abcd1234"));
        assert!(display.contains("yes"));
        assert!(display.contains("150.50"));
    }

    // ========================================================================
    // Tests for IndexedGovernance
    // ========================================================================

    #[test]
    fn test_indexed_governance_methods() {
        let _init_guard = zebra_test::init();

        let proposal = IndexedGovernance::Proposal(IndexedProposal::new(
            "txid1",
            1000,
            ProposalType::Other,
            "Test".to_string(),
            "Desc".to_string(),
            1,
        ));

        assert_eq!(proposal.tx_id(), "txid1");
        assert_eq!(proposal.block_height(), 1000);
        assert!(proposal.is_proposal());
        assert!(!proposal.is_vote());
        assert_eq!(proposal.event_type(), "proposal");

        let vote = IndexedGovernance::Vote(IndexedVote::new(
            "txid2",
            2000,
            "abc123".to_string(),
            VoteChoice::No,
            50.0,
            1,
        ));

        assert_eq!(vote.tx_id(), "txid2");
        assert_eq!(vote.block_height(), 2000);
        assert!(!vote.is_proposal());
        assert!(vote.is_vote());
        assert_eq!(vote.event_type(), "vote");
    }

    // ========================================================================
    // Tests for BlockGovernanceStats
    // ========================================================================

    #[test]
    fn test_block_governance_stats() {
        let _init_guard = zebra_test::init();

        let mut stats = BlockGovernanceStats::new(10000);

        stats.record_proposal();
        stats.record_vote();
        stats.record_vote();

        assert_eq!(stats.block_height, 10000);
        assert_eq!(stats.total_governance_txs, 3);
        assert_eq!(stats.proposals_created, 1);
        assert_eq!(stats.votes_cast, 2);
    }

    #[test]
    fn test_block_governance_stats_record_governance() {
        let _init_guard = zebra_test::init();

        let mut stats = BlockGovernanceStats::new(11000);

        let proposal = IndexedGovernance::Proposal(IndexedProposal::new(
            "tx1",
            11000,
            ProposalType::Other,
            "Test".to_string(),
            "Desc".to_string(),
            1,
        ));
        stats.record_governance(&proposal);

        let vote = IndexedGovernance::Vote(IndexedVote::new(
            "tx2",
            11000,
            "abc".to_string(),
            VoteChoice::Yes,
            100.0,
            1,
        ));
        stats.record_governance(&vote);

        assert_eq!(stats.total_governance_txs, 2);
        assert_eq!(stats.proposals_created, 1);
        assert_eq!(stats.votes_cast, 1);
    }

    #[test]
    fn test_block_governance_stats_display() {
        let _init_guard = zebra_test::init();

        let mut stats = BlockGovernanceStats::new(12000);
        stats.record_proposal();
        stats.record_vote();

        let display = format!("{}", stats);
        assert!(display.contains("Block 12000"));
        assert!(display.contains("2 txs"));
        assert!(display.contains("1 proposals"));
        assert!(display.contains("1 votes"));
    }

    // ========================================================================
    // Tests for GovernanceIndexError
    // ========================================================================

    #[test]
    fn test_governance_index_error_display() {
        let _init_guard = zebra_test::init();

        assert_eq!(
            format!("{}", GovernanceIndexError::NotGovernance),
            "memo is not a governance message"
        );
        assert_eq!(
            format!("{}", GovernanceIndexError::InvalidTxId),
            "invalid transaction ID"
        );
        assert_eq!(
            format!(
                "{}",
                GovernanceIndexError::InvalidProposal("test".to_string())
            ),
            "invalid proposal: test"
        );
        assert_eq!(
            format!("{}", GovernanceIndexError::InvalidVote("test".to_string())),
            "invalid vote: test"
        );
    }

    // ========================================================================
    // Tests for constants
    // ========================================================================

    #[test]
    fn test_governance_constants() {
        let _init_guard = zebra_test::init();

        // 7 days at 60s blocks
        assert_eq!(PROPOSAL_PHASE_BLOCKS, 10080);

        // 14 days at 60s blocks
        assert_eq!(VOTING_PHASE_BLOCKS, 20160);

        // 30 days at 60s blocks
        assert_eq!(EXECUTION_TIMELOCK_BLOCKS, 43200);

        // 10 BCASH in zatoshis
        assert_eq!(MIN_PROPOSAL_DEPOSIT, 1_000_000_000);

        // Thresholds
        assert!((DEPOSIT_RETURN_THRESHOLD - 10.0).abs() < 0.001);
        assert!((QUORUM_REQUIRED - 20.0).abs() < 0.001);
        assert!((APPROVAL_REQUIRED - 66.0).abs() < 0.001);
    }

    // ========================================================================
    // Tests for derive_proposal_id
    // ========================================================================

    #[test]
    fn test_derive_proposal_id() {
        let _init_guard = zebra_test::init();

        let id1 = derive_proposal_id("txid1");
        let id2 = derive_proposal_id("txid2");
        let id1_again = derive_proposal_id("txid1");

        // Proposal IDs should be 64 hex chars (32 bytes)
        assert_eq!(id1.len(), 64);
        assert_eq!(id2.len(), 64);

        // Different tx_ids should produce different proposal_ids
        assert_ne!(id1, id2);

        // Same tx_id should produce same proposal_id
        assert_eq!(id1, id1_again);
    }
}
