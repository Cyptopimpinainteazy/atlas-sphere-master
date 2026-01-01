// crates/x3-compiler/src/main.rs
// X3 Compiler entrypoint

use anyhow::Result;
use std::path::PathBuf;

mod analyzer;
mod ast;
mod bench;
mod cli;
mod codegen;
mod lexer;
mod parser;
mod verifier;

#[cfg(test)]
mod smoke_tests;

use cli::Args;
use clap::Parser;

fn main() -> Result<()> {
    env_logger::init();

    let args = Args::parse();

    match args.command {
        cli::Command::Compile { input, output } => {
            compile_file(&input, &output)?;
        }
        cli::Command::Check { input } => {
            check_file(&input)?;
        }
        cli::Command::Bench { config } => {
            bench::run(&config)?;
        }
    }

    Ok(())
}

fn compile_file(input: &PathBuf, output: &PathBuf) -> Result<()> {
    log::info!("Compiling {} -> {}", input.display(), output.display());

    let source = std::fs::read_to_string(input)?;

    // Lex
    let tokens = lexer::tokenize(&source)?;
    log::debug!("Lexed {} tokens", tokens.len());

    // Parse
    let ast = parser::parse(tokens)?;
    log::debug!("Parsed AST");

    // Analyze (types, imports, etc.)
    let analyzed = analyzer::analyze(&ast)?;
    log::debug!("Analysis complete");

    // Verify (ZK, flashloan safety, AI sig checks)
    verifier::verify(&analyzed)?;
    log::info!("Verification passed");

    // Codegen (WASM + host ABI)
    let wasm = codegen::generate(&analyzed)?;
    std::fs::write(output, wasm)?;

    log::info!("✓ Compiled to {}", output.display());
    Ok(())
}

fn check_file(input: &PathBuf) -> Result<()> {
    log::info!("Checking {}", input.display());

    let source = std::fs::read_to_string(input)?;
    let tokens = lexer::tokenize(&source)?;
    let ast = parser::parse(tokens)?;
    let analyzed = analyzer::analyze(&ast)?;
    verifier::verify(&analyzed)?;

    log::info!("✓ Check passed");
    Ok(())
}
