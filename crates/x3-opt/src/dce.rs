use crate::cfg::Cfg;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use x3_mir::{MirBlock, MirBlockId, MirFunction, MirRhs, MirStatement, MirTerminator, MirValue};

/// Run liveness-based dead code elimination on the function.
/// Returns `true` if any instructions were removed.
pub fn run_dce<F>(func: &mut MirFunction, mut has_side_effect: F) -> bool
where
    F: FnMut(&MirStatement) -> bool,
{
    if func.blocks.is_empty() {
        return false;
    }

    let cfg = Cfg::from_function(func);
    let mut block_index: BTreeMap<MirBlockId, usize> = BTreeMap::new();
    for (idx, block) in func.blocks.iter().enumerate() {
        block_index.insert(block.id, idx);
    }

    let n = func.blocks.len();
    let mut live_in = vec![BTreeSet::new(); n];
    let mut live_out = vec![BTreeSet::new(); n];
    let mut work_queue: VecDeque<usize> = VecDeque::new();
    let mut queued = BTreeSet::new();

    for idx in 0..n {
        work_queue.push_back(idx);
        queued.insert(idx);
    }

    while let Some(idx) = work_queue.pop_front() {
        queued.remove(&idx);
        let block = &func.blocks[idx];

        let mut new_live_out = BTreeSet::new();
        if let Some(succs) = cfg.succs.get(&block.id) {
            for succ in succs.iter() {
                if let Some(&succ_idx) = block_index.get(succ) {
                    for &val in live_in[succ_idx].iter() {
                        new_live_out.insert(val);
                    }
                }
            }
        }

        if new_live_out != live_out[idx] {
            live_out[idx] = new_live_out.clone();
        }

        let mut new_live_in = new_live_out.clone();
        for stmt in block.statements.iter().rev() {
            for def in defs_of(stmt).iter() {
                new_live_in.remove(def);
            }
            for use_val in uses_of(stmt).iter() {
                new_live_in.insert(*use_val);
            }
        }

        if let Some(term) = &block.terminator {
            match term {
                MirTerminator::Branch { cond, .. } => {
                    new_live_in.insert(*cond);
                }
                MirTerminator::Return(Some(val)) => {
                    new_live_in.insert(*val);
                }
                _ => {}
            }
        }

        if new_live_in != live_in[idx] {
            live_in[idx] = new_live_in.clone();
            if let Some(preds) = cfg.preds.get(&block.id) {
                for pred in preds.iter() {
                    if let Some(&pred_idx) = block_index.get(pred) {
                        if queued.insert(pred_idx) {
                            work_queue.push_back(pred_idx);
                        }
                    }
                }
            }
        }
    }

    let mut changed = false;
    for idx in 0..n {
        let mut live = live_out[idx].clone();

        // Add liveness from terminator
        let block = &func.blocks[idx];
        if let Some(term) = &block.terminator {
            match term {
                MirTerminator::Branch { cond, .. } => {
                    live.insert(*cond);
                }
                MirTerminator::Return(Some(val)) => {
                    live.insert(*val);
                }
                _ => {}
            }
        }

        let mut new_statements = Vec::new();
        for stmt in block.statements.iter().rev() {
            let defs = defs_of(stmt);
            let uses = uses_of(stmt);
            let keep = has_side_effect(stmt) || defs.iter().any(|d| live.contains(d));
            if keep {
                for def in defs.iter() {
                    live.remove(def);
                }
                for use_val in uses.iter() {
                    live.insert(*use_val);
                }
                new_statements.push(stmt.clone());
            } else {
                changed = true;
            }
        }
        new_statements.reverse();
        func.blocks[idx].statements = new_statements;
    }

    changed
}

fn uses_of(stmt: &MirStatement) -> Vec<MirValue> {
    match &stmt.rhs {
        MirRhs::Literal(_) => vec![],
        MirRhs::Unary(_, operand) => vec![*operand],
        MirRhs::Binary(_, left, right) => vec![*left, *right],
        MirRhs::Call { args, .. } => args.clone(),
        MirRhs::Load { addr, .. } => vec![*addr],
        MirRhs::Store { addr, val, .. } => vec![*addr, *val],
    }
}

fn defs_of(stmt: &MirStatement) -> Vec<MirValue> {
    vec![stmt.target]
}

#[cfg(test)]
mod tests {
    use super::*;
    use x3_ast::BinaryOp;
    use x3_common::{Literal, Span};
    use x3_mir::{
        MirBlock, MirBlockId, MirFunction, MirStatement, MirTerminator, MirValue, SymbolId,
    };

    fn mk_block(id: usize, statements: Vec<MirStatement>, terminator: MirTerminator) -> MirBlock {
        MirBlock {
            id: MirBlockId(id),
            statements,
            terminator: Some(terminator),
        }
    }

    fn mk_function(blocks: Vec<MirBlock>) -> MirFunction {
        MirFunction {
            symbol: SymbolId(0),
            params: vec![],
            entry: MirBlockId(0),
            blocks,
            span: Span::dummy(),
        }
    }

    #[test]
    fn remove_dead_instruction() {
        // Both assignments are dead since return uses no values
        let func = mk_function(vec![mk_block(
            0,
            vec![
                MirStatement {
                    target: MirValue(0),
                    rhs: MirRhs::Literal(Literal::Integer(1)),
                },
                MirStatement {
                    target: MirValue(1),
                    rhs: MirRhs::Literal(Literal::Integer(2)),
                },
            ],
            MirTerminator::Return(None),
        )]);

        let mut func = func;
        let changed = run_dce(&mut func, |_| false);
        assert!(changed);
        // Both statements removed since return(void) uses nothing
        assert_eq!(func.blocks[0].statements.len(), 0);
    }

    #[test]
    fn keep_side_effecting_call() {
        let mut func = mk_function(vec![mk_block(
            0,
            vec![MirStatement {
                target: MirValue(0),
                rhs: MirRhs::Call {
                    target: SymbolId(1),
                    args: vec![MirValue(0)],
                },
            }],
            MirTerminator::Return(None),
        )]);

        let changed = run_dce(&mut func, |stmt| matches!(&stmt.rhs, MirRhs::Call { .. }));
        assert!(!changed);
        assert_eq!(func.blocks[0].statements.len(), 1);
    }

    #[test]
    fn liveness_across_blocks() {
        let mut func = mk_function(vec![
            mk_block(
                0,
                vec![MirStatement {
                    target: MirValue(0),
                    rhs: MirRhs::Literal(Literal::Integer(1)),
                }],
                MirTerminator::Goto(MirBlockId(1)),
            ),
            mk_block(
                1,
                vec![MirStatement {
                    target: MirValue(1),
                    rhs: MirRhs::Binary(BinaryOp::Add, MirValue(0), MirValue(0)),
                }],
                MirTerminator::Return(Some(MirValue(1))),
            ),
        ]);

        let changed = run_dce(&mut func, |_| false);
        // No changes: v0 is live (used in v1's binary op), v1 is live (returned)
        assert!(!changed);
        assert_eq!(func.blocks[0].statements.len(), 1);
        assert_eq!(func.blocks[1].statements.len(), 1);
    }
}
