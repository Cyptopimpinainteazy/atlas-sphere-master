//! Audit Governance Runtime API
//!
//! Exposes audit governance queries to the runtime and RPC layer.

use sp_api::decl_runtime_apis;
use sp_core::H256;

decl_runtime_apis! {
	/// Audit Governance API for runtime queries
	pub trait AuditGovernanceApi {
		/// Check if an agent is locked from execution
		fn is_agent_locked(agent: Vec<u8>) -> bool;

		/// Check if system is in emergency pause
		fn is_emergency_paused() -> bool;

		/// Get decision for a commit's audit
		fn get_audit_decision(commit_hash: H256) -> Option<Vec<u8>>;

		/// Get full audit artifact (JSON serialized)
		fn get_audit_artifact(audit_id: H256) -> Option<Vec<u8>>;
	}
}
