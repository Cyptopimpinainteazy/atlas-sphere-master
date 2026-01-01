//! Type errors for the X3 type checker.

use std::fmt;
use thiserror::Error;
use x3_common::Span;

use crate::types::Type;

/// Result type for type checking operations.
pub type TypeResult<T> = Result<T, Vec<TypeError>>;

/// A type error with location and diagnostic information.
#[derive(Clone, Debug)]
pub struct TypeError {
    /// The kind of type error.
    pub kind: TypeErrorKind,
    /// Location where the error occurred.
    pub span: Span,
    /// Optional hint for fixing the error.
    pub hint: Option<String>,
    /// Optional note with additional context.
    pub note: Option<String>,
}

impl TypeError {
    pub fn new(kind: TypeErrorKind, span: Span) -> Self {
        Self {
            kind,
            span,
            hint: None,
            note: None,
        }
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    // === Convenience constructors ===

    pub fn unknown_type(name: &str, span: Span) -> Self {
        Self::new(TypeErrorKind::UnknownType(name.to_string()), span)
    }

    pub fn type_mismatch(expected: Type, found: Type, span: Span) -> Self {
        Self::new(TypeErrorKind::TypeMismatch { expected, found }, span)
    }

    pub fn invalid_operation(op: &str, ty: Type, span: Span) -> Self {
        Self::new(
            TypeErrorKind::InvalidOperation {
                operation: op.to_string(),
                operand_type: ty,
            },
            span,
        )
    }

    pub fn invalid_binary_op(op: &str, left: Type, right: Type, span: Span) -> Self {
        Self::new(
            TypeErrorKind::InvalidBinaryOperation {
                operation: op.to_string(),
                left_type: left,
                right_type: right,
            },
            span,
        )
    }

    pub fn return_type_mismatch(expected: Type, found: Type, span: Span) -> Self {
        Self::new(TypeErrorKind::ReturnTypeMismatch { expected, found }, span)
    }

    pub fn missing_return(expected: Type, span: Span) -> Self {
        Self::new(TypeErrorKind::MissingReturn { expected }, span)
    }

    pub fn not_callable(ty: Type, span: Span) -> Self {
        Self::new(TypeErrorKind::NotCallable(ty), span)
    }

    pub fn wrong_argument_count(expected: usize, found: usize, span: Span) -> Self {
        Self::new(TypeErrorKind::WrongArgumentCount { expected, found }, span)
    }

    pub fn argument_type_mismatch(
        param_index: usize,
        expected: Type,
        found: Type,
        span: Span,
    ) -> Self {
        Self::new(
            TypeErrorKind::ArgumentTypeMismatch {
                param_index,
                expected,
                found,
            },
            span,
        )
    }

    pub fn not_indexable(ty: Type, span: Span) -> Self {
        Self::new(TypeErrorKind::NotIndexable(ty), span)
    }

    pub fn invalid_index_type(expected: Type, found: Type, span: Span) -> Self {
        Self::new(TypeErrorKind::InvalidIndexType { expected, found }, span)
    }

    pub fn no_field(ty: Type, field: &str, span: Span) -> Self {
        Self::new(
            TypeErrorKind::NoSuchField {
                type_name: ty,
                field: field.to_string(),
            },
            span,
        )
    }

    pub fn no_method(ty: Type, method: &str, span: Span) -> Self {
        Self::new(
            TypeErrorKind::NoSuchMethod {
                type_name: ty,
                method: method.to_string(),
            },
            span,
        )
    }

    pub fn condition_not_bool(found: Type, span: Span) -> Self {
        Self::new(TypeErrorKind::ConditionNotBool(found), span)
    }

    pub fn cross_vm_type_error(msg: &str, span: Span) -> Self {
        Self::new(TypeErrorKind::CrossVmTypeError(msg.to_string()), span)
    }

    pub fn atomic_type_violation(msg: &str, span: Span) -> Self {
        Self::new(TypeErrorKind::AtomicTypeViolation(msg.to_string()), span)
    }

    pub fn cannot_infer(span: Span) -> Self {
        Self::new(TypeErrorKind::CannotInfer, span)
            .with_hint("consider adding an explicit type annotation")
    }

    pub fn recursive_type(name: &str, span: Span) -> Self {
        Self::new(TypeErrorKind::RecursiveType(name.to_string()), span)
    }

    pub fn incompatible_branch_types(then_ty: Type, else_ty: Type, span: Span) -> Self {
        Self::new(
            TypeErrorKind::IncompatibleBranchTypes {
                then_type: then_ty,
                else_type: else_ty,
            },
            span,
        )
    }
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind)?;
        if let Some(hint) = &self.hint {
            write!(f, "\n  hint: {}", hint)?;
        }
        if let Some(note) = &self.note {
            write!(f, "\n  note: {}", note)?;
        }
        Ok(())
    }
}

