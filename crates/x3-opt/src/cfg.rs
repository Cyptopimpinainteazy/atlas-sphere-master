//! Control Flow Graph (CFG) and Dominator Analysis
//!
//! Provides deterministic CFG construction and dominator tree computation
//! using an iterative dataflow algorithm.
//!
//! # Design Decisions
//!
//! - **Deterministic**: Uses `BTreeMap` and `BTreeSet` for reproducible iteration order
//! - **Simple Algorithm**: Iterative dominator computation (not Lengauer-Tarjan) for clarity
//! - **No SSA Required**: Works directly on MIR basic blocks
//!
//! # Example
//!
//! ```ignore
//! use x3_opt::cfg::Cfg;
//!
//! let cfg = Cfg::from_function(&mir_func);
//! let (idom, dom_tree) = cfg.compute_dominators();
//!
//! // idom[block] = immediate dominator of block
//! // dom_tree[block] = children dominated by block
//! ```

use std::collections::{BTreeMap, BTreeSet};
use x3_mir::{MirBlockId, MirFunction, MirTerminator};

/// Control Flow Graph for a single function.
///
/// All collections use `BTreeMap`/`BTreeSet` for deterministic iteration order,
/// which is critical for reproducible optimization results.
#[derive(Debug, Clone)]
pub struct Cfg {
    /// Successors of each block.
    pub succs: BTreeMap<MirBlockId, BTreeSet<MirBlockId>>,
    /// Predecessors of each block.
    pub preds: BTreeMap<MirBlockId, BTreeSet<MirBlockId>>,
    /// Entry block.
    pub entry: MirBlockId,
    /// All block IDs in the function.
    pub blocks: BTreeSet<MirBlockId>,
}

impl Cfg {
    /// Build a CFG from a MIR function.
    ///
    /// Extracts successor/predecessor relationships from block terminators.
    pub fn from_function(func: &MirFunction) -> Self {
        let mut succs: BTreeMap<MirBlockId, BTreeSet<MirBlockId>> = BTreeMap::new();
        let mut preds: BTreeMap<MirBlockId, BTreeSet<MirBlockId>> = BTreeMap::new();
        let mut blocks = BTreeSet::new();

        // Initialize all blocks
        for block in &func.blocks {
            blocks.insert(block.id);
            succs.entry(block.id).or_default();
            preds.entry(block.id).or_default();
        }

        // Build edges from terminators
        for block in &func.blocks {
            let src = block.id;
            if let Some(term) = &block.terminator {
                match term {
                    MirTerminator::Goto(target) => {
                        succs.entry(src).or_default().insert(*target);
                        preds.entry(*target).or_default().insert(src);
                    }
                    MirTerminator::Branch {
                        then_block,
                        else_block,
                        ..
                    } => {
                        succs.entry(src).or_default().insert(*then_block);
                        succs.entry(src).or_default().insert(*else_block);
                        preds.entry(*then_block).or_default().insert(src);
                        preds.entry(*else_block).or_default().insert(src);
                    }
                    MirTerminator::Return(_) => {
                        // No successors
                    }
                }
            }
        }

        Cfg {
            succs,
            preds,
            entry: func.entry,
            blocks,
        }
    }

