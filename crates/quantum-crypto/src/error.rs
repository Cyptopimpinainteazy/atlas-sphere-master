//! Error types for Quantum Crypto

use thiserror::Error;

pub type QuantumResult<T> = Result<T, QuantumError>;

#[derive(Debug, Error)]
pub enum QuantumError {
    #[error("Key generation failed: {0}")]
    KeyGenFailed(String),

    #[error("Signature verification failed")]
    SignatureVerificationFailed,

    #[error("Decapsulation failed")]
    DecapsulationFailed,

    #[error("Invalid key size: expected {expected}, got {actual}")]
    InvalidKeySize { expected: usize, actual: usize },

    #[error("Invalid signature size: expected {expected}, got {actual}")]
    InvalidSignatureSize { expected: usize, actual: usize },

    #[error("Invalid ciphertext")]
    InvalidCiphertext,

    #[error("Encoding error: {0}")]
    EncodingError(String),

    #[error("Random number generation failed: {0}")]
    RngError(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<std::io::Error> for QuantumError {
    fn from(e: std::io::Error) -> Self {
        QuantumError::Internal(e.to_string())
    }
}
