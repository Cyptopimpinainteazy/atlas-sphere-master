//! X3 MIR Optimizer
//!
//! This crate provides optimization passes for the X3 Mid-level Intermediate Representation.
//! Optimizations operate on `MirModule` and produce transformed, equivalent code with
//! improved performance characteristics (fewer instructions, faster execution, lower gas).
//!
//! # Architecture
//!
//! The optimizer follows a pass-based architecture:
//!
//! ```text
//! MirModule → [Pass 1] → [Pass 2] → ... → [Pass N] → Optimized MirModule
//! ```
//!
//! Each pass implements the `Pass` trait and performs a specific transformation.
//! The `Optimizer` orchestrates passes, running them until a fixpoint is reached
//! or a maximum iteration count is exceeded.
//!
//! # Implemented Passes
//!
//! - **Constant Folding**: Evaluate constant expressions at compile time
//! - **Peephole Optimization**: Local instruction sequence simplifications
//! - **Dead Code Elimination**: Remove unreachable code and unused assignments
//! - **Copy Propagation**: Replace uses of copied values with originals
//! - **Dominator Constant Propagation**: Propagate constants across basic blocks
//!
//! # Control Flow Analysis
//!
//! The `cfg` module provides CFG construction and dominator tree computation,
//! enabling advanced cross-block optimizations like dominator-based constant
//! propagation and dead block elimination.
//!
//! # Example
//!
//! ```ignore
//! use x3_opt::{Optimizer, OptLevel};
//! use x3_mir::MirModule;
//!
//! let mut module: MirModule = /* ... */;
//! let mut optimizer = Optimizer::new(OptLevel::Default);
//! optimizer.run(&mut module)?;
//! ```
//!
//! # Determinism
//!
//! All passes are deterministic: the same input always produces the same output.
//! This is critical for blockchain VMs where execution must be reproducible.

pub mod cfg;
pub mod dce;
pub mod edge_const_prop;
pub mod error;
pub mod licm;
pub mod loop_detection;
pub mod loop_pack_v1;
pub mod loop_unswitching;
pub mod memory_analysis;
pub mod optimizer;
pub mod pass;
pub mod passes;
pub mod peephole_autogen;
pub mod regalloc;
pub mod rule_miner;
pub mod run_yolo;
pub mod ssa_lite;
pub mod strength_reduction;
pub mod superoptimizer;
pub mod telemetry;
pub mod value_numbering;

pub use error::{OptError, OptResult};
pub use optimizer::{default_passes, OptLevel, OptStats, Optimizer, PassObserver};
pub use pass::{Pass, PassResult};
pub use run_yolo::{
    count_instructions, estimate_bytes, run_yolo_once, simulate_gas, OptimizationReport, PassDelta,
};

// Re-export memory analysis helpers
pub use memory_analysis::{
    memory_access_is_hoistable, memory_accesses_can_reorder, rhs_has_aliasing_access,
    rhs_has_observable_effects, rhs_is_memory_access, rhs_is_persistent_load,
    rhs_is_persistent_store, rhs_is_sideeffecting_store,
};

// Re-export CFG types for convenience
pub use cfg::Cfg;

// Re-export passes for convenience
pub use licm::{analyze_invariants, perform_licm, InvariantAnalysis};
pub use loop_detection::{detect_loops, LoopTree};
pub use loop_pack_v1::run_loop_optimizations;
pub use loop_unswitching::{apply_unswitch, find_unswitch_opportunities};

// Phase 6 & Tier A-D: Advanced Optimizations
pub use passes::{
    block_fusion::BlockFusionPass, branch_inversion::BranchInversionPass,
    branch_opt::BranchOptPass, cond_fold::ConditionalFoldPass, constant_fold::ConstantFoldPass,
    copy_propagation::CopyPropagationPass, dead_code_elimination::DeadCodeEliminationPass,
    dom_const_prop::DomConstPropPass, edge_const_prop::EdgeConstPropPass,
    global_const_prop::GlobalConstPropPass, peephole::PeepholePass,
    pre_simple::PartialRedundancyEliminationPass, speculative_hoist::SpeculativeHoistPass,
};
pub use peephole_autogen::{ExecutionTelemetry, PeepholeAutogen, PeepholePattern};
pub use regalloc::ChaitinAllocator;
pub use regalloc::RegAllocator;
pub use rule_miner::RuleMiner;
pub use ssa_lite::global_ssa_opt;
pub use strength_reduction::{analyze_strength_reduction, apply_strength_reduction};
pub use superoptimizer::{Cost, Superoptimizer, SymbolicValue};
