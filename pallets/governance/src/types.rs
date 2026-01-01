//! Types for the governance pallet.

use frame_support::pallet_prelude::*;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_runtime::{Percent, RuntimeDebug};
use sp_std::prelude::*;

/// Conviction multiplier for voting power.
#[derive(
    Clone, Copy, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen, RuntimeDebug, Default,
)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum Conviction {
    /// 0.1x voting power, no lock.
    #[default]
    None,
    /// 1x voting power, 1 period lock.
    Locked1x,
    /// 2x voting power, 2 period lock.
    Locked2x,
    /// 3x voting power, 4 period lock.
    Locked3x,
    /// 4x voting power, 8 period lock.
    Locked4x,
    /// 5x voting power, 16 period lock.
    Locked5x,
    /// 6x voting power, 32 period lock.
    Locked6x,
}

impl Conviction {
    /// Get the vote multiplier for this conviction level.
    /// Returns (multiplier_num, multiplier_denom) for proper integer math.
    pub fn multiplier(self) -> (u32, u32) {
        match self {
            Conviction::None => (1, 10), // 0.1x
            Conviction::Locked1x => (1, 1),
            Conviction::Locked2x => (2, 1),
            Conviction::Locked3x => (3, 1),
            Conviction::Locked4x => (4, 1),
            Conviction::Locked5x => (5, 1),
            Conviction::Locked6x => (6, 1),
        }
    }

    /// Get the number of lock periods for this conviction.
    pub fn lock_periods(self) -> u32 {
        match self {
            Conviction::None => 0,
            Conviction::Locked1x => 1,
            Conviction::Locked2x => 2,
            Conviction::Locked3x => 4,
            Conviction::Locked4x => 8,
            Conviction::Locked5x => 16,
            Conviction::Locked6x => 32,
        }
    }
}

/// Direction of a vote.
#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum VoteDirection {
    /// Vote in favor.
    Aye,
    /// Vote against.
    Nay,
    /// Abstain from voting (counts toward quorum).
    Abstain,
}

/// Status of a proposal.
#[derive(
    Clone, Copy, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen, RuntimeDebug, Default,
)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub enum ProposalStatus {
    /// Proposal is in voting period.
    #[default]
    Voting,
    /// Proposal was approved.
    Approved,
    /// Proposal was rejected.
    Rejected,
    /// Proposal was enacted.
    Enacted,
    /// Proposal was cancelled.
    Cancelled,
}

/// A governance proposal.
#[derive(Clone, PartialEq, Eq, Encode, Decode, TypeInfo, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct Proposal<AccountId, Balance, BlockNumber, Call> {
    /// Unique proposal ID.
    pub id: u32,
    /// Account that submitted the proposal.
    pub proposer: AccountId,
    /// The call to execute if approved.
    pub call: Call,
    /// Short title.
    pub title: BoundedVec<u8, ConstU32<256>>,
    /// Detailed description.
    pub description: BoundedVec<u8, ConstU32<4096>>,
    /// Deposit amount.
    pub deposit: Balance,
    /// Current status.
    pub status: ProposalStatus,
    /// Block when submitted.
    pub submitted_at: BlockNumber,
    /// Block when voting ends.
    pub voting_end: BlockNumber,
    /// Block when enacted (if applicable).
    pub enacted_at: Option<BlockNumber>,
}

/// Tally of votes for a proposal.
#[derive(Clone, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen, RuntimeDebug, Default)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct ProposalTally<Balance: Default> {
    /// Total voting power for Aye.
    pub ayes: Balance,
    /// Total voting power for Nay.
    pub nays: Balance,
    /// Total voting power for Abstain.
    pub abstains: Balance,
    /// Number of Aye voters.
    pub aye_voters: u32,
    /// Number of Nay voters.
    pub nay_voters: u32,
    /// Total token turnout (raw balances).
    pub turnout: Balance,
}

/// An individual vote.
#[derive(Clone, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct Vote<Balance> {
    /// Direction of vote.
    pub direction: VoteDirection,
    /// Balance used for voting.
    pub balance: Balance,
    /// Conviction multiplier.
    pub conviction: Conviction,
    /// Calculated voting power.
    pub voting_power: Balance,
}

/// Delegation of voting power.
#[derive(Clone, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct Delegation<AccountId, Balance> {
    /// Account to delegate to.
    pub target: AccountId,
    /// Conviction for delegated votes.
    pub conviction: Conviction,
    /// Balance being delegated.
    pub balance: Balance,
}

/// Token lock for conviction voting.
#[derive(Clone, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct VoteLock<Balance, BlockNumber> {
    /// Locked amount.
    pub amount: Balance,
    /// Block when tokens unlock.
    pub unlock_at: BlockNumber,
    /// Conviction level of the lock.
    pub conviction: Conviction,
}

