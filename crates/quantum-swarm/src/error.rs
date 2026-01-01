//! Error types for the Quantum Swarm Executor

use std::fmt;
use thiserror::Error;

/// Result type for quantum swarm operations
pub type SwarmResult<T> = Result<T, SwarmError>;

/// Errors that can occur in the quantum swarm
#[derive(Error, Debug)]
pub enum SwarmError {
    /// Quantum circuit error
    #[error("Quantum circuit error: {0}")]
    QuantumCircuit(String),

    /// Quantum backend error
    #[error("Quantum backend error: {0}")]
    QuantumBackend(String),

    /// Classical computation error
    #[error("Classical computation error: {0}")]
    Classical(String),

    /// Strategy error
    #[error("Strategy error: {0}")]
    Strategy(String),

    /// Evolution error
    #[error("Evolution error: {0}")]
    Evolution(String),

    /// Arena error
    #[error("Arena error: {0}")]
    Arena(String),

    /// Arena is full
    #[error("Arena is full (max: {max})")]
    ArenaFull { max: usize },

    /// Not enough combatants for tournament
    #[error("Not enough combatants (required: {required}, available: {available})")]
    NotEnoughCombatants { required: usize, available: usize },

    /// Combatant not found
    #[error("Combatant not found: {id}")]
    CombatantNotFound { id: crate::types::StrategyId },

    /// Compute fabric error
    #[error("Compute fabric error: {0}")]
    ComputeFabric(String),

    /// X3 emission error
    #[error("X3 emission error: {0}")]
    X3Emission(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Timeout error
    #[error("Operation timed out: {0}")]
    Timeout(String),

    /// Resource exhaustion
    #[error("Resource exhausted: {0}")]
    ResourceExhausted(String),

    /// Invalid state
    #[error("Invalid state: {0}")]
    InvalidState(String),

    /// External QPU error
    #[error("External QPU error: {0}")]
    ExternalQpu(String),

    /// Network error
    #[error("Network error: {0}")]
    Network(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

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
        SwarmError::Serialization(err.to_string())
    }
}

impl From<tokio::sync::mpsc::error::SendError<super::types::SwarmMessage>> for SwarmError {
    fn from(err: tokio::sync::mpsc::error::SendError<super::types::SwarmMessage>) -> Self {
        SwarmError::Internal(format!("Channel send error: {}", err))
    }
}

/// Error severity for logging and alerting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    /// Recoverable warning
    Warning,
    /// Error that can be retried
    Retryable,
    /// Critical error requiring intervention
    Critical,
    /// Fatal error - system should halt
    Fatal,
}

impl SwarmError {
    /// Get the severity of this error
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            SwarmError::Timeout(_) => ErrorSeverity::Retryable,
            SwarmError::ResourceExhausted(_) => ErrorSeverity::Retryable,
            SwarmError::Network(_) => ErrorSeverity::Retryable,
            SwarmError::ExternalQpu(_) => ErrorSeverity::Retryable,
            SwarmError::Strategy(_) => ErrorSeverity::Warning,
            SwarmError::Evolution(_) => ErrorSeverity::Warning,
            SwarmError::InvalidState(_) => ErrorSeverity::Critical,
            SwarmError::Config(_) => ErrorSeverity::Critical,
            SwarmError::Internal(_) => ErrorSeverity::Fatal,
            _ => ErrorSeverity::Warning,
        }
    }

    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(self.severity(), ErrorSeverity::Retryable)
    }
}
