//! X3 Semantic Analysis - Name Resolution and Scope Graph
//!
//! This crate provides the first semantic pass over the X3 AST:
//! - Symbol table construction
//! - Scope management with proper lifetime boundaries
//! - Name resolution (binding identifiers to definitions)
//! - Shadowing validation
//! - Context validation (break/continue/return in correct scopes)
//!
//! # Architecture
//!
//! ```text
//! AST → Resolver → ResolvedModule
//!         ↓
//!    SymbolTable (scopes + symbols)
//!         ↓
//!    SemanticErrors (if any)
//! ```
//!
//! # Usage
//!
//! ```ignore
//! use x3_semantics::Resolver;
//! use x3_parser::Parser;
//!
//! let source = "fn main() { let x = 1; return x; }";
//! let mut parser = Parser::from_source(source);
//! let module = parser.parse_module().unwrap();
//!
//! let mut resolver = Resolver::new();
//! let result = resolver.resolve(&module);
//! ```

mod error;
mod resolver;
mod scope;
mod symbol;

pub use error::{SemanticError, SemanticErrorKind, SemanticResult};
pub use resolver::{ResolvedModule, Resolver};
pub use scope::{Scope, ScopeId, ScopeKind};
pub use symbol::{Symbol, SymbolId, SymbolKind, SymbolTable};
