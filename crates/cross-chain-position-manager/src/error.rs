//! Error handling for Cross-Chain Position Manager
//!
//! This module defines all error types used throughout the system,
//! providing detailed error information for debugging and recovery.

use sp_core::H160;
use sp_std::vec::Vec;
use thiserror::Error;

/// Result type alias
pub type Result<T> = std::result::Result<T, PositionManagerError>;

/// Main error type for the Cross-Chain Position Manager
#[derive(Error, Debug)]
pub enum PositionManagerError {
    /// Configuration errors
    #[error("Configuration error: {0}")]
    Config(String),

    /// Chain connection errors
    #[error("Chain connection error for chain {chain_id}: {message}")]
    ChainConnection {
        chain_id: u64,
        message: String,
    },

    /// Position tracking errors
    #[error("Position tracking error: {0}")]
    PositionTracking(String),

    /// Migration errors
    #[error("Migration error for position {position_id}: {message}")]
    Migration {
        position_id: String,
        message: String,
    },

    /// Rebalancing errors
    #[error("Rebalancing error: {0}")]
    Rebalancing(String),

    /// Arbitrage errors
    #[error("Arbitrage error: {0}")]
    Arbitrage(String),

    /// Risk management errors
    #[error("Risk management error: {0}")]
    RiskManagement(String),

    /// State management errors
    #[error("State management error: {0}")]
    StateManagement(String),

    /// Event system errors
    #[error("Event system error: {0}")]
    EventSystem(String),

    /// External API errors
    #[error("External API error: {0}")]
    ExternalApi(String),

    /// Database errors
    #[error("Database error: {0}")]
    Database(String),

    /// Serialization/deserialization errors
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Network errors
    #[error("Network error: {0}")]
    Network(String),

    /// Smart contract errors
    #[error("Smart contract error on chain {chain_id}: {contract_address} - {message}")]
    SmartContract {
        chain_id: u64,
        contract_address: H160,
        message: String,
    },

    /// Insufficient funds
    #[error("Insufficient funds for operation. Required: {required}, Available: {available}")]
    InsufficientFunds {
        required: String,
        available: String,
    },

    /// Gas price too high
    #[error("Gas price too high. Current: {current}, Maximum: {maximum}")]
    GasPriceTooHigh {
        current: String,
        maximum: String,
    },

    /// Slippage exceeded
    #[error("Slippage exceeded. Expected: {expected}, Actual: {actual}, Tolerance: {tolerance}")]
    SlippageExceeded {
        expected: String,
        actual: String,
        tolerance: f64,
    },

    /// Timeout
    #[error("Operation timed out after {timeout_ms}ms")]
    Timeout {
        timeout_ms: u64,
    },

    /// Invalid parameters
    #[error("Invalid parameters: {message}")]
    InvalidParameters {
        message: String,
    },

    /// Unsupported operation
    #[error("Unsupported operation: {operation} on chain {chain_id}")]
    UnsupportedOperation {
        operation: String,
        chain_id: u64,
    },

    /// Liquidity insufficient
    #[error("Insufficient liquidity for asset {asset_address} on chain {chain_id}")]
    InsufficientLiquidity {
        asset_address: H160,
        chain_id: u64,
    },

    /// Chain not supported
    #[error("Chain {chain_id} is not supported")]
    ChainNotSupported {
        chain_id: u64,
    },

    /// Asset not found
    #[error("Asset {asset_address} not found on chain {chain_id}")]
    AssetNotFound {
        asset_address: H160,
        chain_id: u64,
    },

    /// Position not found
    #[error("Position {position_id} not found")]
    PositionNotFound {
        position_id: String,
    },

    /// Route not found
    #[error("No viable route found from chain {from_chain} to chain {to_chain}")]
    RouteNotFound {
        from_chain: u64,
        to_chain: u64,
    },

    /// Atomic bundle execution failed
    #[error("Atomic bundle execution failed: {bundle_id}")]
    AtomicBundleFailed {
        bundle_id: String,
    },

    /// Kill switch triggered
    #[error("Kill switch triggered: {trigger_type} on chain {chain_id}")]
    KillSwitchTriggered {
        trigger_type: String,
        chain_id: u64,
    },

    /// Risk threshold exceeded
    #[error("Risk threshold exceeded: {risk_type} - {description}")]
    RiskThresholdExceeded {
        risk_type: String,
        description: String,
    },

    /// Validation failed
    #[error("Validation failed: {field} - {reason}")]
    ValidationFailed {
        field: String,
        reason: String,
    },

    /// Integration error
    #[error("Integration error with {service}: {message}")]
    IntegrationError {
        service: String,
        message: String,
    },

    /// Internal error
    #[error("Internal error: {message}")]
    Internal {
        message: String,
    },
}

/// Error context for additional debugging information
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub component: String,
    pub operation: String,
    pub chain_id: Option<u64>,
    pub position_id: Option<String>,
    pub additional_data: Vec<(String, String)>,
}

