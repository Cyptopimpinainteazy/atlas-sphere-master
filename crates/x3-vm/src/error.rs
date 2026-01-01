//! Error types for the X3 VM.

use std::fmt;
use thiserror::Error;

// =============================================================================
// Verifier Errors
// =============================================================================

/// Errors that can occur during bytecode verification.
#[derive(Debug)]
pub struct VerifierError {
    pub kind: VerifierErrorKind,
    pub offset: Option<usize>,
}

impl VerifierError {
    pub fn new(kind: VerifierErrorKind, offset: usize) -> Self {
        Self {
            kind,
            offset: Some(offset),
        }
    }

    pub fn without_offset(kind: VerifierErrorKind) -> Self {
        Self { kind, offset: None }
    }
}

impl std::error::Error for VerifierError {}

/// Specific verifier error kinds.
#[derive(Debug, Error)]
pub enum VerifierErrorKind {
    #[error("empty module")]
    EmptyModule,

    #[error("invalid magic bytes (expected 'X3BC')")]
    BadMagic,

    #[error("unsupported bytecode version {0}")]
    UnsupportedVersion(u32),

    #[error("checksum mismatch")]
    ChecksumMismatch,

    #[error("module too large ({0} bytes, max {1})")]
    ModuleTooLarge(usize, usize),

    #[error("decode error: unexpected end of input")]
    UnexpectedEof,

    #[error("invalid opcode 0x{0:02x}")]
    InvalidOpcode(u8),

    #[error("operand read out of bounds")]
    OperandOutOfBounds,

    #[error("jump target {0} out of bounds (code length {1})")]
    JumpTargetOutOfBounds(u32, usize),

    #[error("jump target {0} not aligned to instruction boundary")]
    JumpTargetUnaligned(u32),

    #[error("constant pool index {0} out of bounds (pool size {1})")]
    ConstPoolIndexOutOfBounds(u32, usize),

    #[error("function index {0} out of bounds (function count {1})")]
    FunctionIndexOutOfBounds(u32, usize),

    #[error("global index {0} out of bounds (global count {1})")]
    GlobalIndexOutOfBounds(u32, usize),

    #[error("register index {0} exceeds maximum ({1})")]
    RegisterOutOfBounds(u16, u16),

    #[error("atomic markers unbalanced: {0} unclosed blocks")]
    AtomicUnbalanced(i32),

    #[error("atomic block ID {0} mismatch")]
    AtomicIdMismatch(u16),

    #[error("forbidden opcode 0x{0:02x} in on-chain context")]
    ForbiddenOnChain(u8),

    #[error("gas budget {0} exceeds maximum allowed {1}")]
    GasBudgetExceeded(u128, u128),

    #[error("function {0} has entry point {1} beyond code section")]
    FunctionEntryOutOfBounds(usize, u32),

    #[error("function {0} has invalid parameter count {1}")]
    InvalidParamCount(usize, u8),

    #[error("unreachable code detected at offset {0}")]
    UnreachableCode(usize),

    #[error("missing return at end of function {0}")]
    MissingReturn(usize),

    #[error("type mismatch: expected {0}, found {1}")]
    TypeMismatch(String, String),

    #[error("stack overflow at offset {0}")]
    StackOverflow(usize),

    #[error("stack underflow at offset {0}")]
    StackUnderflow(usize),

    #[error("parse error: {0}")]
    ParseError(String),
}

// =============================================================================
// VM Runtime Errors
// =============================================================================

/// Errors that can occur during VM execution.
#[derive(Debug)]
pub struct VMError {
    pub kind: VMErrorKind,
    pub ip: Option<usize>,
}

impl VMError {
    pub fn new(kind: VMErrorKind, ip: usize) -> Self {
        Self { kind, ip: Some(ip) }
    }

    pub fn without_ip(kind: VMErrorKind) -> Self {
        Self { kind, ip: None }
    }

    pub fn at_ip(ip: usize, kind: VMErrorKind) -> Self {
        Self { kind, ip: Some(ip) }
    }
}

impl std::error::Error for VMError {}

