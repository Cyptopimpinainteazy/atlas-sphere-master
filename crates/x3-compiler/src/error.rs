//! Compiler error types

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CompilerError {
    #[error("Lexer error: {0}")]
    Lexer(String),

    #[error("Parser error: {0}")]
    Parser(String),

    #[error("Type check error: {0}")]
    TypeCheck(String),

    #[error("HIR generation error: {0}")]
    HirGeneration(String),

    #[error("MIR lowering error: {0}")]
    MirLowering(String),

    #[error("Optimization error: {0}")]
    Optimization(String),

    #[error("Backend error: {0}")]
    Backend(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Other error: {0}")]
    Other(String),
}

pub type CompilerResult<T> = Result<T, CompilerError>;
