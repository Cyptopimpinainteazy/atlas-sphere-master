use std::collections::{BTreeMap, BTreeSet};

use crate::cfg::Cfg;
use x3_ast::{BinaryOp, UnaryOp};
use x3_common::Literal;
use x3_mir::{MirBlockId, MirFunction, MirRhs, MirValue};

/// Lightweight SSA representation that keeps dominator and frontier metadata alongside MIR.
#[derive(Clone, Debug)]
pub struct SsaFunction {
    pub mir: MirFunction,
    pub cfg: Cfg,
    pub idom: BTreeMap<MirBlockId, MirBlockId>,
    pub dom_tree: BTreeMap<MirBlockId, BTreeSet<MirBlockId>>,
    pub dom_frontier: BTreeMap<MirBlockId, Vec<MirBlockId>>,
    pub phi_counts: BTreeMap<MirBlockId, usize>,
}

/// Convert MIR into a naive SSA view that is ready for phi placement and renaming.
pub fn convert_to_ssa(func: &MirFunction) -> SsaFunction {
    let cfg = Cfg::from_function(func);
    let (idom, dom_tree) = cfg.compute_dominators();
    let dom_frontier = compute_dom_frontier(&cfg, &idom);

    SsaFunction {
        mir: func.clone(),
        cfg,
        idom,
        dom_tree,
        dom_frontier,
        phi_counts: BTreeMap::new(),
    }
}

/// Insert placeholder phi nodes for any block whose frontier is non-empty.
pub fn insert_phis_and_rename(ssa: &mut SsaFunction) -> usize {
    let mut inserted = 0;

    for (&block, frontier_blocks) in &ssa.dom_frontier {
        if frontier_blocks.is_empty() {
            continue;
        }
        ssa.phi_counts.insert(block, frontier_blocks.len());
        inserted += frontier_blocks.len();
    }

    inserted
}

/// Naive SSA-friendly constant propagation that collapses binary/unary chains within a block.
pub fn ssa_const_prop(ssa: &mut SsaFunction) -> bool {
    let mut changed = false;

    for block in &mut ssa.mir.blocks {
        let mut known_constants: BTreeMap<MirValue, Literal> = BTreeMap::new();

        for stmt in &mut block.statements {
            match &stmt.rhs {
                MirRhs::Unary(op, operand) => {
                    if let Some(value) = known_constants.get(operand) {
                        if let Some(result) = eval_unary(*op, value) {
                            stmt.rhs = MirRhs::Literal(result.clone());
                            changed = true;
                        }
                    }
                }
                MirRhs::Binary(op, lhs, rhs_val) => {
                    if let (Some(left), Some(right)) =
                        (known_constants.get(lhs), known_constants.get(rhs_val))
                    {
                        if let Some(result) = eval_binary(*op, left, right) {
                            stmt.rhs = MirRhs::Literal(result.clone());
                            changed = true;
                        }
                    }
                }
                MirRhs::Literal(_)
                | MirRhs::Call { .. }
                | MirRhs::Load { .. }
                | MirRhs::Store { .. } => {}
            }

            if let MirRhs::Literal(literal) = &stmt.rhs {
                known_constants.insert(stmt.target, literal.clone());
            }
        }
    }

    changed
}

/// Placeholder for future SSA copy propagation heuristics.
pub fn ssa_copy_prop(_ssa: &mut SsaFunction) -> bool {
    false
}

/// Lower the SSA view back into MIR after analysis/mutation.
pub fn lower_ssa_to_mir(ssa: &SsaFunction) -> MirFunction {
    ssa.mir.clone()
}

/// Apply SSA-style dominator-aware passes to a single MIR function.
pub fn global_ssa_opt(func: &MirFunction) -> MirFunction {
    let mut ssa = convert_to_ssa(func);
    insert_phis_and_rename(&mut ssa);
    ssa_const_prop(&mut ssa);
    ssa_copy_prop(&mut ssa);
    lower_ssa_to_mir(&ssa)
}

