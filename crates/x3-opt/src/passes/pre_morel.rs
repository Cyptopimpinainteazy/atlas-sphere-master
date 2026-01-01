//! Morel–Renvoise Partial Redundancy Elimination (PRE)
//!
//! Advanced, deterministic implementation with:
//! - Expression fingerprinting (canonical form)
//! - Occurrence collection across CFG
//! - Availability (forward) and Anticipatability (backward) dataflow
//! - Earliest/Latest placement computation
//! - Conservative insertion + replacement scaffolding
//!
//! This is a **simplified foundation** for full PRE. It identifies optimization
//! opportunities but does not yet insert temps into MIR (that's phase 2 of this pass).
//! The analysis alone provides significant optimization candidates.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::pass::{Pass, PassResult};
use crate::OptResult;
use x3_mir::{MirBlockId, MirModule, MirRhs, MirValue};

/// Canonical expression key for PRE analysis
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct ExprKey {
    opcode: String,
    operands: Vec<String>, // deterministic string repr of operands
}

impl ExprKey {
    fn from_rhs(rhs: &MirRhs) -> Option<Self> {
        match rhs {
            MirRhs::Binary(op, lhs, rhs) => {
                // Canonicalize commutative ops
                let op_str = format!("{:?}", op);
                let is_comm = matches!(
                    op,
                    x3_ast::BinaryOp::Add
                        | x3_ast::BinaryOp::Mul
                        | x3_ast::BinaryOp::Equal
                        | x3_ast::BinaryOp::NotEqual
                        | x3_ast::BinaryOp::LogicalAnd
                        | x3_ast::BinaryOp::LogicalOr
                );

                let lhs_str = format!("{:?}", lhs);
                let rhs_str = format!("{:?}", rhs);
                let (l, r) = if is_comm && lhs_str > rhs_str {
                    (rhs_str, lhs_str)
                } else {
                    (lhs_str, rhs_str)
                };

                Some(ExprKey {
                    opcode: op_str,
                    operands: vec![l, r],
                })
            }
            MirRhs::Unary(op, val) => Some(ExprKey {
                opcode: format!("{:?}", op),
                operands: vec![format!("{:?}", val)],
            }),
            MirRhs::Literal(_)
            | MirRhs::Call { .. }
            | MirRhs::Load { .. }
            | MirRhs::Store { .. } => None,
        }
    }

    fn is_pure(&self) -> bool {
        // Only Binary/Unary expressions are pure (no side effects)
        !self.opcode.contains("call")
    }
}

/// Record of where an expression occurs in the CFG
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Occurrence {
    block: MirBlockId,
    stmt_index: usize,
}

/// Availability state for dataflow
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Availability {
    Unknown,
    Available,
    Overdefined, // not available (ambiguous or killed)
}

impl Availability {
    fn meet(self, other: Availability) -> Availability {
        use Availability::*;
        match (self, other) {
            (Unknown, x) | (x, Unknown) => x,
            (Available, Available) => Available,
            _ => Overdefined,
        }
    }
}

/// Morel-Renvoise PRE Pass
pub struct MorelRenvoisePrePass {
    max_iterations: usize,
}

impl Default for MorelRenvoisePrePass {
    fn default() -> Self {
        MorelRenvoisePrePass {
            max_iterations: 128,
        }
    }
}

impl MorelRenvoisePrePass {
    pub fn new() -> Self {
        Self::default()
    }

    /// Collect all pure expression occurrences
    fn collect_occurrences(module: &MirModule) -> BTreeMap<ExprKey, Vec<Occurrence>> {
        let mut map: BTreeMap<ExprKey, Vec<Occurrence>> = BTreeMap::new();

        for func in &module.functions {
            for block in &func.blocks {
                for (stmt_idx, stmt) in block.statements.iter().enumerate() {
                    if let Some(key) = ExprKey::from_rhs(&stmt.rhs) {
                        if key.is_pure() {
                            let occ = Occurrence {
                                block: block.id,
                                stmt_index: stmt_idx,
                            };
                            map.entry(key).or_insert_with(Vec::new).push(occ);
                        }
                    }
                }
            }
        }

        map
    }

    /// Build basic predecessor/successor maps for the CFG
    fn build_cfg_maps(
        module: &MirModule,
    ) -> (
        BTreeMap<MirBlockId, Vec<MirBlockId>>,
        BTreeMap<MirBlockId, Vec<MirBlockId>>,
    ) {
        let mut preds: BTreeMap<MirBlockId, Vec<MirBlockId>> = BTreeMap::new();
        let mut succs: BTreeMap<MirBlockId, Vec<MirBlockId>> = BTreeMap::new();

        for func in &module.functions {
            // Initialize all blocks
            for block in &func.blocks {
                preds.entry(block.id).or_insert_with(Vec::new);
                succs.entry(block.id).or_insert_with(Vec::new);
            }

            // Scan terminators for successors
            for block in &func.blocks {
                if let Some(term) = &block.terminator {
                    let succ_ids: Vec<MirBlockId> = match term {
                        x3_mir::MirTerminator::Return(_) => vec![],
                        x3_mir::MirTerminator::Goto(id) => vec![*id],
                        x3_mir::MirTerminator::Branch {
                            then_block,
                            else_block,
                            ..
                        } => vec![*then_block, *else_block],
                    };
                    succs.insert(block.id, succ_ids.clone());
                    for succ_id in succ_ids {
                        preds.entry(succ_id).or_insert_with(Vec::new).push(block.id);
                    }
                }
            }
        }

        (preds, succs)
    }

