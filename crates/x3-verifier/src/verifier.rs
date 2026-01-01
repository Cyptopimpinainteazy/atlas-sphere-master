//! Core verification logic
//!
//! Performs comprehensive static verification on X3 MIR and bytecode.

use x3_mir::{MirFunction, MirModule, MirRhs, MirStatement, MirTerminator};

use crate::error::VerifierResult;
use crate::gas::{GasAnalyzer, GasReport};
use crate::rules::{OpcodeClass, SafetyRules};

/// Severity level of verification issues
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Informational note
    Info,
    /// Non-blocking warning
    Warning,
    /// Blocking error
    Error,
    /// Critical security issue
    Critical,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Info => write!(f, "INFO"),
            Severity::Warning => write!(f, "WARN"),
            Severity::Error => write!(f, "ERROR"),
            Severity::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// A single verification error or warning
#[derive(Debug, Clone)]
pub struct VerificationError {
    /// Error code (e.g., E001)
    pub code: String,
    /// Severity level
    pub severity: Severity,
    /// Human-readable message
    pub message: String,
    /// Location in source (function, block, instruction)
    pub location: Option<String>,
    /// Suggested fix
    pub suggestion: Option<String>,
}

impl std::fmt::Display for VerificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}: {}", self.code, self.severity, self.message)?;
        if let Some(ref loc) = self.location {
            write!(f, " at {}", loc)?;
        }
        if let Some(ref sug) = self.suggestion {
            write!(f, " (suggestion: {})", sug)?;
        }
        Ok(())
    }
}

/// Complete verification report
#[derive(Debug, Clone)]
pub struct VerificationReport {
    /// All verification findings
    pub errors: Vec<VerificationError>,
    /// Gas analysis report
    pub gas_report: Option<GasReport>,
    /// Whether verification passed
    pub passed: bool,
    /// Summary statistics
    pub stats: VerificationStats,
}

#[derive(Debug, Clone, Default)]
pub struct VerificationStats {
    pub functions_checked: usize,
    pub statements_checked: usize,
    pub forbidden_ops_found: usize,
    pub restricted_ops_found: usize,
    pub gas_limit_violations: usize,
    pub determinism_violations: usize,
}

impl VerificationReport {
    /// Check if verification passed (no errors or critical issues)
    pub fn passed(&self) -> bool {
        self.passed
    }

    /// Get all errors (excludes info and warnings)
    pub fn errors(&self) -> impl Iterator<Item = &VerificationError> {
        self.errors.iter().filter(|e| e.severity >= Severity::Error)
    }

    /// Get all warnings
    pub fn warnings(&self) -> impl Iterator<Item = &VerificationError> {
        self.errors
            .iter()
            .filter(|e| e.severity == Severity::Warning)
    }

    /// Get critical issues only
    pub fn critical(&self) -> impl Iterator<Item = &VerificationError> {
        self.errors
            .iter()
            .filter(|e| e.severity == Severity::Critical)
    }
}

/// Main verifier for X3 contracts
pub struct Verifier {
    rules: SafetyRules,
    gas_analyzer: GasAnalyzer,
}

impl Verifier {
    /// Create a new verifier with the given safety rules
    pub fn new(rules: SafetyRules) -> Self {
        let gas_analyzer = GasAnalyzer::new(rules.clone());
        Verifier {
            rules,
            gas_analyzer,
        }
    }

    /// Create a verifier with default rules
    pub fn default() -> Self {
        Self::new(SafetyRules::default())
    }

    /// Verify a complete MIR module
    pub fn verify_mir(&self, module: &MirModule) -> VerifierResult<VerificationReport> {
        let mut errors = Vec::new();
        let mut stats = VerificationStats::default();

        // Check each function
        for func in &module.functions {
            stats.functions_checked += 1;
            self.verify_function(func, &mut errors, &mut stats);
        }

        // Perform gas analysis
        let gas_report = self.gas_analyzer.analyze(&module.functions);

        // Check gas limits
        for func_gas in &gas_report.functions {
            if func_gas.max_gas > self.rules.limits.max_function_gas {
                stats.gas_limit_violations += 1;
                errors.push(VerificationError {
                    code: "E008".to_string(),
                    severity: Severity::Error,
                    message: format!(
                        "Function '{}' exceeds gas limit: {} > {}",
                        func_gas.name, func_gas.max_gas, self.rules.limits.max_function_gas
                    ),
                    location: Some(func_gas.name.clone()),
                    suggestion: Some(
                        "Reduce function complexity or split into smaller functions".to_string(),
                    ),
                });
            }
        }

        // Determine if verification passed
        let passed = errors.iter().all(|e| e.severity < Severity::Error);

        Ok(VerificationReport {
            errors,
            gas_report: Some(gas_report),
            passed,
            stats,
        })
    }

