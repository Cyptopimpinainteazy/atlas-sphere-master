//! X3 Type System and Type Checker
//!
//! This crate implements Semantic Pass 2: type inference and type checking.
//! It builds on the name resolution pass to assign types to all expressions
//! and validate type constraints across the program.
//!
//! # Architecture
//!
//! ```text
//! ResolvedModule → TypeChecker → TypedModule
//!                      ↓
//!                  TypeEnv (types for all symbols)
//!                      ↓
//!                  TypeError (if any violations)
//! ```
//!
//! # Type System Features
//!
//! - **Primitive types**: u8, u16, u32, u64, u256, i8, i32, i64, bool
//! - **Blockchain types**: address (EVM), pubkey (SVM), byte arrays
//! - **High-level types**: agents, functions, arrays, vectors
//! - **Cross-VM types**: evm::asset, svm::account, x3::agent_id
//! - **Meta types**: Any, Never, Unit
//! - **Type inference**: Hindley-Milner style constraint solving
//!
//! # Usage
//!
//! ```ignore
//! use x3_typeck::TypeChecker;
//! use x3_semantics::Resolver;
//! use x3_parser::Parser;
//!
//! let source = "fn add(a: u64, b: u64) -> u64 { return a + b; }";
//! let mut parser = Parser::from_source(source);
//! let module = parser.parse_module().unwrap();
//!
//! let resolver = Resolver::new();
//! let resolved = resolver.resolve(&module).unwrap();
//!
//! let mut checker = TypeChecker::new();
//! let typed = checker.check(&module, &resolved).unwrap();
//! ```

mod checker;
mod env;
mod error;
mod infer;
mod types;

pub use checker::{TypeChecker, TypedModule};
pub use env::{TypeBinding, TypeEnv};
pub use error::{TypeError, TypeErrorKind, TypeResult};
pub use types::{FunctionSignature, PrimitiveType, Type, TypeId, TypeKind};