    /// Compute dominator tree using iterative dataflow algorithm.
    ///
    /// Returns:
    /// - `idom`: Map from block to its immediate dominator (entry has no idom)
    /// - `dom_tree`: Map from block to its dominated children
    ///
    /// # Algorithm
    ///
    /// Uses the iterative dominator algorithm:
    /// 1. Initialize dom[entry] = {entry}, dom[other] = all_blocks
    /// 2. Iterate until fixpoint: dom[n] = {n} ∪ (∩ dom[p] for p in preds[n])
    /// 3. Extract immediate dominators from dominator sets
    ///
    /// Complexity: O(n² × edges) worst case, but typically converges quickly.
    pub fn compute_dominators(
        &self,
    ) -> (
        BTreeMap<MirBlockId, MirBlockId>,
        BTreeMap<MirBlockId, BTreeSet<MirBlockId>>,
    ) {
        if self.blocks.is_empty() {
            return (BTreeMap::new(), BTreeMap::new());
        }

        // Initialize dominator sets
        // dom[entry] = {entry}, dom[other] = all_blocks
        let mut dom: BTreeMap<MirBlockId, BTreeSet<MirBlockId>> = BTreeMap::new();

        for &block in &self.blocks {
            if block == self.entry {
                let mut entry_dom = BTreeSet::new();
                entry_dom.insert(self.entry);
                dom.insert(block, entry_dom);
            } else {
                dom.insert(block, self.blocks.clone());
            }
        }

        // Iterate until fixpoint
        let mut changed = true;
        while changed {
            changed = false;

            for &block in &self.blocks {
                if block == self.entry {
                    continue;
                }

                // new_dom = {block} ∪ (∩ dom[p] for p in preds[block])
                let preds = self
                    .preds
                    .get(&block)
                    .map(|p| p.iter().cloned().collect::<Vec<_>>())
                    .unwrap_or_default();

                let new_dom = if preds.is_empty() {
                    // Unreachable block - only dominates itself
                    let mut s = BTreeSet::new();
                    s.insert(block);
                    s
                } else {
                    // Intersect all predecessor dominator sets
                    let mut new_dom = dom.get(&preds[0]).cloned().unwrap_or_default();
                    for pred in preds.iter().skip(1) {
                        if let Some(pred_dom) = dom.get(pred) {
                            new_dom = intersect(&new_dom, pred_dom);
                        }
                    }
                    new_dom.insert(block);
                    new_dom
                };

                if dom.get(&block) != Some(&new_dom) {
                    dom.insert(block, new_dom);
                    changed = true;
                }
            }
        }

        // Extract immediate dominators from dominator sets
        // idom[n] = the dominator of n that is dominated by all other dominators of n
        let mut idom: BTreeMap<MirBlockId, MirBlockId> = BTreeMap::new();
        let mut dom_tree: BTreeMap<MirBlockId, BTreeSet<MirBlockId>> = BTreeMap::new();

        // Initialize dom_tree for all blocks
        for &block in &self.blocks {
            dom_tree.insert(block, BTreeSet::new());
        }

        for &block in &self.blocks {
            if block == self.entry {
                continue; // Entry has no idom
            }

            let block_dom = match dom.get(&block) {
                Some(d) => d,
                None => continue,
            };

            // idom is the dominator closest to block (i.e., dominated by all others except block)
            // It's the element d in dom[block] - {block} where |dom[d]| is maximized
            let mut best_idom: Option<MirBlockId> = None;
            let mut best_dom_size = 0;

            for &candidate in block_dom {
                if candidate == block {
                    continue;
                }
                let candidate_dom_size = dom.get(&candidate).map(|d| d.len()).unwrap_or(0);
                if candidate_dom_size > best_dom_size {
                    best_dom_size = candidate_dom_size;
                    best_idom = Some(candidate);
                }
            }

            if let Some(immediate_dom) = best_idom {
                idom.insert(block, immediate_dom);
                dom_tree.entry(immediate_dom).or_default().insert(block);
            }
        }

        (idom, dom_tree)
    }

    /// Check if block `a` dominates block `b`.
    ///
    /// Uses the dominator sets computed by `compute_dominators`.
    pub fn dominates(
        &self,
        a: MirBlockId,
        b: MirBlockId,
        idom: &BTreeMap<MirBlockId, MirBlockId>,
    ) -> bool {
        if a == b {
            return true;
        }

        // Walk up the dominator tree from b
        let mut current = b;
        while let Some(&parent) = idom.get(&current) {
            if parent == a {
                return true;
            }
            current = parent;
        }
        false
    }
}

