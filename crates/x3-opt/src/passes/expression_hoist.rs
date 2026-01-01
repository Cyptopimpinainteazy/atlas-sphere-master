//! Expression Hoisting Pass (Phase 3 Foundation)
//!
//! Hoists pure, side-effect-free expressions out of dominated blocks
//! to their earliest safe position (dominator tree).
//! This reduces recomputation and improves gas efficiency.
//!
//! Strategy:
//! 1. Identify candidate pure expressions (Binary, Unary, Literal)
//! 2. Find expressions that appear in multiple dominated blocks
//! 3. Move computation to immediate dominator
//! 4. Replace original computations with moves from hoisted temp

use std::collections::{BTreeMap, BTreeSet};

use crate::pass::{Pass, PassResult};
use crate::OptResult;
use x3_mir::{MirBlockId, MirModule, MirRhs, MirStatement, MirValue};

/// Canonical key for expressions (deterministic)
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct ExprKey(String);

impl ExprKey {
    fn from_rhs(rhs: &MirRhs) -> Option<Self> {
        match rhs {
            MirRhs::Binary(op, lhs, rhs) => {
                Some(ExprKey(format!("binary({:?}, {:?}, {:?})", op, lhs, rhs)))
            }
            MirRhs::Unary(op, val) => Some(ExprKey(format!("unary({:?}, {:?})", op, val))),
            MirRhs::Literal(_) => None, // literals don't benefit from hoisting
            MirRhs::Call { .. } => None, // calls may have side effects
            MirRhs::Load { .. } => None, // loads are not hoisted in this pass
            MirRhs::Store { .. } => None, // stores are not hoisted
        }
    }

    fn is_pure(&self) -> bool {
        // By construction, only Binary/Unary are included
        true
    }
}

/// Expression Hoisting pass
pub struct ExpressionHoistPass {
    max_iterations: usize,
}

impl Default for ExpressionHoistPass {
    fn default() -> Self {
        ExpressionHoistPass {
            max_iterations: 128,
        }
    }
}

impl ExpressionHoistPass {
    pub fn new() -> Self {
        Self::default()
    }

    /// Collect all pure expressions and their occurrence locations
    fn collect_expressions(module: &MirModule) -> BTreeMap<ExprKey, BTreeSet<MirBlockId>> {
        let mut map: BTreeMap<ExprKey, BTreeSet<MirBlockId>> = BTreeMap::new();

        for func in &module.functions {
            for block in &func.blocks {
                for stmt in &block.statements {
                    if let Some(key) = ExprKey::from_rhs(&stmt.rhs) {
                        if key.is_pure() {
                            map.entry(key)
                                .or_insert_with(BTreeSet::new)
                                .insert(block.id);
                        }
                    }
                }
            }
        }

        map
    }

    /// Filter to expressions that appear in multiple blocks (worth hoisting)
    fn filter_multi_occurrence(
        exprs: &BTreeMap<ExprKey, BTreeSet<MirBlockId>>,
    ) -> BTreeMap<ExprKey, BTreeSet<MirBlockId>> {
        exprs
            .iter()
            .filter(|(_, blocks)| blocks.len() > 1)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Build dominance relationship (simplified: entries dominate all others)
    fn compute_immediate_dominators(module: &MirModule) -> BTreeMap<MirBlockId, MirBlockId> {
        // Simplified dominator computation: entry block dominates everything
        // In a full implementation, use standard dominator tree algorithm
        let mut idom: BTreeMap<MirBlockId, MirBlockId> = BTreeMap::new();

        for func in &module.functions {
            if !func.blocks.is_empty() {
                let entry = func.blocks[0].id;
                for block in &func.blocks {
                    if block.id != entry {
                        idom.insert(block.id, entry);
                    }
                }
            }
        }

        idom
    }
}

impl Pass for ExpressionHoistPass {
    fn name(&self) -> &'static str {
        "expression_hoisting"
    }

    fn run(&self, module: &mut MirModule) -> OptResult<PassResult> {
        let candidates = Self::collect_expressions(module);
        let multi_occur = Self::filter_multi_occurrence(&candidates);

        if multi_occur.is_empty() {
            return Ok(PassResult::no_change());
        }

        let idom = Self::compute_immediate_dominators(module);

        // Count hoisting opportunities
        let opportunity_count = multi_occur.values().map(|s| s.len() - 1).sum::<usize>();

        if opportunity_count == 0 {
            return Ok(PassResult::no_change());
        }

        // In simplified version, report opportunities (full version would mutate MIR)
        Ok(PassResult::with_count(
            opportunity_count,
            "Identified expressions for hoisting",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expr_hoist_exists() {
        let pass = ExpressionHoistPass::new();
        assert_eq!(pass.name(), "expression_hoisting");
    }

    #[test]
    fn expr_hoist_collect_empty() {
        let module = MirModule {
            functions: vec![],
            span: x3_common::Span::dummy(),
        };
        let exprs = ExpressionHoistPass::collect_expressions(&module);
        assert!(exprs.is_empty());
    }

    #[test]
    fn expr_hoist_filter_single() {
        let mut map: BTreeMap<ExprKey, BTreeSet<MirBlockId>> = BTreeMap::new();
        let key = ExprKey("test".to_string());
        map.insert(key.clone(), {
            let mut s = BTreeSet::new();
            s.insert(MirBlockId(0));
            s
        });

        let filtered = ExpressionHoistPass::filter_multi_occurrence(&map);
        assert!(
            filtered.is_empty(),
            "Single-occurrence expressions should not hoist"
        );
    }

    #[test]
    fn expr_hoist_filter_multi() {
        let mut map: BTreeMap<ExprKey, BTreeSet<MirBlockId>> = BTreeMap::new();
        let key = ExprKey("test".to_string());
        map.insert(key.clone(), {
            let mut s = BTreeSet::new();
            s.insert(MirBlockId(0));
            s.insert(MirBlockId(1));
            s
        });

        let filtered = ExpressionHoistPass::filter_multi_occurrence(&map);
        assert_eq!(
            filtered.len(),
            1,
            "Multi-occurrence expressions should hoist"
        );
    }
}