impl ErrorContext {
    pub fn new(component: String, operation: String) -> Self {
        Self {
            component,
            operation,
            chain_id: None,
            position_id: None,
            additional_data: Vec::new(),
        }
    }

    pub fn with_chain(mut self, chain_id: u64) -> Self {
        self.chain_id = Some(chain_id);
        self
    }

    pub fn with_position(mut self, position_id: String) -> Self {
        self.position_id = Some(position_id);
        self
    }

    pub fn with_data(mut self, key: String, value: String) -> Self {
        self.additional_data.push((key, value));
        self
    }
}

impl PositionManagerError {
    /// Add context to an error
    pub fn with_context(self, context: ErrorContext) -> PositionManagerError {
        match self {
            PositionManagerError::Config(msg) => {
                PositionManagerError::Config(format!("{} - Context: {}:{}", msg, context.component, context.operation))
            }
            PositionManagerError::ChainConnection { chain_id, message } => {
                PositionManagerError::ChainConnection {
                    chain_id,
                    message: format!("{} - Context: {}:{}", message, context.component, context.operation)
                }
            }
            PositionManagerError::PositionTracking(msg) => {
                PositionManagerError::PositionTracking(format!("{} - Context: {}:{}", msg, context.component, context.operation))
            }
            PositionManagerError::Migration { position_id, message } => {
                PositionManagerError::Migration {
                    position_id,
                    message: format!("{} - Context: {}:{}", message, context.component, context.operation)
                }
            }
            PositionManagerError::Rebalancing(msg) => {
                PositionManagerError::Rebalancing(format!("{} - Context: {}:{}", msg, context.component, context.operation))
            }
            PositionManagerError::Arbitrage(msg) => {
                PositionManagerError::Arbitrage(format!("{} - Context: {}:{}", msg, context.component, context.operation))
            }
            PositionManagerError::RiskManagement(msg) => {
                PositionManagerError::RiskManagement(format!("{} - Context: {}:{}", msg, context.component, context.operation))
            }
            PositionManagerError::StateManagement(msg) => {
                PositionManagerError::StateManagement(format!("{} - Context: {}:{}", msg, context.component, context.operation))
            }
            PositionManagerError::EventSystem(msg) => {
                PositionManagerError::EventSystem(format!("{} - Context: {}:{}", msg, context.component, context.operation))
            }
            PositionManagerError::ExternalApi(msg) => {
                PositionManagerError::ExternalApi(format!("{} - Context: {}:{}", msg, context.component, context.operation))
            }
            PositionManagerError::Database(msg) => {
                PositionManagerError::Database(format!("{} - Context: {}:{}", msg, context.component, context.operation))
            }
            PositionManagerError::Serialization(msg) => {
                PositionManagerError::Serialization(format!("{} - Context: {}:{}", msg, context.component, context.operation))
            }
            PositionManagerError::Network(msg) => {
                PositionManagerError::Network(format!("{} - Context: {}:{}", msg, context.component, context.operation))
            }
            PositionManagerError::SmartContract { chain_id, contract_address, message } => {
                PositionManagerError::SmartContract {
                    chain_id,
                    contract_address,
                    message: format!("{} - Context: {}:{}", message, context.component, context.operation)
                }
            }
            PositionManagerError::InsufficientFunds { required, available } => {
                PositionManagerError::InsufficientFunds { required, available }
            }
            PositionManagerError::GasPriceTooHigh { current, maximum } => {
                PositionManagerError::GasPriceTooHigh { current, maximum }
            }
            PositionManagerError::SlippageExceeded { expected, actual, tolerance } => {
                PositionManagerError::SlippageExceeded { expected, actual, tolerance }
            }
            PositionManagerError::Timeout { timeout_ms } => {
                PositionManagerError::Timeout { timeout_ms }
            }
            PositionManagerError::InvalidParameters { message } => {
                PositionManagerError::InvalidParameters { message }
            }
            PositionManagerError::UnsupportedOperation { operation, chain_id } => {
                PositionManagerError::UnsupportedOperation { operation, chain_id }
            }
            PositionManagerError::InsufficientLiquidity { asset_address, chain_id } => {
                PositionManagerError::InsufficientLiquidity { asset_address, chain_id }
            }
            PositionManagerError::ChainNotSupported { chain_id } => {
                PositionManagerError::ChainNotSupported { chain_id }
            }
            PositionManagerError::AssetNotFound { asset_address, chain_id } => {
                PositionManagerError::AssetNotFound { asset_address, chain_id }
            }
            PositionManagerError::PositionNotFound { position_id } => {
                PositionManagerError::PositionNotFound { position_id }
            }
            PositionManagerError::RouteNotFound { from_chain, to_chain } => {
                PositionManagerError::RouteNotFound { from_chain, to_chain }
            }
            PositionManagerError::AtomicBundleFailed { bundle_id } => {
                PositionManagerError::AtomicBundleFailed { bundle_id }
            }
            PositionManagerError::KillSwitchTriggered { trigger_type, chain_id } => {
                PositionManagerError::KillSwitchTriggered { trigger_type, chain_id }
            }
            PositionManagerError::RiskThresholdExceeded { risk_type, description } => {
                PositionManagerError::RiskThresholdExceeded { risk_type, description }
            }
            PositionManagerError::ValidationFailed { field, reason } => {
                PositionManagerError::ValidationFailed { field, reason }
            }
            PositionManagerError::IntegrationError { service, message } => {
                PositionManagerError::IntegrationError { service, message }
            }
            PositionManagerError::Internal { message } => {
                PositionManagerError::Internal { message }
            }
        }
    }

