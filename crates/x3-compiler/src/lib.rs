//! X3 Full Compiler Pipeline with YOLO + Loop-Pack v1 Optimization
//!
//! Complete compilation pipeline from source code to optimized bytecode:
//! ```text
//! X3 Source Code
//!     │
//!     ▼
//! ┌──────────────────────┐
//! │ Lexer                │ (x3-lexer)
//! └──────────┬───────────┘
//!            │
//!            ▼
//! ┌──────────────────────┐
//! │ Parser               │ (x3-parser)
//! └──────────┬───────────┘
//!            │
//!            ▼
//! ┌──────────────────────┐
//! │ Type Checker         │ (x3-typeck)
//! └──────────┬───────────┘
//!            │
//!            ▼
//! ┌──────────────────────┐
//! │ HIR Generation       │ (x3-hir)
//! └──────────┬───────────┘
//!            │
//!            ▼
//! ┌──────────────────────┐
//! │ MIR Lowering         │ (x3-mir)
//! └──────────┬───────────┘
//!            │
//!            ▼
//! ┌──────────────────────────────┐
//! │ YOLO OPTIMIZATION ⭐ OPTIONAL │ (x3-opt: 14-pass pipeline)
//! │ - 13 YOLO passes             │
//! │ - Loop-Pack v1 (4 techs)     │
//! │ Configurable: O0/O1/O2/O3    │
//! └──────────┬────────────────────┘
//!            │
//!            ▼
//! ┌──────────────────────────────┐
//! │ CONTRACT ANALYSIS (optional) │ (x3-verifier)
//! │ - Gas estimation             │
//! │ - Safety verification        │
//! └──────────┬────────────────────┘
//!            │
//!            ▼
//! ┌──────────────────────┐
//! │ Bytecode Generation  │ (x3-backend)
//! └──────────────────────┘
//! ```
//!
//! # Example: Compile with Optimization
//!
//! ```ignore
//! use x3_compiler::{CompilationOptions, OptLevel, Compiler};
//!
//! let source = "func main() { ... }";
//!
//! let options = CompilationOptions {
//!     opt_level: OptLevel::Default,  // O2: YOLO + Loop-Pack v1
//!     debug: false,
//!     verbose: true,
//! };
//!
//! let bytecode = Compiler::compile(source, options)?;
//! ```
//!
//! # Example: Contract Mode with Gas Analysis
//!
//! ```ignore
//! use x3_compiler::{CompilationOptions, Compiler};
//!
//! let source = "contract Token { ... }";
//!
//! let options = CompilationOptions::contract_mode()
//!     .with_verbose(true);
//!
//! let output = Compiler::compile(source, options)?;
//!
//! if let Some(artifacts) = output.artifacts {
//!     if let Some(gas) = artifacts.gas_report {
//!         println!("Estimated gas: {}", gas.total_gas);
//!     }
//! }
//! ```

pub mod compiler;
pub mod error;
pub mod options;

pub use compiler::{CompilationArtifacts, CompilationOutput, Compiler};
pub use error::{CompilerError, CompilerResult};
pub use options::{CompilationOptions, OptLevel};
pub use x3_opt::OptLevel as XOptLevel;

// Re-export verifier types for gas analysis
pub use x3_verifier::{FunctionGas, GasReport, SafetyRules, VerificationReport};
