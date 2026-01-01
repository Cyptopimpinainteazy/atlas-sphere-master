//! Deploy command.

use crate::error::{CliError, Result};
use crate::project::Project;
use atlas_sdk::{AtlasClient, ComitBuilder};
use clap::Args;
use colored::Colorize;

#[derive(Args)]
pub struct DeployArgs {
    /// Contract or program to deploy
    pub target: String,

    /// Network to deploy to
    #[arg(short, long)]
    pub network: Option<String>,

    /// Private key hex (or use X3_PRIVATE_KEY env var)
    #[arg(long)]
    pub private_key: Option<String>,

    /// Constructor arguments (JSON array)
    #[arg(long)]
    pub args: Option<String>,

    /// Gas limit
    #[arg(long)]
    pub gas_limit: Option<u64>,

    /// Skip confirmation prompt
    #[arg(short, long)]
    pub yes: bool,

    /// Dry run (simulate only)
    #[arg(long)]
    pub dry_run: bool,
}

pub async fn execute(args: DeployArgs) -> Result<()> {
    let project = Project::load_current()?;

    // Get network endpoint
    let network = args.network.as_deref();
    let endpoint = project.config.network.get_endpoint(network);

    println!("{} Deploying to: {}", "→".blue(), endpoint);

    // Load contract bytecode
    let bytecode = load_bytecode(&project, &args.target)?;

    println!(
        "  {} Contract bytecode: {} bytes",
        "→".blue(),
        bytecode.len()
    );

    if args.dry_run {
        println!("{} Dry run mode - simulating deployment...", "→".yellow());
        return simulate_deploy(endpoint, &bytecode).await;
    }

    // Get private key from args or env var
    let private_key = args
        .private_key
        .or_else(|| std::env::var("X3_PRIVATE_KEY").ok())
        .ok_or_else(|| {
            CliError::Config(
                "Private key required. Use --private-key or X3_PRIVATE_KEY env var".to_string(),
            )
        })?;

    // Confirmation prompt
    if !args.yes {
        println!();
        println!("  Network: {}", network.unwrap_or("local"));
        println!("  Contract: {}", args.target);
        println!("  Bytecode: {} bytes", bytecode.len());
        println!();
        print!("Continue with deployment? [y/N] ");

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("{} Deployment cancelled", "✗".red());
            return Ok(());
        }
    }

    // Create client
    let client = AtlasClient::connect(endpoint).await?;

    // Create signer
    let seed_bytes = hex::decode(private_key.trim_start_matches("0x"))
        .map_err(|e| CliError::Config(format!("Invalid private key: {}", e)))?;

    if seed_bytes.len() != 32 {
        return Err(CliError::Config("Private key must be 32 bytes".to_string()));
    }

    let mut seed = [0u8; 32];
    seed.copy_from_slice(&seed_bytes);

    let signer = atlas_sdk::Sr25519Signer::from_seed(&seed);
    let client = client.with_signer(signer);

    // Build deployment Comit
    let gas_limit = args.gas_limit.unwrap_or(1_000_000);

    let comit = ComitBuilder::evm(&bytecode)
        .with_evm_gas_limit(gas_limit)
        .with_auto_fee()
        .build()?;

    println!("{} Submitting deployment transaction...", "→".blue());

    let result = client.submit_comit(comit).await?;

    if result.success {
        println!("{} Contract deployed successfully!", "✓".green());
        println!("  Transaction: 0x{}", hex::encode(result.tx_hash.0));
        println!("  Block: #{}", result.block_number);

        if let Some(receipt) = &result.evm_receipt {
            if let Some(addr) = &receipt.contract_address {
                println!("  Contract address: 0x{}", hex::encode(addr));
            }
        }
    } else {
        println!("{} Deployment failed", "✗".red());
        if let Some(receipt) = &result.evm_receipt {
            if let Some(reason) = &receipt.revert_reason {
                println!("  Reason: {}", reason);
            }
        }
    }

    Ok(())
}

fn load_bytecode(project: &Project, target: &str) -> Result<Vec<u8>> {
    // Try to find compiled bytecode
    let out_dir = project.out_dir();

    // Try various patterns
    let patterns = vec![
        out_dir.join(format!("{}.bin", target)),
        out_dir.join(target).join(format!("{}.bin", target)),
        out_dir.join(format!("{}.json", target)),
    ];

    for path in patterns {
        if path.exists() {
            if path.extension().map_or(false, |e| e == "json") {
                // Parse JSON artifact
                let content = std::fs::read_to_string(&path)?;
                let artifact: serde_json::Value = serde_json::from_str(&content)?;

                if let Some(bytecode) = artifact.get("bytecode").and_then(|v| v.as_str()) {
                    return hex::decode(bytecode.trim_start_matches("0x"))
                        .map_err(|e| CliError::Build(format!("Invalid bytecode: {}", e)));
                }

                if let Some(bin) = artifact.get("bin").and_then(|v| v.as_str()) {
                    return hex::decode(bin.trim_start_matches("0x"))
                        .map_err(|e| CliError::Build(format!("Invalid bytecode: {}", e)));
                }
            } else {
                // Read binary file
                let content = std::fs::read_to_string(&path)?;
                return hex::decode(content.trim().trim_start_matches("0x"))
                    .map_err(|e| CliError::Build(format!("Invalid bytecode: {}", e)));
            }
        }
    }

    Err(CliError::Build(format!(
        "Could not find compiled bytecode for '{}'. Run 'x3 build' first.",
        target
    )))
}

async fn simulate_deploy(endpoint: &str, bytecode: &[u8]) -> Result<()> {
    println!("  {} Connecting to {}...", "→".blue(), endpoint);

    let client = AtlasClient::connect(endpoint).await?;

    println!("  {} Estimating gas...", "→".blue());

    let gas = client
        .evm_estimate_gas("0x0000000000000000000000000000000000000000", bytecode)
        .await?;

    println!("  {} Estimated gas: {}", "✓".green(), gas);
    println!(
        "{} Simulation complete - deployment would succeed",
        "✓".green()
    );

    Ok(())
}
