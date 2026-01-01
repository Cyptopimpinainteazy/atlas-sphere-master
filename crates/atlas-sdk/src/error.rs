//! Error types for the Atlas SDK.

use thiserror::Error;

/// Result type for Atlas SDK operations
pub type Result<T> = std::result::Result<T, AtlasError>;

/// Main error type for the Atlas SDK
#[derive(Debug, Error)]
pub enum AtlasError {
    /// Connection error
    #[error("Connection error: {0}")]
    Connection(String),

    /// RPC error
    #[error("RPC error: {0}")]
    Rpc(String),

    /// Encoding error
    #[error("Encoding error: {0}")]
    Encoding(String),

    /// Decoding error
    #[error("Decoding error: {0}")]
    Decoding(String),

    /// Invalid payload
    #[error("Invalid payload: {0}")]
    InvalidPayload(String),

    /// Payload too large
    #[error("Payload too large: {0} bytes")]
    PayloadTooLarge(usize),

    /// Invalid address
    #[error("Invalid address: {0}")]
    InvalidAddress(String),

    /// Invalid signature
    #[error("Invalid signature: {0}")]
    InvalidSignature(String),

    /// Invalid key
    #[error("Invalid key: {0}")]
    InvalidKey(String),

    /// No signer configured
    #[error("No signer configured")]
    NoSigner,

    /// Transaction failed
    #[error("Transaction failed: {0}")]
    TransactionFailed(String),

    /// Insufficient balance
    #[error("Insufficient balance: required {required}, available {available}")]
    InsufficientBalance { required: u128, available: u128 },

    /// Account not authorized
    #[error("Account not authorized: {0}")]
    NotAuthorized(String),

    /// Invalid nonce
    #[error("Invalid nonce: expected {expected}, got {got}")]
    InvalidNonce { expected: u64, got: u64 },

    /// Timeout error
    #[error("Operation timed out after {0}ms")]
    Timeout(u64),

    /// EVM execution error
    #[error("EVM execution error: {0}")]
    EvmExecution(String),

    /// SVM execution error
    #[error("SVM execution error: {0}")]
    SvmExecution(String),

    /// Cross-VM error
    #[error("Cross-VM error: {0}")]
    CrossVm(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// HTTP error
    #[error("HTTP error: {0}")]
    Http(String),

    /// WebSocket error
    #[error("WebSocket error: {0}")]
    WebSocket(String),

    /// Subscription error
    #[error("Subscription error: {0}")]
    Subscription(String),

    /// Asset not found
    #[error("Asset not found: {0}")]
    AssetNotFound(String),

    /// Invalid chain ID
    #[error("Invalid chain ID: expected {expected}, got {got}")]
    InvalidChainId { expected: u64, got: u64 },

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<reqwest::Error> for AtlasError {
    fn from(err: reqwest::Error) -> Self {
        AtlasError::Http(err.to_string())
    }
}

impl From<serde_json::Error> for AtlasError {
    fn from(err: serde_json::Error) -> Self {
        AtlasError::Serialization(err.to_string())
    }
}

impl From<tokio_tungstenite::tungstenite::Error> for AtlasError {
    fn from(err: tokio_tungstenite::tungstenite::Error) -> Self {
        AtlasError::WebSocket(err.to_string())
    }
}

impl From<url::ParseError> for AtlasError {
    fn from(err: url::ParseError) -> Self {
        AtlasError::Connection(err.to_string())
    }
}

impl From<codec::Error> for AtlasError {
    fn from(err: codec::Error) -> Self {
        AtlasError::Encoding(err.to_string())
    }
}

impl From<hex::FromHexError> for AtlasError {
    fn from(err: hex::FromHexError) -> Self {
        AtlasError::Decoding(err.to_string())
    }
}

impl AtlasError {
    /// Create an RPC error
    pub fn rpc(message: impl Into<String>) -> Self {
        AtlasError::Rpc(message.into())
    }

    /// Create a payload too large error
    pub fn payload_too_large(size: usize) -> Self {
        AtlasError::PayloadTooLarge(size)
    }

    /// Create an insufficient balance error
    pub fn insufficient_balance(required: u128, available: u128) -> Self {
        AtlasError::InsufficientBalance {
            required,
            available,
        }
    }

    /// Create an invalid nonce error
    pub fn invalid_nonce(expected: u64, got: u64) -> Self {
        AtlasError::InvalidNonce { expected, got }
    }

    /// Create an invalid chain ID error
    pub fn invalid_chain_id(expected: u64, got: u64) -> Self {
        AtlasError::InvalidChainId { expected, got }
    }
}
