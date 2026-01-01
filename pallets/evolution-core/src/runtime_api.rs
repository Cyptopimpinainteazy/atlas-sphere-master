//! Runtime API definitions for Evolution Core pallet
//!
//! These APIs allow external callers (RPC, off-chain workers) to query
//! evolution state without submitting transactions.

use codec::{Codec, Decode, Encode};
use scale_info::TypeInfo;
use sp_std::vec::Vec;

/// Evolvable parameters (serializable version for RPC)
#[derive(Clone, Encode, Decode, Debug, PartialEq, Eq, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct EvolvableParamsResponse {
    /// Base gas price multiplier (100 = 1x)
    pub gas_multiplier: u32,
    /// EVM execution weight percentage (0-100)
    pub evm_weight_pct: u8,
    /// SVM execution weight percentage (0-100)
    pub svm_weight_pct: u8,
    /// JIT compilation threshold
    pub jit_threshold: u32,
    /// Max parallel executions
    pub max_parallel: u32,
    /// MEV smoothing factor
    pub mev_smooth_factor: u32,
}

/// Block metrics (serializable version for RPC)
#[derive(Clone, Encode, Decode, Debug, PartialEq, Eq, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct BlockMetricsResponse {
    /// Block gas used
    pub gas_used: u128,
    /// EVM call count
    pub evm_calls: u32,
    /// SVM call count
    pub svm_calls: u32,
    /// Cross-VM call count
    pub cross_vm_calls: u32,
    /// Mempool depth
    pub mempool_depth: u32,
    /// MEV pressure (0-100)
    pub mev_pressure: u8,
    /// X3 hotpath hits
    pub x3_hotpath_hits: u32,
    /// Swap volume
    pub swap_volume: u128,
    /// Flashloan volume
    pub flashloan_volume: u128,
}

/// Mutation proposal status
#[derive(Clone, Encode, Decode, Debug, PartialEq, Eq, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct ProposalResponse<AccountId, BlockNumber> {
    /// Proposal ID
    pub id: u32,
    /// Proposer account
    pub proposer: AccountId,
    /// Proposal description
    pub reason: Vec<u8>,
    /// Block when proposed
    pub proposed_at: BlockNumber,
    /// Current approval count
    pub approvals: u32,
    /// Status: 0=Pending, 1=Approved, 2=Rejected, 3=Applied
    pub status: u8,
}

/// Evolution status summary
#[derive(Clone, Encode, Decode, Debug, PartialEq, Eq, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct EvolutionStatusResponse {
    /// Whether evolution is enabled
    pub evolution_enabled: bool,
    /// Whether auto-evolution is enabled
    pub auto_evolution_enabled: bool,
    /// Number of pending proposals
    pub pending_proposals: u32,
    /// Number of registered AI agents
    pub ai_agents_count: u32,
    /// Total mutations applied
    pub total_mutations_applied: u64,
}

sp_api::decl_runtime_apis! {
    /// Runtime API for querying evolution core state
    pub trait EvolutionCoreApi<AccountId, BlockNumber>
    where
        AccountId: Codec,
        BlockNumber: Codec,
    {
        /// Get current evolvable parameters
        fn get_params() -> EvolvableParamsResponse;

        /// Get evolution status summary
        fn get_status() -> EvolutionStatusResponse;

        /// Get recent block metrics
        fn get_recent_metrics(depth: u32) -> Vec<(BlockNumber, BlockMetricsResponse)>;

        /// Get pending proposals
        fn get_pending_proposals() -> Vec<ProposalResponse<AccountId, BlockNumber>>;

        /// Check if an account is an AI agent approver
        fn is_ai_agent(account: AccountId) -> bool;

        /// Check if evolution is enabled
        fn is_evolution_enabled() -> bool;
    }
}
