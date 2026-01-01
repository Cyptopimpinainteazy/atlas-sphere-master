//! Optimizer orchestration and pass pipeline.

use crate::loop_pack_v1::LoopPackV1Pass;
use crate::pass::{BoxedPass, Pass};
use crate::passes::{
    block_fusion::BlockFusionPass, branch_inversion::BranchInversionPass,
    branch_opt::BranchOptPass, cond_fold::ConditionalFoldPass, constant_fold::ConstantFoldPass,
    copy_propagation::CopyPropagationPass, dead_code_elimination::DeadCodeEliminationPass,
    dom_const_prop::DomConstPropPass, edge_const_prop::EdgeConstPropPass,
    expression_hoist::ExpressionHoistPass, global_const_prop::GlobalConstPropPass,
    peephole::PeepholePass, pre_morel::MorelRenvoisePrePass,
    pre_simple::PartialRedundancyEliminationPass, speculative_hoist::SpeculativeHoistPass,
};
use crate::OptResult;
use x3_mir::MirModule;

/// Optimization level controlling which passes are enabled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptLevel {
    /// No optimizations (O0).
    None,
    /// Basic optimizations: constant folding, peephole (O1).
    Basic,
    /// Default optimizations: O1 + DCE + copy propagation (O2).
    Default,
    /// Aggressive optimizations: O2 + multiple iterations (O3).
    Aggressive,
}

impl Default for OptLevel {
    fn default() -> Self {
        OptLevel::Default
    }
}

/// Statistics from an optimization run.
#[derive(Debug, Clone, Default)]
pub struct OptStats {
    /// Total passes executed.
    pub passes_run: usize,
    /// Passes that made changes.
    pub passes_changed: usize,
    /// Total transformations applied.
    pub total_transformations: usize,
    /// Number of iterations until fixpoint.
    pub iterations: usize,
}

/// Observer notified after every pass run.
pub trait PassObserver {
    fn after_pass(&mut self, pass: &str, module: &MirModule) -> OptResult<()>;
}

struct NoopObserver;

impl PassObserver for NoopObserver {
    fn after_pass(&mut self, _pass: &str, _module: &MirModule) -> OptResult<()> {
        Ok(())
    }
}

/// Get default passes for optimization (used by run_yolo and other orchestrators).
pub fn default_passes() -> Vec<BoxedPass> {
    vec![
        Box::new(ConstantFoldPass::new()),
        Box::new(PeepholePass::new()),
        Box::new(DomConstPropPass::new()),
        Box::new(EdgeConstPropPass::new()),
        Box::new(ConditionalFoldPass::new()),
        Box::new(PartialRedundancyEliminationPass::new()),
        Box::new(MorelRenvoisePrePass::new()), // Advanced PRE (Morel-Renvoise)
        Box::new(ExpressionHoistPass::new()),  // Expression hoisting (Phase 3)
        Box::new(GlobalConstPropPass),
        Box::new(BranchOptPass),
        Box::new(BranchInversionPass),
        Box::new(BlockFusionPass),
        Box::new(SpeculativeHoistPass),
        Box::new(DeadCodeEliminationPass::new()),
        Box::new(LoopPackV1Pass::new()), // Loop optimizations (LICM, SR, unswitching)
        Box::new(CopyPropagationPass::new()),
    ]
}

/// The MIR optimizer orchestrating transformation passes.
pub struct Optimizer {
    /// Optimization level.
    level: OptLevel,
    /// Registered passes.
    passes: Vec<BoxedPass>,
    /// Maximum iterations before giving up.
    max_iterations: usize,
}

impl Optimizer {
    /// Create a new optimizer with the given optimization level.
    pub fn new(level: OptLevel) -> Self {
        let mut opt = Optimizer {
            level,
            passes: Vec::new(),
            max_iterations: 10,
        };
        opt.register_default_passes();
        opt
    }

    /// Create an optimizer with no passes (for custom pipelines).
    pub fn empty() -> Self {
        Optimizer {
            level: OptLevel::None,
            passes: Vec::new(),
            max_iterations: 10,
        }
    }

