//! X3 Backend: HIR → Bytecode Lowering
//!
//! This crate transforms typed HIR into executable X3 bytecode.
//!
//! # Architecture
//!
//! ```text
//! HIR Module
//!     │
//!     ▼
//! ┌─────────────────┐
//! │  lower.rs       │  HIR → instruction stream
//! └────────┬────────┘
//!          │
//!          ▼
//! ┌─────────────────┐
//! │  emit.rs        │  Build bytecode with forward refs
//! └────────┬────────┘
//!          │
//!          ▼
//! ┌─────────────────┐
//! │  layout.rs      │  Compute offsets, patch jumps
//! └────────┬────────┘
//!          │
//!          ▼
//! ┌─────────────────┐
//! │  bc_format.rs   │  Serialize to binary format
//! └─────────────────┘
//! ```
//!
//! # Instruction Set
//!
//! The X3 instruction set (~70 opcodes) is designed for:
//! - **Determinism**: Same bytecode produces identical execution
//! - **Mutation-friendly**: AI agents can compare bytecode diffs
//! - **GPU-parallelizable**: Minimal divergent branches
//! - **Cross-VM portable**: EVM/SVM intrinsics are explicit opcodes
//!
//! # Binary Format
//!
//! ```text
//! ┌────────────────────────────────┐
//! │ Magic: "X3BC" (4 bytes)        │
//! │ Version: u32                   │
//! │ Flags: u32                     │
//! ├────────────────────────────────┤
//! │ Constant Pool                  │
//! │   - Integers, floats, strings  │
//! │   - Function references        │
//! ├────────────────────────────────┤
//! │ Function Table                 │
//! │   - Entry points               │
//! │   - Local counts               │
//! │   - Max stack depth            │
//! ├────────────────────────────────┤
//! │ Instruction Stream             │
//! │   - Opcode + operands          │
//! ├────────────────────────────────┤
//! │ Debug Info (optional)          │
//! │   - Source maps                │
//! │   - Symbol names               │
//! └────────────────────────────────┘
//! ```
//!
//! # Example
//!
//! ```ignore
//! use x3_backend::{BytecodeCompiler, Module};
//!
//! let hir_module = /* ... */;
//! let bytecode = BytecodeCompiler::compile(&hir_module)?;
//!
//! // Serialize to bytes
//! let bytes = bytecode.to_bytes();
//!
//! // Load in VM
//! let module = Module::from_bytes(&bytes)?;
//! ```

pub mod bc_format;
pub mod bc_format_helpers;
pub mod emit;
pub mod error;
pub mod layout;
pub mod lower;
pub mod mir_lower;
pub mod opcode;

pub use bc_format::{
    BytecodeModule, ConstPool, DebugInfo, FeatureFlags, FunctionEntry, GlobalEntry, ModuleFlags,
    ModuleMetadata, SourceMapEntry, VersionInfo, MAGIC, MAX_BYTECODE_SIZE, MAX_CONST_POOL,
    MAX_FUNCTIONS, MAX_STRING_LEN, MIN_SUPPORTED_VERSION, VERSION,
};
pub use bc_format_helpers::{
    assemble_branch_module, assemble_halt_module, assemble_param_module, assemble_simple_module,
};
pub use emit::BytecodeEmitter;
pub use error::{BackendError, BackendErrorKind};
pub use layout::LayoutComputer;
pub use lower::BytecodeCompiler;
pub use mir_lower::MirBytecodeCompiler;
pub use opcode::{Opcode, Register};
