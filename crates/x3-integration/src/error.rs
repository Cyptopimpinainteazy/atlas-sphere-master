//! Error types for X3 integration

#[cfg(not(feature = "std"))]
use alloc::string::String;

use codec::{Decode, Encode};
use scale_info::TypeInfo;

/// Result type for X3 integration operations
pub type X3Result<T> = Result<T, X3IntegrationError>;

/// Errors that can occur during X3 integration
#[derive(Clone, Debug, Encode, Decode, TypeInfo, PartialEq, Eq)]
pub enum X3IntegrationError {
    /// Bytecode verification failed
    VerificationFailed(String),
    /// Bytecode format is invalid
    InvalidBytecode(String),
    /// Gas limit exceeded during execution
    GasExhausted { used: u64, limit: u64 },
    /// Execution failed with error
    ExecutionFailed(String),
    /// Hostcall failed
    HostcallFailed { id: u32, reason: String },
    /// Invalid function index
    InvalidFunctionIndex(u32),
    /// Type mismatch in arguments or return value
    TypeMismatch(String),
    /// Stack overflow
    StackOverflow,
    /// Memory access out of bounds
    MemoryOutOfBounds,
    /// Atomic block violation (unbalanced begin/end)
    AtomicViolation(String),
    /// Module not found
    ModuleNotFound,
    /// Compilation failed (when compile feature enabled)
    CompilationFailed(String),
}

#[cfg(feature = "std")]
impl std::fmt::Display for X3IntegrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::VerificationFailed(msg) => write!(f, "Verification failed: {}", msg),
            Self::InvalidBytecode(msg) => write!(f, "Invalid bytecode: {}", msg),
            Self::GasExhausted { used, limit } => {
                write!(f, "Gas exhausted: used {} of {} limit", used, limit)
            }
            Self::ExecutionFailed(msg) => write!(f, "Execution failed: {}", msg),
            Self::HostcallFailed { id, reason } => {
                write!(f, "Hostcall {} failed: {}", id, reason)
            }
            Self::InvalidFunctionIndex(idx) => write!(f, "Invalid function index: {}", idx),
            Self::TypeMismatch(msg) => write!(f, "Type mismatch: {}", msg),
            Self::StackOverflow => write!(f, "Stack overflow"),
            Self::MemoryOutOfBounds => write!(f, "Memory access out of bounds"),
            Self::AtomicViolation(msg) => write!(f, "Atomic block violation: {}", msg),
            Self::ModuleNotFound => write!(f, "Module not found"),
            Self::CompilationFailed(msg) => write!(f, "Compilation failed: {}", msg),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for X3IntegrationError {}

impl From<X3IntegrationError> for frame_support::pallet_prelude::DispatchError {
    fn from(err: X3IntegrationError) -> Self {
        frame_support::pallet_prelude::DispatchError::Other(match err {
            X3IntegrationError::VerificationFailed(_) => "X3: Verification failed",
            X3IntegrationError::InvalidBytecode(_) => "X3: Invalid bytecode",
            X3IntegrationError::GasExhausted { .. } => "X3: Gas exhausted",
            X3IntegrationError::ExecutionFailed(_) => "X3: Execution failed",
            X3IntegrationError::HostcallFailed { .. } => "X3: Hostcall failed",
            X3IntegrationError::InvalidFunctionIndex(_) => "X3: Invalid function index",
            X3IntegrationError::TypeMismatch(_) => "X3: Type mismatch",
            X3IntegrationError::StackOverflow => "X3: Stack overflow",
            X3IntegrationError::MemoryOutOfBounds => "X3: Memory out of bounds",
            X3IntegrationError::AtomicViolation(_) => "X3: Atomic violation",
            X3IntegrationError::ModuleNotFound => "X3: Module not found",
            X3IntegrationError::CompilationFailed(_) => "X3: Compilation failed",
        })
    }
}
