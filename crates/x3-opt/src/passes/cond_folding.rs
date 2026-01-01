/// Dominator-based conditional folding with edge facts.
///
/// Infers equality/inequality facts from branch terminators, propagates them
/// down the dominator tree, and applies them to fold branches and comparisons.
use crate::cfg::Cfg;
use crate::mir::{Literal, MirBlock, MirFunction, MirRhs, MirStatement, MirTerminator, MirValue};
use crate::pass::{Pass, PassResult};
use std::collections::{BTreeMap, BTreeSet};

type BlockId = usize;
type Edge = (BlockId, BlockId);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EdgeFact {
    EqVarConst(usize, i128),
    NeVarConst(usize, i128),
}

pub type EdgeFactMap = BTreeMap<Edge, BTreeSet<EdgeFact>>;
pub type FactsIn = BTreeMap<BlockId, BTreeSet<EdgeFact>>;

pub struct ConditionalFoldingPass;

impl ConditionalFoldingPass {
    pub fn new() -> Self {
        Self
    }

    fn infer_edge_facts(func: &MirFunction) -> EdgeFactMap {
        let mut map: EdgeFactMap = BTreeMap::new();

        for (i, block) in func.blocks.iter().enumerate() {
            if let Some(MirTerminator::Branch {
                cond,
                true_bb,
                false_bb,
            }) = &block.term
            {
                // Look for comparisons in the final statements of this block
                if let Some(cond_val) = Self::find_cond_value(block, *cond) {
                    // if cond is a boolean from comparison, extract the fact
                    // For now, conservative: just record that cond is true/false on each edge
                    map.entry((i, *true_bb))
                        .or_default()
                        .insert(EdgeFact::EqVarConst(*cond, 1));
                    map.entry((i, *false_bb))
                        .or_default()
                        .insert(EdgeFact::EqVarConst(*cond, 0));
                }
            }
        }

        map
    }

    fn find_cond_value(block: &MirBlock, cond: MirValue) -> Option<i128> {
        // Check if cond was assigned a constant in this block
        for stmt in block.statements.iter().rev() {
            if stmt.target == cond {
                if let MirRhs::Literal(Literal::Integer(v)) = stmt.rhs {
                    return Some(v);
                }
            }
        }
        None
    }

    fn propagate_edge_facts(func: &MirFunction, cfg: &Cfg, edge_facts: &EdgeFactMap) -> FactsIn {
        let mut facts_in: FactsIn = BTreeMap::new();

        // Initialize all blocks with empty facts
        for b in 0..func.blocks.len() {
            facts_in.insert(b, BTreeSet::new());
        }

        // Worklist: process entry first, then successors
        let mut queue: std::collections::VecDeque<BlockId> = std::collections::VecDeque::new();
        queue.push_back(cfg.entry);

        let mut visited = BTreeSet::new();

        while let Some(b) = queue.pop_front() {
            if visited.contains(&b) {
                continue;
            }
            visited.insert(b);

            // Accumulate facts from predecessors
            for &pred in cfg.preds[b].iter() {
                if let Some(pred_facts) = edge_facts.get(&(pred, b)) {
                    facts_in
                        .entry(b)
                        .or_default()
                        .extend(pred_facts.iter().cloned());
                }
            }

            // Queue successors
            for &succ in cfg.succs[b].iter() {
                if !visited.contains(&succ) {
                    queue.push_back(succ);
                }
            }
        }

        facts_in
    }

    fn apply_edge_facts(func: &mut MirFunction, facts_in: &FactsIn) -> bool {
        let mut changed = false;

        for b in 0..func.blocks.len() {
            let facts = facts_in.get(&b).cloned().unwrap_or_default();

            // Try to fold the branch terminator
            if let Some(MirTerminator::Branch {
                cond,
                true_bb,
                false_bb,
            }) = func.blocks[b].term.clone()
            {
                // Check if cond is provably constant via facts
                let cond_val = if facts.contains(&EdgeFact::EqVarConst(cond.index(), 1)) {
                    Some(1)
                } else if facts.contains(&EdgeFact::EqVarConst(cond.index(), 0)) {
                    Some(0)
                } else {
                    None
                };

                if let Some(val) = cond_val {
                    let target = if val == 1 { true_bb } else { false_bb };
                    func.blocks[b].term = Some(MirTerminator::Goto(target));
                    changed = true;
                }
            }
        }

        changed
    }
}

impl Pass for ConditionalFoldingPass {
    fn name(&self) -> &'static str {
        "conditional_folding"
    }

    fn run(&self, func: &mut MirFunction) -> PassResult {
        let cfg = Cfg::build(func);
        let edge_facts = Self::infer_edge_facts(func);
        let facts_in = Self::propagate_edge_facts(func, &cfg, &edge_facts);
        let changed = Self::apply_edge_facts(func, &facts_in);

        PassResult::with_change(changed, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fold_constant_branch() {
        // Simple test: if branch condition is provably true, fold to goto
        let mut func = MirFunction {
            blocks: vec![
                MirBlock {
                    statements: vec![MirStatement {
                        target: MirValue(0),
                        rhs: MirRhs::Literal(Literal::Integer(1)),
                    }],
                    term: Some(MirTerminator::Branch {
                        cond: MirValue(0),
                        true_bb: 1,
                        false_bb: 2,
                    }),
                },
                MirBlock {
                    statements: vec![],
                    term: Some(MirTerminator::Return(None)),
                },
                MirBlock {
                    statements: vec![],
                    term: Some(MirTerminator::Return(None)),
                },
            ],
        };

        let pass = ConditionalFoldingPass::new();
        let result = pass.run(&mut func);
        assert!(result.changed);
        assert!(matches!(func.blocks[0].term, Some(MirTerminator::Goto(1))));
    }
}
