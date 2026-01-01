//! Error types for HIR lowering.
//!
//! Provides structured errors for all phases of AST→HIR transformation,
//! including symbol resolution, type annotation, desugar, and validation.

use std::fmt;
use thiserror::Error;
use x3_common::Span;
use x3_typeck::Type;

/// Result type for HIR operations.
pub type HirResult<T> = Result<T, HirError>;

/// Errors that can occur while building the HIR module.
#[derive(Debug, Error)]
#[error("{kind}")]
pub struct HirError {
    /// The specific kind of error.
    pub kind: HirErrorKind,
    /// Location in source where error occurred.
    pub span: Span,
    /// Optional help message.
    pub help: Option<String>,
}

impl HirError {
    pub fn new(kind: HirErrorKind, span: Span) -> Self {
        Self {
            kind,
            span,
            help: None,
        }
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    // === Convenience constructors ===

    pub fn duplicate_symbol(name: &str, span: Span) -> Self {
        Self::new(HirErrorKind::DuplicateSymbol(name.to_string()), span)
    }

    pub fn unknown_symbol(name: &str, span: Span) -> Self {
        Self::new(HirErrorKind::UnknownSymbol(name.to_string()), span)
    }

    pub fn type_mismatch(expected: Type, found: Type, span: Span) -> Self {
        Self::new(HirErrorKind::TypeMismatch { expected, found }, span)
    }

    pub fn immutable_assign(name: &str, span: Span) -> Self {
        Self::new(HirErrorKind::ImmutableAssign(name.to_string()), span)
    }

    pub fn not_callable(ty: Type, span: Span) -> Self {
        Self::new(HirErrorKind::NotCallable(ty), span)
    }

    pub fn break_outside_loop(span: Span) -> Self {
        Self::new(HirErrorKind::BreakOutsideLoop, span)
    }

    pub fn continue_outside_loop(span: Span) -> Self {
        Self::new(HirErrorKind::ContinueOutsideLoop, span)
    }

    pub fn atomic_nesting(span: Span) -> Self {
        Self::new(HirErrorKind::AtomicNesting, span)
    }

    pub fn not_implemented(feature: &str, span: Span) -> Self {
        Self::new(HirErrorKind::NotImplemented(feature.to_string()), span)
    }
}

/// Specific kinds of HIR errors.
#[derive(Debug, Clone)]
pub enum HirErrorKind {
    // === Symbol Resolution Errors ===
    /// Duplicate symbol declaration.
    DuplicateSymbol(String),
    /// Reference to unknown symbol.
    UnknownSymbol(String),
    /// Symbol not visible in current scope.
    SymbolNotVisible { name: String, defined_in: String },

    // === Type Errors ===
    /// Type mismatch in expression.
    TypeMismatch { expected: Type, found: Type },
    /// Expression is not callable.
    NotCallable(Type),
    /// Wrong number of arguments.
    ArgumentCountMismatch { expected: usize, found: usize },
    /// Cannot infer type.
    CannotInferType,
    /// Invalid type annotation.
    InvalidTypeAnnotation(String),

    // === Assignment Errors ===
    /// Attempt to assign to immutable binding.
    ImmutableAssign(String),
    /// Invalid assignment target.
    InvalidAssignTarget,
    /// Field not found on type.
    FieldNotFound { ty: Type, field: String },

    // === Control Flow Errors ===
    /// Break statement outside of loop.
    BreakOutsideLoop,
    /// Continue statement outside of loop.
    ContinueOutsideLoop,
    /// Invalid loop label reference.
    InvalidLabel(String),
    /// Return with value in void function.
    ReturnWithValueInVoid,
    /// Missing return value.
    MissingReturnValue(Type),

    // === X3-Specific Errors ===
    /// Nested atomic blocks (not allowed).
    AtomicNesting,
    /// Atomic block incomplete (missing end).
    AtomicIncomplete,
    /// Invalid emit in non-agent context.
    EmitOutsideAgent,
    /// Agent initialization error.
    AgentInitError(String),
    /// Invalid context access.
    InvalidContextAccess(String),
    /// VM intrinsic used in wrong context.
    VmIntrinsicMismatch {
        intrinsic: String,
        expected_vm: String,
    },

    // === Structural Errors ===
    /// Duplicate function parameter.
    DuplicateParam(String),
    /// Duplicate agent field.
    DuplicateField(String),
    /// Invalid global initializer.
    InvalidGlobalInit(String),

