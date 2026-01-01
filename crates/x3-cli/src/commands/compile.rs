//! Compile command - standalone X3 file compilation without project.
//!
//! This command allows compiling individual X3 source files without
//! needing a full project structure.

use crate::error::{CliError, Result};
use clap::{Args, ValueEnum};
use colored::Colorize;
use std::path::PathBuf;
use x3_compiler::{CompilationOptions, Compiler};

/// Emit format for X3 compilation
#[derive(Clone, ValueEnum, Default)]
pub enum EmitType {
    /// Emit bytecode (default)
    #[default]
    Bytecode,
    /// Emit MIR representation
    Mir,
    /// Emit HIR representation
    Hir,
}

#[derive(Args)]
pub struct CompileArgs {
    /// X3 source file to compile
    #[arg(required = true)]
    pub input: PathBuf,

    /// Output file (defaults to input with .x3b extension)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Optimization level (0-3)
    #[arg(short = 'O', long = "opt-level", value_parser = clap::value_parser!(u8).range(0..=3), default_value = "2")]
    pub optimization: u8,

    /// Verbose output (shows compilation progress)
    #[arg(short, long)]
    pub verbose: bool,

    /// Emit intermediate representation (mir, hir, bytecode)
    #[arg(long, value_enum)]
    pub emit: Option<EmitType>,

    /// Emit optimization statistics
    #[arg(long)]
    pub stats: bool,

    /// Emit optimized MIR alongside bytecode
    #[arg(long)]
    pub emit_mir_opt: bool,

    /// Enable debug info in output
    #[arg(short = 'g', long)]
    pub debug: bool,

    /// Disable optimization (shorthand for -O0)
    #[arg(long = "no-opt")]
    pub no_opt: bool,
}

pub async fn execute(args: CompileArgs) -> Result<()> {
    // Validate input file
    if !args.input.exists() {
        return Err(CliError::Build(format!(
            "Input file not found: {}",
            args.input.display()
        )));
    }

    if args.input.extension().map_or(true, |ext| ext != "x3") {
        return Err(CliError::Build(format!(
            "Input file must have .x3 extension: {}",
            args.input.display()
        )));
    }

    // Read source
    let source = std::fs::read_to_string(&args.input)?;
    let file_stem = args.input.file_stem().unwrap().to_string_lossy();

    // Build compilation options
    let opt_level = if args.no_opt {
        x3_compiler::OptLevel::None
    } else {
        match args.optimization {
            0 => x3_compiler::OptLevel::None,
            1 => x3_compiler::OptLevel::Basic,
            2 => x3_compiler::OptLevel::Default,
            _ => x3_compiler::OptLevel::Aggressive,
        }
    };

    let emit_format = match args.emit.as_ref().unwrap_or(&EmitType::Bytecode) {
        EmitType::Bytecode => x3_compiler::options::EmitFormat::Bytecode,
        EmitType::Mir => x3_compiler::options::EmitFormat::Mir,
        EmitType::Hir => x3_compiler::options::EmitFormat::Hir,
    };

    let options = CompilationOptions {
        opt_level,
        debug: args.debug,
        verbose: args.verbose,
        emit_hir: matches!(args.emit, Some(EmitType::Hir)),
        emit_mir: matches!(args.emit, Some(EmitType::Mir)),
        emit_mir_opt: args.emit_mir_opt || args.stats,
        emit_stats: args.stats,
        emit_format,
        analyze_gas: false,
        verify_contract: false,
    };

    if args.verbose {
        println!("{} X3 Compiler v0.1.0", "🔧".blue());
        println!("  → Input: {}", args.input.display());
        println!("  → Optimization: {:?}", opt_level);
    }

    // Compile
    let output = Compiler::compile(&source, options.clone())
        .map_err(|e| CliError::Build(format!("Compilation failed: {:?}", e)))?;

    // Determine output path
    let out_dir = args.input.parent().unwrap_or(std::path::Path::new("."));
    let bytecode_file = args
        .output
        .clone()
        .unwrap_or_else(|| out_dir.join(format!("{}.x3b", file_stem)));

    // Write bytecode
    std::fs::write(&bytecode_file, &output.bytecode.code)?;

    println!(
        "{} Compiled: {} → {} ({} bytes)",
        "✓".green(),
        args.input.display(),
        bytecode_file.display(),
        output.bytecode.code.len()
    );

    // Write stats if requested
    if args.stats {
        if let Some(ref artifacts) = output.artifacts {
            if let Some(ref stats) = artifacts.opt_stats {
                println!("\n{}", "📊 Optimization Statistics:".blue().bold());
                println!("   Passes run:        {}", stats.passes_run);
                println!("   Passes changed:    {}", stats.passes_changed);
                println!("   Transformations:   {}", stats.total_transformations);
                println!("   Iterations:        {}", stats.iterations);
            }
        }
    }

    // Write MIR if requested
    if options.emit_mir {
        if let Some(ref artifacts) = output.artifacts {
            if let Some(ref mir) = artifacts.mir_unoptimized {
                let mir_file = out_dir.join(format!("{}.mir", file_stem));
                std::fs::write(&mir_file, format!("{:#?}", mir))?;
                println!("   → MIR: {}", mir_file.display());
            }
        }
    }

    if options.emit_mir_opt {
        if let Some(ref artifacts) = output.artifacts {
            if let Some(ref mir) = artifacts.mir_optimized {
                let mir_file = out_dir.join(format!("{}.mir.opt", file_stem));
                std::fs::write(&mir_file, format!("{:#?}", mir))?;
                println!("   → Optimized MIR: {}", mir_file.display());
            }
        }
    }

    Ok(())
}
