//! Dead Code Elimination Pass
//!
//! Removes unreachable code and unused assignments from the MIR.
//!
//! # Transformations
//!
//! 1. **Unreachable block elimination**: Remove blocks not reachable from entry
//! 2. **Dead assignment elimination**: Remove assignments whose results are never used
//! 3. **Dead terminator cleanup**: Remove terminators after unconditional returns
//!
//! # Algorithm
//!
//! 1. Build reachability set via BFS from entry block
//! 2. Remove unreachable blocks
//! 3. Compute use-def chains
//! 4. Remove assignments to values that are never used (unless side-effecting)

use crate::dce::run_dce;
use crate::pass::{Pass, PassResult};
use crate::OptResult;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use x3_mir::{MirBlockId, MirModule, MirRhs, MirTerminator, MirValue};

/// Dead code elimination pass.
pub struct DeadCodeEliminationPass;

impl DeadCodeEliminationPass {
    pub fn new() -> Self {
        DeadCodeEliminationPass
    }

    /// Compute reachable blocks via BFS from entry.
    fn compute_reachable(
        &self,
        entry: MirBlockId,
        blocks: &[x3_mir::MirBlock],
    ) -> BTreeSet<MirBlockId> {
        let mut reachable = BTreeSet::new();
        let mut worklist = VecDeque::new();

        let block_map: BTreeMap<MirBlockId, &x3_mir::MirBlock> =
            blocks.iter().map(|b| (b.id, b)).collect();

        worklist.push_back(entry);
        reachable.insert(entry);

        while let Some(block_id) = worklist.pop_front() {
            if let Some(block) = block_map.get(&block_id) {
                if let Some(term) = &block.terminator {
                    for succ in Self::terminator_successors(term) {
                        if reachable.insert(succ) {
                            worklist.push_back(succ);
                        }
                    }
                }
            }
        }

        reachable
    }

    fn terminator_successors(term: &MirTerminator) -> Vec<MirBlockId> {
        match term {
            MirTerminator::Return(_) => vec![],
            MirTerminator::Goto(target) => vec![*target],
            MirTerminator::Branch {
                then_block,
                else_block,
                ..
            } => vec![*then_block, *else_block],
        }
    }
}

impl Default for DeadCodeEliminationPass {
    fn default() -> Self {
        Self::new()
    }
}