/// Governance configuration parameters.
#[derive(Clone, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen, RuntimeDebug, Default)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct GovernanceParams<Balance: Default, BlockNumber: Default> {
    /// Minimum percentage of issuance that must vote.
    pub quorum: Percent,
    /// Percentage of votes required for approval.
    pub approval_threshold: Percent,
    /// Duration of voting period in blocks.
    pub voting_period: BlockNumber,
    /// Delay between approval and enactment.
    pub enactment_period: BlockNumber,
    /// Deposit required to submit proposal.
    pub proposal_deposit: Balance,
}

/// Summary of a proposal for API responses.
#[derive(Clone, PartialEq, Eq, Encode, Decode, TypeInfo, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct ProposalSummary<AccountId, Balance, BlockNumber> {
    /// Proposal ID.
    pub id: u32,
    /// Proposer account.
    pub proposer: AccountId,
    /// Current status.
    pub status: ProposalStatus,
    /// When voting ends.
    pub voting_end: BlockNumber,
    /// Total Aye votes.
    pub ayes: Balance,
    /// Total Nay votes.
    pub nays: Balance,
    /// Total turnout.
    pub turnout: Balance,
}

/// Snapshot of governance state for offchain consumers.
#[derive(Clone, PartialEq, Eq, Encode, Decode, TypeInfo, RuntimeDebug)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct GovernanceSnapshot<AccountId, Balance: Default, BlockNumber: Default> {
    /// Total proposals submitted.
    pub proposal_count: u32,
    /// Currently active proposals in voting.
    pub active_proposals: Vec<ProposalSummary<AccountId, Balance, BlockNumber>>,
    /// Proposals pending enactment.
    pub pending_enactments: Vec<u32>,
    /// Current governance configuration.
    pub config: GovernanceParams<Balance, BlockNumber>,
}

// ============================================================================
// AI Governance Types
// ============================================================================

/// AI Proposal inert object (no direct execution capability)
#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq, Eq)]
#[scale_info(skip_type_params(T))]
pub struct AIProposal<T: frame_system::Config> {
    /// Unique proposal ID
    pub id: u64,
    /// AI agent proposer
    pub proposer: T::AccountId,
    /// Proposal type
    pub proposal_type: AIProposalType,
    /// Inert payload (description + metadata, no executable code)
    pub payload: BoundedVec<u8, ConstU32<10240>>, // 10KB max
    /// Expected impact assessment
    pub impact_assessment: ImpactAssessment,
    /// Simulation requirements
    pub simulation_requirements: SimulationRequirements,
    /// Proposed at block
    pub proposed_at: T::BlockNumber,
    /// Status
    pub status: AIProposalStatus,
}

/// Types of AI proposals
#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq, Eq)]
pub enum AIProposalType {
    /// Runtime parameter evolution
    RuntimeEvolution,
    /// New AI agent registration
    AgentRegistration,
    /// Protocol optimization
    ProtocolOptimization,
    /// Economic parameter adjustment
    EconomicAdjustment,
    /// Security enhancement
    SecurityEnhancement,
    /// Custom proposal type
    Custom(u32),
}

/// Impact assessment for AI proposals
#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq, Eq)]
pub struct ImpactAssessment {
    /// Risk level (0-100)
    pub risk_level: u8,
    /// Expected improvement (0-100)
    pub expected_improvement: u8,
    /// Affected subsystems
    pub affected_subsystems: Vec<Subsystem>,
    /// Rollback difficulty (0-100)
    pub rollback_difficulty: u8,
}

/// Subsystems that can be affected by AI proposals
#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq, Eq)]
pub enum Subsystem {
    Consensus,
    Execution,
    Economic,
    Governance,
    Security,
    Storage,
}

/// Simulation requirements for AI proposals
#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq, Eq)]
pub struct SimulationRequirements {
    /// Required simulation duration (blocks)
    pub simulation_blocks: u32,
    /// Gas limit for simulation
    pub gas_limit: u64,
    /// Required success rate (0-100)
    pub success_rate_threshold: u8,
    /// Deterministic test requirements
    pub deterministic_tests: bool,
}

/// AI proposal status
#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq, Eq, Default)]
pub enum AIProposalStatus {
    #[default]
    Proposed,
    UnderReview,
    SimulationPassed,
    SimulationFailed,
    Approved,
    Rejected,
    Executed,
    RolledBack,
}

/// Simulation result
#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq, Eq)]
pub struct SimulationResult {
    /// Success status
    pub success: bool,
    /// Gas used
    pub gas_used: u64,
    /// Execution time (blocks)
    pub execution_time: u32,
    /// State changes preview
    pub state_changes: Vec<StateChange>,
    /// Warnings/issues found
    pub warnings: Vec<BoundedVec<u8, ConstU32<256>>>,
}

