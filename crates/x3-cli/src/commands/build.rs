//! Build command.

use crate::error::{CliError, Result};
use crate::project::Project;
use clap::{Args, ValueEnum};
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::PathBuf;
use std::process::Command;
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
pub struct BuildArgs {
    /// Build only EVM contracts
    #[arg(long)]
    pub evm_only: bool,

    /// Build only SVM programs
    #[arg(long)]
    pub svm_only: bool,

    /// Build only X3 programs
    #[arg(long)]
    pub x3_only: bool,

    /// X3 source file to compile (optional, defaults to all .x3 files)
    #[arg(long)]
    pub x3_file: Option<PathBuf>,

    /// Optimization level (0-3)
    #[arg(short = 'O', long = "opt-level", value_parser = clap::value_parser!(u8).range(0..=3))]
    pub optimization: Option<u8>,

    /// Skip compilation, only generate ABIs
    #[arg(long)]
    pub abi_only: bool,

    /// Verbose output (shows compilation progress)
    #[arg(short, long)]
    pub verbose: bool,

    /// Emit intermediate representation (mir, hir, bytecode)
    #[arg(long, value_enum)]
    pub emit: Option<EmitType>,

    /// Emit optimization statistics
    #[arg(long)]
    pub stats: bool,

    /// Emit optimized MIR
    #[arg(long)]
    pub emit_mir_opt: bool,

    /// Enable debug info in output
    #[arg(long)]
    pub debug: bool,
}

pub async fn execute(args: BuildArgs) -> Result<()> {
    let project = Project::load_current()?;

    println!("{} Building project: {}", "→".blue(), project.config.name);

    // Create output directory
    std::fs::create_dir_all(project.out_dir())?;

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.blue} {msg}")
            .unwrap(),
    );

    let build_evm = !args.svm_only && !args.x3_only;
    let build_svm = !args.evm_only && !args.x3_only;
    let build_x3 = !args.evm_only && !args.svm_only;

    // Build X3 programs
    if build_x3 {
        pb.set_message("Compiling X3 programs...");
        build_x3_programs(&project, &args)?;
        pb.set_message("X3 programs compiled");
    }

    // Build EVM contracts
    if build_evm {
        pb.set_message("Compiling EVM contracts...");
        build_evm_contracts(&project, &args)?;
        pb.set_message("EVM contracts compiled");
    }

    // Build SVM programs
    if build_svm {
        pb.set_message("Compiling SVM programs...");
        build_svm_programs(&project, &args)?;
        pb.set_message("SVM programs compiled");
    }

    pb.finish_with_message("Build complete");

    println!(
        "{} Build artifacts written to: {}",
        "✓".green(),
        project.out_dir().display()
    );

    Ok(())
}

/// Build X3 programs using x3-compiler
fn build_x3_programs(project: &Project, args: &BuildArgs) -> Result<()> {
    // Find X3 source files
    let x3_files = if let Some(ref file) = args.x3_file {
        vec![file.clone()]
    } else {
        find_x3_files(&project.root)?
    };

    if x3_files.is_empty() {
        println!("  {} No X3 files found", "○".yellow());
        return Ok(());
    }

    println!("  {} Found {} X3 file(s)", "→".blue(), x3_files.len());

    // Build compilation options
    let opt_level = match args.optimization {
        Some(0) => x3_compiler::OptLevel::None,
        Some(1) => x3_compiler::OptLevel::Basic,
        Some(2) | None => x3_compiler::OptLevel::Default,
        Some(3) | Some(_) => x3_compiler::OptLevel::Aggressive,
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
        emit_mir_opt: args.emit_mir_opt,
        emit_stats: args.stats,
        emit_format,
        analyze_gas: false,
        verify_contract: false,
    };

    // Compile each X3 file
    for file in &x3_files {
        compile_x3_file(project, file, &options, args)?;
    }

    Ok(())
}

fn find_x3_files(root: &std::path::Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    // Check src/x3 and contracts/x3 directories
    for subdir in &["src", "contracts", "x3", "src/x3"] {
        let dir = root.join(subdir);
        if dir.exists() {
            for entry in walkdir::WalkDir::new(&dir)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "x3") {
                    files.push(path.to_path_buf());
                }
            }
        }
    }

    Ok(files)
}

