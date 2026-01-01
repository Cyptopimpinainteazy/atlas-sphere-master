use crate::cfg::Cfg;
use crate::ssa_lite::{eval_binary, eval_unary};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use x3_common::Literal;
use x3_mir::{MirBlock, MirBlockId, MirFunction, MirRhs, MirStatement, MirTerminator, MirValue};

/// Three-state lattice of values flowing along CFG edges.
#[derive(Clone, Debug, PartialEq)]
pub enum ConstVal {
    Unknown,
    Const(Literal),
    Overdefined,
}

impl ConstVal {
    pub fn meet(&self, other: &ConstVal) -> ConstVal {
        match (self, other) {
            (ConstVal::Unknown, rhs) | (rhs, ConstVal::Unknown) => rhs.clone(),
            (ConstVal::Const(a), ConstVal::Const(b)) => {
                if a == b {
                    ConstVal::Const(a.clone())
                } else {
                    ConstVal::Overdefined
                }
            }
            _ => ConstVal::Overdefined,
        }
    }
}

/// Edge identifier (from block -> to block).
pub type Edge = (MirBlockId, MirBlockId);

/// Mapping from edge to per-variable constants.
pub type EdgeConstMap = BTreeMap<Edge, BTreeMap<MirValue, ConstVal>>;

/// Compute edge-sensitive constants for every successor edge in the function.
pub fn compute_edge_constants(func: &MirFunction) -> EdgeConstMap {
    let cfg = Cfg::from_function(func);
    let vars = collect_vars(func);
    let mut block_index: BTreeMap<MirBlockId, &MirBlock> = BTreeMap::new();
    for block in &func.blocks {
        block_index.insert(block.id, block);
    }

    let mut edge_map: EdgeConstMap = BTreeMap::new();
    for (from, succs) in cfg.succs.iter() {
        for succ in succs.iter() {
            edge_map.insert((*from, *succ), init_env(&vars));
        }
    }

    let mut queue: VecDeque<Edge> = edge_map.keys().cloned().collect();
    let mut queued: BTreeSet<Edge> = queue.iter().cloned().collect();

    while let Some(edge) = queue.pop_front() {
        queued.remove(&edge);
        let (from, to) = edge;
        let mut in_map = init_env(&vars);

        if let Some(preds) = cfg.preds.get(&from) {
            for pred in preds.iter() {
                if let Some(pred_edge) = edge_map.get(&(*pred, from)) {
                    for var in vars.iter() {
                        let cur = in_map.get(var).unwrap();
                        let cand = pred_edge.get(var).unwrap();
                        in_map.insert(*var, cur.meet(cand));
                    }
                }
            }
        }

        let block = match block_index.get(&from) {
            Some(block) => block,
            None => continue,
        };
        let out_map = transfer_block(&in_map, block);

        if let Some(current) = edge_map.get_mut(&edge) {
            let mut changed = false;
            for var in vars.iter() {
                let existing = current.get(var).unwrap();
                let incoming = out_map.get(var).unwrap();
                let merged = existing.meet(incoming);
                if &merged != existing {
                    current.insert(*var, merged);
                    changed = true;
                }
            }

            if changed {
                if let Some(succs) = cfg.succs.get(&to) {
                    for succ in succs.iter() {
                        let succ_edge = (to, *succ);
                        if queued.insert(succ_edge) {
                            queue.push_back(succ_edge);
                        }
                    }
                }
            }
        }
    }

    edge_map
}

