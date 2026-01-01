//! Atlas Sphere DNS Server - Error Types
//!
//! Comprehensive error handling for DNS operations

use config::ConfigError;
use std::io;
use thiserror::Error;
use trust_dns_proto::error::ProtoError;

/// DNS Result type alias
pub type DnsResult<T> = Result<T, DnsError>;

/// DNS Server Error Types
#[derive(Error, Debug)]
pub enum DnsError {
    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("DNS protocol error: {0}")]
    Protocol(#[from] ProtoError),

    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("Domain not found: {domain}")]
    DomainNotFound { domain: String },

    #[error("Invalid domain name: {name}")]
    InvalidDomainName { name: String },

    #[error("Zone not found: {zone}")]
    ZoneNotFound { zone: String },

    #[error("Invalid DNS record: {record}")]
    InvalidRecord { record: String },

    #[error("Blockchain integration error: {0}")]
    Blockchain(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("DNSSEC error: {0}")]
    DnsSec(String),

    #[error("Cache error: {0}")]
    Cache(String),

    #[error("Registry error: {0}")]
    Registry(String),

    #[error("API error: {0}")]
    Api(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Rate limiting error: {0}")]
    RateLimit(String),

    #[error("Timeout error: {0}")]
    Timeout(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Security error: {0}")]
    Security(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl DnsError {
    /// Create a configuration error
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Configuration(msg.into())
    }

    /// Create a blockchain error
    pub fn blockchain(msg: impl Into<String>) -> Self {
        Self::Blockchain(msg.into())
    }

    /// Create a database error
    pub fn database(msg: impl Into<String>) -> Self {
        Self::Database(msg.into())
    }

    /// Create a domain not found error
    pub fn domain_not_found(domain: impl Into<String>) -> Self {
        Self::DomainNotFound {
            domain: domain.into(),
        }
    }

    /// Create an invalid domain name error
    pub fn invalid_domain_name(name: impl Into<String>) -> Self {
        Self::InvalidDomainName { name: name.into() }
    }

    /// Create a zone not found error
    pub fn zone_not_found(zone: impl Into<String>) -> Self {
        Self::ZoneNotFound { zone: zone.into() }
    }

    /// Create an API error
    pub fn api(msg: impl Into<String>) -> Self {
        Self::Api(msg.into())
    }

    /// Create a validation error
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }

    /// Create a security error
    pub fn security(msg: impl Into<String>) -> Self {
        Self::Security(msg.into())
    }
}

/// Error context for DNS operations
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub operation: String,
    pub domain: Option<String>,
    pub zone: Option<String>,
    pub timestamp: std::time::SystemTime,
    pub client_ip: Option<String>,
}

impl ErrorContext {
    /// Create new error context
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            domain: None,
            zone: None,
            timestamp: std::time::SystemTime::now(),
            client_ip: None,
        }
    }

    /// Set domain for context
    pub fn with_domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = Some(domain.into());
        self
    }

    /// Set zone for context
    pub fn with_zone(mut self, zone: impl Into<String>) -> Self {
        self.zone = Some(zone.into());
        self
    }

    /// Set client IP for context
    pub fn with_client_ip(mut self, ip: impl Into<String>) -> Self {
        self.client_ip = Some(ip.into());
        self
    }
}

/// Enhanced error with context
#[derive(Debug)]
pub struct ContextualError {
    pub error: DnsError,
    pub context: ErrorContext,
}

impl ContextualError {
    /// Create new contextual error
    pub fn new(error: DnsError, context: ErrorContext) -> Self {
        Self { error, context }
    }

    /// Convert to standard error
    pub fn into_error(self) -> DnsError {
        self.error
    }
}

impl std::fmt::Display for ContextualError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DNS Error in {}: {}", self.context.operation, self.error)?;

        if let Some(domain) = &self.context.domain {
            write!(f, " (domain: {})", domain)?;
        }

        if let Some(zone) = &self.context.zone {
            write!(f, " (zone: {})", zone)?;
        }

        if let Some(ip) = &self.context.client_ip {
            write!(f, " (client: {})", ip)?;
        }

        Ok(())
    }
}

impl std::error::Error for ContextualError {}

/// Result type for operations with context
pub type ContextualResult<T> = Result<T, ContextualError>;

/// Error handling macros
#[macro_export]
macro_rules! dns_err {
    ($($arg:tt)*) => {
        Err(DnsError::Internal(format!($($arg)*)))
    };
}

#[macro_export]
macro_rules! dns_context_err {
    ($context:expr, $($arg:tt)*) => {
        Err(ContextualError::new(
            DnsError::Internal(format!($($arg)*)),
            $context.clone()
        ))
    };
}

#[macro_export]
macro_rules! dns_validation_err {
    ($($arg:tt)*) => {
        Err(DnsError::Validation(format!($($arg)*)))
    };
}

#[macro_export]
macro_rules! dns_api_err {
    ($($arg:tt)*) => {
        Err(DnsError::Api(format!($($arg)*)))
    };
}

/// Error categories for monitoring and reporting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    Configuration,
    Network,
    Database,
    Blockchain,
    Validation,
    Security,
    Internal,
}

impl DnsError {
    /// Get error category for monitoring
    pub fn category(&self) -> ErrorCategory {
        match self {
            Self::Configuration(_) => ErrorCategory::Configuration,
            Self::Io(_) => ErrorCategory::Network,
            Self::Protocol(_) => ErrorCategory::Network,
            Self::Database(_) => ErrorCategory::Database,
            Self::Blockchain(_) => ErrorCategory::Blockchain,
            Self::Validation(_) => ErrorCategory::Validation,
            Self::Authentication(_) | Self::Security(_) => ErrorCategory::Security,
            _ => ErrorCategory::Internal,
        }
    }

    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Io(_) | Self::Protocol(_) | Self::Timeout(_) | Self::Network(_)
        )
    }

    /// Check if error is critical
    pub fn is_critical(&self) -> bool {
        matches!(
            self,
            Self::Configuration(_) | Self::Security(_) | Self::Authentication(_)
        )
    }
}
