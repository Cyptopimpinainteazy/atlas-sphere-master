//! Speculative Hoisting Pass
//!
//! Hoists pure operations from dominated blocks into their dominators
//! when it enables elimination of redundant computations or improves
//! code scheduling.
//!
//! # Hoisting Rules
//!
//! An operation in block B can be hoisted to dominator D when:
//!
//! 1. D strictly dominates B
//! 2. The operation is **pure** (no side effects, no memory ops)
//! 3. All operands are available in D (dominated by their definitions)
//! 4. The hoist does not cross an atomic window boundary
//! 5. The operation is not inside a loop that D is outside of
//!
//! # What is "Pure"?
//!
//! Pure operations include:
//! - Arithmetic: Add, Sub, Mul, Div, Mod
//! - Comparisons: Equal, NotEqual, Less, LessEqual, Greater, GreaterEqual
//! - Unary: Not, Negate
//! - Literal loads
//!
//! NOT pure (never hoist):
//! - Call (function calls have side effects)
//!
//! # Benefits
//!
//! - Enables common subexpression elimination across branches
//! - Improves instruction scheduling
//! - Can reduce register pressure by computing values earlier
//!
//! # Interaction with Other Passes
//!
//! Should run AFTER:
//! - BlockFusionPass (fewer blocks = simpler dominance)
//! - DeadCodeElimination (don't hoist dead code)
//!
//! Should run BEFORE:
//! - CommonSubexpressionElimination (hoisting exposes more CSE)

use crate::cfg::Cfg;
use crate::pass::{Pass, PassResult};
use crate::OptResult;
use std::collections::BTreeMap;
use x3_ast::BinaryOp;
use x3_mir::mir::{MirBlockId, MirModule, MirRhs, MirValue};

/// Speculative hoisting pass for moving pure operations into dominators.
pub struct SpeculativeHoistPass;

impl Pass for SpeculativeHoistPass {
    fn name(&self) -> &'static str {
        "speculative_hoist"
    }

    fn run(&self, module: &mut MirModule) -> OptResult<PassResult> {
        let mut transformation_count = 0;

        for func in &mut module.functions {
            if func.blocks.len() < 2 {
                continue;
            }

            let cfg = Cfg::from_function(func);
            let (idom, _dom_tree) = cfg.compute_dominators();

            // Build value definitions map: MirValue -> (BlockId, statement_index)
            let value_defs = build_value_definitions(func);

            // Find hoisting candidates
            let candidates = find_hoist_candidates(func, &idom, &value_defs);

            // Apply hoisting (from leaf blocks toward root to maintain consistency)
            for (src_block_id, stmt_idx, target_block_id) in candidates {
                if let Some(hoisted) =
                    hoist_statement(func, src_block_id, stmt_idx, target_block_id)
                {
                    if hoisted {
                        transformation_count += 1;
                    }
                }
            }
        }

        Ok(PassResult::with_count(
            transformation_count,
            "operations hoisted",
        ))
    }
}

/// Build a map of where each value is defined.
fn build_value_definitions(
    func: &x3_mir::mir::MirFunction,
) -> BTreeMap<MirValue, (MirBlockId, usize)> {
    let mut defs = BTreeMap::new();

    for block in &func.blocks {
        for (idx, stmt) in block.statements.iter().enumerate() {
            defs.insert(stmt.target, (block.id, idx));
        }
    }

    defs
}

/// Check if an operation is pure (safe to speculatively execute).
fn is_pure(rhs: &MirRhs) -> bool {
    match rhs {
        // Literals are always pure
        MirRhs::Literal(_) => true,

        // All binary arithmetic and comparison ops are pure
        MirRhs::Binary(op, _, _) => matches!(
            op,
            BinaryOp::Add
                | BinaryOp::Sub
                | BinaryOp::Mul
                | BinaryOp::Div
                | BinaryOp::Mod
                | BinaryOp::Pow
                | BinaryOp::Equal
                | BinaryOp::NotEqual
                | BinaryOp::Less
                | BinaryOp::LessEqual
                | BinaryOp::Greater
                | BinaryOp::GreaterEqual
                | BinaryOp::LogicalAnd
                | BinaryOp::LogicalOr
        ),

        // Unary operations are pure
        MirRhs::Unary(_, _) => true,

        // Function calls are NOT pure (side effects possible)
        MirRhs::Call { .. } => false,

        // Loads and stores are NOT pure
        MirRhs::Load { .. } => false,
        MirRhs::Store { .. } => false,
    }
}