/// Fold branches whose incoming edges deliver a constant condition value.
pub fn fold_branch_on_edge_consts(func: &mut MirFunction, edge_consts: &EdgeConstMap) -> bool {
    if func.blocks.is_empty() {
        return false;
    }

    let cfg = Cfg::from_function(func);
    let mut changed = false;

    for block in func.blocks.iter_mut() {
        if let Some(MirTerminator::Branch {
            cond,
            then_block,
            else_block,
        }) = &block.terminator
        {
            if let Some(succs) = cfg.succs.get(&block.id) {
                let mut const_val: Option<bool> = None;
                let mut consistent = true;

                for succ in succs {
                    let edge = (block.id, *succ);
                    if let Some(edge_map) = edge_consts.get(&edge) {
                        match edge_map.get(cond) {
                            Some(ConstVal::Const(lit)) => {
                                if let Some(pred) = literal_as_bool(lit) {
                                    if let Some(existing) = const_val {
                                        if existing != pred {
                                            consistent = false;
                                            break;
                                        }
                                    } else {
                                        const_val = Some(pred);
                                    }
                                } else {
                                    consistent = false;
                                    break;
                                }
                            }
                            _ => {
                                consistent = false;
                                break;
                            }
                        }
                    } else {
                        consistent = false;
                        break;
                    }
                }

                if consistent {
                    if let Some(pred) = const_val {
                        let target = if pred { *then_block } else { *else_block };
                        block.terminator = Some(MirTerminator::Goto(target));
                        changed = true;
                    }
                }
            }
        }
    }

    changed
}

fn collect_vars(func: &MirFunction) -> Vec<MirValue> {
    let mut vars = BTreeSet::new();
    for &param in &func.params {
        vars.insert(param);
    }

    for block in &func.blocks {
        for stmt in &block.statements {
            vars.insert(stmt.target);
            match &stmt.rhs {
                MirRhs::Literal(_) => {}
                MirRhs::Unary(_, operand) => {
                    vars.insert(*operand);
                }
                MirRhs::Binary(_, left, right) => {
                    vars.insert(*left);
                    vars.insert(*right);
                }
                MirRhs::Call { args, .. } => {
                    for arg in args {
                        vars.insert(*arg);
                    }
                }
                MirRhs::Load { addr, .. } => {
                    vars.insert(*addr);
                }
                MirRhs::Store { addr, val, .. } => {
                    vars.insert(*addr);
                    vars.insert(*val);
                }
            }
        }

        if let Some(term) = &block.terminator {
            match term {
                MirTerminator::Branch { cond, .. } => {
                    vars.insert(*cond);
                }
                MirTerminator::Return(Some(val)) => {
                    vars.insert(*val);
                }
                _ => {}
            }
        }
    }

    vars.into_iter().collect()
}

fn init_env(vars: &[MirValue]) -> BTreeMap<MirValue, ConstVal> {
    let mut env = BTreeMap::new();
    for &var in vars {
        env.insert(var, ConstVal::Unknown);
    }
    env
}

fn transfer_block(
    in_map: &BTreeMap<MirValue, ConstVal>,
    block: &MirBlock,
) -> BTreeMap<MirValue, ConstVal> {
    let mut out = in_map.clone();
    for stmt in &block.statements {
        let val = evaluate_rhs(&stmt.rhs, &out);
        out.insert(stmt.target, val);
    }
    out
}

fn evaluate_rhs(rhs: &MirRhs, env: &BTreeMap<MirValue, ConstVal>) -> ConstVal {
    match rhs {
        MirRhs::Literal(lit) => ConstVal::Const(lit.clone()),
        MirRhs::Unary(op, operand) => match env.get(operand) {
            Some(ConstVal::Const(value)) => {
                if let Some(result) = eval_unary(*op, value) {
                    ConstVal::Const(result)
                } else {
                    ConstVal::Overdefined
                }
            }
            Some(ConstVal::Overdefined) => ConstVal::Overdefined,
            _ => ConstVal::Unknown,
        },
        MirRhs::Binary(op, left, right) => {
            let left_val = env.get(left).cloned().unwrap_or(ConstVal::Unknown);
            let right_val = env.get(right).cloned().unwrap_or(ConstVal::Unknown);
            match (left_val, right_val) {
                (ConstVal::Const(a), ConstVal::Const(b)) => {
                    if let Some(result) = eval_binary(*op, &a, &b) {
                        ConstVal::Const(result)
                    } else {
                        ConstVal::Overdefined
                    }
                }
                (ConstVal::Overdefined, _) | (_, ConstVal::Overdefined) => ConstVal::Overdefined,
                _ => ConstVal::Unknown,
            }
        }
        MirRhs::Call { .. } => ConstVal::Overdefined,
        MirRhs::Load { .. } => ConstVal::Overdefined, // loads are not const-foldable
        MirRhs::Store { .. } => ConstVal::Overdefined, // stores have side effects
    }
}

