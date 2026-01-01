//! Runtime API definitions for X3 Verifier pallet
//!
//! These APIs allow external callers (RPC, off-chain workers) to query
//! verifier state without submitting transactions.

use codec::{Codec, Decode, Encode};
use scale_info::TypeInfo;
use sp_core::H256;
use sp_std::vec::Vec;

/// Job ID type for runtime API responses
pub type JobId = H256;

/// Executor information (serializable version for RPC)
#[derive(Clone, Encode, Decode, Debug, PartialEq, Eq, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct ExecutorResponse<AccountId, Balance> {
    /// Executor account
    pub account: AccountId,
    /// Stake amount
    pub stake: Balance,
    /// Total jobs completed
    pub jobs_completed: u64,
    /// Total jobs failed
    pub jobs_failed: u64,
    /// Reputation score (0-100)
    pub reputation: u8,
    /// Whether executor is active
    pub active: bool,
}

/// Job information (serializable version for RPC)
#[derive(Clone, Encode, Decode, Debug, PartialEq, Eq, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct JobResponse<AccountId, Balance, BlockNumber> {
    /// Job ID
    pub job_id: JobId,
    /// Submitter account
    pub submitter: AccountId,
    /// Bytecode hash
    pub bytecode_hash: H256,
    /// Input data hash
    pub input_hash: H256,
    /// Gas limit
    pub gas_limit: u128,
    /// Reward amount
    pub reward: Balance,
    /// Assigned executor (if any)
    pub executor: Option<AccountId>,
    /// Job status: 0=Pending, 1=Submitted, 2=Verified, 3=Applied, 4=Failed, 5=Disputed
    pub status: u8,
    /// Block when submitted
    pub submitted_at: BlockNumber,
    /// Receipt hash (if submitted)
    pub receipt_hash: Option<H256>,
}

/// Receipt information (serializable version for RPC)
#[derive(Clone, Encode, Decode, Debug, PartialEq, Eq, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct ReceiptResponse<AccountId> {
    /// Job ID this receipt is for
    pub job_id: JobId,
    /// Executor who submitted the receipt
    pub executor: AccountId,
    /// Hash of the input
    pub input_hash: H256,
    /// Hash of the output
    pub output_hash: H256,
    /// State root before execution
    pub state_root_before: H256,
    /// State root after execution
    pub state_root_after: H256,
    /// Gas used
    pub gas_used: u128,
    /// Execution timestamp
    pub timestamp: u64,
}

/// Verifier status summary
#[derive(Clone, Encode, Decode, Debug, PartialEq, Eq, TypeInfo)]
#[cfg_attr(feature = "std", derive(serde::Serialize, serde::Deserialize))]
pub struct VerifierStatusResponse {
    /// Whether verification is enabled
    pub verification_enabled: bool,
    /// Total active executors
    pub active_executors: u32,
    /// Total pending jobs
    pub pending_jobs: u32,
    /// Total jobs submitted
    pub total_jobs_submitted: u64,
    /// Total jobs verified
    pub total_jobs_verified: u64,
}

sp_api::decl_runtime_apis! {
    /// Runtime API for querying X3 verifier state
    pub trait X3VerifierApi<AccountId, Balance, BlockNumber>
    where
        AccountId: Codec,
        Balance: Codec,
        BlockNumber: Codec,
    {
        /// Get verifier status summary
        fn get_status() -> VerifierStatusResponse;

        /// Get executor information
        fn get_executor(account: AccountId) -> Option<ExecutorResponse<AccountId, Balance>>;

        /// Get all active executors
        fn get_active_executors() -> Vec<ExecutorResponse<AccountId, Balance>>;

        /// Get job information
        fn get_job(job_id: JobId) -> Option<JobResponse<AccountId, Balance, BlockNumber>>;

        /// Get pending jobs
        fn get_pending_jobs() -> Vec<JobResponse<AccountId, Balance, BlockNumber>>;

        /// Get receipt for a job
        fn get_receipt(job_id: JobId) -> Option<ReceiptResponse<AccountId>>;

        /// Check if verification is enabled
        fn is_verification_enabled() -> bool;

        /// Check if an account is a registered executor
        fn is_executor(account: AccountId) -> bool;
    }
}
