//! Simulate command for Comit transactions.

use crate::error::{CliError, Result};
use crate::project::Project;
use atlas_sdk::{AtlasClient, ComitBuilder};
use clap::Args;
use colored::Colorize;

#[derive(Args)]
pub struct SimulateArgs {
    /// EVM calldata (hex)
    #[arg(long)]
    pub evm: Option<String>,

    /// SVM instructions (hex)
    #[arg(long)]
    pub svm: Option<String>,

    /// Load EVM payload from file
    #[arg(long)]
    pub evm_file: Option<String>,

    /// Load SVM payload from file
    #[arg(long)]
    pub svm_file: Option<String>,

    /// Network to simulate on
    #[arg(short, long)]
    pub network: Option<String>,

    /// EVM gas limit
    #[arg(long, default_value = "500000")]
    pub evm_gas: u64,

    /// SVM compute units
    #[arg(long, default_value = "200000")]
    pub svm_compute: u64,

    /// Show detailed execution trace
    #[arg(short, long)]
    pub verbose: bool,
}

pub async fn execute(args: SimulateArgs) -> Result<()> {
    // Load payloads
    let evm_payload = load_payload(&args.evm, &args.evm_file)?;
    let svm_payload = load_payload(&args.svm, &args.svm_file)?;

    if evm_payload.is_none() && svm_payload.is_none() {
        return Err(CliError::Config(
            "At least one payload (--evm or --svm) is required".to_string(),
        ));
    }

    // Get network endpoint
    let endpoint = if let Ok(project) = Project::load_current() {
        project
            .config
            .network
            .get_endpoint(args.network.as_deref())
            .to_string()
    } else {
        args.network
            .clone()
            .unwrap_or_else(|| atlas_sdk::DEFAULT_HTTP_ENDPOINT.to_string())
    };

    println!("{} Simulating Comit transaction", "→".blue());
    println!("  Network: {}", endpoint);

    // Build Comit
    let mut builder = ComitBuilder::new();

    if let Some(ref evm) = evm_payload {
        println!("  EVM payload: {} bytes", evm.len());
        builder = builder.with_evm_payload(evm);
        builder = builder.with_evm_gas_limit(args.evm_gas);
    }

    if let Some(ref svm) = svm_payload {
        println!("  SVM payload: {} bytes", svm.len());
        builder = builder.with_svm_payload(svm);
        builder = builder.with_svm_compute_limit(args.svm_compute);
    }

    let comit = builder.with_auto_fee().build()?;

    println!();
    println!("{}", "Comit Details".bold());
    println!("{}", "─".repeat(40));
    println!("  Nonce: {}", comit.nonce);
    println!("  EVM gas limit: {}", comit.evm_gas_limit);
    println!("  SVM compute limit: {}", comit.svm_compute_limit);
    println!("  Fee: {} ATLAS", format_balance(comit.fee));
    println!(
        "  Prepare root: 0x{}",
        hex::encode(&comit.prepare_root.0[..8])
    );

    // Connect and simulate
    println!();
    println!("{} Connecting to node...", "→".blue());

    let client = AtlasClient::connect(&endpoint).await?;

    // Simulate via eth_call for EVM portion
    if let Some(ref evm) = comit.evm_payload {
        println!("{} Simulating EVM execution...", "→".blue());

        match client
            .evm_estimate_gas("0x0000000000000000000000000000000000000000", evm)
            .await
        {
            Ok(gas) => {
                println!("  {} Estimated gas: {}", "✓".green(), gas);
            }
            Err(e) => {
                println!("  {} EVM simulation failed: {}", "✗".red(), e);
            }
        }
    }

    println!();
    println!("{} Simulation complete", "✓".green());
    println!();
    println!("To submit this transaction:");
    println!(
        "  x3 tx submit --evm <calldata> --network {}",
        args.network.as_deref().unwrap_or("local")
    );

    Ok(())
}

fn load_payload(hex: &Option<String>, file: &Option<String>) -> Result<Option<Vec<u8>>> {
    if let Some(hex_str) = hex {
        let bytes = hex::decode(hex_str.trim_start_matches("0x"))
            .map_err(|e| CliError::Config(format!("Invalid hex: {}", e)))?;
        return Ok(Some(bytes));
    }

    if let Some(file_path) = file {
        let content = std::fs::read_to_string(file_path)?;
        let bytes = hex::decode(content.trim().trim_start_matches("0x"))
            .map_err(|e| CliError::Config(format!("Invalid hex in file: {}", e)))?;
        return Ok(Some(bytes));
    }

    Ok(None)
}

fn format_balance(balance: u128) -> String {
    let whole = balance / 1_000_000_000_000_000_000;
    let frac = (balance % 1_000_000_000_000_000_000) / 1_000_000_000_000_000;
    format!("{}.{:03}", whole, frac)
}