/// Specific VM error kinds.
#[derive(Debug, Error)]
pub enum VMErrorKind {
    #[error("verification failed: {0}")]
    VerificationFailed(String),

    #[error("module load error: {0}")]
    ModuleLoadError(String),

    #[error("invalid opcode 0x{0:02x}")]
    InvalidOpcode(u8),

    #[error("unimplemented opcode 0x{0:02x}")]
    UnimplementedOpcode(u8),

    #[error("instruction pointer out of bounds")]
    InstructionPointerOutOfBounds,

    #[error("register {0} out of bounds")]
    RegisterOutOfBounds(u16),

    #[error("stack underflow")]
    StackUnderflow,

    #[error("stack overflow (depth {0}, max {1})")]
    StackOverflow(usize, usize),

    #[error("call stack overflow (depth {0}, max {1})")]
    CallStackOverflow(usize, usize),

    #[error("constant pool index {0} out of bounds")]
    ConstPoolOutOfBounds(usize),

    #[error("function index {0} not found")]
    FunctionNotFound(usize),

    #[error("function '{0}' not found")]
    FunctionNotFoundByName(String),

    #[error("global index {0} out of bounds (max {1})")]
    GlobalOutOfBounds(usize, usize),

    #[error("array index {0} out of bounds (length {1})")]
    ArrayIndexOutOfBounds(i64, usize),

    #[error("tuple index {0} out of bounds (length {1})")]
    TupleIndexOutOfBounds(u16, usize),

    #[error("field index {0} out of bounds")]
    FieldIndexOutOfBounds(u16),

    #[error("type mismatch: expected {0}, found {1}")]
    TypeMismatch(String, String),

    #[error("division by zero")]
    DivisionByZero,

    #[error("integer overflow")]
    IntegerOverflow,

    #[error("gas limit exceeded")]
    GasLimitExceeded,

    #[error("atomic end without matching begin")]
    AtomicEndWithoutBegin,

    #[error("atomic rollback without matching begin")]
    AtomicRollbackWithoutBegin,

    #[error("atomic block {0} already active")]
    AtomicAlreadyActive(u16),

    #[error("atomic block {0} not active")]
    AtomicNotActive(u16),

    #[error("atomic transaction aborted")]
    AtomicAborted,

    #[error("hostcall {0} not found")]
    HostcallNotFound(u8),

    #[error("hostcall error: {0}")]
    HostcallError(String),

    #[error("halt instruction executed")]
    Halted,

    #[error("invalid function: {0}")]
    InvalidFunction(String),

    #[error("argument count mismatch: expected {0}, got {1}")]
    ArgumentCountMismatch(usize, usize),

    #[error("return from empty call stack")]
    ReturnFromEmptyStack,

    #[error("execution timeout")]
    Timeout,

    #[error("user panic: {0}")]
    UserPanic(String),

    #[error("assertion failed")]
    AssertionFailed,

    #[error("not implemented: {0}")]
    NotImplemented(String),
}

// =============================================================================
// Result Types
// =============================================================================

/// Result type for verifier operations.
pub type VerifierResult<T> = Result<T, VerifierError>;

/// Result type for VM operations.
pub type VMResult<T> = Result<T, VMError>;

// =============================================================================
// Display Implementations
// =============================================================================

impl fmt::Display for VerifierError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(offset) = self.offset {
            write!(f, "verification error at offset {}: {}", offset, self.kind)
        } else {
            write!(f, "verification error: {}", self.kind)
        }
    }
}

impl fmt::Display for VMError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ip) = self.ip {
            write!(f, "runtime error at IP {}: {}", ip, self.kind)
        } else {
            write!(f, "runtime error: {}", self.kind)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifier_error_display() {
        let err = VerifierError::new(VerifierErrorKind::InvalidOpcode(0xFF), 100);
        let msg = format!("{}", err);
        assert!(msg.contains("0xff"));
        assert!(msg.contains("100"));
    }

    #[test]
    fn vm_error_display() {
        let err = VMError::new(VMErrorKind::DivisionByZero, 42);
        let msg = format!("{}", err);
        assert!(msg.contains("division by zero"));
        assert!(msg.contains("42"));
    }
}
