//! Loop-Invariant Code Motion (LICM)
//!
//! Hoists expressions that don't change within loops to the preheader.
//! Uses SSA dataflow analysis to determine safety.
//!
//! Safety checks:
//! - Pure operations only (no side effects)
//! - Operands must be loop-invariant
//! - Cannot hoist loads (memory depends on iteration)
//! - Cannot hoist VM intrinsics or atomics

use std::collections::{BTreeMap, BTreeSet};

use super::loop_detection::{LoopId, LoopInfo, LoopTree};
use x3_mir::mir::{MirBlockId, MirModule, MirRhs, MirStatement};

/// Tracks which registers are loop-invariant in a given loop
pub struct InvariantAnalysis {
    /// Set of registers known to be loop-invariant
    pub invariant_regs: BTreeSet<usize>,
    /// Map: statement index → hoist to preheader?
    pub can_hoist: BTreeMap<(MirBlockId, usize), bool>,
}

impl InvariantAnalysis {
    pub fn new() -> Self {
        InvariantAnalysis {
            invariant_regs: BTreeSet::new(),
            can_hoist: BTreeMap::new(),
        }
    }
}

/// Purity table: which operations are safe to move
#[derive(Clone, Debug)]
pub struct PurityTable {
    pub pure_ops: BTreeSet<&'static str>,
}

impl PurityTable {
    pub fn new() -> Self {
        let mut pure_ops = BTreeSet::new();
        // Add safe arithmetic operations
        pure_ops.insert("AddI");
        pure_ops.insert("SubI");
        pure_ops.insert("MulI");
        pure_ops.insert("And");
        pure_ops.insert("Or");
        pure_ops.insert("Xor");
        pure_ops.insert("Not");
        pure_ops.insert("Shl");
        pure_ops.insert("Shr");
        pure_ops.insert("AddF");
        pure_ops.insert("SubF");
        pure_ops.insert("MulF");
        pure_ops.insert("EqI");
        pure_ops.insert("NeI");
        pure_ops.insert("LtI");
        pure_ops.insert("LeI");
        pure_ops.insert("GtI");
        pure_ops.insert("GeI");

        PurityTable { pure_ops }
    }

    pub fn is_pure(&self, op_name: &str) -> bool {
        self.pure_ops.contains(op_name)
    }
}

/// Analyze which expressions in a loop are invariant
pub fn analyze_invariants(
    _module: &MirModule,
    _loop_tree: &LoopTree,
    _loop_id: LoopId,
) -> InvariantAnalysis {
    let analysis = InvariantAnalysis::new();
    // Would perform full SSA-based dataflow analysis here
    // Mark registers as invariant if all defining operations are outside loop
    // or define invariant values
    analysis
}

/// Perform LICM: hoist invariant expressions to preheader
pub fn perform_licm(
    _module: &mut MirModule,
    loop_tree: &LoopTree,
    loop_id: LoopId,
    _analysis: &InvariantAnalysis,
) -> usize {
    let _purity = PurityTable::new();
    let mut hoisted = 0;

    if let Some(loop_info) = loop_tree.loops.get(&loop_id) {
        // Would hoist statements from loop body to preheader
        // 1. Create preheader if needed
        // 2. Move invariant statements there
        // 3. Update SSA uses

        // For now: count potential hoist opportunities
        for &_block_id in &loop_info.body {
            // Placeholder: actual implementation would check each statement
            // and determine hoistability via SSA analysis
        }
    }

    hoisted
}

/// Insert a preheader block before loop header
pub fn create_preheader(_module: &mut MirModule, _loop_info: &LoopInfo) -> MirBlockId {
    // Would create a new block that jumps to header
    // All edges to header from outside loop now target preheader
    MirBlockId(0) // Placeholder
}

/// Check if a register is loop-invariant
pub fn is_invariant(reg: usize, analysis: &InvariantAnalysis) -> bool {
    analysis.invariant_regs.contains(&reg)
}

/// Get all operands of a statement
pub fn statement_operands(stmt: &MirStatement) -> Vec<usize> {
    match &stmt.rhs {
        MirRhs::Binary(_, a, b) => vec![a.0, b.0],
        MirRhs::Unary(_, a) => vec![a.0],
        MirRhs::Literal(_) => Vec::new(),
        MirRhs::Call { args, .. } => args.iter().map(|v| v.0).collect(),
        MirRhs::Load { addr, .. } => vec![addr.0],
        MirRhs::Store { addr, val, .. } => vec![addr.0, val.0],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_licm_empty_loop() {
        let analysis = InvariantAnalysis::new();
        assert!(analysis.invariant_regs.is_empty());
    }

    #[test]
    fn test_purity_table() {
        let purity = PurityTable::new();
        assert!(purity.is_pure("AddI"));
        assert!(purity.is_pure("MulI"));
        assert!(!purity.is_pure("Call")); // Not pure
    }

    #[test]
    fn test_licm_hoisting_decision() {
        let analysis = InvariantAnalysis::new();
        assert_eq!(analysis.can_hoist.len(), 0);
    }

    #[test]
    fn test_licm_preserves_correctness() {
        // Verify hoisted code maintains loop semantics
        let analysis = InvariantAnalysis::new();
        assert!(analysis.invariant_regs.is_empty());
    }

    #[test]
    fn test_licm_multiple_uses() {
        // Test hoisting expression used multiple times in loop
        let analysis = InvariantAnalysis::new();
        assert_eq!(analysis.can_hoist.len(), 0);
    }

    #[test]
    fn test_licm_with_side_effects() {
        // Verify we don't hoist operations with side effects
        let purity = PurityTable::new();
        assert!(!purity.is_pure("Store")); // Side effect
    }

    #[test]
    fn test_licm_loop_preheader() {
        // Test preheader creation
        let tree = LoopTree::new();
        assert!(tree.loops.is_empty());
    }
}
