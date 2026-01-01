//! Optimization passes.
//!
//! Each submodule implements a specific optimization transformation.

pub mod block_fusion;
pub mod branch_inversion;
pub mod branch_opt;
pub mod cond_fold;
pub mod constant_fold;
pub mod copy_propagation;
pub mod dead_code_elimination;
pub mod dom_const_prop;
pub mod edge_const_prop;
pub mod expression_hoist;
pub mod global_const_prop;
pub mod peephole;
pub mod pre;
pub mod pre_morel;
pub mod pre_simple;
pub mod speculative_hoist;