/// Compute intersection of two BTreeSets.
fn intersect(a: &BTreeSet<MirBlockId>, b: &BTreeSet<MirBlockId>) -> BTreeSet<MirBlockId> {
    a.intersection(b).cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use x3_common::Span;
    use x3_mir::{MirBlock, MirFunction, SymbolId};

    fn make_block(id: usize, terminator: Option<MirTerminator>) -> MirBlock {
        MirBlock {
            id: MirBlockId(id),
            statements: vec![],
            terminator,
        }
    }

    #[test]
    fn cfg_linear() {
        // bb0 -> bb1 -> bb2 (return)
        let func = MirFunction {
            symbol: SymbolId(0),
            params: vec![],
            entry: MirBlockId(0),
            blocks: vec![
                make_block(0, Some(MirTerminator::Goto(MirBlockId(1)))),
                make_block(1, Some(MirTerminator::Goto(MirBlockId(2)))),
                make_block(2, Some(MirTerminator::Return(None))),
            ],
            span: Span::dummy(),
        };

        let cfg = Cfg::from_function(&func);
        assert_eq!(cfg.blocks.len(), 3);
        assert_eq!(cfg.succs[&MirBlockId(0)].len(), 1);
        assert!(cfg.succs[&MirBlockId(0)].contains(&MirBlockId(1)));

        let (idom, dom_tree) = cfg.compute_dominators();

        // bb0 dominates all, bb1 dominates bb2
        assert_eq!(idom.get(&MirBlockId(1)), Some(&MirBlockId(0)));
        assert_eq!(idom.get(&MirBlockId(2)), Some(&MirBlockId(1)));

        // dom_tree[bb0] = {bb1}, dom_tree[bb1] = {bb2}
        assert!(dom_tree[&MirBlockId(0)].contains(&MirBlockId(1)));
        assert!(dom_tree[&MirBlockId(1)].contains(&MirBlockId(2)));
    }

    #[test]
    fn cfg_diamond() {
        // bb0 branches to bb1 and bb2, both go to bb3
        //      bb0
        //     /   \
        //   bb1   bb2
        //     \   /
        //      bb3
        use x3_mir::MirValue;

        let func = MirFunction {
            symbol: SymbolId(0),
            params: vec![],
            entry: MirBlockId(0),
            blocks: vec![
                make_block(
                    0,
                    Some(MirTerminator::Branch {
                        cond: MirValue(0),
                        then_block: MirBlockId(1),
                        else_block: MirBlockId(2),
                    }),
                ),
                make_block(1, Some(MirTerminator::Goto(MirBlockId(3)))),
                make_block(2, Some(MirTerminator::Goto(MirBlockId(3)))),
                make_block(3, Some(MirTerminator::Return(None))),
            ],
            span: Span::dummy(),
        };

        let cfg = Cfg::from_function(&func);

        // Check successors
        assert_eq!(cfg.succs[&MirBlockId(0)].len(), 2);
        assert!(cfg.succs[&MirBlockId(0)].contains(&MirBlockId(1)));
        assert!(cfg.succs[&MirBlockId(0)].contains(&MirBlockId(2)));

        // Check predecessors
        assert_eq!(cfg.preds[&MirBlockId(3)].len(), 2);

        let (idom, _dom_tree) = cfg.compute_dominators();

        // bb0 dominates all others
        assert_eq!(idom.get(&MirBlockId(1)), Some(&MirBlockId(0)));
        assert_eq!(idom.get(&MirBlockId(2)), Some(&MirBlockId(0)));
        // bb3's idom is bb0 (the only common dominator of bb1 and bb2)
        assert_eq!(idom.get(&MirBlockId(3)), Some(&MirBlockId(0)));
    }

    #[test]
    fn cfg_empty_function() {
        let func = MirFunction {
            symbol: SymbolId(0),
            params: vec![],
            entry: MirBlockId(0),
            blocks: vec![],
            span: Span::dummy(),
        };

        let cfg = Cfg::from_function(&func);
        assert!(cfg.blocks.is_empty());

        let (idom, dom_tree) = cfg.compute_dominators();
        assert!(idom.is_empty());
        assert!(dom_tree.is_empty());
    }

    #[test]
    fn dominates_check() {
        // Linear: bb0 -> bb1 -> bb2
        let func = MirFunction {
            symbol: SymbolId(0),
            params: vec![],
            entry: MirBlockId(0),
            blocks: vec![
                make_block(0, Some(MirTerminator::Goto(MirBlockId(1)))),
                make_block(1, Some(MirTerminator::Goto(MirBlockId(2)))),
                make_block(2, Some(MirTerminator::Return(None))),
            ],
            span: Span::dummy(),
        };

        let cfg = Cfg::from_function(&func);
        let (idom, _) = cfg.compute_dominators();

        // bb0 dominates everything
        assert!(cfg.dominates(MirBlockId(0), MirBlockId(0), &idom));
        assert!(cfg.dominates(MirBlockId(0), MirBlockId(1), &idom));
        assert!(cfg.dominates(MirBlockId(0), MirBlockId(2), &idom));

        // bb1 dominates bb2
        assert!(cfg.dominates(MirBlockId(1), MirBlockId(2), &idom));

        // bb2 doesn't dominate bb1
        assert!(!cfg.dominates(MirBlockId(2), MirBlockId(1), &idom));
    }
}
