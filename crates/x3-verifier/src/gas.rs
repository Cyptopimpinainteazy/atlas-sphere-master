//! Gas analysis for X3 contracts
//!
//! Performs static gas analysis on MIR to compute worst-case gas bounds.

use std::collections::BTreeMap;

use x3_mir::{MirFunction, MirRhs, MirStatement, MirTerminator};

use crate::rules::SafetyRules;

/// Gas usage report for a single function
#[derive(Debug, Clone)]
pub struct FunctionGas {
    /// Function name (symbol ID as string)
    pub name: String,
    /// Minimum gas (best case path)
    pub min_gas: u64,
    /// Maximum gas (worst case path)
    pub max_gas: u64,
    /// Average gas (estimated typical case)
    pub avg_gas: u64,
    /// Number of instructions
    pub instruction_count: usize,
    /// Breakdown by opcode
    pub opcode_breakdown: BTreeMap<String, u64>,
    /// Contains loops (unbounded gas)
    pub has_loops: bool,
    /// Contains external calls (dynamic gas)
    pub has_external_calls: bool,
}

/// Complete gas analysis report
#[derive(Debug, Clone)]
pub struct GasReport {
    /// Per-function gas analysis
    pub functions: Vec<FunctionGas>,
    /// Total contract gas (sum of all functions)
    pub total_gas: u64,
    /// Whether any function has unbounded gas (loops without limits)
    pub has_unbounded: bool,
    /// Functions that exceed the limit
    pub exceeds_limit: Vec<String>,
}

impl GasReport {
    /// Check if all functions are within limits
    pub fn all_within_limits(&self) -> bool {
        self.exceeds_limit.is_empty() && !self.has_unbounded
    }
}

/// Gas analyzer for MIR
pub struct GasAnalyzer {
    rules: SafetyRules,
}

impl GasAnalyzer {
    /// Create a new gas analyzer with the given rules
    pub fn new(rules: SafetyRules) -> Self {
        GasAnalyzer { rules }
    }

    /// Analyze gas usage for a single function
    pub fn analyze_function(&self, func: &MirFunction) -> FunctionGas {
        let mut min_gas = 0u64;
        let mut max_gas = 0u64;
        let mut opcode_breakdown = BTreeMap::new();
        let mut has_loops = false;
        let mut has_external_calls = false;
        let mut instruction_count = 0usize;

        // Analyze each basic block
        for block in &func.blocks {
            let mut block_gas = 0u64;

            // Analyze statements
            for stmt in &block.statements {
                let opcode = self.statement_opcode(stmt);
                let cost = self.rules.gas_cost(&opcode);

                block_gas += cost.base;
                *opcode_breakdown.entry(opcode.clone()).or_insert(0) += cost.base;
                instruction_count += 1;

                // Check for external calls
                if self.is_external_call_stmt(stmt) {
                    has_external_calls = true;
                }
            }

            // Analyze terminator
            if let Some(ref term) = block.terminator {
                let opcode = self.terminator_opcode(term);
                let cost = self.rules.gas_cost(&opcode);

                block_gas += cost.base;
                *opcode_breakdown.entry(opcode.clone()).or_insert(0) += cost.base;
                instruction_count += 1;

                // Check for loops (backward branches)
                if self.is_loop_terminator(term, block.id) {
                    has_loops = true;
                }
            }

            min_gas += block_gas;
            max_gas += block_gas;
        }

        // If has loops, max_gas is unbounded (mark as very high)
        if has_loops {
            max_gas = u64::MAX / 2;
        }

        FunctionGas {
            name: format!("{:?}", func.symbol),
            min_gas,
            max_gas,
            avg_gas: (min_gas + max_gas) / 2,
            instruction_count,
            opcode_breakdown,
            has_loops,
            has_external_calls,
        }
    }

    /// Analyze all functions and produce a complete report
    pub fn analyze(&self, functions: &[MirFunction]) -> GasReport {
        let mut func_reports = Vec::new();
        let mut total_gas = 0u64;
        let mut has_unbounded = false;
        let mut exceeds_limit = Vec::new();

        for func in functions {
            let report = self.analyze_function(func);

            if report.has_loops {
                has_unbounded = true;
            }

            if report.max_gas > self.rules.limits.max_function_gas {
                exceeds_limit.push(report.name.clone());
            }

            total_gas = total_gas.saturating_add(report.max_gas);
            func_reports.push(report);
        }

        GasReport {
            functions: func_reports,
            total_gas,
            has_unbounded,
            exceeds_limit,
        }
    }

    /// Get opcode name from a MIR statement
    fn statement_opcode(&self, stmt: &MirStatement) -> String {
        match &stmt.rhs {
            MirRhs::Literal(_) => "const".to_string(),
            MirRhs::Unary(op, _) => format!("{:?}", op).to_lowercase(),
            MirRhs::Binary(op, _, _) => format!("{:?}", op).to_lowercase(),
            MirRhs::Call { .. } => "call".to_string(),
            MirRhs::Load { .. } => "load".to_string(),
            MirRhs::Store { .. } => "store".to_string(),
        }
    }

    /// Get opcode name from a terminator
    fn terminator_opcode(&self, term: &MirTerminator) -> String {
        match term {
            MirTerminator::Return(_) => "return".to_string(),
            MirTerminator::Goto(_) => "jump".to_string(),
            MirTerminator::Branch { .. } => "branch".to_string(),
        }
    }

    /// Check if statement is an external call
    fn is_external_call_stmt(&self, stmt: &MirStatement) -> bool {
        matches!(&stmt.rhs, MirRhs::Call { .. })
    }

    /// Check if terminator represents a loop (backward branch)
    fn is_loop_terminator(&self, term: &MirTerminator, current_block: x3_mir::MirBlockId) -> bool {
        match term {
            MirTerminator::Goto(target) => target.0 <= current_block.0,
            MirTerminator::Branch {
                then_block,
                else_block,
                ..
            } => then_block.0 <= current_block.0 || else_block.0 <= current_block.0,
            MirTerminator::Return(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gas_analyzer_creation() {
        let rules = SafetyRules::default();
        let _analyzer = GasAnalyzer::new(rules);
    }
}
