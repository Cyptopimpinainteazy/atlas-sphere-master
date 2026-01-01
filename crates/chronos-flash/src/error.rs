//! ChronosFlash error types

use thiserror::Error;

/// Result type for ChronosFlash operations
pub type ChronosResult<T> = Result<T, ChronosError>;

/// ChronosFlash error variants
#[derive(Debug, Error)]
pub enum ChronosError {
    #[error("Mempool scan failed: {0}")]
    MempoolScanFailed(String),

    #[error("Intent prediction failed: {0}")]
    PredictionFailed(String),

    #[error("Route computation failed: {0}")]
    RouteFailed(String),

    #[error("Pre-execution failed: {0}")]
    PreExecutionFailed(String),

    #[error("Time-warp execution failed: {0}")]
    TimeWarpFailed(String),

    #[error("Checkpoint rollback failed: {0}")]
    RollbackFailed(String),

    #[error("Chain {chain_id} not supported")]
    ChainNotSupported { chain_id: u64 },

    #[error("Latency exceeded: {actual_ms}ms > {max_ms}ms")]
    LatencyExceeded { actual_ms: u64, max_ms: u64 },

    #[error("Insufficient liquidity: need {needed}, have {available}")]
    InsufficientLiquidity { needed: u128, available: u128 },

    #[error("Slippage too high: {actual_bps}bps > {max_bps}bps")]
    SlippageExceeded { actual_bps: u32, max_bps: u32 },

    #[error("Oracle not ready: {0}")]
    OracleNotReady(String),

    #[error("Oracle already running")]
    AlreadyRunning,

    #[error("Intent expired: submitted {elapsed_ms}ms ago")]
    IntentExpired { elapsed_ms: u64 },

    #[error("Bundle expired")]
    BundleExpired,

    #[error("Quantum router failed: {0}")]
    QuantumRouterFailed(String),

    #[error("AI swarm unavailable: {0}")]
    SwarmUnavailable(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<std::io::Error> for ChronosError {
    fn from(e: std::io::Error) -> Self {
        ChronosError::Network(e.to_string())
    }
}

impl From<serde_json::Error> for ChronosError {
    fn from(e: serde_json::Error) -> Self {
        ChronosError::Serialization(e.to_string())
    }
}
