//! Global Constant Propagation Pass
//!
//! A lattice-based dataflow analysis for constant propagation across the entire
//! function CFG. Unlike local constant folding, this pass propagates constants
//! through control flow merges using a proper lattice:
//!
//! ```text
//!       ⊤ (Top/Unknown)
//!      / | \
//!     1  2  3 ... (Concrete values)
//!      \ | /
//!       ⊥ (Bottom/Varying)
//! ```
//!
//! # Algorithm
//!
//! 1. Initialize all values to ⊤ (unknown)
//! 2. Worklist iteration:
//!    - For each block, compute transfer function
//!    - Meet incoming values at φ-points (merge points)
//!    - If different constants meet → ⊥ (varying)
//! 3. Replace uses of constant values with literals
//!
//! This handles diamond CFG patterns that dom_const_prop cannot:
//! ```text
//!       B0: x = ?
//!      /        \
//!   B1: x = 5   B2: x = 5
//!      \        /
//!       B3: use x  ← x is provably 5 from both paths
//! ```

use crate::cfg::Cfg;
use crate::pass::{Pass, PassResult};
use crate::OptResult;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use x3_ast::BinaryOp;
use x3_common::Literal;
use x3_mir::mir::{MirBlockId, MirModule, MirRhs, MirValue};

/// Lattice value for constant propagation.
#[derive(Debug, Clone, PartialEq)]
pub enum ConstLattice {
    /// Top: value not yet known (could be anything)
    Top,
    /// A concrete constant value
    Const(Literal),
    /// Bottom: value is varying (different values on different paths)
    Bottom,
}

impl ConstLattice {
    /// Meet operation: combine two lattice values.
    /// Top ⊓ x = x
    /// x ⊓ Top = x
    /// Const(c) ⊓ Const(c) = Const(c)
    /// Const(c1) ⊓ Const(c2) = Bottom (if c1 ≠ c2)
    /// Bottom ⊓ x = Bottom
    pub fn meet(&self, other: &ConstLattice) -> ConstLattice {
        match (self, other) {
            (ConstLattice::Top, x) | (x, ConstLattice::Top) => x.clone(),
            (ConstLattice::Bottom, _) | (_, ConstLattice::Bottom) => ConstLattice::Bottom,
            (ConstLattice::Const(c1), ConstLattice::Const(c2)) => {
                if c1 == c2 {
                    ConstLattice::Const(c1.clone())
                } else {
                    ConstLattice::Bottom
                }
            }
        }
    }

    /// Check if this is a concrete constant.
    pub fn as_const(&self) -> Option<&Literal> {
        match self {
            ConstLattice::Const(c) => Some(c),
            _ => None,
        }
    }
}

/// Global constant propagation pass.
pub struct GlobalConstPropPass;

impl GlobalConstPropPass {
    /// Evaluate a binary operation on two known constants.
    fn eval_binary(op: &BinaryOp, left: &Literal, right: &Literal) -> Option<Literal> {
        match (op, left, right) {
            (BinaryOp::Add, Literal::Integer(a), Literal::Integer(b)) => {
                Some(Literal::Integer(a.wrapping_add(*b)))
            }
            (BinaryOp::Sub, Literal::Integer(a), Literal::Integer(b)) => {
                Some(Literal::Integer(a.wrapping_sub(*b)))
            }
            (BinaryOp::Mul, Literal::Integer(a), Literal::Integer(b)) => {
                Some(Literal::Integer(a.wrapping_mul(*b)))
            }
            (BinaryOp::Div, Literal::Integer(a), Literal::Integer(b)) if *b != 0 => {
                Some(Literal::Integer(a / b))
            }
            (BinaryOp::Mod, Literal::Integer(a), Literal::Integer(b)) if *b != 0 => {
                Some(Literal::Integer(a % b))
            }
            (BinaryOp::Equal, Literal::Integer(a), Literal::Integer(b)) => {
                Some(Literal::Bool(a == b))
            }
            (BinaryOp::NotEqual, Literal::Integer(a), Literal::Integer(b)) => {
                Some(Literal::Bool(a != b))
            }
            (BinaryOp::Less, Literal::Integer(a), Literal::Integer(b)) => {
                Some(Literal::Bool(a < b))
            }
            (BinaryOp::LessEqual, Literal::Integer(a), Literal::Integer(b)) => {
                Some(Literal::Bool(a <= b))
            }
            (BinaryOp::Greater, Literal::Integer(a), Literal::Integer(b)) => {
                Some(Literal::Bool(a > b))
            }
            (BinaryOp::GreaterEqual, Literal::Integer(a), Literal::Integer(b)) => {
                Some(Literal::Bool(a >= b))
            }
            // Note: BinaryOp doesn't have And/Or variants in x3_ast
            // Boolean AND/OR would use BitAnd/BitOr if they existed
            _ => None,
        }
    }

