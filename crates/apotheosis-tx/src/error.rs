//! Error types for Apotheosis Transaction

use thiserror::Error;

pub type ApotheosisResult<T> = Result<T, ApotheosisError>;

#[derive(Debug, Error)]
pub enum ApotheosisError {
    #[error("No destination specified for apotheosis")]
    NoDestination,

    #[error("No source chains specified")]
    NoSourceChains,

    #[error("Invalid chain ID: {0}")]
    InvalidChainId(u64),

    #[error("Asset not found: {asset} on chain {chain_id}")]
    AssetNotFound { chain_id: u64, asset: String },

    #[error("Insufficient balance: need {required}, have {available}")]
    InsufficientBalance { required: String, available: String },

    #[error("Bridge unavailable between chains {from} and {to}")]
    BridgeUnavailable { from: u64, to: u64 },

    #[error("Route not found from chain {from} to chain {to}")]
    RouteNotFound { from: u64, to: u64 },

    #[error("Gas estimation failed: {0}")]
    GasEstimationFailed(String),

    #[error("Transaction timeout on chain {chain_id}")]
    TransactionTimeout { chain_id: u64 },

    #[error("Atomic guarantee violated - rollback initiated")]
    AtomicViolation,

    #[error("Chain {chain_id} is not supported")]
    UnsupportedChain { chain_id: u64 },

    #[error("Maximum chains exceeded: {count} > {max}")]
    MaxChainsExceeded { count: usize, max: usize },

    #[error("Maximum assets exceeded on chain {chain_id}: {count} > {max}")]
    MaxAssetsExceeded {
        chain_id: u64,
        count: usize,
        max: usize,
    },

    #[error("Signature verification failed")]
    SignatureVerificationFailed,

    #[error("Authorization required for apotheosis")]
    AuthorizationRequired,

    #[error("Execution failed on chain {chain_id}: {reason}")]
    ExecutionFailed { chain_id: u64, reason: String },

    #[error("Partial completion - {completed}/{total} chains migrated")]
    PartialCompletion { completed: usize, total: usize },

    #[error("Internal error: {0}")]
    Internal(String),
}