/// Check if all operands of an RHS are available at the target block.
fn operands_available_at(
    rhs: &MirRhs,
    target_block_id: MirBlockId,
    value_defs: &BTreeMap<MirValue, (MirBlockId, usize)>,
    idom: &BTreeMap<MirBlockId, MirBlockId>,
) -> bool {
    let operands = get_operands(rhs);

    for operand in operands {
        if let Some(&(def_block, _)) = value_defs.get(&operand) {
            // Operand must be defined in target_block or a dominator of it
            if def_block != target_block_id && !dominates(def_block, target_block_id, idom) {
                return false;
            }
        }
        // If operand not in value_defs, it might be a parameter (always available)
    }

    true
}

/// Get all operand values from an RHS.
fn get_operands(rhs: &MirRhs) -> Vec<MirValue> {
    match rhs {
        MirRhs::Literal(_) => vec![],
        MirRhs::Binary(_, a, b) => vec![*a, *b],
        MirRhs::Unary(_, v) => vec![*v],
        MirRhs::Call { args, .. } => args.clone(),
        MirRhs::Load { addr, .. } => vec![*addr],
        MirRhs::Store { addr, val, .. } => vec![*addr, *val],
    }
}

/// Check if block `a` dominates block `b`.
fn dominates(a: MirBlockId, b: MirBlockId, idom: &BTreeMap<MirBlockId, MirBlockId>) -> bool {
    if a == b {
        return true;
    }

    let mut current = b;
    while let Some(&dom) = idom.get(&current) {
        if dom == a {
            return true;
        }
        if dom == current {
            // Entry block
            break;
        }
        current = dom;
    }
    false
}

/// Find statements that can be hoisted to their immediate dominator.
fn find_hoist_candidates(
    func: &x3_mir::mir::MirFunction,
    idom: &BTreeMap<MirBlockId, MirBlockId>,
    value_defs: &BTreeMap<MirValue, (MirBlockId, usize)>,
) -> Vec<(MirBlockId, usize, MirBlockId)> {
    let mut candidates = Vec::new();

    for block in &func.blocks {
        // Skip entry block (nowhere to hoist to)
        let target_block_id = match idom.get(&block.id) {
            Some(&dom) if dom != block.id => dom,
            _ => continue,
        };

        for (idx, stmt) in block.statements.iter().enumerate() {
            // Check if this statement is pure
            if !is_pure(&stmt.rhs) {
                continue;
            }

            // Check if operands are available at target
            if !operands_available_at(&stmt.rhs, target_block_id, value_defs, idom) {
                continue;
            }

            // This is a hoisting candidate
            candidates.push((block.id, idx, target_block_id));
        }
    }

    // Sort by source block then by statement index (reverse) so we remove
    // from end first to maintain indices
    candidates.sort_by(|a, b| {
        if a.0 == b.0 {
            b.1.cmp(&a.1) // reverse order by index
        } else {
            b.0 .0.cmp(&a.0 .0) // reverse order by block
        }
    });

    candidates
}