fn compute_dom_frontier(
    cfg: &Cfg,
    idom: &BTreeMap<MirBlockId, MirBlockId>,
) -> BTreeMap<MirBlockId, Vec<MirBlockId>> {
    let mut frontier: BTreeMap<MirBlockId, Vec<MirBlockId>> = BTreeMap::new();

    for &block in &cfg.blocks {
        frontier.entry(block).or_default();
    }

    for &node in &cfg.blocks {
        let preds = cfg.preds.get(&node).cloned().unwrap_or_default();
        if preds.len() < 2 {
            continue;
        }

        let node_idom = match idom.get(&node) {
            Some(&idom) => idom,
            None => continue,
        };

        for pred in preds {
            let mut runner = pred;
            while runner != node_idom {
                frontier.entry(runner).or_default().push(node);

                match idom.get(&runner) {
                    Some(&next) if next != runner => runner = next,
                    _ => break,
                }
            }
        }
    }

    for values in frontier.values_mut() {
        values.sort();
        values.dedup();
    }

    frontier
}

pub(crate) fn eval_binary(op: BinaryOp, left: &Literal, right: &Literal) -> Option<Literal> {
    use BinaryOp::*;

    match op {
        Add => numerical_op(left, right, |a, b| a + b, |a, b| Some(a + b)),
        Sub => numerical_op(left, right, |a, b| a - b, |a, b| Some(a - b)),
        Mul => numerical_op(left, right, |a, b| a * b, |a, b| Some(a * b)),
        Div => numerical_op(left, right, |a, b| a / b, |a, b| Some(a / b)),
        Mod => numerical_op(left, right, |a, b| a % b, |_, _| None),
        Equal => eq_op(left, right, |a, b| a == b, |a, b| a == b),
        NotEqual => eq_op(left, right, |a, b| a != b, |a, b| a != b),
        Less => cmp_op(left, right, |a, b| a < b, |a, b| a < b),
        LessEqual => cmp_op(left, right, |a, b| a <= b, |a, b| a <= b),
        Greater => cmp_op(left, right, |a, b| a > b, |a, b| a > b),
        GreaterEqual => cmp_op(left, right, |a, b| a >= b, |a, b| a >= b),
        LogicalAnd => bool_op(left, right, |a, b| *a && *b),
        LogicalOr => bool_op(left, right, |a, b| *a || *b),
        Pow => None,
    }
}

pub(crate) fn eval_unary(op: UnaryOp, value: &Literal) -> Option<Literal> {
    match op {
        UnaryOp::Negate => match value {
            Literal::Integer(i) => Some(Literal::Integer(-i)),
            Literal::Float(f) => Some(Literal::Float(-f)),
            _ => None,
        },
        UnaryOp::Not => match value {
            Literal::Bool(b) => Some(Literal::Bool(!b)),
            _ => None,
        },
    }
}

fn numerical_op<F, G>(left: &Literal, right: &Literal, int_op: F, float_op: G) -> Option<Literal>
where
    F: Fn(i64, i64) -> i64,
    G: Fn(f64, f64) -> Option<f64>,
{
    match (left, right) {
        (Literal::Integer(lhs), Literal::Integer(rhs)) => {
            Some(Literal::Integer(int_op(*lhs, *rhs)))
        }
        (Literal::Float(lhs), Literal::Float(rhs)) => float_op(*lhs, *rhs).map(Literal::Float),
        _ => None,
    }
}

fn eq_op<F, G>(left: &Literal, right: &Literal, int_eq: F, float_eq: G) -> Option<Literal>
where
    F: Fn(i64, i64) -> bool,
    G: Fn(f64, f64) -> bool,
{
    match (left, right) {
        (Literal::Integer(lhs), Literal::Integer(rhs)) => Some(Literal::Bool(int_eq(*lhs, *rhs))),
        (Literal::Float(lhs), Literal::Float(rhs)) => Some(Literal::Bool(float_eq(*lhs, *rhs))),
        (Literal::Bool(lhs), Literal::Bool(rhs)) => Some(Literal::Bool(*lhs == *rhs)),
        _ => None,
    }
}

