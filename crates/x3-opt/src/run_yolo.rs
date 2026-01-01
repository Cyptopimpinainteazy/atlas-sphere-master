//! Orchestrate a single YOLO optimization round for a MirModule.
//! This module provides deterministic runners used by bench tooling and the optimizer.
//!
//! YOLO = "You Only Live Once" - aggressive, sequential, multi-pass optimization
//! in a single round with per-pass telemetry.

use crate::pass::PassResult;
use crate::OptResult;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use x3_mir::MirModule;

/// Per-pass delta metrics.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PassDelta {
    pub name: String,
    pub instr_delta: isize,
    pub gas_delta: i64,
    pub changed: bool,
}

/// Complete YOLO round report.
#[derive(Serialize, Deserialize, Debug)]
pub struct OptimizationReport {
    pub instr_before: usize,
    pub instr_after: usize,
    pub gas_before: u64,
    pub gas_after: u64,
    pub bytes_before: usize,
    pub bytes_after: usize,
    pub changed: bool,
    pub per_pass: BTreeMap<String, PassDelta>,
}

/// Run a single deterministic YOLO optimization round.
/// Executes all default passes sequentially with per-pass metrics.
pub fn run_yolo_once(module: &mut MirModule) -> OptResult<OptimizationReport> {
    let instr_before = count_instructions(module);
    let gas_before = simulate_gas(module);
    let bytes_before = estimate_bytes(module);

    let mut changed = false;
    let mut per_pass: BTreeMap<String, PassDelta> = BTreeMap::new();

    // Get list of passes to run (from optimizer defaults)
    let passes = crate::optimizer::default_passes();

    for pass in passes {
        let before_instr = count_instructions(module);
        let before_gas = simulate_gas(module);

        let result = pass.run(module)?;

        let after_instr = count_instructions(module);
        let after_gas = simulate_gas(module);

        per_pass.insert(
            pass.name().to_string(),
            PassDelta {
                name: pass.name().to_string(),
                instr_delta: (after_instr as isize) - (before_instr as isize),
                gas_delta: (after_gas as i64) - (before_gas as i64),
                changed: result.changed,
            },
        );

        changed |= result.changed;
    }

    let instr_after = count_instructions(module);
    let gas_after = simulate_gas(module);
    let bytes_after = estimate_bytes(module);

    Ok(OptimizationReport {
        instr_before,
        instr_after,
        gas_before,
        gas_after,
        bytes_before,
        bytes_after,
        changed,
        per_pass,
    })
}

/// Count total instructions in all functions.
pub fn count_instructions(module: &MirModule) -> usize {
    module
        .functions
        .iter()
        .map(|f| f.blocks.iter().map(|b| b.statements.len()).sum::<usize>())
        .sum()
}

/// Simulate total gas cost (deterministic approximation).
/// Each statement costs 1 gas, terminators cost 2 (simplified).
pub fn simulate_gas(module: &MirModule) -> u64 {
    let mut gas = 0u64;
    for func in module.functions.iter() {
        for block in func.blocks.iter() {
            // Each statement = 1 gas
            gas += block.statements.len() as u64;
            // Terminator = 1 gas if present
            if block.terminator.is_some() {
                gas += 1;
            }
        }
    }
    gas
}

/// Estimate code size in bytes (deterministic).
/// Each statement ~8 bytes, each terminator ~4 bytes.
pub fn estimate_bytes(module: &MirModule) -> usize {
    let mut bytes = 0usize;
    for func in module.functions.iter() {
        for block in func.blocks.iter() {
            bytes += block.statements.len() * 8;
            if block.terminator.is_some() {
                bytes += 4;
            }
        }
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_yolo_once_empty_module() {
        let mut module = MirModule {
            functions: vec![],
            span: x3_common::Span::new(0, 0),
        };
        let report = run_yolo_once(&mut module).unwrap();
        assert_eq!(report.instr_before, 0);
        assert_eq!(report.instr_after, 0);
        assert!(!report.changed);
    }

    #[test]
    fn metrics_are_deterministic() {
        let mut module = MirModule {
            functions: vec![],
            span: x3_common::Span::new(0, 0),
        };
        let report1 = run_yolo_once(&mut module).unwrap();
        let report2 = run_yolo_once(&mut module).unwrap();
        assert_eq!(report1.gas_before, report2.gas_before);
        assert_eq!(report1.gas_after, report2.gas_after);
    }
}
