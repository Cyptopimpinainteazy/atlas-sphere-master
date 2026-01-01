//! x3 CLI - Command-line interface for Atlas Sphere development.
//!
//! This tool provides commands for building, deploying, testing,
//! and interacting with Atlas Sphere blockchain.

use clap::Parser;
use colored::Colorize;
use std::process::ExitCode;

mod commands;
mod config;
mod error;
mod project;
mod templates;

use commands::{Cli, Commands};
use error::Result;

#[tokio::main]
async fn main() -> ExitCode {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let cli = Cli::parse();

    match run(cli).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{}: {}", "error".red().bold(), e);
            ExitCode::FAILURE
        }
    }
}

async fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Init(args) => commands::init::execute(args).await,
        Commands::Build(args) => commands::build::execute(args).await,
        Commands::Compile(args) => commands::compile::execute(args).await,
        #[cfg(feature = "sdk")]
        Commands::Deploy(args) => commands::deploy::execute(args).await,
        Commands::Test(args) => commands::test::execute(args).await,
        #[cfg(feature = "sdk")]
        Commands::Trace(args) => commands::trace::execute(args).await,
        #[cfg(feature = "sdk")]
        Commands::Simulate(args) => commands::simulate::execute(args).await,
        Commands::Docgen(args) => commands::docgen::execute(args).await,
        #[cfg(feature = "sdk")]
        Commands::Account(args) => commands::account::execute(args).await,
        #[cfg(feature = "sdk")]
        Commands::Query(args) => commands::query::execute(args).await,
        #[cfg(feature = "sdk")]
        Commands::Tx(args) => commands::tx::execute(args).await,
        Commands::Repl(args) => commands::repl::execute(args).await,
        Commands::Swap(args) => commands::swap::execute(args).await,
        Commands::Chains(args) => commands::swap::execute_chains(args).await,
    }
}
