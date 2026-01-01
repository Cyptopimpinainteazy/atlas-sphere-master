//! Error types for the indexer.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, IndexerError>;

#[derive(Error, Debug)]
pub enum IndexerError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("RPC error: {0}")]
    Rpc(String),

    #[error("Subxt error: {0}")]
    Subxt(#[from] subxt::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Block not found: {0}")]
    BlockNotFound(u64),

    #[error("Decode error: {0}")]
    Decode(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<config::ConfigError> for IndexerError {
    fn from(e: config::ConfigError) -> Self {
        IndexerError::Config(e.to_string())
    }
}
