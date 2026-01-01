//! Error types for external chain operations

use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_std::vec::Vec;

/// Errors that can occur during external chain operations
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, TypeInfo)]
pub enum ExternalChainError {
    /// Connection to external chain failed
    ConnectionFailed(Vec<u8>),
    /// RPC call failed
    RpcError(Vec<u8>),
    /// Transaction failed
    TransactionFailed(Vec<u8>),
    /// Transaction reverted
    TransactionReverted(Vec<u8>),
    /// Insufficient gas
    InsufficientGas,
    /// Insufficient balance
    InsufficientBalance,
    /// Invalid chain ID
    InvalidChainId(u64),
    /// Chain not supported
    ChainNotSupported(u64),
    /// Message verification failed
    MessageVerificationFailed,
    /// Invalid proof
    InvalidProof,
    /// Transfer not found
    TransferNotFound,
    /// Transfer already processed
    TransferAlreadyProcessed,
    /// Transfer expired
    TransferExpired,
    /// Invalid address
    InvalidAddress,
    /// Invalid amount
    InvalidAmount,
    /// Token not supported
    TokenNotSupported,
    /// Bridge paused
    BridgePaused,
    /// Nonce mismatch
    NonceMismatch,
    /// Timeout waiting for confirmation
    Timeout,
    /// Rate limit exceeded
    RateLimitExceeded,
    /// Serialization error
    SerializationError,
    /// Internal error
    InternalError(Vec<u8>),
}

impl ExternalChainError {
    /// Create connection failed error with message
    pub fn connection_failed(msg: &str) -> Self {
        Self::ConnectionFailed(msg.as_bytes().to_vec())
    }

    /// Create RPC error with message
    pub fn rpc_error(msg: &str) -> Self {
        Self::RpcError(msg.as_bytes().to_vec())
    }

    /// Create transaction failed error
    pub fn tx_failed(msg: &str) -> Self {
        Self::TransactionFailed(msg.as_bytes().to_vec())
    }

    /// Create transaction reverted error
    pub fn tx_reverted(msg: &str) -> Self {
        Self::TransactionReverted(msg.as_bytes().to_vec())
    }

    /// Create internal error
    pub fn internal(msg: &str) -> Self {
        Self::InternalError(msg.as_bytes().to_vec())
    }
}

impl core::fmt::Display for ExternalChainError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ConnectionFailed(msg) => {
                write!(f, "Connection failed: {}", String::from_utf8_lossy(msg))
            }
            Self::RpcError(msg) => write!(f, "RPC error: {}", String::from_utf8_lossy(msg)),
            Self::TransactionFailed(msg) => {
                write!(f, "Transaction failed: {}", String::from_utf8_lossy(msg))
            }
            Self::TransactionReverted(msg) => {
                write!(f, "Transaction reverted: {}", String::from_utf8_lossy(msg))
            }
            Self::InsufficientGas => write!(f, "Insufficient gas"),
            Self::InsufficientBalance => write!(f, "Insufficient balance"),
            Self::InvalidChainId(id) => write!(f, "Invalid chain ID: {}", id),
            Self::ChainNotSupported(id) => write!(f, "Chain not supported: {}", id),
            Self::MessageVerificationFailed => write!(f, "Message verification failed"),
            Self::InvalidProof => write!(f, "Invalid proof"),
            Self::TransferNotFound => write!(f, "Transfer not found"),
            Self::TransferAlreadyProcessed => write!(f, "Transfer already processed"),
            Self::TransferExpired => write!(f, "Transfer expired"),
            Self::InvalidAddress => write!(f, "Invalid address"),
            Self::InvalidAmount => write!(f, "Invalid amount"),
            Self::TokenNotSupported => write!(f, "Token not supported"),
            Self::BridgePaused => write!(f, "Bridge is paused"),
            Self::NonceMismatch => write!(f, "Nonce mismatch"),
            Self::Timeout => write!(f, "Timeout waiting for confirmation"),
            Self::RateLimitExceeded => write!(f, "Rate limit exceeded"),
            Self::SerializationError => write!(f, "Serialization error"),
            Self::InternalError(msg) => {
                write!(f, "Internal error: {}", String::from_utf8_lossy(msg))
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ExternalChainError {}
