//! High-level Intermediate Representation (HIR) for the X3 language.
//!
//! HIR is the canonical typed representation that serves as the backbone
//! for all major compiler phases:
//! - AI mutation engine (agents modify HIR nodes, not AST or bytecode)
//! - Optimization passes
//! - MIR/bytecode lowering
//! - Cross-VM code generation (EVM/SVM)
//!
//! # Architecture
//!
//! ```text
//! AST → Resolver → TypeChecker → [HIR] → MIR → Bytecode
//!                                  ↑
//!                            AI Mutation Engine
//! ```
//!
//! # Key Properties
//!
//! - **Fully typed**: Every expression carries its resolved type
//! - **Fully desugared**: `for` → `while`, `loop` → `while true`, etc.
//! - **Control-flow explicit**: No implicit fallthrough, explicit break/continue
//! - **Agent-safe**: Agent boundaries and lifecycle explicitly marked
//! - **Atomic-safe**: Atomic blocks use explicit begin/end markers
//! - **Deterministic**: Canonical ordering for reproducible compilation
//!
//! # Usage
//!
//! ```ignore
//! use x3_hir::HirLowerer;
//! use x3_parser::Parser;
//!
//! let mut parser = Parser::from_source(source);
//! let ast = parser.parse_module()?;
//! let hir = HirLowerer::lower(ast)?;
//! ```

pub mod builder;
pub mod error;
pub mod hir;
pub mod lower;

pub use builder::{ExprBuilder, FunctionBuilder, HirBuilder, StmtBuilder};
pub use error::{HirError, HirErrorKind, HirErrors, HirResult};
pub use hir::*;
pub use lower::HirLowerer;