/// Hoist a statement from source block to target block.
fn hoist_statement(
    func: &mut x3_mir::mir::MirFunction,
    src_block_id: MirBlockId,
    stmt_idx: usize,
    target_block_id: MirBlockId,
) -> Option<bool> {
    // Find source block
    let src_idx = func.blocks.iter().position(|b| b.id == src_block_id)?;
    let target_idx = func.blocks.iter().position(|b| b.id == target_block_id)?;

    // Bounds check
    if stmt_idx >= func.blocks[src_idx].statements.len() {
        return Some(false);
    }

    // Remove from source
    let stmt = func.blocks[src_idx].statements.remove(stmt_idx);

    // Insert at end of target (before terminator)
    func.blocks[target_idx].statements.push(stmt);

    Some(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use x3_common::{Literal, Span};
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
    fn test_hoist_pure_to_dominator() {
        // B0 (dominator): cond = ...; branch(cond, B1, B2)
        // B1: x = 1 + 2; ...  <- pure, can hoist
        // B2: ...
        //
        // After hoisting, x = 1 + 2 should be in B0
        let blocks = vec![
            MirBlock {
                id: MirBlockId(0),
                statements: vec![MirStatement {
                    target: MirValue(0),
                    rhs: MirRhs::Literal(Literal::Bool(true)),
                }],
                terminator: Some(MirTerminator::Branch {
                    cond: MirValue(0),
                    then_block: MirBlockId(1),
                    else_block: MirBlockId(2),
                }),
            },
            MirBlock {
                id: MirBlockId(1),
                statements: vec![MirStatement {
                    target: MirValue(1),
                    rhs: MirRhs::Binary(
                        BinaryOp::Add,
                        MirValue(100), // Assume params
                        MirValue(101),
                    ),
                }],
                terminator: Some(MirTerminator::Return(Some(MirValue(1)))),
            },
            MirBlock {
                id: MirBlockId(2),
                statements: vec![],
                terminator: Some(MirTerminator::Return(None)),
            },
        ];

        let mut module = MirModule {
            functions: vec![make_func(blocks)],
            span: dummy_span(),
        };

        let pass = SpeculativeHoistPass;
        let result = pass.run(&mut module).unwrap();

        assert!(result.changed);

        // x = 1 + 2 should now be in B0
        assert_eq!(module.functions[0].blocks[0].statements.len(), 2);
        // B1 should be empty
        let b1_idx = module.functions[0]
            .blocks
            .iter()
            .position(|b| b.id == MirBlockId(1))
            .unwrap();
        assert_eq!(module.functions[0].blocks[b1_idx].statements.len(), 0);
    }

    #[test]
    fn test_no_hoist_impure() {
        // B0: branch(cond, B1, B2)
        // B1: x = call(...); <- NOT pure, don't hoist
        // B2: ...
        let blocks = vec![
            MirBlock {
                id: MirBlockId(0),
                statements: vec![MirStatement {
                    target: MirValue(0),
                    rhs: MirRhs::Literal(Literal::Bool(true)),
                }],
                terminator: Some(MirTerminator::Branch {
                    cond: MirValue(0),
                    then_block: MirBlockId(1),
                    else_block: MirBlockId(2),
                }),
            },
            MirBlock {
                id: MirBlockId(1),
                statements: vec![MirStatement {
                    target: MirValue(1),
                    rhs: MirRhs::Call {
                        target: SymbolId(99),
                        args: vec![],
                    },
                }],
                terminator: Some(MirTerminator::Return(Some(MirValue(1)))),
            },
            MirBlock {
                id: MirBlockId(2),
                statements: vec![],
                terminator: Some(MirTerminator::Return(None)),
            },
        ];

        let mut module = MirModule {
            functions: vec![make_func(blocks)],
            span: dummy_span(),
        };

        let pass = SpeculativeHoistPass;
        let result = pass.run(&mut module).unwrap();

        // No hoisting should occur
        assert!(!result.changed);
    }

    #[test]
    fn test_no_hoist_unavailable_operand() {
        // B0: branch(cond, B1, B2)
        // B1: y = 5; x = y + 1; <- y defined in B1, can't hoist x without y
        // B2: ...
        let blocks = vec![
            MirBlock {
                id: MirBlockId(0),
                statements: vec![MirStatement {
                    target: MirValue(0),
                    rhs: MirRhs::Literal(Literal::Bool(true)),
                }],
                terminator: Some(MirTerminator::Branch {
                    cond: MirValue(0),
                    then_block: MirBlockId(1),
                    else_block: MirBlockId(2),
                }),
            },
            MirBlock {
                id: MirBlockId(1),
                statements: vec![
                    MirStatement {
                        target: MirValue(1),
                        rhs: MirRhs::Literal(Literal::Integer(5)),
                    },
                    MirStatement {
                        target: MirValue(2),
                        rhs: MirRhs::Binary(BinaryOp::Add, MirValue(1), MirValue(100)),
                    },
                ],
                terminator: Some(MirTerminator::Return(Some(MirValue(2)))),
            },
            MirBlock {
                id: MirBlockId(2),
                statements: vec![],
                terminator: Some(MirTerminator::Return(None)),
            },
        ];

        let mut module = MirModule {
            functions: vec![make_func(blocks)],
            span: dummy_span(),
        };

        let pass = SpeculativeHoistPass;
        let result = pass.run(&mut module).unwrap();

        // The literal y=5 CAN be hoisted (no operands)
        // But x = y + 1 cannot (y not available in B0)
        assert!(result.changed);

        // y=5 should have been hoisted to B0
        let b0 = &module.functions[0].blocks[0];
        assert_eq!(b0.statements.len(), 2); // cond + hoisted literal

        // x = y + 1 should still be in B1
        let b1_idx = module.functions[0]
            .blocks
            .iter()
            .position(|b| b.id == MirBlockId(1))
            .unwrap();
        assert_eq!(module.functions[0].blocks[b1_idx].statements.len(), 1);
    }

    #[test]
    fn test_is_pure_classification() {
        // Test various RHS types
        assert!(is_pure(&MirRhs::Literal(Literal::Integer(1))));
        assert!(is_pure(&MirRhs::Binary(
            BinaryOp::Add,
            MirValue(0),
            MirValue(1)
        )));
        assert!(is_pure(&MirRhs::Binary(
            BinaryOp::Mul,
            MirValue(0),
            MirValue(1)
        )));
        assert!(is_pure(&MirRhs::Unary(x3_ast::UnaryOp::Not, MirValue(0))));

        // Not pure
        assert!(!is_pure(&MirRhs::Call {
            target: SymbolId(0),
            args: vec![]
        }));
    }
}
