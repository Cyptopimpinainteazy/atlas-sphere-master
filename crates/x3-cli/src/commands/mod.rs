//! CLI commands for x3.

use clap::{Parser, Subcommand};

#[cfg(feature = "sdk")]
pub mod account;
pub mod build;
pub mod compile;
#[cfg(feature = "sdk")]
pub mod deploy;
pub mod docgen;
pub mod init;
#[cfg(feature = "sdk")]
pub mod query;
pub mod repl;
#[cfg(feature = "sdk")]
pub mod simulate;
pub mod swap;
pub mod test;
#[cfg(feature = "sdk")]
pub mod trace;
#[cfg(feature = "sdk")]
pub mod tx;

/// x3 - Atlas Sphere CLI
#[derive(Parser)]
#[command(name = "x3")]
#[command(author = "Atlas Sphere Team")]
#[command(version)]
#[command(about = "CLI for Atlas Sphere blockchain development", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// Available commands
#[derive(Subcommand)]
pub enum Commands {
    /// Initialize a new Atlas Sphere project
    Init(init::InitArgs),

    /// Build contracts and programs (requires project)
    Build(build::BuildArgs),

    /// Compile a single X3 source file (standalone)
    Compile(compile::CompileArgs),

    #[cfg(feature = "sdk")]
    /// Deploy contracts to the network
    Deploy(deploy::DeployArgs),

    /// Run tests
    Test(test::TestArgs),

    #[cfg(feature = "sdk")]
    /// Trace a transaction
    Trace(trace::TraceArgs),

    #[cfg(feature = "sdk")]
    /// Simulate a Comit transaction
    Simulate(simulate::SimulateArgs),

    /// Generate documentation
    Docgen(docgen::DocgenArgs),

    #[cfg(feature = "sdk")]
    /// Account management
    Account(account::AccountArgs),

    #[cfg(feature = "sdk")]
    /// Query blockchain state
    Query(query::QueryArgs),

    #[cfg(feature = "sdk")]
    /// Send transactions
    Tx(tx::TxArgs),

    /// Start interactive X3 REPL
    Repl(repl::ReplArgs),

    /// Cross-chain atomic swap (103 EVM chains)
    Swap(swap::SwapArgs),

    /// List and search supported chains
    Chains(swap::ChainsArgs),
}
