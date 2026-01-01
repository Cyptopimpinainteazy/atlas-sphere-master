//! Verifier error types

use thiserror::Error;

pub type VerifierResult<T> = Result<T, VerifierError>;

#[derive(Debug, Error)]
pub enum VerifierError {
    #[error("Failed to load safety rules: {0}")]
    RulesLoad(String),

    #[error("Failed to parse safety rules: {0}")]
    RulesParse(String),

    #[error("Invalid MIR: {0}")]
    InvalidMir(String),

    #[error("Gas analysis failed: {0}")]
    GasAnalysis(String),

    #[error("Verification failed with {0} errors")]
    VerificationFailed(usize),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),
}
