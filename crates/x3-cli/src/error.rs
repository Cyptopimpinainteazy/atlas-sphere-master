//! Error types for x3 CLI.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, CliError>;

#[derive(Debug, Error)]
pub enum CliError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Project error: {0}")]
    Project(String),

    #[error("Build error: {0}")]
    Build(String),

    #[error("Deployment error: {0}")]
    Deploy(String),

    #[error("Test error: {0}")]
    Test(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[cfg(feature = "sdk")]
    #[error("SDK error: {0}")]
    Sdk(#[from] atlas_sdk::AtlasError),

    #[error("Command failed: {0}")]
    Command(String),

    #[error("Not implemented: {0}")]
    NotImplemented(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
}

impl From<serde_json::Error> for CliError {
    fn from(err: serde_json::Error) -> Self {
        CliError::Serialization(err.to_string())
    }
}

impl From<toml::de::Error> for CliError {
    fn from(err: toml::de::Error) -> Self {
        CliError::Config(err.to_string())
    }
}

impl From<toml::ser::Error> for CliError {
    fn from(err: toml::ser::Error) -> Self {
        CliError::Config(err.to_string())
    }
}