    /// Verify a single function
    fn verify_function(
        &self,
        func: &MirFunction,
        errors: &mut Vec<VerificationError>,
        stats: &mut VerificationStats,
    ) {
        let func_name = format!("{:?}", func.symbol);

        // Check instruction count
        let total_instructions: usize = func
            .blocks
            .iter()
            .map(|b| b.statements.len() + if b.terminator.is_some() { 1 } else { 0 })
            .sum();

        if total_instructions > self.rules.limits.max_instructions_per_function {
            errors.push(VerificationError {
                code: "E009".to_string(),
                severity: Severity::Error,
                message: format!(
                    "Function '{}' has too many instructions: {} > {}",
                    func_name, total_instructions, self.rules.limits.max_instructions_per_function
                ),
                location: Some(func_name.clone()),
                suggestion: Some("Split into smaller functions".to_string()),
            });
        }

        // Check each block
        for block in &func.blocks {
            // Check statements
            for stmt in &block.statements {
                stats.statements_checked += 1;
                self.verify_statement(stmt, &func_name, errors, stats);
            }

            // Check terminator
            if let Some(ref term) = block.terminator {
                self.verify_terminator(term, &func_name, errors, stats);
            }
        }
    }

    /// Verify a single statement
    fn verify_statement(
        &self,
        stmt: &MirStatement,
        func_name: &str,
        errors: &mut Vec<VerificationError>,
        stats: &mut VerificationStats,
    ) {
        let opcode = self.statement_opcode(stmt);
        let class = self.rules.classify_opcode(&opcode);

        match class {
            OpcodeClass::Forbidden => {
                stats.forbidden_ops_found += 1;
                errors.push(VerificationError {
                    code: "E001".to_string(),
                    severity: Severity::Critical,
                    message: format!("Forbidden opcode '{}' used", opcode),
                    location: Some(func_name.to_string()),
                    suggestion: Some("Remove or replace with safe alternative".to_string()),
                });
            }
            OpcodeClass::Restricted => {
                stats.restricted_ops_found += 1;
                errors.push(VerificationError {
                    code: "W001".to_string(),
                    severity: Severity::Warning,
                    message: format!("Restricted opcode '{}' used", opcode),
                    location: Some(func_name.to_string()),
                    suggestion: None,
                });
            }
            OpcodeClass::Safe => {
                // No issues
            }
        }

        // Check for determinism violations
        if self.rules.determinism.required {
            if let Some(violation) = self.check_determinism_violation(stmt) {
                stats.determinism_violations += 1;
                errors.push(VerificationError {
                    code: "E010".to_string(),
                    severity: Severity::Error,
                    message: format!("Non-deterministic operation: {}", violation),
                    location: Some(func_name.to_string()),
                    suggestion: Some("Use deterministic alternatives".to_string()),
                });
            }
        }
    }

    /// Verify a terminator
    fn verify_terminator(
        &self,
        _term: &MirTerminator,
        _func_name: &str,
        _errors: &mut Vec<VerificationError>,
        _stats: &mut VerificationStats,
    ) {
        // Terminators are generally safe, but could check for:
        // - Infinite loops (requires more sophisticated analysis)
        // - Unreachable code detection
    }

    /// Get opcode name from statement
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

    /// Check for determinism violations in a statement
    fn check_determinism_violation(&self, stmt: &MirStatement) -> Option<String> {
        // Check for known non-deterministic operations
        match &stmt.rhs {
            MirRhs::Call { target, .. } => {
                // Check if calling known non-deterministic functions
                let name = format!("{:?}", target);
                if name.contains("random") || name.contains("timestamp") {
                    return Some(name);
                }
            }
            _ => {}
        }
        None
    }

    /// Verify bytecode (post-compilation check)
    pub fn verify_bytecode(&self, _bytecode: &[u8]) -> VerifierResult<VerificationReport> {
        // TODO: Implement bytecode verification
        // This would decode the bytecode and verify at the instruction level
        Ok(VerificationReport {
            errors: vec![],
            gas_report: None,
            passed: true,
            stats: VerificationStats::default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verifier_creation() {
        let verifier = Verifier::default();
        assert!(verifier.rules.limits.max_function_gas > 0);
    }

    #[test]
    fn test_verification_error_display() {
        let err = VerificationError {
            code: "E001".to_string(),
            severity: Severity::Critical,
            message: "Test error".to_string(),
            location: Some("test_func".to_string()),
            suggestion: Some("Fix it".to_string()),
        };
        let display = format!("{}", err);
        assert!(display.contains("E001"));
        assert!(display.contains("CRITICAL"));
    }
}