fn cmp_op<F, G>(left: &Literal, right: &Literal, int_cmp: F, float_cmp: G) -> Option<Literal>
where
    F: Fn(i64, i64) -> bool,
    G: Fn(f64, f64) -> bool,
{
    match (left, right) {
        (Literal::Integer(lhs), Literal::Integer(rhs)) => Some(Literal::Bool(int_cmp(*lhs, *rhs))),
        (Literal::Float(lhs), Literal::Float(rhs)) => Some(Literal::Bool(float_cmp(*lhs, *rhs))),
        _ => None,
    }
}

fn bool_op<F>(left: &Literal, right: &Literal, op: F) -> Option<Literal>
where
    F: Fn(&bool, &bool) -> bool,
{
    match (left, right) {
        (Literal::Bool(lhs), Literal::Bool(rhs)) => Some(Literal::Bool(op(lhs, rhs))),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use x3_ast::BinaryOp;
    use x3_common::Literal;
    use x3_mir::{MirBlock, MirStatement, MirTerminator};

    fn make_block(id: usize, terminator: Option<MirTerminator>) -> MirBlock {
        MirBlock {
            id: MirBlockId(id),
            statements: vec![],
            terminator,
        }
    }

    #[test]
    fn dom_frontier_handles_diamond() {
        let function = MirFunction {
            symbol: x3_mir::SymbolId(0),
            params: vec![],
            entry: MirBlockId(0),
            blocks: vec![
                MirBlock {
                    id: MirBlockId(0),
                    statements: vec![],
                    terminator: Some(MirTerminator::Branch {
                        cond: MirValue(0),
                        then_block: MirBlockId(1),
                        else_block: MirBlockId(2),
                    }),
                },
                make_block(1, Some(MirTerminator::Goto(MirBlockId(3)))),
                make_block(2, Some(MirTerminator::Goto(MirBlockId(3)))),
                make_block(3, Some(MirTerminator::Return(None))),
            ],
            span: x3_common::Span::dummy(),
        };

        let ssa = convert_to_ssa(&function);
        assert_eq!(ssa.dom_frontier[&MirBlockId(1)], vec![MirBlockId(3)]);
        assert_eq!(ssa.dom_frontier[&MirBlockId(2)], vec![MirBlockId(3)]);
        assert!(ssa.dom_frontier[&MirBlockId(0)].is_empty());
    }

    #[test]
    fn const_prop_turns_binary_to_literal() {
        let mut function = MirFunction {
            symbol: x3_mir::SymbolId(1),
            params: vec![],
            entry: MirBlockId(0),
            blocks: vec![MirBlock {
                id: MirBlockId(0),
                statements: vec![
                    MirStatement {
                        target: MirValue(0),
                        rhs: MirRhs::Literal(Literal::Integer(2)),
                    },
                    MirStatement {
                        target: MirValue(1),
                        rhs: MirRhs::Literal(Literal::Integer(3)),
                    },
                    MirStatement {
                        target: MirValue(2),
                        rhs: MirRhs::Binary(BinaryOp::Add, MirValue(0), MirValue(1)),
                    },
                ],
                terminator: Some(MirTerminator::Return(Some(MirValue(2)))),
            }],
            span: x3_common::Span::dummy(),
        };

        let mut ssa = convert_to_ssa(&function);
        let changed = ssa_const_prop(&mut ssa);
        assert!(changed);

        if let MirRhs::Literal(Literal::Integer(sum)) = &ssa.mir.blocks[0].statements[2].rhs {
            assert_eq!(*sum, 5);
        } else {
            panic!("expected literal after propagation");
        }
    }
}