    /// Set maximum iterations for fixpoint computation.
    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.max_iterations = max;
        self
    }

    /// Add a pass to the optimizer.
    pub fn add_pass<P: Pass + 'static>(&mut self, pass: P) {
        self.passes.push(Box::new(pass));
    }

    /// Register default passes based on optimization level.
    fn register_default_passes(&mut self) {
        match self.level {
            OptLevel::None => {
                // No passes
            }
            OptLevel::Basic => {
                self.passes.push(Box::new(ConstantFoldPass::new()));
                self.passes.push(Box::new(PeepholePass::new()));
            }
            OptLevel::Default => {
                self.passes.push(Box::new(ConstantFoldPass::new()));
                self.passes.push(Box::new(PeepholePass::new()));
                self.passes.push(Box::new(DomConstPropPass::new()));
                self.passes.push(Box::new(EdgeConstPropPass::new()));
                self.passes.push(Box::new(ConditionalFoldPass::new()));
                self.passes
                    .push(Box::new(PartialRedundancyEliminationPass::new()));
                self.passes.push(Box::new(MorelRenvoisePrePass::new()));
                self.passes.push(Box::new(GlobalConstPropPass));
                self.passes.push(Box::new(BranchOptPass));
                self.passes.push(Box::new(BranchInversionPass));
                self.passes.push(Box::new(BlockFusionPass));
                self.passes.push(Box::new(SpeculativeHoistPass));
                self.passes.push(Box::new(DeadCodeEliminationPass::new()));
                self.passes.push(Box::new(LoopPackV1Pass::new()));
                self.passes.push(Box::new(CopyPropagationPass::new()));
            }
            OptLevel::Aggressive => {
                // Same as default but with more iterations + loop optimizations
                self.passes.push(Box::new(ConstantFoldPass::new()));
                self.passes.push(Box::new(PeepholePass::new()));
                self.passes.push(Box::new(DomConstPropPass::new()));
                self.passes.push(Box::new(EdgeConstPropPass::new()));
                self.passes.push(Box::new(ConditionalFoldPass::new()));
                self.passes
                    .push(Box::new(PartialRedundancyEliminationPass::new()));
                self.passes.push(Box::new(MorelRenvoisePrePass::new()));
                self.passes.push(Box::new(GlobalConstPropPass));
                self.passes.push(Box::new(BranchOptPass));
                self.passes.push(Box::new(BranchInversionPass));
                self.passes.push(Box::new(BlockFusionPass));
                self.passes.push(Box::new(SpeculativeHoistPass));
                self.passes.push(Box::new(DeadCodeEliminationPass::new()));
                self.passes.push(Box::new(LoopPackV1Pass::new())); // Loop optimizations
                self.passes.push(Box::new(CopyPropagationPass::new()));
                self.max_iterations = 20;
            }
        }
    }

    /// Run all passes until fixpoint or max iterations.
    pub fn run(&self, module: &mut MirModule) -> OptResult<OptStats> {
        let mut observer = NoopObserver;
        self.run_with_observer(module, &mut observer)
    }

    /// Run with a pass observer that can inspect the module after every pass.
    pub fn run_with_observer(
        &self,
        module: &mut MirModule,
        observer: &mut dyn PassObserver,
    ) -> OptResult<OptStats> {
        let mut stats = OptStats::default();

        if self.passes.is_empty() {
            return Ok(stats);
        }

        for iteration in 0..self.max_iterations {
            stats.iterations = iteration + 1;
            let mut any_changed = false;

            for pass in &self.passes {
                let result = pass.run(module)?;
                stats.passes_run += 1;

                if result.changed {
                    stats.passes_changed += 1;
                    stats.total_transformations += result.transformations;
                    any_changed = true;

                    log::debug!(
                        "pass '{}' made {} changes: {}",
                        pass.name(),
                        result.transformations,
                        result.description.as_deref().unwrap_or("(no description)")
                    );
                }

                observer.after_pass(pass.name(), module)?;
            }

            // Fixpoint reached
            if !any_changed {
                log::info!(
                    "optimizer reached fixpoint after {} iterations ({} transformations)",
                    stats.iterations,
                    stats.total_transformations
                );
                return Ok(stats);
            }
        }

        log::warn!(
            "optimizer did not converge after {} iterations",
            self.max_iterations
        );
        Ok(stats)
    }

    /// Run a single iteration of all passes (no fixpoint loop).
    pub fn run_once(&self, module: &mut MirModule) -> OptResult<OptStats> {
        let mut stats = OptStats {
            iterations: 1,
            ..Default::default()
        };

        for pass in &self.passes {
            let result = pass.run(module)?;
            stats.passes_run += 1;

            if result.changed {
                stats.passes_changed += 1;
                stats.total_transformations += result.transformations;
            }
        }

        Ok(stats)
    }

    /// Return the registered pass names in execution order.
    pub fn pass_names(&self) -> Vec<&'static str> {
        self.passes.iter().map(|p| p.name()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use x3_common::Span;
    use x3_mir::{MirBlock, MirBlockId, MirFunction, MirModule, MirValue};

    fn empty_module() -> MirModule {
        MirModule {
            functions: vec![],
            span: Span::dummy(),
        }
    }

    fn simple_module() -> MirModule {
        MirModule {
            functions: vec![MirFunction {
                symbol: x3_mir::SymbolId(0),
                params: vec![],
                entry: MirBlockId(0),
                blocks: vec![MirBlock {
                    id: MirBlockId(0),
                    statements: vec![],
                    terminator: Some(x3_mir::MirTerminator::Return(None)),
                }],
                span: Span::dummy(),
            }],
            span: Span::dummy(),
        }
    }

    #[test]
    fn optimizer_empty_module() {
        let mut module = empty_module();
        let opt = Optimizer::new(OptLevel::Default);
        let stats = opt.run(&mut module).unwrap();
        assert_eq!(stats.iterations, 1);
    }

    #[test]
    fn optimizer_no_passes() {
        let mut module = simple_module();
        let opt = Optimizer::empty();
        let stats = opt.run(&mut module).unwrap();
        assert_eq!(stats.passes_run, 0);
    }

    #[test]
    fn optimizer_levels() {
        let none = Optimizer::new(OptLevel::None);
        assert!(none.passes.is_empty());

        let basic = Optimizer::new(OptLevel::Basic);
        assert_eq!(basic.passes.len(), 2); // constant fold + peephole

        let default = Optimizer::new(OptLevel::Default);
        assert_eq!(default.passes.len(), 15); // 14 base passes + loop-pack-v1 (includes MorelRenvoisePrePass)

        let aggressive = Optimizer::new(OptLevel::Aggressive);
        assert_eq!(aggressive.passes.len(), 15); // 14 base passes + loop-pack-v1 (includes MorelRenvoisePrePass)
        assert_eq!(aggressive.max_iterations, 20);
    }
}
