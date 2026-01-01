//! Runtime API for Governance pallet.
//!
//! Provides offchain access to governance state snapshots.

use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_std::prelude::*;

/// Proposal snapshot for API.
#[derive(Clone, Encode, Decode, TypeInfo, Debug, Default)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct ProposalSnapshot {
    /// Proposal ID.
    pub proposal_id: u32,
    /// Proposer account (as bytes).
    pub proposer: Vec<u8>,
    /// Description/title.
    pub description: Vec<u8>,
    /// Status (0=Voting, 1=Approved, 2=Rejected, 3=Cancelled, 4=Executed).
    pub status: u8,
    /// Ayes voting power.
    pub ayes: u128,
    /// Nays voting power.
    pub nays: u128,
    /// Abstain voting power.
    pub abstains: u128,
    /// Total turnout.
    pub turnout: u128,
    /// Deposit amount.
    pub deposit: u128,
    /// Block when created.
    pub created_at: u64,
    /// Block when voting ends.
    pub voting_end: u64,
    /// Delay before execution.
    pub execution_delay: u64,
}

/// Governance configuration snapshot.
#[derive(Clone, Encode, Decode, TypeInfo, Debug, Default)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct GovernanceConfigSnapshot {
    /// Voting period in blocks.
    pub voting_period: u64,
    /// Execution delay in blocks.
    pub execution_delay: u64,
    /// Minimum deposit required.
    pub minimum_deposit: u128,
    /// Quorum threshold (percent * 100).
    pub quorum_threshold: u32,
    /// Approval threshold (percent * 100).
    pub approval_threshold: u32,
    /// Total issuance.
    pub total_issuance: u128,
}

/// Full governance snapshot for API responses.
#[derive(Clone, Encode, Decode, TypeInfo, Debug, Default)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct GovernanceSnapshotResponse {
    /// Current block number.
    pub block_number: u64,
    /// Active proposals.
    pub active_proposals: Vec<ProposalSnapshot>,
    /// Total proposal count.
    pub total_proposals: u32,
    /// Configuration.
    pub config: GovernanceConfigSnapshot,
}

/// Vote record for API.
#[derive(Clone, Encode, Decode, TypeInfo, Debug, Default)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct VoteRecord {
    /// Voter account (as bytes).
    pub voter: Vec<u8>,
    /// Vote direction (0=Aye, 1=Nay, 2=Abstain).
    pub direction: u8,
    /// Vote amount (base, before conviction).
    pub amount: u128,
    /// Voting power (after conviction multiplier).
    pub voting_power: u128,
    /// Conviction level (0-6).
    pub conviction: u8,
}

sp_api::decl_runtime_apis! {
    /// Governance Runtime API.
    pub trait GovernanceApi {
        /// Get full governance snapshot.
        fn get_governance_snapshot() -> GovernanceSnapshotResponse;

        /// Get proposal by ID.
        fn get_proposal(proposal_id: u32) -> Option<ProposalSnapshot>;

        /// Get active proposals.
        fn get_active_proposals() -> Vec<ProposalSnapshot>;

        /// Get votes for a proposal.
        fn get_proposal_votes(proposal_id: u32) -> Vec<VoteRecord>;

        /// Get vote by account for a proposal.
        fn get_vote(proposal_id: u32, voter: Vec<u8>) -> Option<VoteRecord>;

        /// Check if account has voted on proposal.
        fn has_voted(proposal_id: u32, voter: Vec<u8>) -> bool;
    }
}