/// State change preview
#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq, Eq)]
pub struct StateChange {
    /// Storage key affected
    pub key: Vec<u8>,
    /// Previous value
    pub old_value: Option<Vec<u8>>,
    /// New value
    pub new_value: Vec<u8>,
}

/// Authorization requirements for AI proposals
#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq, Eq)]
#[scale_info(skip_type_params(T))]
pub struct AuthorizationRequirements<T: frame_system::Config> {
    /// Required multisig approvals
    pub multisig_threshold: u32,
    /// Time lock duration (blocks)
    pub time_lock_blocks: T::BlockNumber,
    /// Required reviewer approvals
    pub reviewer_approvals: u32,
}

/// Sandboxed execution context
#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq, Eq)]
pub struct SandboxedExecution {
    /// Gas ceiling
    pub gas_ceiling: u64,
    /// Block time limit
    pub block_limit: u32,
    /// State rollback checkpoint
    pub rollback_checkpoint: Vec<u8>,
    /// Execution status
    pub status: ExecutionStatus,
}

/// Execution status
#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq, Eq, Default)]
pub enum ExecutionStatus {
    #[default]
    Pending,
    Executing,
    Completed,
    Failed,
    RolledBack,
}

/// Kill switch levels (graduated emergency controls)
#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum KillSwitchLevel {
    /// Normal operation
    Normal = 0,
    /// Pause specific AI subsystems
    SubsystemPause = 1,
    /// Freeze economic activity
    EconomicFreeze = 2,
    /// Prevent any upgrades
    UpgradeFreeze = 3,
    /// Complete system halt
    EmergencyHalt = 4,
}

/// Kill switch activation record
#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq, Eq)]
#[scale_info(skip_type_params(T))]
pub struct KillSwitchActivation<T: frame_system::Config> {
    /// Activation level
    pub level: KillSwitchLevel,
    /// Activated by
    pub activator: T::AccountId,
    /// Reason
    pub reason: BoundedVec<u8, ConstU32<512>>,
    /// Activated at
    pub activated_at: T::BlockNumber,
    /// Auto-deactivation block (if set)
    pub auto_deactivate_at: Option<T::BlockNumber>,
}

/// AI Governance configuration
#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, PartialEq, Eq)]
pub struct AIGovernanceConfig {
    /// Maximum AI proposal payload size
    pub max_proposal_payload: u32,
    /// Default simulation duration
    pub default_simulation_blocks: u32,
    /// Default gas limit for simulations
    pub default_simulation_gas: u64,
    /// Minimum reviewer approvals
    pub min_reviewer_approvals: u32,
    /// Default time lock
    pub default_time_lock: u32,
    /// Emergency quorum threshold
    pub emergency_quorum: Percent,
    /// Kill switch activation threshold
    pub kill_switch_threshold: Percent,
}

impl Default for AIGovernanceConfig {
    fn default() -> Self {
        Self {
            max_proposal_payload: 1024 * 10, // 10KB
            default_simulation_blocks: 100,
            default_simulation_gas: 1_000_000,
            min_reviewer_approvals: 3,
            default_time_lock: 100, // blocks
            emergency_quorum: Percent::from_percent(75),
            kill_switch_threshold: Percent::from_percent(80),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conviction_multipliers_work() {
        // Test conviction multipliers
        assert_eq!(Conviction::None.multiplier(), (1, 10)); // 0.1x
        assert_eq!(Conviction::Locked1x.multiplier(), (1, 1)); // 1x
        assert_eq!(Conviction::Locked2x.multiplier(), (2, 1)); // 2x
        assert_eq!(Conviction::Locked3x.multiplier(), (3, 1)); // 3x
        assert_eq!(Conviction::Locked4x.multiplier(), (4, 1)); // 4x
        assert_eq!(Conviction::Locked5x.multiplier(), (5, 1)); // 5x
        assert_eq!(Conviction::Locked6x.multiplier(), (6, 1)); // 6x
    }

    #[test]
    fn conviction_lock_periods_work() {
        assert_eq!(Conviction::None.lock_periods(), 0);
        assert_eq!(Conviction::Locked1x.lock_periods(), 1);
        assert_eq!(Conviction::Locked2x.lock_periods(), 2);
        assert_eq!(Conviction::Locked3x.lock_periods(), 4);
        assert_eq!(Conviction::Locked4x.lock_periods(), 8);
        assert_eq!(Conviction::Locked5x.lock_periods(), 16);
        assert_eq!(Conviction::Locked6x.lock_periods(), 32);
    }
}
