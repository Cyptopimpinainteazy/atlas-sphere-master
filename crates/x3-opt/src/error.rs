//! Optimizer error types.

use thiserror::Error;

/// Result type for optimizer operations.
pub type OptResult<T> = Result<T, OptError>;

/// Errors that can occur during optimization.
#[derive(Debug, Error)]
pub enum OptError {
    /// A pass encountered an internal consistency error.
    #[error("internal optimizer error in pass '{pass}': {message}")]
    Internal { pass: String, message: String },

    /// The optimizer exceeded its maximum iteration count without reaching fixpoint.
    #[error("optimizer did not converge after {iterations} iterations")]
    NoConvergence { iterations: usize },

    /// Invalid MIR structure detected during optimization.
    #[error("invalid MIR structure: {0}")]
    InvalidMir(String),
}

impl OptError {
    /// Create an internal error for a specific pass.
    pub fn internal(pass: impl Into<String>, message: impl Into<String>) -> Self {
        OptError::Internal {
            pass: pass.into(),
            message: message.into(),
        }
    }
}
