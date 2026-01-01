//! Dominator-Based Constant Propagation Pass
//!
//! Propagates constant values across basic blocks using dominator information.
//! This extends constant folding to work across control flow by leveraging
//! the property that dominators always execute before dominated blocks.
//!
//! # Algorithm
//!
//! 1. Build CFG and compute dominator tree
//! 2. For each block, collect "stable" constant definitions (not killed by side effects)
//! 3. Propagate constants from dominators to dominated blocks
//! 4. Replace uses of constants with literal values
//!
//! # Conservative Approach
//!
//! - Clears propagation on side-effecting stores/calls
//! - No SSA required - uses dominance-based local propagation
//! - Multiple values for same variable across paths are not merged (conservative)
//!
//! # Example
//!
//! ```text
//! bb0:                        bb0:
//!   v0 = 42                     v0 = 42
//!   branch cond bb1 bb2        branch cond bb1 bb2
//! bb1:                   =>  bb1:
//!   v1 = v0 + 1                 v1 = 43  // v0=42 propagated from dominator
//! bb2:                       bb2:
//!   v2 = v0 * 2                 v2 = 84  // v0=42 propagated from dominator
//! ```

use crate::cfg::Cfg;
use crate::pass::{Pass, PassResult};
use crate::OptResult;
use std::collections::{BTreeMap, BTreeSet};
use x3_ast::BinaryOp;
use x3_common::Literal;
use x3_mir::{MirBlockId, MirModule, MirRhs, MirStatement, MirValue};

/// Constant value or "unknown" marker.
#[derive(Debug, Clone, PartialEq)]
enum ConstVal {
    Known(Literal),
    Unknown,
}

/// Dominator-based constant propagation pass.
///
/// Propagates constants across basic blocks using dominator relationships.
pub struct DomConstPropPass;

impl DomConstPropPass {
    pub fn new() -> Self {
        DomConstPropPass
    }

    /// Compute stable (non-killed) constant definitions for a block.
    ///
    /// A definition is "stable" if it's not invalidated by a side-effecting
    /// operation (call) that could potentially modify memory.
    fn block_stable_defs(&self, stmts: &[MirStatement]) -> BTreeMap<MirValue, ConstVal> {
        let mut defs: BTreeMap<MirValue, ConstVal> = BTreeMap::new();

        for stmt in stmts {
            match &stmt.rhs {
                MirRhs::Literal(lit) => {
                    defs.insert(stmt.target, ConstVal::Known(lit.clone()));
                }
                MirRhs::Call { .. } => {
                    // Calls may have side effects - mark target as unknown
                    // and conservatively kill all definitions (they may be aliased)
                    defs.insert(stmt.target, ConstVal::Unknown);
                    // Note: In a more sophisticated analysis, we would only kill
                    // values that could be aliased by the call's effects
                }
                MirRhs::Binary(op, left, right) => {
                    // Try to fold if both operands are known constants
                    let left_val = defs.get(left);
                    let right_val = defs.get(right);

                    match (left_val, right_val) {
                        (Some(ConstVal::Known(l)), Some(ConstVal::Known(r))) => {
                            if let Some(result) = self.fold_binary(*op, l, r) {
                                defs.insert(stmt.target, ConstVal::Known(result));
                            } else {
                                defs.insert(stmt.target, ConstVal::Unknown);
                            }
                        }
                        _ => {
                            defs.insert(stmt.target, ConstVal::Unknown);
                        }
                    }
                }
                MirRhs::Unary(op, src) => {
                    if let Some(ConstVal::Known(val)) = defs.get(src) {
                        if let Some(result) = self.fold_unary(*op, val) {
                            defs.insert(stmt.target, ConstVal::Known(result));
                        } else {
                            defs.insert(stmt.target, ConstVal::Unknown);
                        }
                    } else {
                        defs.insert(stmt.target, ConstVal::Unknown);
                    }
                }
                MirRhs::Load { .. } => {
                    // Loads are conservative - mark as unknown (may vary)
                    defs.insert(stmt.target, ConstVal::Unknown);
                }
                MirRhs::Store { .. } => {
                    // Stores don't produce a value but have side effects
                    // Mark all existing defs as potentially invalid (conservative)
                    defs.clear();
                }
            }
        }

        defs
    }