impl std::error::Error for TypeError {}

/// The kind of type error.
#[derive(Clone, Debug, Error)]
pub enum TypeErrorKind {
    /// Reference to an unknown type name.
    #[error("unknown type `{0}`")]
    UnknownType(String),

    /// Type mismatch between expected and found types.
    #[error("type mismatch: expected `{expected}`, found `{found}`")]
    TypeMismatch { expected: Type, found: Type },

    /// Invalid unary operation for the operand type.
    #[error("invalid operation `{operation}` for type `{operand_type}`")]
    InvalidOperation {
        operation: String,
        operand_type: Type,
    },

    /// Invalid binary operation for the operand types.
    #[error("invalid operation `{operation}` between `{left_type}` and `{right_type}`")]
    InvalidBinaryOperation {
        operation: String,
        left_type: Type,
        right_type: Type,
    },

    /// Return type doesn't match function signature.
    #[error("return type mismatch: expected `{expected}`, found `{found}`")]
    ReturnTypeMismatch { expected: Type, found: Type },

    /// Function is missing a return statement.
    #[error("missing return statement, expected type `{expected}`")]
    MissingReturn { expected: Type },

    /// Attempting to call a non-function type.
    #[error("type `{0}` is not callable")]
    NotCallable(Type),

    /// Wrong number of arguments in function call.
    #[error("expected {expected} argument(s), found {found}")]
    WrongArgumentCount { expected: usize, found: usize },

    /// Argument type doesn't match parameter type.
    #[error("argument {param_index} type mismatch: expected `{expected}`, found `{found}`")]
    ArgumentTypeMismatch {
        param_index: usize,
        expected: Type,
        found: Type,
    },

    /// Attempting to index a non-indexable type.
    #[error("type `{0}` cannot be indexed")]
    NotIndexable(Type),

    /// Index has wrong type.
    #[error("invalid index type: expected `{expected}`, found `{found}`")]
    InvalidIndexType { expected: Type, found: Type },

    /// Field doesn't exist on type.
    #[error("type `{type_name}` has no field `{field}`")]
    NoSuchField { type_name: Type, field: String },

    /// Method doesn't exist on type.
    #[error("type `{type_name}` has no method `{method}`")]
    NoSuchMethod { type_name: Type, method: String },

    /// Condition expression is not boolean.
    #[error("condition must be `bool`, found `{0}`")]
    ConditionNotBool(Type),

    /// Cross-VM type constraint violation.
    #[error("cross-VM type error: {0}")]
    CrossVmTypeError(String),

    /// Atomic block type constraint violation.
    #[error("atomic type violation: {0}")]
    AtomicTypeViolation(String),

    /// Type cannot be inferred.
    #[error("cannot infer type")]
    CannotInfer,

    /// Recursive type definition detected.
    #[error("recursive type `{0}` has infinite size")]
    RecursiveType(String),

    /// If/else branches have incompatible types.
    #[error("incompatible types in if/else branches: `{then_type}` vs `{else_type}`")]
    IncompatibleBranchTypes { then_type: Type, else_type: Type },

    /// Integer literal too large for target type.
    #[error("integer literal `{value}` is out of range for type `{target_type}`")]
    IntegerOverflow { value: String, target_type: Type },

    /// Cannot unify two types during inference.
    #[error("cannot unify types `{type1}` and `{type2}`")]
    UnificationFailure { type1: Type, type2: Type },

    /// Assignment to incompatible type.
    #[error("cannot assign `{found}` to variable of type `{expected}`")]
    AssignmentTypeMismatch { expected: Type, found: Type },

    /// Array element types don't match.
    #[error("array elements have inconsistent types")]
    InconsistentArrayElements,

    /// Range bounds have wrong types.
    #[error("range bounds must be integers")]
    InvalidRangeBounds,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = TypeError::type_mismatch(Type::u64(), Type::bool(), Span::new(0, 10));
        assert!(err.to_string().contains("type mismatch"));
        assert!(err.to_string().contains("u64"));
        assert!(err.to_string().contains("bool"));
    }

    #[test]
    fn test_error_with_hint() {
        let err = TypeError::cannot_infer(Span::new(0, 10));
        assert!(err.hint.is_some());
        assert!(err.to_string().contains("type annotation"));
    }
}
