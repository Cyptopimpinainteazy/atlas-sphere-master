//! Runtime API for Agent Accounts pallet.
//!
//! Provides offchain access to agent state information.

use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_std::prelude::*;

/// Agent state snapshot for offchain subscribers.
#[derive(Clone, Encode, Decode, TypeInfo, Debug, Default)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct AgentStateSnapshot {
    /// Agent ID.
    pub agent_id: u32,
    /// Agent name.
    pub name: Vec<u8>,
    /// Controller account (as bytes).
    pub controller: Vec<u8>,
    /// Operator account (as bytes).
    pub operator: Vec<u8>,
    /// Status (0=Active, 1=Suspended, 2=Terminated).
    pub status: u8,
    /// Reputation score.
    pub reputation: u32,
    /// Deposit amount.
    pub deposit: u128,
    /// Block when registered.
    pub registered_at: u64,
    /// Last active block.
    pub last_active: u64,
    /// Gas used in current epoch.
    pub gas_used_epoch: u128,
    /// Compute used in current epoch.
    pub compute_used_epoch: u128,
    /// Permissions bitmask.
    pub permissions: u32,
}

/// Agent list response.
#[derive(Clone, Encode, Decode, TypeInfo, Debug, Default)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct AgentListResponse {
    /// List of agent IDs.
    pub agents: Vec<u32>,
    /// Total count.
    pub total: u32,
}

sp_api::decl_runtime_apis! {
    /// Agent Accounts Runtime API.
    pub trait AgentAccountsApi {
        /// Get agent state by ID.
        fn get_agent_state(agent_id: u32) -> Option<AgentStateSnapshot>;

        /// Get all agents for a controller.
        fn get_agents_by_controller(controller: Vec<u8>) -> AgentListResponse;

        /// Get agent by operator.
        fn get_agent_by_operator(operator: Vec<u8>) -> Option<u32>;

        /// Check if agent can perform action.
        fn can_agent_perform(agent_id: u32, permission: u8) -> bool;

        /// Get total agent count.
        fn get_total_agents() -> u32;
    }
}