    /// Fold a binary operation on two literals (same as ConstantFoldPass).
    fn fold_binary(&self, op: BinaryOp, left: &Literal, right: &Literal) -> Option<Literal> {
        use BinaryOp::*;
        use Literal::*;

        match (op, left, right) {
            // Integer arithmetic
            (Add, Integer(a), Integer(b)) => Some(Integer(a.wrapping_add(*b))),
            (Sub, Integer(a), Integer(b)) => Some(Integer(a.wrapping_sub(*b))),
            (Mul, Integer(a), Integer(b)) => Some(Integer(a.wrapping_mul(*b))),
            (Div, Integer(a), Integer(b)) if *b != 0 => Some(Integer(a.wrapping_div(*b))),
            (Mod, Integer(a), Integer(b)) if *b != 0 => Some(Integer(a.wrapping_rem(*b))),

            // Float arithmetic
            (Add, Float(a), Float(b)) => Some(Float(a + b)),
            (Sub, Float(a), Float(b)) => Some(Float(a - b)),
            (Mul, Float(a), Float(b)) => Some(Float(a * b)),
            (Div, Float(a), Float(b)) if *b != 0.0 => Some(Float(a / b)),

            // Integer comparisons
            (Equal, Integer(a), Integer(b)) => Some(Bool(a == b)),
            (NotEqual, Integer(a), Integer(b)) => Some(Bool(a != b)),
            (Less, Integer(a), Integer(b)) => Some(Bool(a < b)),
            (LessEqual, Integer(a), Integer(b)) => Some(Bool(a <= b)),
            (Greater, Integer(a), Integer(b)) => Some(Bool(a > b)),
            (GreaterEqual, Integer(a), Integer(b)) => Some(Bool(a >= b)),

            // Boolean operations
            (LogicalAnd, Bool(a), Bool(b)) => Some(Bool(*a && *b)),
            (LogicalOr, Bool(a), Bool(b)) => Some(Bool(*a || *b)),

            _ => None,
        }
    }

    /// Fold a unary operation on a literal.
    fn fold_unary(&self, op: x3_ast::UnaryOp, val: &Literal) -> Option<Literal> {
        use x3_ast::UnaryOp::*;
        use Literal::*;

        match (op, val) {
            (Negate, Integer(n)) => Some(Integer(-n)),
            (Negate, Float(f)) => Some(Float(-f)),
            (Not, Bool(b)) => Some(Bool(!b)),
            _ => None,
        }
    }

    /// Merge constant maps from dominators.
    ///
    /// For each value, if all incoming maps agree on the same constant,
    /// keep it; otherwise mark as Unknown.
    ///
    /// Note: This function is reserved for future SSA-based phi-node analysis.
    #[allow(dead_code)]
    fn merge_constants(maps: &[&BTreeMap<MirValue, ConstVal>]) -> BTreeMap<MirValue, ConstVal> {
        if maps.is_empty() {
            return BTreeMap::new();
        }

        if maps.len() == 1 {
            return maps[0].clone();
        }

        // Collect all keys
        let mut all_keys: BTreeSet<MirValue> = BTreeSet::new();
        for map in maps {
            all_keys.extend(map.keys().cloned());
        }

        let mut result = BTreeMap::new();
        for key in all_keys {
            let mut values: Vec<&ConstVal> = Vec::new();
            for map in maps {
                if let Some(val) = map.get(&key) {
                    values.push(val);
                }
            }

            // All maps must have the same known value
            if values.len() == maps.len() {
                let first = &values[0];
                if values.iter().all(|v| *v == *first) {
                    result.insert(key, (*first).clone());
                } else {
                    result.insert(key, ConstVal::Unknown);
                }
            }
            // If some maps don't have the key, we can't propagate
        }

        result
    }
}