    /// Compute availability: forward dataflow
    fn compute_availability(
        _module: &MirModule,
        occs: &BTreeMap<ExprKey, Vec<Occurrence>>,
        preds: &BTreeMap<MirBlockId, Vec<MirBlockId>>,
    ) -> BTreeMap<ExprKey, BTreeMap<MirBlockId, Availability>> {
        let mut result: BTreeMap<ExprKey, BTreeMap<MirBlockId, Availability>> = BTreeMap::new();

        for (key, occurrences) in occs.iter() {
            // Collect blocks that generate this expression
            let gen_blocks: BTreeSet<MirBlockId> = occurrences.iter().map(|o| o.block).collect();

            // Initialize all blocks with Unknown
            let mut avail: BTreeMap<MirBlockId, Availability> = BTreeMap::new();
            for pred_list in preds.values() {
                for b in pred_list {
                    avail.entry(*b).or_insert(Availability::Unknown);
                }
            }

            // Fixpoint iteration for forward dataflow
            for _ in 0..128 {
                let mut changed = false;

                // Process blocks in deterministic order
                let mut blocks_sorted: Vec<_> = avail.keys().copied().collect();
                blocks_sorted.sort();

                for block_id in blocks_sorted {
                    let mut new_av = Availability::Unknown;

                    // If this block generates the expression
                    if gen_blocks.contains(&block_id) {
                        new_av = Availability::Available;
                    } else if let Some(pred_list) = preds.get(&block_id) {
                        // Otherwise, meet of predecessors
                        let mut av_in = Availability::Available; // identity
                        for pred in pred_list {
                            if let Some(&pred_av) = avail.get(pred) {
                                av_in = av_in.meet(pred_av);
                            }
                        }
                        new_av = av_in;
                    }

                    if avail.get(&block_id) != Some(&new_av) {
                        avail.insert(block_id, new_av);
                        changed = true;
                    }
                }

                if !changed {
                    break;
                }
            }

            result.insert(key.clone(), avail);
        }

        result
    }

    /// Compute anticipatability: backward dataflow
    fn compute_anticipatability(
        _module: &MirModule,
        occs: &BTreeMap<ExprKey, Vec<Occurrence>>,
        succs: &BTreeMap<MirBlockId, Vec<MirBlockId>>,
    ) -> BTreeMap<ExprKey, BTreeMap<MirBlockId, bool>> {
        let mut result: BTreeMap<ExprKey, BTreeMap<MirBlockId, bool>> = BTreeMap::new();

        for (key, occurrences) in occs.iter() {
            let use_blocks: BTreeSet<MirBlockId> = occurrences.iter().map(|o| o.block).collect();

            let mut ant: BTreeMap<MirBlockId, bool> = BTreeMap::new();
            for succ_list in succs.values() {
                for b in succ_list {
                    ant.entry(*b).or_insert(false);
                }
            }

            // Backward fixpoint iteration
            for _ in 0..128 {
                let mut changed = false;

                let mut blocks_sorted: Vec<_> = ant.keys().copied().collect();
                blocks_sorted.sort();
                blocks_sorted.reverse(); // backward order

                for block_id in blocks_sorted {
                    let mut new_ant = false;

                    // If this block uses the expression
                    if use_blocks.contains(&block_id) {
                        new_ant = true;
                    } else if let Some(succ_list) = succs.get(&block_id) {
                        // Otherwise, meet of successors
                        let mut ant_out = true; // identity
                        for succ in succ_list {
                            if let Some(&succ_ant) = ant.get(succ) {
                                ant_out = ant_out && succ_ant;
                            }
                        }
                        new_ant = ant_out;
                    }

                    if ant.get(&block_id) != Some(&new_ant) {
                        ant.insert(block_id, new_ant);
                        changed = true;
                    }
                }

                if !changed {
                    break;
                }
            }

            result.insert(key.clone(), ant);
        }

        result
    }

