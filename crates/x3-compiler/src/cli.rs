// crates/x3-compiler/src/cli.rs
// CLI argument parsing

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "x3c")]
#[command(about = "X3 Language Compiler", long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,

    #[arg(global = true, long, default_value = "info")]
    pub log_level: String,
}

#[derive(Subcommand)]
pub enum Command {
    /// Compile X3 source to WASM
    Compile {
        #[arg(short, long)]
        input: PathBuf,

        #[arg(short, long)]
        output: PathBuf,
    },

    /// Check syntax and types without emitting
    Check {
        #[arg(short, long)]
        input: PathBuf,
    },

    /// Run benchmarks
    Bench {
        #[arg(short, long)]
        config: PathBuf,
    },
}
