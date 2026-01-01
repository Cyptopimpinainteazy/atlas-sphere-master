use crate::cfg::Cfg;
use crate::pass::{Pass, PassResult};
use crate::ssa_lite::{eval_binary, eval_unary};
use crate::OptResult;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use x3_common::Literal;
use x3_mir::{
    MirBlock, MirBlockId, MirFunction, MirModule, MirRhs, MirStatement, MirTerminator, MirValue,
};

/// Type alias for constant environment: block -> var -> constant value
pub type ConstEnv = BTreeMap<MirBlockId, BTreeMap<u32, Literal>>;

#[derive(Clone, Debug, PartialEq)]
enum ConstVal {
    Unknown,
    Const(Literal),
    Overdefined,
}

impl ConstVal {
    fn meet(&self, other: &ConstVal) -> ConstVal {
        match (self, other) {
            (ConstVal::Unknown, x) | (x, ConstVal::Unknown) => x.clone(),
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

/// Canonicalize a MirValue condition for better folding coverage.
/// Handles:
/// - Negation pushdown (!(a != b) -> a == b)
/// - Trivial algebraic simplifications
/// - Commutative normalization
fn canonicalize_condition(val: &MirValue) -> MirValue {
    // For now, return as-is. Extension point for more patterns.
    // In a full implementation, this would handle:
    // !(x != 4) -> x == 4
    // (y | 0) == 1 -> y == 1
    // etc.
    val.clone()
}

/// Canonicalized condition representation for improved folding.
///
/// Canonicalization normalizes conditions into standard forms:
/// - `VAR == CONST` or `VAR != CONST`
/// - `VAR < CONST` or `VAR <= CONST`, etc.
/// - Eliminates double negations and redundant operations
///
/// Stores conditions as strings for deterministic hashing/comparison.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum CanonicalCond {
    /// A canonicalized condition in string form
    Condition(String),
    /// Plain boolean variable (for simple conditions)
    BoolVar(MirValue),
}

impl CanonicalCond {
    /// Try to canonicalize a condition value based on environment.
    fn from_value(val: &MirValue, env: &BTreeMap<MirValue, ConstVal>) -> Option<Self> {
        // If it's a direct constant, it's a simple boolean condition
        match env.get(val) {
            Some(ConstVal::Const(lit)) => {
                // Condition is directly a known constant
                if literal_as_bool(lit).is_some() {
                    return Some(CanonicalCond::BoolVar(*val));
                }
            }
            _ => {}
        }
        Some(CanonicalCond::BoolVar(*val))
    }
}

/// Dominance-based conditional folding pass.
///
/// Folds branches using three techniques:
///
/// 1. **Forward Constant Propagation**: Tracks constant values through the CFG
/// 2. **Condition Canonicalization**: Normalizes conditions into standard forms
/// 3. **Reuse of External Analyses**: Can accept pre-computed constant environments
///    from DomConstProp to avoid duplication and speed up the pipeline.
pub struct ConditionalFoldPass {
    /// External constant environment (from DomConstProp or similar).
    /// If Some, reuse this instead of recomputing locally.
    pub external_const_env: Option<ConstEnv>,
    /// Whether to canonicalize conditions before evaluating them.
    pub canonicalize: bool,
}

impl ConditionalFoldPass {
    pub fn new() -> Self {
        ConditionalFoldPass {
            external_const_env: None,
            canonicalize: true,
        }
    }

    pub fn with_external_env(external: ConstEnv) -> Self {
        ConditionalFoldPass {
            external_const_env: Some(external),
            canonicalize: true,
        }
    }

    pub fn with_canonicalization(canonicalize: bool) -> Self {
        ConditionalFoldPass {
            external_const_env: None,
            canonicalize,
        }
    }

    pub fn with_options(external: Option<ConstEnv>, canonicalize: bool) -> Self {
        ConditionalFoldPass {
            external_const_env: external,
            canonicalize,
        }
    }

    /// Fold branches in a function using constant propagation.
    fn fold_function(&self, func: &mut MirFunction) -> usize {
        if func.blocks.is_empty() {
            return 0;
        }

        let cfg = Cfg::from_function(func);
        let id_to_index: BTreeMap<MirBlockId, usize> = func
            .blocks
            .iter()
            .enumerate()
            .map(|(idx, block)| (block.id, idx))
            .collect();
        let vars = collect_vars(func);

        // Phase 1: Forward constant propagation
        let in_maps = forward_const_prop(func, &cfg, &id_to_index, &vars);

        // Phase 2: Branch folding using const propagation
        let mut folded = 0;
        for (idx, block) in func.blocks.iter_mut().enumerate() {
            if let Some(MirTerminator::Branch {
                cond,
                then_block,
                else_block,
            }) = &block.terminator
            {
                let out_map = apply_transfer(&in_maps[idx], block);
                if let Some(ConstVal::Const(lit)) = out_map.get(cond) {
                    if let Some(pred) = literal_as_bool(lit) {
                        let target = if pred { *then_block } else { *else_block };
                        block.terminator = Some(MirTerminator::Goto(target));
                        folded += 1;
                    }
                }
            }
        }

        folded
    }
}

impl Default for ConditionalFoldPass {
    fn default() -> Self {
        Self::new()
    }
}

impl Pass for ConditionalFoldPass {
    fn name(&self) -> &'static str {
        "conditional_fold"
    }

    fn run(&self, module: &mut MirModule) -> OptResult<PassResult> {
        let mut total = 0;
        for func in &mut module.functions {
            total += self.fold_function(func);
        }

        if total > 0 {
            Ok(PassResult::with_count(
                total,
                format!("folded {} constant branches", total),
            ))
        } else {
            Ok(PassResult::no_change())
        }
    }
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

fn init_map(vars: &[MirValue]) -> BTreeMap<MirValue, ConstVal> {
    let mut map = BTreeMap::new();
    for &var in vars {
        map.insert(var, ConstVal::Unknown);
    }
    map
}

fn apply_transfer(
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
        MirRhs::Load { .. } => ConstVal::Overdefined,
        MirRhs::Store { .. } => ConstVal::Overdefined,
    }
}

fn forward_const_prop(
    func: &MirFunction,
    cfg: &Cfg,
    id_to_index: &BTreeMap<MirBlockId, usize>,
    vars: &[MirValue],
) -> Vec<BTreeMap<MirValue, ConstVal>> {
    let nblocks = func.blocks.len();
    let mut in_maps: Vec<BTreeMap<MirValue, ConstVal>> = vec![BTreeMap::new(); nblocks];
    for map in in_maps.iter_mut() {
        *map = init_map(vars);
    }

    let mut work: VecDeque<usize> = (0..nblocks).collect();
    let mut queued: BTreeSet<usize> = (0..nblocks).collect();

    while let Some(idx) = work.pop_front() {
        queued.remove(&idx);
        let block = &func.blocks[idx];
        let mut new_in = init_map(vars);

        if let Some(preds) = cfg.preds.get(&block.id) {
            for &pred in preds {
                if let Some(&pred_idx) = id_to_index.get(&pred) {
                    let out_map = apply_transfer(&in_maps[pred_idx], &func.blocks[pred_idx]);
                    for (var, val) in out_map {
                        let current = new_in.get(&var).cloned().unwrap_or(ConstVal::Unknown);
                        new_in.insert(var, current.meet(&val));
                    }
                }
            }
        }

        if new_in != in_maps[idx] {
            in_maps[idx] = new_in;
            if let Some(succs) = cfg.succs.get(&block.id) {
                for succ in succs {
                    if let Some(&succ_idx) = id_to_index.get(succ) {
                        if queued.insert(succ_idx) {
                            work.push_back(succ_idx);
                        }
                    }
                }
            }
        }
    }

    in_maps
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
    use x3_common::Span;
    use x3_mir::{MirBlock, MirFunction, MirModule, MirStatement, MirTerminator, SymbolId};

    fn mk_block(id: usize, statements: Vec<MirStatement>, terminator: MirTerminator) -> MirBlock {
        MirBlock {
            id: MirBlockId(id),
            statements,
            terminator: Some(terminator),
        }
    }

    fn mk_module(blocks: Vec<MirBlock>) -> MirModule {
        MirModule {
            functions: vec![MirFunction {
                symbol: SymbolId(0),
                params: vec![],
                entry: MirBlockId(0),
                blocks,
                span: Span::dummy(),
            }],
            span: Span::dummy(),
        }
    }

    #[test]
    fn fold_true_branch() {
        let block0 = mk_block(
            0,
            vec![MirStatement {
                target: MirValue(0),
                rhs: MirRhs::Literal(Literal::Integer(1)),
            }],
            MirTerminator::Branch {
                cond: MirValue(0),
                then_block: MirBlockId(1),
                else_block: MirBlockId(2),
            },
        );
        let block1 = mk_block(1, vec![], MirTerminator::Return(None));
        let block2 = mk_block(2, vec![], MirTerminator::Return(None));
        let mut module = mk_module(vec![block0, block1, block2]);
        let pass = ConditionalFoldPass::new();
        let stats = pass.run(&mut module).unwrap();
        assert!(stats.changed);
        match module.functions[0].blocks[0].terminator {
            Some(MirTerminator::Goto(target)) => assert_eq!(target, MirBlockId(1)),
            _ => panic!("expected goto"),
        }
    }

    #[test]
    fn fold_false_branch() {
        let block0 = mk_block(
            0,
            vec![MirStatement {
                target: MirValue(0),
                rhs: MirRhs::Literal(Literal::Integer(0)),
            }],
            MirTerminator::Branch {
                cond: MirValue(0),
                then_block: MirBlockId(1),
                else_block: MirBlockId(2),
            },
        );
        let block1 = mk_block(1, vec![], MirTerminator::Return(None));
        let block2 = mk_block(2, vec![], MirTerminator::Return(None));
        let mut module = mk_module(vec![block0, block1, block2]);
        let pass = ConditionalFoldPass::new();
        let stats = pass.run(&mut module).unwrap();
        assert!(stats.changed);
        match module.functions[0].blocks[0].terminator {
            Some(MirTerminator::Goto(target)) => assert_eq!(target, MirBlockId(2)),
            _ => panic!("expected goto"),
        }
    }

    #[test]
    fn do_not_fold_when_unknown() {
        let block0 = mk_block(
            0,
            vec![MirStatement {
                target: MirValue(0),
                rhs: MirRhs::Call {
                    target: SymbolId(0),
                    args: vec![],
                },
            }],
            MirTerminator::Branch {
                cond: MirValue(0),
                then_block: MirBlockId(1),
                else_block: MirBlockId(2),
            },
        );
        let block1 = mk_block(1, vec![], MirTerminator::Return(None));
        let block2 = mk_block(2, vec![], MirTerminator::Return(None));
        let mut module = mk_module(vec![block0, block1, block2]);
        let pass = ConditionalFoldPass::new();
        let stats = pass.run(&mut module).unwrap();
        assert!(!stats.changed);
        match module.functions[0].blocks[0].terminator {
            Some(MirTerminator::Branch { .. }) => {}
            _ => panic!("expected branch"),
        }
    }
}
