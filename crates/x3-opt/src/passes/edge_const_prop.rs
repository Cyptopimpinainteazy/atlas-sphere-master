use crate::edge_const_prop::{compute_edge_constants, fold_branch_on_edge_consts};
use crate::pass::{Pass, PassResult};
use crate::OptResult;
use x3_mir::MirModule;

/// Pass that folds branches using edge-sensitive constant information.
pub struct EdgeConstPropPass;

impl EdgeConstPropPass {
    pub fn new() -> Self {
        EdgeConstPropPass
    }
}

impl Pass for EdgeConstPropPass {
    fn name(&self) -> &'static str {
        "edge_const_prop"
    }

    fn run(&self, module: &mut MirModule) -> OptResult<PassResult> {
        let mut folded = 0;
        for func in module.functions.iter_mut() {
            let edge_consts = compute_edge_constants(func);
            if fold_branch_on_edge_consts(func, &edge_consts) {
                folded += 1;
            }
        }

        if folded > 0 {
            Ok(PassResult::with_count(
                folded,
                format!("edge-const prop folded {} branches", folded),
            ))
        } else {
            Ok(PassResult::no_change())
        }
    }
}