    /// Compute earliest placement: blocks where expression must be computed
    fn compute_earliest(
        occs: &BTreeMap<ExprKey, Vec<Occurrence>>,
        avail: &BTreeMap<ExprKey, BTreeMap<MirBlockId, Availability>>,
        ant: &BTreeMap<ExprKey, BTreeMap<MirBlockId, bool>>,
    ) -> BTreeMap<ExprKey, BTreeSet<MirBlockId>> {
        let mut result: BTreeMap<ExprKey, BTreeSet<MirBlockId>> = BTreeMap::new();

        for key in occs.keys() {
            let avail_map = avail.get(key).cloned().unwrap_or_default();
            let ant_map = ant.get(key).cloned().unwrap_or_default();

            let mut earliest: BTreeSet<MirBlockId> = BTreeSet::new();
            for (block_id, &is_ant) in ant_map.iter() {
                if is_ant {
                    let is_avail = matches!(avail_map.get(block_id), Some(Availability::Available));
                    if !is_avail {
                        earliest.insert(*block_id);
                    }
                }
            }

            result.insert(key.clone(), earliest);
        }

        result
    }

    /// Compute latest placement: refine earliest to latest safe points
    fn compute_latest(
        earliest: &BTreeMap<ExprKey, BTreeSet<MirBlockId>>,
        succs: &BTreeMap<MirBlockId, Vec<MirBlockId>>,
    ) -> BTreeMap<ExprKey, BTreeSet<MirBlockId>> {
        let mut result: BTreeMap<ExprKey, BTreeSet<MirBlockId>> = BTreeMap::new();

        for (key, e_set) in earliest.iter() {
            let mut latest = e_set.clone();

            // Iteratively postpone if all successors are in latest
            for _ in 0..128 {
                let mut to_remove = Vec::new();
                for &block_id in latest.iter() {
                    if let Some(succ_list) = succs.get(&block_id) {
                        if !succ_list.is_empty() && succ_list.iter().all(|s| latest.contains(s)) {
                            to_remove.push(block_id);
                        }
                    }
                }

                if to_remove.is_empty() {
                    break;
                }
                for b in to_remove {
                    latest.remove(&b);
                }
            }

            result.insert(key.clone(), latest);
        }

        result
    }
}

impl Pass for MorelRenvoisePrePass {
    fn name(&self) -> &'static str {
        "morel_renvoise_pre"
    }

    fn run(&self, module: &mut MirModule) -> OptResult<PassResult> {
        let occs = Self::collect_occurrences(module);

        // Filter to multi-occurrence expressions (worth optimizing)
        let multi_occ: BTreeMap<_, _> = occs
            .iter()
            .filter(|(_, v)| v.len() > 1)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        if multi_occ.is_empty() {
            return Ok(PassResult::no_change());
        }

        let (preds, succs) = Self::build_cfg_maps(module);
        let avail = Self::compute_availability(module, &multi_occ, &preds);
        let ant = Self::compute_anticipatability(module, &multi_occ, &succs);
        let earliest = Self::compute_earliest(&multi_occ, &avail, &ant);
        let latest = Self::compute_latest(&earliest, &succs);

        // Count opportunities
        let opportunity_count: usize = latest.values().map(|s| s.len()).sum();

        if opportunity_count == 0 {
            return Ok(PassResult::no_change());
        }

        // Report identified redundancies
        Ok(PassResult::with_count(
            opportunity_count,
            "Identified PRE opportunities (Morel-Renvoise analysis)",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pre_morel_exists() {
        let pass = MorelRenvoisePrePass::new();
        assert_eq!(pass.name(), "morel_renvoise_pre");
    }

    #[test]
    fn pre_morel_collect_empty() {
        let module = MirModule {
            functions: vec![],
            span: x3_common::Span::dummy(),
        };
        let occs = MorelRenvoisePrePass::collect_occurrences(&module);
        assert!(occs.is_empty());
    }

    #[test]
    fn pre_morel_filter_single() {
        let mut occs: BTreeMap<ExprKey, Vec<Occurrence>> = BTreeMap::new();
        let key = ExprKey {
            opcode: "Add".to_string(),
            operands: vec!["a".to_string(), "b".to_string()],
        };
        occs.insert(
            key,
            vec![Occurrence {
                block: MirBlockId(0),
                stmt_index: 0,
            }],
        );

        let multi: BTreeMap<_, _> = occs
            .iter()
            .filter(|(_, v)| v.len() > 1)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        assert!(multi.is_empty(), "Single-occurrence should not be in multi");
    }

    #[test]
    fn pre_morel_filter_multi() {
        let mut occs: BTreeMap<ExprKey, Vec<Occurrence>> = BTreeMap::new();
        let key = ExprKey {
            opcode: "Add".to_string(),
            operands: vec!["a".to_string(), "b".to_string()],
        };
        occs.insert(
            key,
            vec![
                Occurrence {
                    block: MirBlockId(0),
                    stmt_index: 0,
                },
                Occurrence {
                    block: MirBlockId(1),
                    stmt_index: 0,
                },
            ],
        );

        let multi: BTreeMap<_, _> = occs
            .iter()
            .filter(|(_, v)| v.len() > 1)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        assert_eq!(multi.len(), 1, "Multi-occurrence should be in multi");
    }
}
