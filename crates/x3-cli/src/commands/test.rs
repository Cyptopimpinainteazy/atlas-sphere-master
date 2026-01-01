//! Test command.

use crate::error::{CliError, Result};
use crate::project::Project;
use clap::Args;
use colored::Colorize;
use std::process::Command;

#[derive(Args)]
pub struct TestArgs {
    /// Test pattern to match
    pub pattern: Option<String>,

    /// Verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Generate gas report
    #[arg(long)]
    pub gas_report: bool,

    /// Fork from network
    #[arg(long)]
    pub fork: Option<String>,

    /// Fork block number
    #[arg(long)]
    pub fork_block: Option<u64>,

    /// Run only failing tests
    #[arg(long)]
    pub fail_fast: bool,
}

pub async fn execute(args: TestArgs) -> Result<()> {
    let project = Project::load_current()?;

    println!("{} Running tests for: {}", "→".blue(), project.config.name);

    let framework = &project.config.test.framework;

    match framework.as_str() {
        "forge" => run_forge_tests(&project, &args)?,
        "hardhat" => run_hardhat_tests(&project, &args)?,
        _ => {
            println!("{} Unknown test framework: {}", "!".yellow(), framework);
            println!("  Trying forge...");
            run_forge_tests(&project, &args)?;
        }
    }

    Ok(())
}

fn run_forge_tests(project: &Project, args: &TestArgs) -> Result<()> {
    let mut cmd = Command::new("forge");
    cmd.arg("test");

    if let Some(ref pattern) = args.pattern {
        cmd.arg("--match-test").arg(pattern);
    }

    if args.verbose {
        cmd.arg("-vvv");
    }

    if args.gas_report {
        cmd.arg("--gas-report");
    }

    if args.fail_fast {
        cmd.arg("--fail-fast");
    }

    // Fork configuration
    if let Some(ref fork) = args.fork {
        let fork_url = project.config.network.get_endpoint(Some(fork));
        cmd.arg("--fork-url").arg(fork_url);

        if let Some(block) = args.fork_block {
            cmd.arg("--fork-block-number").arg(block.to_string());
        }
    }

    cmd.current_dir(&project.root);

    // Run with inherited stdio for real-time output
    let status = cmd.status()?;

    if !status.success() {
        return Err(CliError::Test("Some tests failed".to_string()));
    }

    println!("{} All tests passed!", "✓".green());
    Ok(())
}

fn run_hardhat_tests(project: &Project, args: &TestArgs) -> Result<()> {
    let mut cmd = Command::new("npx");
    cmd.arg("hardhat").arg("test");

    if let Some(ref pattern) = args.pattern {
        cmd.arg("--grep").arg(pattern);
    }

    // Fork configuration via environment
    if let Some(ref fork) = args.fork {
        let fork_url = project.config.network.get_endpoint(Some(fork));
        cmd.env("HARDHAT_FORK_URL", fork_url);

        if let Some(block) = args.fork_block {
            cmd.env("HARDHAT_FORK_BLOCK", block.to_string());
        }
    }

    cmd.current_dir(&project.root);

    let status = cmd.status()?;

    if !status.success() {
        return Err(CliError::Test("Some tests failed".to_string()));
    }

    println!("{} All tests passed!", "✓".green());
    Ok(())
}
