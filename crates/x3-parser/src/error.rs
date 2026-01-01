//! Parse error types with spans and optional recovery hints.

use std::fmt;
use thiserror::Error;
use x3_common::Span;

/// An error produced during parsing.
#[derive(Debug, Error)]
pub struct ParseError {
    /// Human-readable message describing the issue.
    pub message: String,
    /// Source span where the error occurred.
    pub span: Span,
    /// Optional hint for recovery or correction.
    pub hint: Option<String>,
}

impl ParseError {
    /// Create a new `ParseError` without a hint.
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
            hint: None,
        }
    }

    /// Create a new `ParseError` with a recovery hint.
    pub fn with_hint(message: impl Into<String>, span: Span, hint: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            span,
            hint: Some(hint.into()),
        }
    }

    /// Add a hint to an existing error.
    pub fn add_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} [{}:{}]",
            self.message, self.span.start, self.span.end
        )?;
        if let Some(hint) = &self.hint {
            write!(f, " (hint: {})", hint)?;
        }
        Ok(())
    }
}

/// Result alias used by parsing routines.
pub type ParseResult<T> = Result<T, ParseError>;