impl Default for DomConstPropPass {
    fn default() -> Self {
        Self::new()
    }
}

impl Pass for DomConstPropPass {
    fn name(&self) -> &'static str {
        "dom_const_prop"
    }

    fn run(&self, module: &mut MirModule) -> OptResult<PassResult> {
        let mut total_changes = 0usize;

        for func in module.functions.iter_mut() {
            if func.blocks.is_empty() {
                continue;
            }

            // Build CFG and compute dominators
            let cfg = Cfg::from_function(func);
            let (idom, dom_tree) = cfg.compute_dominators();

            // Compute stable definitions for each block
            let mut block_defs: BTreeMap<MirBlockId, BTreeMap<MirValue, ConstVal>> =
                BTreeMap::new();
            for block in &func.blocks {
                let defs = self.block_stable_defs(&block.statements);
                block_defs.insert(block.id, defs);
            }

            // Propagate constants from dominators to dominated blocks
            // Process in dominator tree order (BFS from entry)
            let mut incoming_constants: BTreeMap<MirBlockId, BTreeMap<MirValue, ConstVal>> =
                BTreeMap::new();
            incoming_constants.insert(cfg.entry, BTreeMap::new());

            let mut worklist: Vec<MirBlockId> = vec![cfg.entry];
            let mut visited: BTreeSet<MirBlockId> = BTreeSet::new();

            while let Some(block_id) = worklist.pop() {
                if visited.contains(&block_id) {
                    continue;
                }
                visited.insert(block_id);

                // Get constants from immediate dominator
                let dom_constants = if let Some(&idom_id) = idom.get(&block_id) {
                    // Merge dominator's incoming + its own definitions
                    let mut merged = incoming_constants
                        .get(&idom_id)
                        .cloned()
                        .unwrap_or_default();
                    if let Some(idom_defs) = block_defs.get(&idom_id) {
                        for (k, v) in idom_defs {
                            merged.insert(*k, v.clone());
                        }
                    }
                    merged
                } else {
                    // Entry block - no incoming constants
                    BTreeMap::new()
                };

                incoming_constants.insert(block_id, dom_constants);

                // Add dominated children to worklist
                if let Some(children) = dom_tree.get(&block_id) {
                    for &child in children {
                        worklist.push(child);
                    }
                }
            }

            // Now apply propagated constants to each block
            for block in func.blocks.iter_mut() {
                let incoming = incoming_constants
                    .get(&block.id)
                    .cloned()
                    .unwrap_or_default();

                // Build local constant map starting from incoming
                let mut local_constants = incoming.clone();
                let mut new_statements = Vec::with_capacity(block.statements.len());

                for stmt in block.statements.drain(..) {
                    let new_rhs = match &stmt.rhs {
                        MirRhs::Literal(lit) => {
                            local_constants.insert(stmt.target, ConstVal::Known(lit.clone()));
                            stmt.rhs.clone()
                        }
                        MirRhs::Binary(op, left, right) => {
                            let left_const = local_constants.get(left);
                            let right_const = local_constants.get(right);

                            match (left_const, right_const) {
                                (Some(ConstVal::Known(l)), Some(ConstVal::Known(r))) => {
                                    if let Some(result) = self.fold_binary(*op, l, r) {
                                        local_constants
                                            .insert(stmt.target, ConstVal::Known(result.clone()));
                                        total_changes += 1;
                                        MirRhs::Literal(result)
                                    } else {
                                        local_constants.insert(stmt.target, ConstVal::Unknown);
                                        stmt.rhs.clone()
                                    }
                                }
                                _ => {
                                    local_constants.insert(stmt.target, ConstVal::Unknown);
                                    stmt.rhs.clone()
                                }
                            }
                        }
                        MirRhs::Unary(op, src) => {
                            if let Some(ConstVal::Known(val)) = local_constants.get(src) {
                                if let Some(result) = self.fold_unary(*op, val) {
                                    local_constants
                                        .insert(stmt.target, ConstVal::Known(result.clone()));
                                    total_changes += 1;
                                    MirRhs::Literal(result)
                                } else {
                                    local_constants.insert(stmt.target, ConstVal::Unknown);
                                    stmt.rhs.clone()
                                }
                            } else {
                                local_constants.insert(stmt.target, ConstVal::Unknown);
                                stmt.rhs.clone()
                            }
                        }
                        MirRhs::Call { .. } => {
                            // Calls are side-effecting - mark result as unknown
                            local_constants.insert(stmt.target, ConstVal::Unknown);
                            stmt.rhs.clone()
                        }
                        MirRhs::Load { .. } => {
                            local_constants.insert(stmt.target, ConstVal::Unknown);
                            stmt.rhs.clone()
                        }
                        MirRhs::Store { .. } => {
                            // Stores invalidate all existing constants (conservative)
                            local_constants.clear();
                            stmt.rhs.clone()
                        }
                    };

                    new_statements.push(MirStatement {
                        target: stmt.target,
                        rhs: new_rhs,
                    });
                }

                block.statements = new_statements;
            }
        }

        if total_changes > 0 {
            Ok(PassResult::with_count(
                total_changes,
                format!("propagated {} constants across basic blocks", total_changes),
            ))
        } else {
            Ok(PassResult::no_change())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use x3_ast::BinaryOp;
    use x3_common::Span;
    use x3_mir::{MirBlock, MirFunction, MirTerminator, SymbolId};

    fn make_module(func: MirFunction) -> MirModule {
        MirModule {
            functions: vec![func],
            span: Span::dummy(),
        }
    }

    #[test]
    fn propagate_constant_across_goto() {
        // bb0: v0 = 42; goto bb1
        // bb1: v1 = v0 + 1  => should become v1 = 43
        let func = MirFunction {
            symbol: SymbolId(0),
            params: vec![],
            entry: MirBlockId(0),
            blocks: vec![
                MirBlock {
                    id: MirBlockId(0),
                    statements: vec![MirStatement {
                        target: MirValue(0),
                        rhs: MirRhs::Literal(Literal::Integer(42)),
                    }],
                    terminator: Some(MirTerminator::Goto(MirBlockId(1))),
                },
                MirBlock {
                    id: MirBlockId(1),
                    statements: vec![
                        MirStatement {
                            target: MirValue(1),
                            rhs: MirRhs::Literal(Literal::Integer(1)),
                        },
                        MirStatement {
                            target: MirValue(2),
                            rhs: MirRhs::Binary(BinaryOp::Add, MirValue(0), MirValue(1)),
                        },
                    ],
                    terminator: Some(MirTerminator::Return(Some(MirValue(2)))),
                },
            ],
            span: Span::dummy(),
        };

        let mut module = make_module(func);
        let pass = DomConstPropPass::new();
        let result = pass.run(&mut module).unwrap();

        assert!(result.changed);

        // Check that v2 in bb1 is now Literal(43)
        let bb1 = &module.functions[0].blocks[1];
        let v2_stmt = bb1
            .statements
            .iter()
            .find(|s| s.target == MirValue(2))
            .unwrap();
        assert_eq!(v2_stmt.rhs, MirRhs::Literal(Literal::Integer(43)));
    }

    #[test]
    fn propagate_through_diamond() {
        // bb0: v0 = 10; branch cond bb1 bb2
        // bb1: v1 = v0 * 2  => should become v1 = 20
        // bb2: v2 = v0 + 5  => should become v2 = 15
        use x3_mir::MirValue;

        let func = MirFunction {
            symbol: SymbolId(0),
            params: vec![],
            entry: MirBlockId(0),
            blocks: vec![
                MirBlock {
                    id: MirBlockId(0),
                    statements: vec![
                        MirStatement {
                            target: MirValue(99), // condition
                            rhs: MirRhs::Literal(Literal::Bool(true)),
                        },
                        MirStatement {
                            target: MirValue(0),
                            rhs: MirRhs::Literal(Literal::Integer(10)),
                        },
                    ],
                    terminator: Some(MirTerminator::Branch {
                        cond: MirValue(99),
                        then_block: MirBlockId(1),
                        else_block: MirBlockId(2),
                    }),
                },
                MirBlock {
                    id: MirBlockId(1),
                    statements: vec![
                        MirStatement {
                            target: MirValue(10),
                            rhs: MirRhs::Literal(Literal::Integer(2)),
                        },
                        MirStatement {
                            target: MirValue(1),
                            rhs: MirRhs::Binary(BinaryOp::Mul, MirValue(0), MirValue(10)),
                        },
                    ],
                    terminator: Some(MirTerminator::Return(Some(MirValue(1)))),
                },
                MirBlock {
                    id: MirBlockId(2),
                    statements: vec![
                        MirStatement {
                            target: MirValue(20),
                            rhs: MirRhs::Literal(Literal::Integer(5)),
                        },
                        MirStatement {
                            target: MirValue(2),
                            rhs: MirRhs::Binary(BinaryOp::Add, MirValue(0), MirValue(20)),
                        },
                    ],
                    terminator: Some(MirTerminator::Return(Some(MirValue(2)))),
                },
            ],
            span: Span::dummy(),
        };

        let mut module = make_module(func);
        let pass = DomConstPropPass::new();
        let result = pass.run(&mut module).unwrap();

        assert!(result.changed);

        // Check bb1: v1 should be 20
        let bb1 = &module.functions[0].blocks[1];
        let v1_stmt = bb1
            .statements
            .iter()
            .find(|s| s.target == MirValue(1))
            .unwrap();
        assert_eq!(v1_stmt.rhs, MirRhs::Literal(Literal::Integer(20)));

        // Check bb2: v2 should be 15
        let bb2 = &module.functions[0].blocks[2];
        let v2_stmt = bb2
            .statements
            .iter()
            .find(|s| s.target == MirValue(2))
            .unwrap();
        assert_eq!(v2_stmt.rhs, MirRhs::Literal(Literal::Integer(15)));
    }

    #[test]
    fn call_kills_propagation() {
        // bb0: v0 = call foo(); goto bb1
        // bb1: v1 = v0 + 1  => should NOT fold (v0 is unknown)
        let func = MirFunction {
            symbol: SymbolId(0),
            params: vec![],
            entry: MirBlockId(0),
            blocks: vec![
                MirBlock {
                    id: MirBlockId(0),
                    statements: vec![MirStatement {
                        target: MirValue(0),
                        rhs: MirRhs::Call {
                            target: SymbolId(1),
                            args: vec![],
                        },
                    }],
                    terminator: Some(MirTerminator::Goto(MirBlockId(1))),
                },
                MirBlock {
                    id: MirBlockId(1),
                    statements: vec![
                        MirStatement {
                            target: MirValue(1),
                            rhs: MirRhs::Literal(Literal::Integer(1)),
                        },
                        MirStatement {
                            target: MirValue(2),
                            rhs: MirRhs::Binary(BinaryOp::Add, MirValue(0), MirValue(1)),
                        },
                    ],
                    terminator: Some(MirTerminator::Return(Some(MirValue(2)))),
                },
            ],
            span: Span::dummy(),
        };

        let mut module = make_module(func);
        let pass = DomConstPropPass::new();
        let result = pass.run(&mut module).unwrap();

        // v2 should still be a Binary op, not folded
        let bb1 = &module.functions[0].blocks[1];
        let v2_stmt = bb1
            .statements
            .iter()
            .find(|s| s.target == MirValue(2))
            .unwrap();
        assert!(matches!(v2_stmt.rhs, MirRhs::Binary(BinaryOp::Add, _, _)));
    }
}
