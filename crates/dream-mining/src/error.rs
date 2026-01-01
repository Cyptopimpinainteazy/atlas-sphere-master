//! Error types for Dream Mining

use thiserror::Error;

pub type DreamResult<T> = Result<T, DreamError>;

#[derive(Debug, Error)]
pub enum DreamError {
    #[error("Dream Mining is already running")]
    AlreadyRunning,

    #[error("Not currently running")]
    NotRunning,

    #[error("Task not found: {0}")]
    TaskNotFound(uuid::Uuid),

    #[error("System monitoring error: {0}")]
    MonitorError(String),

    #[error("GPU error: {0}")]
    GpuError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Task execution error: {0}")]
    ExecutionError(String),

    #[error("Resource limit exceeded: {0}")]
    ResourceLimit(String),

    #[error("Schedule error: {0}")]
    ScheduleError(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<std::io::Error> for DreamError {
    fn from(e: std::io::Error) -> Self {
        DreamError::Internal(e.to_string())
    }
}

impl From<serde_json::Error> for DreamError {
    fn from(e: serde_json::Error) -> Self {
        DreamError::Internal(e.to_string())
    }
}
