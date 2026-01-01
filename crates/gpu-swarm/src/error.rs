//! Error types for the GPU swarm

use crate::task::TaskStatus;
use thiserror::Error;
use uuid::Uuid;

/// Result type for swarm operations
pub type SwarmResult<T> = Result<T, SwarmError>;

/// Errors that can occur in the GPU swarm
#[derive(Error, Debug)]
pub enum SwarmError {
    /// Node not found in the swarm
    #[error("Node not found: {}", hex::encode(&.0[..8]))]
    NodeNotFound([u8; 32]),

    /// Task not found
    #[error("Task not found: {0}")]
    TaskNotFound(Uuid),

    /// Task execution failed
    #[error("Task execution failed: {0}")]
    TaskExecutionFailed(String),

    /// Task verification failed
    #[error("Task verification failed: {0}")]
    VerificationFailed(String),

    /// Network error
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Insufficient stake
    #[error("Insufficient stake: required {required}, have {available}")]
    InsufficientStake { required: u64, available: u64 },

    /// Node already registered
    #[error("Node already registered: {0}")]
    NodeAlreadyRegistered(String),

    /// Invalid task payload
    #[error("Invalid task payload: {0}")]
    InvalidPayload(String),

    /// Task timeout
    #[error("Task timed out after {0} seconds")]
    TaskTimeout(u64),

    /// Task expired before execution
    #[error("Task expired: {0}")]
    TaskExpired(Uuid),

    /// GPU not available
    #[error("GPU not available: {0}")]
    GpuNotAvailable(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Signature verification failed
    #[error("Signature verification failed")]
    SignatureVerificationFailed,

    /// Rate limited
    #[error("Rate limited: try again in {0} seconds")]
    RateLimited(u64),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Task queue is full
    #[error("Task queue is full")]
    QueueFull,

    /// Invalid task state transition
    #[error("Invalid task state: expected {expected:?}, found {actual:?}")]
    InvalidTaskState {
        expected: TaskStatus,
        actual: TaskStatus,
    },

    /// Not enough verifiers available
    #[error("Insufficient verifiers: required {required}, available {available}")]
    InsufficientVerifiers { required: u8, available: u8 },

    /// Unauthorized verifier
    #[error("Unauthorized verifier: {}", hex::encode(&.0[..8]))]
    UnauthorizedVerifier([u8; 32]),

    /// Invalid job result
    #[error("Invalid result: {0}")]
    InvalidResult(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<std::io::Error> for SwarmError {
    fn from(err: std::io::Error) -> Self {
        SwarmError::Internal(err.to_string())
    }
}

impl From<serde_json::Error> for SwarmError {
    fn from(err: serde_json::Error) -> Self {
        SwarmError::SerializationError(err.to_string())
    }
}

impl From<bincode::Error> for SwarmError {
    fn from(err: bincode::Error) -> Self {
        SwarmError::SerializationError(err.to_string())
    }
}
