//! Error types for Voice-to-X3

use thiserror::Error;

pub type VoiceResult<T> = Result<T, VoiceError>;

#[derive(Debug, Error)]
pub enum VoiceError {
    #[error("Failed to parse intent: {0}")]
    IntentParseError(String),

    #[error("Unknown contract type: {0}")]
    UnknownContractType(String),

    #[error("Template not found: {0}")]
    TemplateNotFound(String),

    #[error("Code generation failed: {0}")]
    CodeGenError(String),

    #[error("Compilation failed: {0}")]
    CompilationError(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("AI service error: {0}")]
    AIServiceError(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<std::io::Error> for VoiceError {
    fn from(e: std::io::Error) -> Self {
        VoiceError::Internal(e.to_string())
    }
}

impl From<serde_json::Error> for VoiceError {
    fn from(e: serde_json::Error) -> Self {
        VoiceError::Internal(e.to_string())
    }
}
