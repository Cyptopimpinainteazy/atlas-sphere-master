//! Backend error types for bytecode compilation.

use std::fmt;
use thiserror::Error;
use x3_common::Span;
use x3_hir::hir::SymbolId;

/// Error that occurred during bytecode compilation.
#[derive(Debug, Clone)]
pub struct BackendError {
    pub kind: BackendErrorKind,
    pub span: Option<Span>,
}

impl BackendError {
    pub fn new(kind: BackendErrorKind, span: Span) -> Self {
        Self {
            kind,
            span: Some(span),
        }
    }

    pub fn without_span(kind: BackendErrorKind) -> Self {
        Self { kind, span: None }
    }
}

impl std::error::Error for BackendError {}

impl fmt::Display for BackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)?;
        if let Some(span) = self.span {
            write!(f, " at {}:{}", span.start, span.end)?;
        }
        Ok(())
    }
}

/// Categories of backend errors.
#[derive(Debug, Clone, Error)]
pub enum BackendErrorKind {
    // ========================================================================
    // Symbol Resolution Errors
    // ========================================================================
    #[error("unknown symbol: {0}")]
    UnknownSymbol(SymbolId),

    #[error("unknown function: {name}")]
    UnknownFunction { name: String },

    #[error("unknown global: {name}")]
    UnknownGlobal { name: String },

    #[error("unknown event: {name}")]
    UnknownEvent { name: String },

    // ========================================================================
    // Type Errors
    // ========================================================================
    #[error("type mismatch: expected {expected}, found {found}")]
    TypeMismatch { expected: String, found: String },

    #[error("unsupported type for bytecode: {0}")]
    UnsupportedType(String),

    #[error("cannot emit bytecode for unresolved type")]
    UnresolvedType,

    // ========================================================================
    // Register Allocation Errors
    // ========================================================================
    #[error("register overflow: exceeded maximum {max} registers")]
    RegisterOverflow { max: u16 },

    #[error("register not found: {0}")]
    RegisterNotFound(SymbolId),

    #[error("invalid register index: {0}")]
    InvalidRegister(u16),

    // ========================================================================
    // Constant Pool Errors
    // ========================================================================
    #[error("constant pool overflow: exceeded {max} entries")]
    ConstPoolOverflow { max: u32 },

    #[error("string too long: {len} bytes (max {max})")]
    StringTooLong { len: usize, max: usize },

    #[error("duplicate constant")]
    DuplicateConstant,

    // ========================================================================
    // Control Flow Errors
    // ========================================================================
    #[error("undefined label: {0}")]
    UndefinedLabel(String),

    #[error("duplicate label: {0}")]
    DuplicateLabel(String),

    #[error("jump target out of range: offset {offset}")]
    JumpOutOfRange { offset: i64 },

    #[error("unreachable code detected")]
    UnreachableCode,

    #[error("break outside of loop")]
    BreakOutsideLoop,

    #[error("continue outside of loop")]
    ContinueOutsideLoop,

    // ========================================================================
    // Function Errors
    // ========================================================================
    #[error("too many function parameters: {count} (max {max})")]
    TooManyParameters { count: usize, max: usize },

    #[error("too many function arguments: {count} (max {max})")]
    TooManyArguments { count: usize, max: usize },

    #[error("function table overflow: exceeded {max} functions")]
    FunctionTableOverflow { max: u32 },

    #[error("recursive function call limit exceeded")]
    RecursionLimitExceeded,

    // ========================================================================
    // Atomic Block Errors
    // ========================================================================
    #[error("atomic block ID overflow")]
    AtomicIdOverflow,

    #[error("mismatched atomic block: expected {expected}, found {found}")]
    AtomicBlockMismatch { expected: u16, found: u16 },

    #[error("unclosed atomic block: {0}")]
    UnclosedAtomicBlock(u16),

    // ========================================================================
    // Binary Format Errors
    // ========================================================================
    #[error("bytecode too large: {size} bytes (max {max})")]
    BytecodeTooLarge { size: usize, max: usize },

    #[error("invalid bytecode magic")]
    InvalidMagic,

    #[error("unsupported bytecode version: {0}")]
    UnsupportedVersion(u32),

    #[error("corrupted bytecode at offset {offset}")]
    CorruptedBytecode { offset: usize },

    #[error("unexpected end of bytecode")]
    UnexpectedEof,

    // ========================================================================
    // VM Intrinsic Errors
    // ========================================================================
    #[error("unsupported VM intrinsic: {0}")]
    UnsupportedIntrinsic(String),

    #[error("VM intrinsic requires target VM: {0}")]
    MissingTargetVm(String),

    // ========================================================================
    // Internal Errors
    // ========================================================================
    #[error("internal error: {0}")]
    Internal(String),

    #[error("not implemented: {0}")]
    NotImplemented(String),
}

/// Result type for backend operations.
pub type BackendResult<T> = Result<T, BackendError>;

/// Accumulator for multiple backend errors.
#[derive(Debug, Default)]
pub struct BackendErrors {
    errors: Vec<BackendError>,
}

impl BackendErrors {
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    pub fn push(&mut self, error: BackendError) {
        self.errors.push(error);
    }

    pub fn push_kind(&mut self, kind: BackendErrorKind, span: Span) {
        self.errors.push(BackendError::new(kind, span));
    }

    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn len(&self) -> usize {
        self.errors.len()
    }

    pub fn into_vec(self) -> Vec<BackendError> {
        self.errors
    }

    pub fn iter(&self) -> impl Iterator<Item = &BackendError> {
        self.errors.iter()
    }
}

impl IntoIterator for BackendErrors {
    type Item = BackendError;
    type IntoIter = std::vec::IntoIter<BackendError>;

    fn into_iter(self) -> Self::IntoIter {
        self.errors.into_iter()
    }
}

impl fmt::Display for BackendErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, err) in self.errors.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{}", err)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let err = BackendError::new(
            BackendErrorKind::RegisterOverflow { max: 65535 },
            Span { start: 0, end: 10 },
        );
        let msg = format!("{}", err);
        assert!(msg.contains("register overflow"));
        assert!(msg.contains("65535"));
    }

    #[test]
    fn error_accumulator() {
        let mut errors = BackendErrors::new();
        assert!(errors.is_empty());

        errors.push_kind(
            BackendErrorKind::UnknownSymbol(SymbolId(42)),
            Span { start: 0, end: 5 },
        );
        errors.push_kind(
            BackendErrorKind::TypeMismatch {
                expected: "i64".to_string(),
                found: "f64".to_string(),
            },
            Span { start: 10, end: 15 },
        );

        assert_eq!(errors.len(), 2);

        let display = format!("{}", errors);
        assert!(display.contains("unknown symbol"));
        assert!(display.contains("type mismatch"));
    }
}