fn literal_as_bool(value: &Literal) -> Option<bool> {
    match value {
        Literal::Bool(b) => Some(*b),
        Literal::Integer(i) => match i {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use x3_common::{Literal, Span};
    use x3_mir::{MirBlock, MirFunction, MirStatement, SymbolId};

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
    fn edge_constants_track_predicate() {
        let block0 = mk_block(
            0,
            vec![MirStatement {
                target: MirValue(0),
                rhs: MirRhs::Literal(Literal::Bool(true)),
            }],
            MirTerminator::Branch {
                cond: MirValue(0),
                then_block: MirBlockId(1),
                else_block: MirBlockId(2),
            },
        );
        let block1 = mk_block(1, vec![], MirTerminator::Return(None));
        let block2 = mk_block(2, vec![], MirTerminator::Return(None));
        let function = mk_function(vec![block0, block1, block2]);

        let edge_consts = compute_edge_constants(&function);
        let then_const = edge_consts.get(&(MirBlockId(0), MirBlockId(1))).unwrap();
        assert_eq!(
            then_const.get(&MirValue(0)),
            Some(&ConstVal::Const(Literal::Bool(true)))
        );
        let else_const = edge_consts.get(&(MirBlockId(0), MirBlockId(2))).unwrap();
        assert_eq!(
            else_const.get(&MirValue(0)),
            Some(&ConstVal::Const(Literal::Bool(true)))
        );
    }

    #[test]
    fn fold_branch_on_edge_consts_round_trip() {
        let make_blocks = || {
            vec![
                mk_block(
                    0,
                    vec![MirStatement {
                        target: MirValue(0),
                        rhs: MirRhs::Literal(Literal::Bool(true)),
                    }],
                    MirTerminator::Branch {
                        cond: MirValue(0),
                        then_block: MirBlockId(1),
                        else_block: MirBlockId(2),
                    },
                ),
                mk_block(1, vec![], MirTerminator::Return(None)),
                mk_block(2, vec![], MirTerminator::Return(None)),
            ]
        };

        let edge_consts = compute_edge_constants(&mk_function(make_blocks()));
        let mut function = mk_function(make_blocks());
        let changed = fold_branch_on_edge_consts(&mut function, &edge_consts);
        assert!(changed);
        match function.blocks[0].terminator {
            Some(MirTerminator::Goto(target)) => assert_eq!(target, MirBlockId(1)),
            _ => panic!("expected goto"),
        }
    }

    #[test]
    fn meet_over_preds_returns_overdefined() {
        let make_blocks = || {
            vec![
                mk_block(
                    0,
                    vec![MirStatement {
                        target: MirValue(0),
                        rhs: MirRhs::Literal(Literal::Bool(true)),
                    }],
                    MirTerminator::Goto(MirBlockId(2)),
                ),
                mk_block(
                    1,
                    vec![MirStatement {
                        target: MirValue(0),
                        rhs: MirRhs::Literal(Literal::Bool(false)),
                    }],
                    MirTerminator::Goto(MirBlockId(2)),
                ),
                mk_block(
                    2,
                    vec![],
                    MirTerminator::Branch {
                        cond: MirValue(0),
                        then_block: MirBlockId(3),
                        else_block: MirBlockId(4),
                    },
                ),
                mk_block(3, vec![], MirTerminator::Return(None)),
                mk_block(4, vec![], MirTerminator::Return(None)),
            ]
        };

        let edge_consts = compute_edge_constants(&mk_function(make_blocks()));
        let input = edge_consts.get(&(MirBlockId(0), MirBlockId(2))).unwrap();
        assert_eq!(
            input.get(&MirValue(0)),
            Some(&ConstVal::Const(Literal::Bool(true)))
        );
        let input2 = edge_consts.get(&(MirBlockId(1), MirBlockId(2))).unwrap();
        assert_eq!(
            input2.get(&MirValue(0)),
            Some(&ConstVal::Const(Literal::Bool(false)))
        );

        let mut function = mk_function(make_blocks());
        let changed = fold_branch_on_edge_consts(&mut function, &edge_consts);
        assert!(!changed);
        match function.blocks[2].terminator {
            Some(MirTerminator::Branch { .. }) => {}
            _ => panic!("expected branch"),
        }
    }
}
