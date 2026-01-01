//! Branch optimization pass.
//!
//! Optimizations performed:
//! 1. Conditional folding: Branch with constant condition → Goto
//! 2. Jump threading: Goto to block with single Goto → redirect
//! 3. Branch normalization: Branch where both targets are same → Goto
//! 4. Dead branch removal: Remove unreachable blocks after transformations

use crate::pass::{Pass, PassResult};
use crate::OptResult;
use std::collections::{BTreeMap, BTreeSet};
use x3_mir::mir::{MirBlockId, MirFunction, MirModule, MirRhs, MirTerminator, MirValue};

/// Branch optimization pass.
pub struct BranchOptPass;

impl Pass for BranchOptPass {
    fn name(&self) -> &'static str {
        "branch_opt"
    }

    fn run(&self, module: &mut MirModule) -> OptResult<PassResult> {
        let mut transformation_count = 0;

        for func in &mut module.functions {
            // Build constant map from function
            let constants = collect_constants(func);

            // Pass 1: Fold constant branches and normalize same-target branches
            for block in &mut func.blocks {
                if let Some(ref mut term) = block.terminator {
                    match term {
                        MirTerminator::Branch {
                            cond,
                            then_block,
                            else_block,
                        } => {
                            // Same-target normalization
                            if then_block == else_block {
                                let target = *then_block;
                                *term = MirTerminator::Goto(target);
                                transformation_count += 1;
                                continue;
                            }

                            // Constant condition folding
                            if let Some(val) = constants.get(cond) {
                                let target = if *val { *then_block } else { *else_block };
                                *term = MirTerminator::Goto(target);
                                transformation_count += 1;
                            }
                        }
                        _ => {}
                    }
                }
            }

            // Pass 2: Jump threading - Goto to block with single Goto → redirect
            let redirect_map = build_redirect_map(func);
            if !redirect_map.is_empty() {
                for block in &mut func.blocks {
                    if let Some(ref mut term) = block.terminator {
                        match term {
                            MirTerminator::Goto(target) => {
                                if let Some(&final_target) = redirect_map.get(target) {
                                    if *target != final_target {
                                        *target = final_target;
                                        transformation_count += 1;
                                    }
                                }
                            }
                            MirTerminator::Branch {
                                then_block,
                                else_block,
                                ..
                            } => {
                                if let Some(&final_target) = redirect_map.get(then_block) {
                                    if *then_block != final_target {
                                        *then_block = final_target;
                                        transformation_count += 1;
                                    }
                                }
                                if let Some(&final_target) = redirect_map.get(else_block) {
                                    if *else_block != final_target {
                                        *else_block = final_target;
                                        transformation_count += 1;
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }

            // Pass 3: Remove unreachable blocks
            let reachable = compute_reachable(func);
            let before_count = func.blocks.len();
            func.blocks.retain(|b| reachable.contains(&b.id));
            let removed = before_count - func.blocks.len();
            transformation_count += removed;
        }

        Ok(PassResult::with_count(
            transformation_count,
            "branch optimizations",
        ))
    }
}

/// Collect known boolean constants from function statements.
fn collect_constants(func: &MirFunction) -> BTreeMap<MirValue, bool> {
    let mut constants = BTreeMap::new();
    for block in &func.blocks {
        for stmt in &block.statements {
            if let MirRhs::Literal(lit) = &stmt.rhs {
                match lit {
                    x3_common::Literal::Bool(b) => {
                        constants.insert(stmt.target, *b);
                    }
                    // Integer 0 is falsy, non-zero is truthy (for branch folding)
                    x3_common::Literal::Integer(n) => {
                        constants.insert(stmt.target, *n != 0);
                    }
                    _ => {}
                }
            }
        }
    }
    constants
}

/// Build a redirect map for jump threading.
/// Maps block IDs to their final target if the block is a "trampoline"
/// (empty statements + single Goto terminator).
fn build_redirect_map(func: &MirFunction) -> BTreeMap<MirBlockId, MirBlockId> {
    let mut redirect: BTreeMap<MirBlockId, MirBlockId> = BTreeMap::new();

    // First pass: identify trampolines
    for block in &func.blocks {
        if block.statements.is_empty() {
            if let Some(MirTerminator::Goto(target)) = &block.terminator {
                redirect.insert(block.id, *target);
            }
        }
    }

    // Iteratively resolve chains: A → B → C becomes A → C
    let mut changed = true;
    let max_iterations = func.blocks.len(); // Prevent infinite loops
    let mut iterations = 0;

    while changed && iterations < max_iterations {
        changed = false;
        iterations += 1;

        let keys: Vec<_> = redirect.keys().copied().collect();
        for key in keys {
            if let Some(&target) = redirect.get(&key) {
                if let Some(&final_target) = redirect.get(&target) {
                    if target != final_target {
                        redirect.insert(key, final_target);
                        changed = true;
                    }
                }
            }
        }
    }

    redirect
}

/// Compute the set of reachable blocks from the entry.
fn compute_reachable(func: &MirFunction) -> BTreeSet<MirBlockId> {
    let mut reachable = BTreeSet::new();
    let mut worklist = vec![func.entry];

    while let Some(block_id) = worklist.pop() {
        if !reachable.insert(block_id) {
            continue; // Already visited
        }

        // Find the block and add successors
        if let Some(block) = func.blocks.iter().find(|b| b.id == block_id) {
            if let Some(ref term) = block.terminator {
                match term {
                    MirTerminator::Return(_) => {}
                    MirTerminator::Goto(target) => {
                        worklist.push(*target);
                    }
                    MirTerminator::Branch {
                        then_block,
                        else_block,
                        ..
                    } => {
                        worklist.push(*then_block);
                        worklist.push(*else_block);
                    }
                }
            }
        }
    }

    reachable
}

#[cfg(test)]
mod tests {
    use super::*;
    use x3_common::{Literal, Span};
    use x3_mir::mir::{MirBlock, MirStatement, SymbolId};

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
    fn test_constant_branch_folding_true() {
        // Block 0: v0 = true; Branch(v0, B1, B2)
        // → Should become Goto(B1)
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
                statements: vec![],
                terminator: Some(MirTerminator::Return(None)),
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

        let pass = BranchOptPass;
        let result = pass.run(&mut module).unwrap();

        assert!(result.changed);
        let term = module.functions[0].blocks[0].terminator.as_ref().unwrap();
        assert!(matches!(term, MirTerminator::Goto(MirBlockId(1))));
    }

    #[test]
    fn test_constant_branch_folding_false() {
        // Block 0: v0 = false; Branch(v0, B1, B2)
        // → Should become Goto(B2)
        let blocks = vec![
            MirBlock {
                id: MirBlockId(0),
                statements: vec![MirStatement {
                    target: MirValue(0),
                    rhs: MirRhs::Literal(Literal::Bool(false)),
                }],
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
        ];

        let mut module = MirModule {
            functions: vec![make_func(blocks)],
            span: dummy_span(),
        };

        let pass = BranchOptPass;
        let result = pass.run(&mut module).unwrap();

        assert!(result.changed);
        let term = module.functions[0].blocks[0].terminator.as_ref().unwrap();
        assert!(matches!(term, MirTerminator::Goto(MirBlockId(2))));
    }

    #[test]
    fn test_same_target_branch_normalization() {
        // Branch where then and else are same → Goto
        let blocks = vec![MirBlock {
            id: MirBlockId(0),
            statements: vec![MirStatement {
                target: MirValue(0),
                rhs: MirRhs::Literal(Literal::Bool(true)),
            }],
            terminator: Some(MirTerminator::Branch {
                cond: MirValue(0),
                then_block: MirBlockId(1),
                else_block: MirBlockId(1), // Same target!
            }),
        }];

        let mut module = MirModule {
            functions: vec![make_func(blocks)],
            span: dummy_span(),
        };

        let pass = BranchOptPass;
        let result = pass.run(&mut module).unwrap();

        assert!(result.changed);
        let term = module.functions[0].blocks[0].terminator.as_ref().unwrap();
        assert!(matches!(term, MirTerminator::Goto(MirBlockId(1))));
    }

    #[test]
    fn test_jump_threading() {
        // B0: Goto(B1)
        // B1: Goto(B2) <- trampoline
        // B2: Return
        // → B0 should redirect to B2
        let blocks = vec![
            MirBlock {
                id: MirBlockId(0),
                statements: vec![],
                terminator: Some(MirTerminator::Goto(MirBlockId(1))),
            },
            MirBlock {
                id: MirBlockId(1),
                statements: vec![], // Empty = trampoline
                terminator: Some(MirTerminator::Goto(MirBlockId(2))),
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

        let pass = BranchOptPass;
        let result = pass.run(&mut module).unwrap();

        assert!(result.changed);
        // B0 should now go directly to B2
        let term = module.functions[0].blocks[0].terminator.as_ref().unwrap();
        assert!(matches!(term, MirTerminator::Goto(MirBlockId(2))));
    }

    #[test]
    fn test_unreachable_block_removal() {
        // B0: Return (entry)
        // B1: Return (unreachable)
        // → B1 should be removed
        let blocks = vec![
            MirBlock {
                id: MirBlockId(0),
                statements: vec![],
                terminator: Some(MirTerminator::Return(None)),
            },
            MirBlock {
                id: MirBlockId(1),
                statements: vec![],
                terminator: Some(MirTerminator::Return(None)),
            },
        ];

        let mut module = MirModule {
            functions: vec![make_func(blocks)],
            span: dummy_span(),
        };

        let pass = BranchOptPass;
        let result = pass.run(&mut module).unwrap();

        assert!(result.changed);
        assert_eq!(module.functions[0].blocks.len(), 1);
        assert_eq!(module.functions[0].blocks[0].id, MirBlockId(0));
    }

    #[test]
    fn test_integer_zero_branch_folding() {
        // v0 = 0 (falsy); Branch(v0, B1, B2) → Goto(B2)
        let blocks = vec![
            MirBlock {
                id: MirBlockId(0),
                statements: vec![MirStatement {
                    target: MirValue(0),
                    rhs: MirRhs::Literal(Literal::Integer(0)),
                }],
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
        ];

        let mut module = MirModule {
            functions: vec![make_func(blocks)],
            span: dummy_span(),
        };

        let pass = BranchOptPass;
        let result = pass.run(&mut module).unwrap();

        assert!(result.changed);
        let term = module.functions[0].blocks[0].terminator.as_ref().unwrap();
        assert!(matches!(term, MirTerminator::Goto(MirBlockId(2))));
    }

    #[test]
    fn test_no_change_when_already_optimal() {
        // Already optimal: simple Return
        let blocks = vec![MirBlock {
            id: MirBlockId(0),
            statements: vec![],
            terminator: Some(MirTerminator::Return(None)),
        }];

        let mut module = MirModule {
            functions: vec![make_func(blocks)],
            span: dummy_span(),
        };

        let pass = BranchOptPass;
        let result = pass.run(&mut module).unwrap();

        assert!(!result.changed);
    }
}
