//! Types for the Agent Accounts pallet.

use frame_support::pallet_prelude::*;
use parity_scale_codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sp_std::prelude::*;

/// Unique identifier for an agent.
pub type AgentId = u32;

/// Agent status.
#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, Default)]
pub enum AgentStatus {
    /// Agent is active and can operate.
    #[default]
    Active,
    /// Agent is temporarily suspended.
    Suspended,
    /// Agent is permanently terminated.
    Terminated,
}

/// Agent record.
#[derive(Clone, Encode, Decode, TypeInfo, MaxEncodedLen, Debug)]
pub struct Agent<AccountId, Balance, BlockNumber> {
    /// Unique agent ID.
    pub id: AgentId,
    /// Controller account (manages the agent).
    pub controller: AccountId,
    /// Operator account (executes on behalf of agent).
    pub operator: AccountId,
    /// Agent name.
    pub name: BoundedVec<u8, ConstU32<64>>,
    /// Agent metadata (JSON).
    pub metadata: BoundedVec<u8, ConstU32<1024>>,
    /// Current status.
    pub status: AgentStatus,
    /// Reputation score (0-200, 100 = neutral).
    pub reputation: u32,
    /// Deposit amount.
    pub deposit: Balance,
    /// Block when registered.
    pub registered_at: BlockNumber,
    /// Last active block.
    pub last_active: BlockNumber,
}

/// Agent quota limits.
#[derive(Clone, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen, Debug, Default)]
pub struct AgentQuota<BlockNumber: Default> {
    /// Maximum gas per block.
    pub gas_per_block: u128,
    /// Maximum compute units per block.
    pub compute_per_block: u128,
    /// Maximum gas per epoch.
    pub gas_per_epoch: u128,
    /// Maximum compute units per epoch.
    pub compute_per_epoch: u128,
    /// Block when epoch started.
    pub epoch_start: BlockNumber,
}

/// Agent permissions.
#[derive(Clone, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen, Debug)]
pub struct AgentPermissions {
    /// Can deploy contracts.
    pub can_deploy: bool,
    /// Can stake tokens.
    pub can_stake: bool,
    /// Can vote in governance.
    pub can_vote: bool,
    /// Can execute trades.
    pub can_trade: bool,
    /// Can transfer tokens.
    pub can_transfer: bool,
    /// Can call arbitrary contracts.
    pub can_call_contracts: bool,
}

impl Default for AgentPermissions {
    fn default() -> Self {
        Self {
            can_deploy: false,
            can_stake: false,
            can_vote: false,
            can_trade: true,          // Default allow trading
            can_transfer: true,       // Default allow transfers
            can_call_contracts: true, // Default allow contract calls
        }
    }
}

/// Permission type for checking.
#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen, Debug)]
pub enum PermissionType {
    Deploy,
    Stake,
    Vote,
    Trade,
    Transfer,
    CallContracts,
}

/// Agent activity tracking.
#[derive(Clone, Default, Encode, Decode, TypeInfo, MaxEncodedLen, Debug)]
pub struct AgentActivity {
    /// Gas used this block.
    pub gas_used_block: u128,
    /// Compute used this block.
    pub compute_used_block: u128,
    /// Gas used this epoch.
    pub gas_used_epoch: u128,
    /// Compute used this epoch.
    pub compute_used_epoch: u128,
    /// Total actions taken.
    pub total_actions: u64,
}

/// Action types for event streaming.
#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen, Debug)]
pub enum ActionType {
    /// Agent executed a trade.
    Trade,
    /// Agent deployed a contract.
    Deploy,
    /// Agent voted on a proposal.
    Vote,
    /// Agent staked tokens.
    Stake,
    /// Agent transferred tokens.
    Transfer,
    /// Agent called a contract.
    ContractCall,
    /// Agent created a memory entry.
    Memory,
    /// Custom action.
    Custom,
}

/// Combined agent state for runtime API.
#[derive(Clone, Encode, Decode, TypeInfo, Debug)]
pub struct AgentState<AccountId, Balance, BlockNumber: Default> {
    /// Agent details.
    pub agent: Agent<AccountId, Balance, BlockNumber>,
    /// Quota limits.
    pub quota: AgentQuota<BlockNumber>,
    /// Permissions.
    pub permissions: AgentPermissions,
    /// Current activity.
    pub activity: AgentActivity,
}

/// Agent summary for listing.
#[derive(Clone, Encode, Decode, TypeInfo, Debug)]
pub struct AgentSummary<AccountId, BlockNumber> {
    pub id: AgentId,
    pub name: BoundedVec<u8, ConstU32<64>>,
    pub controller: AccountId,
    pub status: AgentStatus,
    pub reputation: u32,
    pub last_active: BlockNumber,
}

/// Statistics snapshot.
#[derive(Clone, Encode, Decode, TypeInfo, Debug)]
pub struct AgentStats {
    pub total_agents: u32,
    pub active_agents: u32,
    pub total_gas_consumed: u128,
    pub total_compute_consumed: u128,
    pub current_epoch: u64,
}
