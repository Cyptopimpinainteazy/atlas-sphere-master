//! Semantic analysis errors.

use x3_common::Span;

/// Result type for semantic analysis operations.
pub type SemanticResult<T> = Result<T, SemanticError>;

/// Errors that can occur during semantic analysis.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticError {
    pub kind: SemanticErrorKind,
    pub span: Span,
    pub hint: Option<String>,
}

impl SemanticError {
    pub fn new(kind: SemanticErrorKind, span: Span) -> Self {
        Self {
            kind,
            span,
            hint: None,
        }
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn undefined_variable(name: &str, span: Span) -> Self {
        Self::new(SemanticErrorKind::UndefinedVariable(name.to_string()), span)
    }

    pub fn duplicate_name(name: &str, span: Span, original_span: Span) -> Self {
        Self::new(
            SemanticErrorKind::DuplicateName {
                name: name.to_string(),
                original_span,
            },
            span,
        )
    }

    pub fn invalid_break(span: Span) -> Self {
        Self::new(SemanticErrorKind::InvalidBreak, span)
            .with_hint("break can only be used inside a loop")
    }

    pub fn invalid_continue(span: Span) -> Self {
        Self::new(SemanticErrorKind::InvalidContinue, span)
            .with_hint("continue can only be used inside a loop")
    }

    pub fn invalid_return(span: Span) -> Self {
        Self::new(SemanticErrorKind::InvalidReturn, span)
            .with_hint("return can only be used inside a function")
    }

    pub fn invalid_shadowing(name: &str, span: Span, original_span: Span) -> Self {
        Self::new(
            SemanticErrorKind::InvalidShadowing {
                name: name.to_string(),
                original_span,
            },
            span,
        )
    }

    pub fn assignment_to_immutable(name: &str, span: Span) -> Self {
        Self::new(
            SemanticErrorKind::AssignmentToImmutable(name.to_string()),
            span,
        )
        .with_hint("consider declaring the variable with 'let mut'")
    }

    pub fn undefined_function(name: &str, span: Span) -> Self {
        Self::new(SemanticErrorKind::UndefinedFunction(name.to_string()), span)
    }

    pub fn not_callable(span: Span) -> Self {
        Self::new(SemanticErrorKind::NotCallable, span)
    }
}

impl std::fmt::Display for SemanticError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.kind)?;
        if let Some(hint) = &self.hint {
            write!(f, " (hint: {})", hint)?;
        }
        Ok(())
    }
}

impl std::error::Error for SemanticError {}

/// Kinds of semantic errors.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SemanticErrorKind {
    /// Reference to an undefined variable.
    UndefinedVariable(String),

    /// Reference to an undefined function.
    UndefinedFunction(String),

    /// Duplicate name declaration in the same scope.
    DuplicateName { name: String, original_span: Span },

    /// Invalid shadowing (e.g., shadowing a const with a let in same scope).
    InvalidShadowing { name: String, original_span: Span },

    /// Break statement outside of a loop.
    InvalidBreak,

    /// Continue statement outside of a loop.
    InvalidContinue,

    /// Return statement outside of a function.
    InvalidReturn,

    /// Assignment to an immutable binding.
    AssignmentToImmutable(String),

    /// Attempt to call something that isn't callable.
    NotCallable,

    /// Agent defined inside another agent (if not allowed).
    NestedAgent,

    /// Atomic block inside another atomic block.
    NestedAtomic,
}

impl std::fmt::Display for SemanticErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UndefinedVariable(name) => write!(f, "undefined variable '{}'", name),
            Self::UndefinedFunction(name) => write!(f, "undefined function '{}'", name),
            Self::DuplicateName { name, .. } => {
                write!(f, "duplicate definition of '{}'", name)
            }
            Self::InvalidShadowing { name, .. } => {
                write!(f, "invalid shadowing of '{}'", name)
            }
            Self::InvalidBreak => write!(f, "break outside of loop"),
            Self::InvalidContinue => write!(f, "continue outside of loop"),
            Self::InvalidReturn => write!(f, "return outside of function"),
            Self::AssignmentToImmutable(name) => {
                write!(f, "cannot assign to immutable variable '{}'", name)
            }
            Self::NotCallable => write!(f, "expression is not callable"),
            Self::NestedAgent => write!(f, "agents cannot be nested"),
            Self::NestedAtomic => write!(f, "atomic blocks cannot be nested"),
        }
    }
}