    /// Check if this is a retryable error
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            PositionManagerError::Network(_) |
            PositionManagerError::Timeout { .. } |
            PositionManagerError::ExternalApi(_) |
            PositionManagerError::ChainConnection { .. } |
            PositionManagerError::Database(_)
        )
    }

    /// Check if this is a fatal error (should not be retried)
    pub fn is_fatal(&self) -> bool {
        matches!(
            self,
            PositionManagerError::InvalidParameters { .. } |
            PositionManagerError::UnsupportedOperation { .. } |
            PositionManagerError::ValidationFailed { .. } |
            PositionManagerError::Config(_)
        )
    }

    /// Get the severity level of the error
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            PositionManagerError::KillSwitchTriggered { .. } |
            PositionManagerError::RiskThresholdExceeded { .. } => ErrorSeverity::Critical,
            
            PositionManagerError::InsufficientFunds { .. } |
            PositionManagerError::GasPriceTooHigh { .. } |
            PositionManagerError::SlippageExceeded { .. } |
            PositionManagerError::AtomicBundleFailed { .. } |
            PositionManagerError::SmartContract { .. } => ErrorSeverity::High,
            
            PositionManagerError::ChainConnection { .. } |
            PositionManagerError::Network(_) |
            PositionManagerError::Timeout { .. } |
            PositionManagerError::ExternalApi(_) |
            PositionManagerError::Database(_) => ErrorSeverity::Medium,
            
            PositionManagerError::Config(_) |
            PositionManagerError::InvalidParameters { .. } |
            PositionManagerError::UnsupportedOperation { .. } |
            PositionManagerError::ValidationFailed { .. } => ErrorSeverity::Low,
            
            _ => ErrorSeverity::Medium,
        }
    }
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    Low = 1,
    Medium = 2,
    High = 3,
    Critical = 4,
}

impl ErrorSeverity {
    pub fn is_critical_or_high(&self) -> bool {
        matches!(self, ErrorSeverity::High | ErrorSeverity::Critical)
    }
}

/// Conversion helpers for common error types
impl From<reqwest::Error> for PositionManagerError {
    fn from(err: reqwest::Error) -> Self {
        PositionManagerError::ExternalApi(format!("HTTP request failed: {}", err))
    }
}

impl From<serde_json::Error> for PositionManagerError {
    fn from(err: serde_json::Error) -> Self {
        PositionManagerError::Serialization(format!("JSON serialization failed: {}", err))
    }
}

impl From<std::io::Error> for PositionManagerError {
    fn from(err: std::io::Error) -> Self {
        PositionManagerError::Database(format!("IO error: {}", err))
    }
}

impl From<tokio::time::error::Elapsed> for PositionManagerError {
    fn from(_err: tokio::time::error::Elapsed) -> Self {
        PositionManagerError::Timeout { timeout_ms: 0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_context() {
        let context = ErrorContext::new("test_component".to_string(), "test_operation".to_string())
            .with_chain(1)
            .with_position("pos123".to_string())
            .with_data("key1".to_string(), "value1".to_string());

        assert_eq!(context.component, "test_component");
        assert_eq!(context.operation, "test_operation");
        assert_eq!(context.chain_id, Some(1));
        assert_eq!(context.position_id, Some("pos123".to_string()));
        assert_eq!(context.additional_data.len(), 1);
    }

    #[test]
    fn test_error_severity() {
        let kill_switch_error = PositionManagerError::KillSwitchTriggered {
            trigger_type: "gas_spike".to_string(),
            chain_id: 1,
        };
        assert_eq!(kill_switch_error.severity(), ErrorSeverity::Critical);

        let config_error = PositionManagerError::Config("invalid config".to_string());
        assert_eq!(config_error.severity(), ErrorSeverity::Low);

        let network_error = PositionManagerError::Network("connection failed".to_string());
        assert_eq!(network_error.severity(), ErrorSeverity::Medium);
    }

    #[test]
    fn test_error_retryability() {
        let network_error = PositionManagerError::Network("timeout".to_string());
        assert!(network_error.is_retryable());
        assert!(!network_error.is_fatal());

        let invalid_params_error = PositionManagerError::InvalidParameters {
            message: "bad parameters".to_string(),
        };
        assert!(!invalid_params_error.is_retryable());
        assert!(invalid_params_error.is_fatal());
    }
}