fn compile_x3_file(
    project: &Project,
    file: &PathBuf,
    options: &CompilationOptions,
    args: &BuildArgs,
) -> Result<()> {
    let source = std::fs::read_to_string(file)?;
    let file_stem = file.file_stem().unwrap().to_string_lossy();

    if args.verbose {
        println!(
            "  {} Compiling {} (opt={:?})",
            "→".blue(),
            file.display(),
            options.opt_level
        );
    }

    let output = Compiler::compile(&source, options.clone())
        .map_err(|e| CliError::Build(format!("X3 compilation failed: {:?}", e)))?;

    // Write bytecode output
    let out_dir = project.out_dir();
    std::fs::create_dir_all(&out_dir)?;

    let bytecode_file = out_dir.join(format!("{}.x3b", file_stem));
    std::fs::write(&bytecode_file, &output.bytecode.code)?;

    println!(
        "  {} Compiled: {} → {} ({} bytes)",
        "✓".green(),
        file.display(),
        bytecode_file.display(),
        output.bytecode.code.len()
    );

    // Write stats if requested
    if args.stats {
        if let Some(ref artifacts) = output.artifacts {
            if let Some(ref stats) = artifacts.opt_stats {
                println!("      📊 Optimization stats:");
                println!("         Passes run: {}", stats.passes_run);
                println!("         Passes changed: {}", stats.passes_changed);
                println!("         Transformations: {}", stats.total_transformations);
                println!("         Iterations: {}", stats.iterations);
            }
        }
    }

    // Write MIR if requested
    if options.emit_mir {
        if let Some(ref artifacts) = output.artifacts {
            if let Some(ref mir) = artifacts.mir_unoptimized {
                let mir_file = out_dir.join(format!("{}.mir", file_stem));
                std::fs::write(&mir_file, format!("{:#?}", mir))?;
                println!("      → MIR written to: {}", mir_file.display());
            }
        }
    }

    if options.emit_mir_opt {
        if let Some(ref artifacts) = output.artifacts {
            if let Some(ref mir) = artifacts.mir_optimized {
                let mir_file = out_dir.join(format!("{}.mir.opt", file_stem));
                std::fs::write(&mir_file, format!("{:#?}", mir))?;
                println!("      → Optimized MIR written to: {}", mir_file.display());
            }
        }
    }

    Ok(())
}

fn build_evm_contracts(project: &Project, args: &BuildArgs) -> Result<()> {
    let sol_files = project.find_solidity_files()?;

    if sol_files.is_empty() {
        println!("  {} No Solidity files found", "○".yellow());
        return Ok(());
    }

    println!(
        "  {} Found {} Solidity file(s)",
        "→".blue(),
        sol_files.len()
    );

    // Check for forge/solc
    let compiler = &project.config.build.evm_compiler;

    match compiler.as_str() {
        "forge" => build_with_forge(project, args)?,
        "solc" | _ => build_with_solc(project, &sol_files, args)?,
    }

    Ok(())
}

fn build_with_forge(project: &Project, args: &BuildArgs) -> Result<()> {
    let mut cmd = Command::new("forge");
    cmd.arg("build");
    cmd.arg("--out").arg(project.out_dir());

    if let Some(opt) = args.optimization {
        cmd.arg("--optimize");
        cmd.arg("--optimizer-runs").arg(opt.to_string());
    }

    cmd.current_dir(&project.root);

    let output = cmd.output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(CliError::Build(format!("Forge build failed: {}", stderr)));
    }

    if args.verbose {
        let stdout = String::from_utf8_lossy(&output.stdout);
        println!("{}", stdout);
    }

    Ok(())
}

fn build_with_solc(
    project: &Project,
    files: &[std::path::PathBuf],
    args: &BuildArgs,
) -> Result<()> {
    for file in files {
        let mut cmd = Command::new("solc");

        // Output options
        if args.abi_only {
            cmd.arg("--abi");
        } else {
            cmd.arg("--combined-json").arg("abi,bin,bin-runtime");
        }

        // Optimization
        if let Some(opt) = args.optimization {
            if opt > 0 {
                cmd.arg("--optimize");
                cmd.arg("--optimize-runs")
                    .arg((200 * opt as u32).to_string());
            }
        }

        // Output directory
        cmd.arg("-o").arg(project.out_dir());
        cmd.arg("--overwrite");

        // Input file
        cmd.arg(file);

        cmd.current_dir(&project.root);

        let output = cmd.output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CliError::Build(format!(
                "Solc compilation failed for {}: {}",
                file.display(),
                stderr
            )));
        }

        println!("  {} Compiled: {}", "✓".green(), file.display());
    }

    Ok(())
}

fn build_svm_programs(project: &Project, args: &BuildArgs) -> Result<()> {
    let svm_src = project.svm_src();

    if !svm_src.exists() {
        println!("  {} No SVM programs found", "○".yellow());
        return Ok(());
    }

    // Check for Cargo.toml in SVM directory
    let cargo_toml = svm_src.join("Cargo.toml");
    if !cargo_toml.exists() {
        println!("  {} No Cargo.toml in SVM directory", "○".yellow());
        return Ok(());
    }

    println!("  {} Building SVM programs with Cargo", "→".blue());

    let mut cmd = Command::new("cargo");
    cmd.arg("build-sbf");

    if !args.verbose {
        cmd.arg("--quiet");
    }

    cmd.current_dir(&svm_src);

    let output = cmd.output();

    match output {
        Ok(output) if output.status.success() => {
            println!("  {} SVM programs built", "✓".green());
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // cargo build-sbf might not be installed, that's ok
            if stderr.contains("no such subcommand") {
                println!(
                    "  {} cargo build-sbf not available, skipping SVM build",
                    "○".yellow()
                );
            } else {
                return Err(CliError::Build(format!("SVM build failed: {}", stderr)));
            }
        }
        Err(e) => {
            println!("  {} Could not run cargo: {}", "○".yellow(), e);
        }
    }

    Ok(())
}
