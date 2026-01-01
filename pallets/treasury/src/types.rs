//! Types for the Treasury pallet.

use frame_support::pallet_prelude::*;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::Percent;
use sp_std::vec::Vec;

/// Spending track for proposals.
#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, Default)]
pub enum SpendingTrack {
    /// Small spending - fast approval, low threshold (~33%)
    #[default]
    Small,
    /// Medium spending - standard approval (~50%)
    Medium,
    /// Large spending - high threshold (~67%)
    Large,
    /// Critical spending - requires all signers (100%)
    Critical,
}

/// Status of a spending proposal.
#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, Default)]
pub enum ProposalStatus {
    /// Awaiting approvals.
    #[default]
    Pending,
    /// Executed successfully.
    Executed,
    /// Rejected by governance.
    Rejected,
}

/// A spending proposal.
#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug)]
#[scale_info(skip_type_params(T))]
pub struct SpendingProposal<AccountId, Balance, BlockNumber> {
    /// Unique proposal ID.
    pub id: u32,
    /// Account that submitted the proposal.
    pub proposer: AccountId,
    /// Account that will receive funds.
    pub beneficiary: AccountId,
    /// Amount requested.
    pub amount: Balance,
    /// Bond reserved from proposer.
    pub bond: Balance,
    /// Description of the spending.
    pub description: BoundedVec<u8, ConstU32<1024>>,
    /// Spending track.
    pub track: SpendingTrack,
    /// Current status.
    pub status: ProposalStatus,
    /// Block when submitted.
    pub submitted_at: BlockNumber,
    /// Block when executed (if any).
    pub executed_at: Option<BlockNumber>,
}

/// A recurring payment schedule.
#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug)]
#[scale_info(skip_type_params(T))]
pub struct RecurringPayment<AccountId, Balance, BlockNumber> {
    /// Unique payment ID.
    pub id: u32,
    /// Beneficiary account.
    pub beneficiary: AccountId,
    /// Amount per payment.
    pub amount: Balance,
    /// Interval between payments in blocks.
    pub interval: BlockNumber,
    /// Next scheduled payment block.
    pub next_payment: BlockNumber,
    /// Number of payments made.
    pub payments_made: u32,
    /// Maximum total payments (None = unlimited).
    pub total_payments: Option<u32>,
    /// Description.
    pub description: BoundedVec<u8, ConstU32<256>>,
    /// Whether payment is active.
    pub active: bool,
}

/// Risk level for yield strategies.
#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, Default)]
pub enum RiskLevel {
    /// Conservative - low risk, low return.
    #[default]
    Low,
    /// Moderate - balanced risk/return.
    Medium,
    /// Aggressive - high risk, high potential return.
    High,
}

/// A yield strategy delegated to an AI agent.
#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug)]
#[scale_info(skip_type_params(T))]
pub struct YieldStrategy<AccountId, Balance> {
    /// Unique strategy ID.
    pub id: u32,
    /// Agent account managing this strategy.
    pub agent: AccountId,
    /// Maximum allocation allowed.
    pub max_allocation: Balance,
    /// Current amount allocated.
    pub current_allocation: Balance,
    /// Minimum expected return percentage.
    pub min_expected_return: Percent,
    /// Total profit earned.
    pub total_profit: Balance,
    /// Total loss incurred.
    pub total_loss: Balance,
    /// Number of executions.
    pub executions: u32,
    /// Risk level.
    pub risk_level: RiskLevel,
    /// Description.
    pub description: BoundedVec<u8, ConstU32<256>>,
    /// Whether strategy is active.
    pub active: bool,
}

/// Emergency pause information.
#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug)]
pub struct EmergencyPause<AccountId, BlockNumber> {
    /// Account that paused.
    pub paused_by: AccountId,
    /// Block when paused.
    pub paused_at: BlockNumber,
    /// Reason for pause.
    pub reason: BoundedVec<u8, ConstU32<256>>,
}

/// Treasury statistics.
#[derive(Clone, Default, Encode, Decode, TypeInfo, MaxEncodedLen, Debug)]
pub struct TreasuryStats<Balance: Default> {
    /// Total deposited to treasury.
    pub total_deposited: Balance,
    /// Total spent from treasury.
    pub total_spent: Balance,
    /// Total yield earned.
    pub total_yield_earned: Balance,
    /// Number of proposals executed.
    pub proposals_executed: u32,
}

/// Summary of a spending proposal for RPC.
#[derive(Clone, Encode, Decode, TypeInfo, Debug)]
pub struct ProposalSummary<AccountId, Balance, BlockNumber> {
    pub id: u32,
    pub proposer: AccountId,
    pub beneficiary: AccountId,
    pub amount: Balance,
    pub track: SpendingTrack,
    pub status: ProposalStatus,
    pub approvals: u32,
    pub threshold: u32,
    pub submitted_at: BlockNumber,
}

/// Treasury snapshot for runtime API.
#[derive(Clone, Encode, Decode, TypeInfo, Debug)]
pub struct TreasurySnapshot<AccountId, Balance: Default, BlockNumber> {
    /// Current treasury balance.
    pub balance: Balance,
    /// Whether treasury is paused.
    pub is_paused: bool,
    /// Number of signers.
    pub signer_count: u32,
    /// Active proposals.
    pub pending_proposals: Vec<ProposalSummary<AccountId, Balance, BlockNumber>>,
    /// Active recurring payments count.
    pub active_recurring_payments: u32,
    /// Active yield strategies count.
    pub active_yield_strategies: u32,
    /// Treasury statistics.
    pub stats: TreasuryStats<Balance>,
}