    /// Evaluate a unary operation on a known constant.
    fn eval_unary(op: &x3_ast::UnaryOp, operand: &Literal) -> Option<Literal> {
        match (op, operand) {
            (x3_ast::UnaryOp::Negate, Literal::Integer(n)) => Some(Literal::Integer(-n)),
            (x3_ast::UnaryOp::Not, Literal::Bool(b)) => Some(Literal::Bool(!b)),
            _ => None,
        }
    }
}

impl Pass for GlobalConstPropPass {
    fn name(&self) -> &'static str {
        "global_const_prop"
    }

    fn run(&self, module: &mut MirModule) -> OptResult<PassResult> {
        let mut transformation_count = 0;

        for func in &mut module.functions {
            if func.blocks.is_empty() {
                continue;
            }

            // Build CFG
            let cfg = Cfg::from_function(func);

            // Initialize lattice state: all values start at Top
            let mut lattice: BTreeMap<MirValue, ConstLattice> = BTreeMap::new();

            // Worklist of blocks to process
            let mut worklist: VecDeque<MirBlockId> = VecDeque::new();
            let mut in_worklist: BTreeSet<MirBlockId> = BTreeSet::new();

            // Start with entry block
            worklist.push_back(func.entry);
            in_worklist.insert(func.entry);

            // Add all blocks to initial worklist (ensures we visit everything)
            for block in &func.blocks {
                if !in_worklist.contains(&block.id) {
                    worklist.push_back(block.id);
                    in_worklist.insert(block.id);
                }
            }

            // Iterative dataflow until fixpoint
            let max_iterations = func.blocks.len() * 10; // Safety limit
            let mut iterations = 0;

            while let Some(block_id) = worklist.pop_front() {
                in_worklist.remove(&block_id);
                iterations += 1;
                if iterations > max_iterations {
                    break; // Prevent infinite loops
                }

                // Find the block
                let block = match func.blocks.iter().find(|b| b.id == block_id) {
                    Some(b) => b,
                    None => continue,
                };

                // Process statements
                let mut changed = false;
                for stmt in &block.statements {
                    let new_value = match &stmt.rhs {
                        MirRhs::Literal(lit) => ConstLattice::Const(lit.clone()),
                        MirRhs::Binary(op, left, right) => {
                            let left_val = lattice.get(left).cloned().unwrap_or(ConstLattice::Top);
                            let right_val =
                                lattice.get(right).cloned().unwrap_or(ConstLattice::Top);

                            match (&left_val, &right_val) {
                                (ConstLattice::Const(l), ConstLattice::Const(r)) => {
                                    if let Some(result) = Self::eval_binary(op, l, r) {
                                        ConstLattice::Const(result)
                                    } else {
                                        ConstLattice::Bottom
                                    }
                                }
                                (ConstLattice::Bottom, _) | (_, ConstLattice::Bottom) => {
                                    ConstLattice::Bottom
                                }
                                _ => ConstLattice::Top, // Still unknown
                            }
                        }
                        MirRhs::Unary(op, operand) => {
                            let operand_val =
                                lattice.get(operand).cloned().unwrap_or(ConstLattice::Top);

                            match &operand_val {
                                ConstLattice::Const(c) => {
                                    if let Some(result) = Self::eval_unary(op, c) {
                                        ConstLattice::Const(result)
                                    } else {
                                        ConstLattice::Bottom
                                    }
                                }
                                ConstLattice::Bottom => ConstLattice::Bottom,
                                ConstLattice::Top => ConstLattice::Top,
                            }
                        }
                        MirRhs::Call { .. } => {
                            // Calls return unknown/varying values
                            ConstLattice::Bottom
                        }
                        MirRhs::Load { .. } => {
                            // Loads return unknown values (may vary)
                            ConstLattice::Bottom
                        }
                        MirRhs::Store { .. } => {
                            // Stores don't return values
                            ConstLattice::Top
                        }
                    };

                    // Update lattice with meet
                    let old_value = lattice
                        .get(&stmt.target)
                        .cloned()
                        .unwrap_or(ConstLattice::Top);
                    let merged = old_value.meet(&new_value);

                    if merged != old_value {
                        lattice.insert(stmt.target, merged);
                        changed = true;
                    }
                }

                // If anything changed, add successors to worklist
                if changed {
                    if let Some(succs) = cfg.succs.get(&block_id) {
                        for succ in succs {
                            if !in_worklist.contains(succ) {
                                worklist.push_back(*succ);
                                in_worklist.insert(*succ);
                            }
                        }
                    }
                }
            }

            // Now apply transformations: replace known constants in operands
            for block in &mut func.blocks {
                for stmt in &mut block.statements {
                    let new_rhs = match &stmt.rhs {
                        MirRhs::Binary(op, left, right) => {
                            let left_const = lattice.get(left).and_then(|v| v.as_const());
                            let right_const = lattice.get(right).and_then(|v| v.as_const());

                            // If both operands are constant, fold the operation
                            if let (Some(l), Some(r)) = (left_const, right_const) {
                                if let Some(result) = Self::eval_binary(op, l, r) {
                                    Some(MirRhs::Literal(result))
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        }
                        MirRhs::Unary(op, operand) => {
                            let operand_const = lattice.get(operand).and_then(|v| v.as_const());

                            if let Some(c) = operand_const {
                                if let Some(result) = Self::eval_unary(op, c) {
                                    Some(MirRhs::Literal(result))
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        }
                        _ => None,
                    };

                    if let Some(new) = new_rhs {
                        stmt.rhs = new;
                        transformation_count += 1;
                    }
                }
            }
        }

        Ok(PassResult::with_count(
            transformation_count,
            "global constant propagation",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use x3_common::Span;
    use x3_mir::mir::{MirBlock, MirFunction, MirStatement, MirTerminator, SymbolId};

    fn dummy_span() -> Span {
        Span::new(0, 0)
    }

    fn make_func(blocks: Vec<MirBlock>) -> MirFunction {
        MirFunction {
            symbol: SymbolId(0),
            params: vec![],
            entry: MirBlockId(0),
            blocks,
            span: dummy_span(),
        }
    }

    #[test]
    fn test_lattice_meet() {
        let top = ConstLattice::Top;
        let bottom = ConstLattice::Bottom;
        let c5 = ConstLattice::Const(Literal::Integer(5));
        let c10 = ConstLattice::Const(Literal::Integer(10));

        // Top ⊓ x = x
        assert_eq!(top.meet(&c5), c5);
        assert_eq!(c5.meet(&top), c5);

        // Bottom ⊓ x = Bottom
        assert_eq!(bottom.meet(&c5), ConstLattice::Bottom);
        assert_eq!(c5.meet(&bottom), ConstLattice::Bottom);

        // Same constant stays constant
        assert_eq!(c5.meet(&c5), c5);

        // Different constants → Bottom
        assert_eq!(c5.meet(&c10), ConstLattice::Bottom);
    }

    #[test]
    fn test_propagate_through_linear_blocks() {
        // B0: v0 = 5
        // B0: v1 = v0 + 3
        // → v1 should be folded to 8
        let blocks = vec![MirBlock {
            id: MirBlockId(0),
            statements: vec![
                MirStatement {
                    target: MirValue(0),
                    rhs: MirRhs::Literal(Literal::Integer(5)),
                },
                MirStatement {
                    target: MirValue(1),
                    rhs: MirRhs::Binary(BinaryOp::Add, MirValue(0), MirValue(2)),
                },
                MirStatement {
                    target: MirValue(2),
                    rhs: MirRhs::Literal(Literal::Integer(3)),
                },
            ],
            terminator: Some(MirTerminator::Return(Some(MirValue(1)))),
        }];

        let mut module = MirModule {
            functions: vec![make_func(blocks)],
            span: dummy_span(),
        };

        let pass = GlobalConstPropPass;
        let result = pass.run(&mut module).unwrap();

        assert!(result.changed);
        // v1 should now be Literal(8)
        let v1_stmt = &module.functions[0].blocks[0].statements[1];
        assert_eq!(v1_stmt.rhs, MirRhs::Literal(Literal::Integer(8)));
    }

    #[test]
    fn test_call_kills_constant() {
        // v0 = call foo() - unknown result
        // v1 = v0 + 1 - cannot fold
        let blocks = vec![MirBlock {
            id: MirBlockId(0),
            statements: vec![
                MirStatement {
                    target: MirValue(0),
                    rhs: MirRhs::Call {
                        target: SymbolId(1),
                        args: vec![],
                    },
                },
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
        }];

        let mut module = MirModule {
            functions: vec![make_func(blocks)],
            span: dummy_span(),
        };

        let pass = GlobalConstPropPass;
        let result = pass.run(&mut module).unwrap();

        // v2 = v0 + v1 cannot be folded since v0 is from a call
        assert!(!result.changed);
    }

    #[test]
    fn test_propagate_boolean() {
        // v0 = true
        // v1 = !v0  → false
        let blocks = vec![MirBlock {
            id: MirBlockId(0),
            statements: vec![
                MirStatement {
                    target: MirValue(0),
                    rhs: MirRhs::Literal(Literal::Bool(true)),
                },
                MirStatement {
                    target: MirValue(1),
                    rhs: MirRhs::Unary(x3_ast::UnaryOp::Not, MirValue(0)),
                },
            ],
            terminator: Some(MirTerminator::Return(Some(MirValue(1)))),
        }];

        let mut module = MirModule {
            functions: vec![make_func(blocks)],
            span: dummy_span(),
        };

        let pass = GlobalConstPropPass;
        let result = pass.run(&mut module).unwrap();

        assert!(result.changed);
        let v1_stmt = &module.functions[0].blocks[0].statements[1];
        assert_eq!(v1_stmt.rhs, MirRhs::Literal(Literal::Bool(false)));
    }
}