impl Pass for DeadCodeEliminationPass {
    fn name(&self) -> &'static str {
        "dead_code_elimination"
    }

    fn run(&self, module: &mut MirModule) -> OptResult<PassResult> {
        let mut total_changes = 0usize;

        for func in module.functions.iter_mut() {
            // Phase 1: Remove unreachable blocks
            let reachable = self.compute_reachable(func.entry, &func.blocks);
            let before_blocks = func.blocks.len();
            func.blocks.retain(|block| reachable.contains(&block.id));
            let removed_blocks = before_blocks - func.blocks.len();
            total_changes += removed_blocks;

            if run_dce(func, |stmt| matches!(stmt.rhs, MirRhs::Call { .. })) {
                total_changes += 1;
            }
        }

        if total_changes > 0 {
            Ok(PassResult::with_count(
                total_changes,
                format!("eliminated {} dead code elements", total_changes),
            ))
        } else {
            Ok(PassResult::no_change())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use x3_common::{Literal, Span};
    use x3_mir::{MirBlock, MirFunction, MirStatement, SymbolId};

    fn make_module(func: MirFunction) -> MirModule {
        MirModule {
            functions: vec![func],
            span: Span::dummy(),
        }
    }

    #[test]
    fn remove_unreachable_block() {
        // Block 0: goto block 2
        // Block 1: (unreachable)
        // Block 2: return
        let func = MirFunction {
            symbol: SymbolId(0),
            params: vec![],
            entry: MirBlockId(0),
            blocks: vec![
                MirBlock {
                    id: MirBlockId(0),
                    statements: vec![],
                    terminator: Some(MirTerminator::Goto(MirBlockId(2))),
                },
                MirBlock {
                    id: MirBlockId(1),
                    statements: vec![MirStatement {
                        target: MirValue(99),
                        rhs: MirRhs::Literal(Literal::Integer(42)),
                    }],
                    terminator: Some(MirTerminator::Return(None)),
                },
                MirBlock {
                    id: MirBlockId(2),
                    statements: vec![],
                    terminator: Some(MirTerminator::Return(None)),
                },
            ],
            span: Span::dummy(),
        };

        let mut module = make_module(func);
        let pass = DeadCodeEliminationPass::new();
        let result = pass.run(&mut module).unwrap();

        assert!(result.changed);
        assert_eq!(module.functions[0].blocks.len(), 2); // Block 1 removed
        let block_ids: Vec<_> = module.functions[0].blocks.iter().map(|b| b.id.0).collect();
        assert!(!block_ids.contains(&1));
    }

    #[test]
    fn remove_dead_assignment() {
        // v0 = 42  (used)
        // v1 = 99  (dead - never used)
        // return v0
        let func = MirFunction {
            symbol: SymbolId(0),
            params: vec![],
            entry: MirBlockId(0),
            blocks: vec![MirBlock {
                id: MirBlockId(0),
                statements: vec![
                    MirStatement {
                        target: MirValue(0),
                        rhs: MirRhs::Literal(Literal::Integer(42)),
                    },
                    MirStatement {
                        target: MirValue(1),
                        rhs: MirRhs::Literal(Literal::Integer(99)),
                    },
                ],
                terminator: Some(MirTerminator::Return(Some(MirValue(0)))),
            }],
            span: Span::dummy(),
        };

        let mut module = make_module(func);
        let pass = DeadCodeEliminationPass::new();
        let result = pass.run(&mut module).unwrap();

        assert!(result.changed);
        assert_eq!(module.functions[0].blocks[0].statements.len(), 1);
        assert_eq!(
            module.functions[0].blocks[0].statements[0].target,
            MirValue(0)
        );
    }

    #[test]
    fn keep_side_effecting_call() {
        // v0 = call print()  (side effect - keep even if unused)
        // return
        let func = MirFunction {
            symbol: SymbolId(0),
            params: vec![],
            entry: MirBlockId(0),
            blocks: vec![MirBlock {
                id: MirBlockId(0),
                statements: vec![MirStatement {
                    target: MirValue(0),
                    rhs: MirRhs::Call {
                        target: SymbolId(1),
                        args: vec![],
                    },
                }],
                terminator: Some(MirTerminator::Return(None)),
            }],
            span: Span::dummy(),
        };

        let mut module = make_module(func);
        let pass = DeadCodeEliminationPass::new();
        let result = pass.run(&mut module).unwrap();

        // Call should be kept even though v0 is never used
        assert!(!result.changed);
        assert_eq!(module.functions[0].blocks[0].statements.len(), 1);
    }

    #[test]
    fn cascading_dead_code() {
        // v0 = 1
        // v1 = v0 + 1  (becomes dead when v2 is removed)
        // v2 = v1 + 1  (dead)
        // v3 = 42      (used)
        // return v3
        let func = MirFunction {
            symbol: SymbolId(0),
            params: vec![],
            entry: MirBlockId(0),
            blocks: vec![MirBlock {
                id: MirBlockId(0),
                statements: vec![
                    MirStatement {
                        target: MirValue(0),
                        rhs: MirRhs::Literal(Literal::Integer(1)),
                    },
                    MirStatement {
                        target: MirValue(1),
                        rhs: MirRhs::Binary(x3_ast::BinaryOp::Add, MirValue(0), MirValue(0)),
                    },
                    MirStatement {
                        target: MirValue(2),
                        rhs: MirRhs::Binary(x3_ast::BinaryOp::Add, MirValue(1), MirValue(0)),
                    },
                    MirStatement {
                        target: MirValue(3),
                        rhs: MirRhs::Literal(Literal::Integer(42)),
                    },
                ],
                terminator: Some(MirTerminator::Return(Some(MirValue(3)))),
            }],
            span: Span::dummy(),
        };

        let mut module = make_module(func);
        let pass = DeadCodeEliminationPass::new();
        let result = pass.run(&mut module).unwrap();

        assert!(result.changed);
        // Only v3 = 42 should remain
        assert_eq!(module.functions[0].blocks[0].statements.len(), 1);
        assert_eq!(
            module.functions[0].blocks[0].statements[0].target,
            MirValue(3)
        );
    }

    #[test]
    fn branch_reachability() {
        // Block 0: branch cond -> 1, 2
        // Block 1: return
        // Block 2: return
        // All blocks reachable
        let func = MirFunction {
            symbol: SymbolId(0),
            params: vec![MirValue(0)], // cond
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
                MirBlock {
                    id: MirBlockId(1),
                    statements: vec![],
                    terminator: Some(MirTerminator::Return(None)),
                },
                MirBlock {
                    id: MirBlockId(2),
                    statements: vec![],
                    terminator: Some(MirTerminator::Return(None)),
                },
            ],
            span: Span::dummy(),
        };

        let mut module = make_module(func);
        let pass = DeadCodeEliminationPass::new();
        let result = pass.run(&mut module).unwrap();

        // All blocks reachable, nothing to remove
        assert!(!result.changed);
        assert_eq!(module.functions[0].blocks.len(), 3);
    }
}
