//! Pass trait and result types for the optimizer.

use crate::OptResult;
use x3_mir::MirModule;

/// Result of running an optimization pass.
#[derive(Debug, Clone)]
pub struct PassResult {
    /// Whether the pass made any changes to the IR.
    pub changed: bool,
    /// Optional description of changes made.
    pub description: Option<String>,
    /// Number of transformations applied (for metrics).
    pub transformations: usize,
}

impl PassResult {
    /// Create a result indicating no changes were made.
    pub fn no_change() -> Self {
        PassResult {
            changed: false,
            description: None,
            transformations: 0,
        }
    }

    /// Create a result indicating changes were made.
    pub fn changed(description: impl Into<String>) -> Self {
        PassResult {
            changed: true,
            description: Some(description.into()),
            transformations: 1,
        }
    }

    /// Create a result with a specific transformation count.
    pub fn with_count(count: usize, description: impl Into<String>) -> Self {
        PassResult {
            changed: count > 0,
            description: Some(description.into()),
            transformations: count,
        }
    }
}

/// An optimization pass that transforms MIR.
///
/// Passes must be:
/// - **Deterministic**: Same input → same output
/// - **Sound**: Preserve program semantics
/// - **Idempotent**: Running twice has no additional effect (preferred)
pub trait Pass: Send + Sync {
    /// Human-readable name of this pass.
    fn name(&self) -> &'static str;

    /// Run the optimization pass on the module.
    ///
    /// Returns `Ok(PassResult)` with information about changes made,
    /// or `Err` if an error occurred.
    fn run(&self, module: &mut MirModule) -> OptResult<PassResult>;

    /// Whether this pass should be run in the default optimization pipeline.
    fn is_default(&self) -> bool {
        true
    }

    /// Estimated cost of running this pass (for scheduling).
    /// Lower is cheaper. Default is 1.
    fn cost(&self) -> usize {
        1
    }
}

/// A boxed pass for dynamic dispatch.
pub type BoxedPass = Box<dyn Pass>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pass_result_no_change() {
        let result = PassResult::no_change();
        assert!(!result.changed);
        assert!(result.description.is_none());
        assert_eq!(result.transformations, 0);
    }

    #[test]
    fn pass_result_changed() {
        let result = PassResult::changed("removed 3 dead instructions");
        assert!(result.changed);
        assert_eq!(
            result.description,
            Some("removed 3 dead instructions".to_string())
        );
    }

    #[test]
    fn pass_result_with_count() {
        let result = PassResult::with_count(5, "folded 5 constants");
        assert!(result.changed);
        assert_eq!(result.transformations, 5);

        let zero = PassResult::with_count(0, "nothing to do");
        assert!(!zero.changed);
    }
}
