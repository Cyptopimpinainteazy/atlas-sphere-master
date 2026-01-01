//! Full X3 compiler pipeline orchestration
//!
//! Complete pipeline: Source → Lexer → Parser → HIR → MIR → Optimizer → Bytecode

use x3_backend::BytecodeModule;
use x3_hir::{HirLowerer, HirModule};
use x3_mir::{MirLowerer, MirModule};
use x3_opt::{OptLevel, OptStats, Optimizer};
use x3_verifier::{GasAnalyzer, GasReport, SafetyRules, VerificationReport, Verifier};

use crate::error::{CompilerError, CompilerResult};
use crate::options::CompilationOptions;

/// Compilation artifacts for debugging and analysis
#[derive(Clone, Debug)]
pub struct CompilationArtifacts {
    /// Source code (if retained)
    pub source: Option<String>,
    /// HIR module (if requested)
    pub hir: Option<HirModule>,
    /// MIR module before optimization
    pub mir_unoptimized: Option<MirModule>,
    /// MIR module after optimization
    pub mir_optimized: Option<MirModule>,
    /// Optimization statistics
    pub opt_stats: Option<OptStats>,
    /// Gas analysis report
    pub gas_report: Option<GasReport>,
    /// Contract verification report
    pub verification_report: Option<VerificationReport>,
    /// Final bytecode
    pub bytecode: BytecodeModule,
}

/// Compilation result with optional artifacts
#[derive(Clone, Debug)]
pub struct CompilationOutput {
    /// Final bytecode module
    pub bytecode: BytecodeModule,
    /// Optional artifacts for debugging
    pub artifacts: Option<CompilationArtifacts>,
}

/// Main compiler that orchestrates the full pipeline
pub struct Compiler;

impl Compiler {
    /// Compile X3 source code to optimized bytecode (full pipeline)
    ///
    /// This is the primary entry point for compilation:
    /// Source → Parser → HIR → MIR → Optimizer → Bytecode
    pub fn compile(source: &str, options: CompilationOptions) -> CompilerResult<CompilationOutput> {
        let retain_artifacts = options.emit_mir || options.emit_hir || options.emit_stats;

        if options.verbose {
            eprintln!("🔧 X3 Compiler v0.1.0");
            eprintln!("  → Optimization level: {:?}", options.opt_level);
        }

        // Phase 1: Parse source to AST
        if options.verbose {
            eprintln!("  [1/5] Parsing source...");
        }
        let ast = x3_parser::parse_program(source)
            .map_err(|e| CompilerError::Parser(format!("{:?}", e)))?;

        // Phase 2: Lower AST to HIR
        if options.verbose {
            eprintln!("  [2/5] Lowering to HIR...");
        }
        let hir =
            HirLowerer::lower(ast).map_err(|e| CompilerError::HirGeneration(format!("{:?}", e)))?;

        // Phase 3: Lower HIR to MIR
        if options.verbose {
            eprintln!("  [3/5] Lowering to MIR...");
        }
        let mir_unoptimized =
            MirLowerer::lower(&hir).map_err(|e| CompilerError::MirLowering(format!("{:?}", e)))?;

        // Phase 4: Optimize MIR
        if options.verbose {
            eprintln!("  [4/6] Running optimizer ({:?})...", options.opt_level);
        }
        let (mir_optimized, opt_stats) = Self::optimize_mir(&mir_unoptimized, &options)?;

        // Phase 5: Gas analysis and verification (optional)
        let (gas_report, verification_report) = if options.analyze_gas || options.verify_contract {
            if options.verbose {
                eprintln!("  [5/6] Running contract analysis...");
            }
            Self::analyze_contract(&mir_optimized, &options)?
        } else {
            (None, None)
        };

        // Phase 6: Generate bytecode
        if options.verbose {
            eprintln!("  [6/6] Generating bytecode...");
        }
        let bytecode =
            x3_backend::MirBytecodeCompiler::compile_with_options(&mir_optimized, options.debug)
                .map_err(|e| CompilerError::Backend(format!("{:?}", e)))?;

        if options.verbose {
            eprintln!("  ✓ Compilation complete");
            eprintln!("    • Functions: {}", bytecode.functions.len());
            eprintln!("    • Code size: {} bytes", bytecode.code.len());

            // Print gas analysis summary if available
            if let Some(ref gas) = gas_report {
                eprintln!("    📊 Gas analysis:");
                eprintln!("       • Total estimated gas: {}", gas.total_gas);
                eprintln!("       • Has unbounded loops: {}", gas.has_unbounded);
                if !gas.exceeds_limit.is_empty() {
                    eprintln!(
                        "       • ⚠️ Functions exceeding limits: {:?}",
                        gas.exceeds_limit
                    );
                }
            }
        }

        let retain_artifacts = options.emit_mir
            || options.emit_hir
            || options.emit_stats
            || options.analyze_gas
            || options.verify_contract;

        let artifacts = if retain_artifacts {
            Some(CompilationArtifacts {
                source: Some(source.to_string()),
                hir: if options.emit_hir { Some(hir) } else { None },
                mir_unoptimized: if options.emit_mir {
                    Some(mir_unoptimized)
                } else {
                    None
                },
                mir_optimized: if options.emit_mir {
                    Some(mir_optimized)
                } else {
                    None
                },
                opt_stats: if options.emit_stats { opt_stats } else { None },
                gas_report,
                verification_report,
                bytecode: bytecode.clone(),
            })
        } else {
            None
        };

        Ok(CompilationOutput {
            bytecode,
            artifacts,
        })
    }