    // === General ===
    /// Feature not yet implemented.
    NotImplemented(String),
    /// Internal compiler error.
    Internal(String),
}

impl fmt::Display for HirErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Symbol resolution
            HirErrorKind::DuplicateSymbol(name) => {
                write!(f, "duplicate symbol `{}`", name)
            }
            HirErrorKind::UnknownSymbol(name) => {
                write!(f, "unknown symbol `{}`", name)
            }
            HirErrorKind::SymbolNotVisible { name, defined_in } => {
                write!(
                    f,
                    "symbol `{}` is not visible (defined in {})",
                    name, defined_in
                )
            }

            // Type errors
            HirErrorKind::TypeMismatch { expected, found } => {
                write!(
                    f,
                    "type mismatch: expected `{}`, found `{}`",
                    expected, found
                )
            }
            HirErrorKind::NotCallable(ty) => {
                write!(f, "type `{}` is not callable", ty)
            }
            HirErrorKind::ArgumentCountMismatch { expected, found } => {
                write!(
                    f,
                    "wrong number of arguments: expected {}, found {}",
                    expected, found
                )
            }
            HirErrorKind::CannotInferType => {
                write!(f, "cannot infer type")
            }
            HirErrorKind::InvalidTypeAnnotation(msg) => {
                write!(f, "invalid type annotation: {}", msg)
            }

            // Assignment errors
            HirErrorKind::ImmutableAssign(name) => {
                write!(f, "cannot assign to immutable binding `{}`", name)
            }
            HirErrorKind::InvalidAssignTarget => {
                write!(f, "invalid assignment target")
            }
            HirErrorKind::FieldNotFound { ty, field } => {
                write!(f, "field `{}` not found on type `{}`", field, ty)
            }

            // Control flow
            HirErrorKind::BreakOutsideLoop => {
                write!(f, "`break` outside of loop")
            }
            HirErrorKind::ContinueOutsideLoop => {
                write!(f, "`continue` outside of loop")
            }
            HirErrorKind::InvalidLabel(label) => {
                write!(f, "invalid loop label `{}`", label)
            }
            HirErrorKind::ReturnWithValueInVoid => {
                write!(f, "return with value in function returning ()")
            }
            HirErrorKind::MissingReturnValue(ty) => {
                write!(f, "missing return value of type `{}`", ty)
            }

            // X3-specific
            HirErrorKind::AtomicNesting => {
                write!(f, "atomic blocks cannot be nested")
            }
            HirErrorKind::AtomicIncomplete => {
                write!(f, "atomic block is incomplete")
            }
            HirErrorKind::EmitOutsideAgent => {
                write!(f, "emit statement outside of agent context")
            }
            HirErrorKind::AgentInitError(msg) => {
                write!(f, "agent initialization error: {}", msg)
            }
            HirErrorKind::InvalidContextAccess(field) => {
                write!(f, "invalid context access: `{}`", field)
            }
            HirErrorKind::VmIntrinsicMismatch {
                intrinsic,
                expected_vm,
            } => {
                write!(
                    f,
                    "VM intrinsic `{}` requires {} target",
                    intrinsic, expected_vm
                )
            }

            // Structural
            HirErrorKind::DuplicateParam(name) => {
                write!(f, "duplicate parameter `{}`", name)
            }
            HirErrorKind::DuplicateField(name) => {
                write!(f, "duplicate field `{}`", name)
            }
            HirErrorKind::InvalidGlobalInit(msg) => {
                write!(f, "invalid global initializer: {}", msg)
            }

            // General
            HirErrorKind::NotImplemented(feature) => {
                write!(f, "{} not yet implemented", feature)
            }
            HirErrorKind::Internal(msg) => {
                write!(f, "internal error: {}", msg)
            }
        }
    }
}

/// Collection of HIR errors with utilities.
#[derive(Debug, Default)]
pub struct HirErrors {
    errors: Vec<HirError>,
}

impl HirErrors {
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    pub fn push(&mut self, error: HirError) {
        self.errors.push(error);
    }

    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn len(&self) -> usize {
        self.errors.len()
    }

    pub fn into_result<T>(self, value: T) -> HirResult<T> {
        if self.is_empty() {
            Ok(value)
        } else {
            Err(self.errors.into_iter().next().unwrap())
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &HirError> {
        self.errors.iter()
    }
}

impl IntoIterator for HirErrors {
    type Item = HirError;
    type IntoIter = std::vec::IntoIter<HirError>;

    fn into_iter(self) -> Self::IntoIter {
        self.errors.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let err = HirError::duplicate_symbol("foo", Span::new(0, 3));
        assert!(err.to_string().contains("duplicate symbol `foo`"));

        let err = HirError::type_mismatch(Type::u64(), Type::bool(), Span::new(0, 1));
        assert!(err.to_string().contains("type mismatch"));
    }
}