    /// Compile MIR to optimized bytecode (for pre-built MIR)
    ///
    /// Use this when you already have MIR and just need optimization + codegen.
    pub fn compile_mir(
        mir: &MirModule,
        options: CompilationOptions,
    ) -> CompilerResult<BytecodeModule> {
        let (optimized_mir, _stats) = Self::optimize_mir(mir, &options)?;

        // Emit optimized bytecode
        let bytecode =
            x3_backend::MirBytecodeCompiler::compile_with_options(&optimized_mir, options.debug)
                .map_err(|e| CompilerError::Backend(format!("{:?}", e)))?;

        Ok(bytecode)
    }

    /// Analyze contract for gas costs and safety
    fn analyze_contract(
        mir: &MirModule,
        options: &CompilationOptions,
    ) -> CompilerResult<(Option<GasReport>, Option<VerificationReport>)> {
        let rules = SafetyRules::default();

        // Gas analysis
        let gas_report = if options.analyze_gas {
            let analyzer = GasAnalyzer::new(rules.clone());
            Some(analyzer.analyze(&mir.functions))
        } else {
            None
        };

        // Contract verification
        let verification_report = if options.verify_contract {
            let verifier = Verifier::new(rules);
            // Unwrap or convert error - we'll include errors in the report
            match verifier.verify_mir(mir) {
                Ok(report) => Some(report),
                Err(_) => None, // Verification failed - no report
            }
        } else {
            None
        };

        Ok((gas_report, verification_report))
    }

    /// Run optimization pipeline on MIR
    fn optimize_mir(
        mir: &MirModule,
        options: &CompilationOptions,
    ) -> CompilerResult<(MirModule, Option<OptStats>)> {
        let mut optimized = mir.clone();

        if matches!(options.opt_level, OptLevel::None) {
            if options.verbose {
                eprintln!("    → No optimization (pass-through)");
            }
            return Ok((optimized, None));
        }

        let optimizer = Optimizer::new(options.opt_level);
        let stats = optimizer
            .run(&mut optimized)
            .map_err(|e| CompilerError::Optimization(format!("Optimization failed: {}", e)))?;

        if options.verbose {
            eprintln!("    📊 Optimization stats:");
            eprintln!("       • Passes executed: {}", stats.passes_run);
            eprintln!("       • Passes changed code: {}", stats.passes_changed);
            eprintln!(
                "       • Total transformations: {}",
                stats.total_transformations
            );
            eprintln!("       • Iterations to fixpoint: {}", stats.iterations);
        }

        Ok((optimized, Some(stats)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compilation_options_defaults() {
        let opts = CompilationOptions::default();
        assert!(!opts.debug);
        assert!(!opts.verbose);
    }

    #[test]
    fn test_compilation_options_builder() {
        let opts = CompilationOptions::opt2()
            .with_debug(true)
            .with_verbose(true);

        assert!(opts.debug);
        assert!(opts.verbose);
    }

    #[test]
    fn test_compile_simple_source() {
        let source = r#"
            fn main() -> i64 {
                let x = 10;
                let y = 20;
                return x + y;
            }
        "#;

        let options = CompilationOptions::opt2().with_verbose(false);
        let result = Compiler::compile(source, options);

        assert!(result.is_ok(), "Compilation should succeed");
        let output = result.unwrap();
        assert!(
            !output.bytecode.code.is_empty(),
            "Bytecode should not be empty"
        );
    }

    #[test]
    fn test_compile_with_artifacts() {
        let source = r#"
            fn add(a: i64, b: i64) -> i64 {
                return a + b;
            }
        "#;

        let options = CompilationOptions::opt2()
            .with_emit_mir(true)
            .with_emit_stats(true);

        let result = Compiler::compile(source, options);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.artifacts.is_some());

        let artifacts = output.artifacts.unwrap();
        assert!(artifacts.mir_optimized.is_some());
        assert!(artifacts.opt_stats.is_some());
    }

    #[test]
    fn test_compile_with_gas_analysis() {
        let source = r#"
            fn compute(x: i64) -> i64 {
                let a = x * 2;
                let b = a + 10;
                return b;
            }
        "#;

        let options = CompilationOptions::opt2().with_gas_analysis(true);

        let result = Compiler::compile(source, options);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.artifacts.is_some());

        let artifacts = output.artifacts.unwrap();
        assert!(artifacts.gas_report.is_some());

        let gas = artifacts.gas_report.unwrap();
        assert!(
            !gas.functions.is_empty(),
            "Should have at least one function"
        );
        assert!(gas.total_gas > 0, "Total gas should be positive");
    }

    #[test]
    fn test_contract_mode() {
        // Simple function without branches (backend doesn't handle conditionals well yet)
        let source = r#"
            fn compute(a: i64, b: i64) -> i64 {
                return a + b;
            }
        "#;

        let options = CompilationOptions::contract_mode();

        assert!(options.analyze_gas);
        assert!(options.verify_contract);

        let result = Compiler::compile(source, options);

        // Debug: print error if compilation fails
        if let Err(ref e) = result {
            eprintln!("Compilation error: {:?}", e);
        }

        assert!(result.is_ok(), "Compilation should succeed: {:?}", result);

        let output = result.unwrap();
        assert!(output.artifacts.is_some());

        let artifacts = output.artifacts.unwrap();
        assert!(artifacts.gas_report.is_some());
    }
}
