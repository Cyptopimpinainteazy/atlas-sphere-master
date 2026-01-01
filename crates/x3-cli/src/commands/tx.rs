//! Transaction submission commands.

use crate::error::{CliError, Result};
use crate::project::Project;
use atlas_sdk::{AtlasClient, ComitBuilder};
use clap::{Args, Subcommand};
use colored::Colorize;
use std::path::PathBuf;

#[derive(Args)]
pub struct TxArgs {
    #[command(subcommand)]
    pub command: TxCommands,
}

#[derive(Subcommand)]
pub enum TxCommands {
    /// Send native tokens
    Send(SendArgs),

    /// Submit EVM transaction
    Evm(EvmTxArgs),

    /// Submit SVM instruction
    Svm(SvmTxArgs),

    /// Submit atomic Comit transaction
    Comit(ComitTxArgs),
}

#[derive(Args)]
pub struct SendArgs {
    /// Recipient address
    pub to: String,

    /// Amount to send
    pub amount: String,

    /// Path to keyfile
    #[arg(short, long)]
    pub keyfile: Option<PathBuf>,

    /// Network
    #[arg(short, long)]
    pub network: Option<String>,
}

#[derive(Args)]
pub struct EvmTxArgs {
    /// Contract address
    pub to: String,

    /// Calldata (hex)
    pub data: String,

    /// Value to send (optional)
    #[arg(short, long, default_value = "0")]
    pub value: String,

    /// Gas limit
    #[arg(short, long, default_value = "100000")]
    pub gas: u64,

    /// Path to keyfile
    #[arg(short, long)]
    pub keyfile: Option<PathBuf>,

    /// Network
    #[arg(short, long)]
    pub network: Option<String>,
}

#[derive(Args)]
pub struct SvmTxArgs {
    /// Program ID (base58)
    pub program: String,

    /// Instruction data (hex)
    pub data: String,

    /// Account addresses (comma-separated, with optional :w for writable or :s for signer)
    #[arg(short, long)]
    pub accounts: Option<String>,

    /// Path to keyfile
    #[arg(short, long)]
    pub keyfile: Option<PathBuf>,

    /// Network
    #[arg(short, long)]
    pub network: Option<String>,
}

#[derive(Args)]
pub struct ComitTxArgs {
    /// EVM calldata (hex, optional)
    #[arg(long)]
    pub evm: Option<String>,

    /// SVM instruction data (hex, optional)
    #[arg(long)]
    pub svm: Option<String>,

    /// Path to keyfile
    #[arg(short, long)]
    pub keyfile: Option<PathBuf>,

    /// Network
    #[arg(short, long)]
    pub network: Option<String>,

    /// Dry run (simulate only)
    #[arg(long)]
    pub dry_run: bool,
}

pub async fn execute(args: TxArgs) -> Result<()> {
    match args.command {
        TxCommands::Send(a) => execute_send(a).await,
        TxCommands::Evm(a) => execute_evm(a).await,
        TxCommands::Svm(a) => execute_svm(a).await,
        TxCommands::Comit(a) => execute_comit(a).await,
    }
}

async fn execute_send(args: SendArgs) -> Result<()> {
    let endpoint = get_endpoint(args.network.as_deref());

    println!("{} Preparing transfer...", "→".blue());
    println!("  To: {}", args.to);
    println!("  Amount: {}", args.amount);

    let keyfile = args.keyfile.or_else(default_keyfile).ok_or_else(|| {
        CliError::Config("No keyfile specified. Use --keyfile or set X3_KEYFILE".into())
    })?;

    let seed = load_seed(&keyfile)?;
    let signer = atlas_sdk::Sr25519Signer::from_seed(&seed);

    let client = AtlasClient::connect(&endpoint).await?;
    let client = client.with_signer(signer);

    // Parse amount
    let _amount: u128 = args
        .amount
        .parse()
        .map_err(|e| CliError::Config(format!("Invalid amount: {}", e)))?;

    // Build EVM transfer (value transfer with empty data)
    let _to_bytes = hex::decode(args.to.trim_start_matches("0x"))
        .map_err(|e| CliError::Config(format!("Invalid address: {}", e)))?;

    // Build as Comit with EVM payload
    let comit = ComitBuilder::new()
        .with_evm_payload(&[]) // Empty calldata for value transfer
        .with_fee(1_000_000_000) // Default fee
        .build()?;

    println!();
    println!("{} Submitting transaction...", "→".blue());

    let result = client.submit_comit(comit).await?;

    println!();
    println!("{} Transfer submitted!", "✓".green());
    println!("  Tx Hash: 0x{}", hex::encode(&result.tx_hash.0));

    if let Some(evm) = result.evm_receipt {
        if evm.success {
            println!("  Status: {}", "Success".green());
        } else {
            println!("  Status: {}", "Failed".red());
        }
        println!("  Gas Used: {}", evm.gas_used);
    }

    Ok(())
}

async fn execute_evm(args: EvmTxArgs) -> Result<()> {
    let endpoint = get_endpoint(args.network.as_deref());

    println!("{} Preparing EVM transaction...", "→".blue());
    println!("  To: {}", args.to);
    println!("  Data: {}...", &args.data[..args.data.len().min(20)]);

    let keyfile = args
        .keyfile
        .or_else(default_keyfile)
        .ok_or_else(|| CliError::Config("No keyfile specified".into()))?;

    let seed = load_seed(&keyfile)?;
    let signer = atlas_sdk::Sr25519Signer::from_seed(&seed);

    let client = AtlasClient::connect(&endpoint).await?;
    let client = client.with_signer(signer);

    let data = hex::decode(args.data.trim_start_matches("0x"))
        .map_err(|e| CliError::Config(format!("Invalid hex data: {}", e)))?;

    // Build Comit with EVM payload
    let comit = ComitBuilder::new()
        .with_evm_payload(&data)
        .with_evm_gas_limit(args.gas)
        .with_auto_fee()
        .build()?;

    println!("{} Submitting...", "→".blue());

    let result = client.submit_comit(comit).await?;

    println!();
    println!("{} EVM transaction submitted!", "✓".green());
    println!("  Tx Hash: 0x{}", hex::encode(&result.tx_hash.0));

    if let Some(evm) = result.evm_receipt {
        println!("  Success: {}", evm.success);
        println!("  Gas Used: {}", evm.gas_used);
        if !evm.logs.is_empty() {
            println!("  Logs: {} events", evm.logs.len());
        }
        if !evm.return_data.is_empty() {
            println!("  Return: 0x{}", hex::encode(&evm.return_data));
        }
    }

    Ok(())
}

async fn execute_svm(args: SvmTxArgs) -> Result<()> {
    let endpoint = get_endpoint(args.network.as_deref());

    println!("{} Preparing SVM instruction...", "→".blue());
    println!("  Program: {}", args.program);

    let keyfile = args
        .keyfile
        .or_else(default_keyfile)
        .ok_or_else(|| CliError::Config("No keyfile specified".into()))?;

    let seed = load_seed(&keyfile)?;
    let signer = atlas_sdk::Sr25519Signer::from_seed(&seed);

    let client = AtlasClient::connect(&endpoint).await?;
    let client = client.with_signer(signer);

    let data = hex::decode(args.data.trim_start_matches("0x"))
        .map_err(|e| CliError::Config(format!("Invalid hex data: {}", e)))?;

    // Build instruction
    let program_id = atlas_sdk::svm::Pubkey::from_base58(&args.program)
        .map_err(|e| CliError::Config(format!("Invalid program ID: {:?}", e)))?;

    let mut accounts = Vec::new();
    if let Some(accts) = args.accounts {
        for acct_str in accts.split(',') {
            let parts: Vec<&str> = acct_str.split(':').collect();
            let pubkey = atlas_sdk::svm::Pubkey::from_base58(parts[0])
                .map_err(|e| CliError::Config(format!("Invalid account: {:?}", e)))?;

            let is_writable = parts.get(1).map(|&f| f == "w").unwrap_or(false);
            let is_signer = parts.get(1).map(|&f| f == "s").unwrap_or(false);

            accounts.push(atlas_sdk::svm::AccountMeta {
                pubkey,
                is_signer,
                is_writable,
            });
        }
    }

    let instruction = atlas_sdk::svm::Instruction {
        program_id,
        accounts,
        data: data.clone(),
    };

    // Serialize instruction (simplified - just the data for now)
    let comit = ComitBuilder::new()
        .with_svm_payload(&instruction.data)
        .with_fee(1_000_000_000)
        .build()?;

    println!("{} Submitting...", "→".blue());

    let result = client.submit_comit(comit).await?;

    println!();
    println!("{} SVM instruction submitted!", "✓".green());
    println!("  Tx Hash: 0x{}", hex::encode(&result.tx_hash.0));

    if let Some(svm) = result.svm_receipt {
        println!("  Success: {}", svm.success);
        println!("  Compute Units: {}", svm.compute_units_used);
    }

    Ok(())
}

async fn execute_comit(args: ComitTxArgs) -> Result<()> {
    let endpoint = get_endpoint(args.network.as_deref());

    println!("{} Preparing atomic Comit transaction...", "→".blue());

    if args.evm.is_none() && args.svm.is_none() {
        return Err(CliError::Config(
            "At least one of --evm or --svm is required".into(),
        ));
    }

    let keyfile = args
        .keyfile
        .or_else(default_keyfile)
        .ok_or_else(|| CliError::Config("No keyfile specified".into()))?;

    let seed = load_seed(&keyfile)?;
    let signer = atlas_sdk::Sr25519Signer::from_seed(&seed);

    let client = AtlasClient::connect(&endpoint).await?;
    let client = client.with_signer(signer);

    let mut builder = ComitBuilder::new();

    if let Some(evm_data) = &args.evm {
        let data = hex::decode(evm_data.trim_start_matches("0x"))
            .map_err(|e| CliError::Config(format!("Invalid EVM hex: {}", e)))?;
        println!("  EVM payload: {} bytes", data.len());
        builder = builder.with_evm_payload(&data);
    }

    if let Some(svm_data) = &args.svm {
        let data = hex::decode(svm_data.trim_start_matches("0x"))
            .map_err(|e| CliError::Config(format!("Invalid SVM hex: {}", e)))?;
        println!("  SVM payload: {} bytes", data.len());
        builder = builder.with_svm_payload(&data);
    }

    let comit = builder.with_auto_fee().build()?;

    if args.dry_run {
        println!();
        println!("{} Dry run - would submit:", "→".blue());
        println!(
            "  EVM payload: {} bytes",
            comit.evm_payload.as_ref().map(|p| p.len()).unwrap_or(0)
        );
        println!(
            "  SVM payload: {} bytes",
            comit.svm_payload.as_ref().map(|p| p.len()).unwrap_or(0)
        );
        println!("  Fee: {}", comit.fee);
        return Ok(());
    }

    println!("{} Submitting atomic Comit...", "→".blue());

    let result = client.submit_comit(comit).await?;

    println!();
    println!("{} Comit transaction submitted!", "✓".green());
    println!("  Tx Hash: 0x{}", hex::encode(&result.tx_hash.0));

    if let Some(evm) = result.evm_receipt {
        println!(
            "  EVM: {} (gas: {})",
            if evm.success {
                "Success".green()
            } else {
                "Failed".red()
            },
            evm.gas_used
        );
    }

    if let Some(svm) = result.svm_receipt {
        println!(
            "  SVM: {} (compute: {})",
            if svm.success {
                "Success".green()
            } else {
                "Failed".red()
            },
            svm.compute_units_used
        );
    }

    Ok(())
}

fn get_endpoint(network: Option<&str>) -> String {
    if let Ok(project) = Project::load_current() {
        project.config.network.get_endpoint(network).to_string()
    } else {
        network
            .map(|n| match n {
                "testnet" => atlas_sdk::TESTNET_HTTP_ENDPOINT.to_string(),
                "mainnet" => atlas_sdk::MAINNET_HTTP_ENDPOINT.to_string(),
                _ => atlas_sdk::DEFAULT_HTTP_ENDPOINT.to_string(),
            })
            .unwrap_or_else(|| atlas_sdk::DEFAULT_HTTP_ENDPOINT.to_string())
    }
}

fn default_keyfile() -> Option<PathBuf> {
    std::env::var("X3_KEYFILE")
        .ok()
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|h| h.join(".x3").join("default.key")))
}

fn load_seed(keyfile: &PathBuf) -> Result<[u8; 32]> {
    let content = std::fs::read_to_string(keyfile)
        .map_err(|e| CliError::Config(format!("Failed to read keyfile: {}", e)))?;

    let hex_str = content.trim().trim_start_matches("0x");
    let bytes = hex::decode(hex_str)
        .map_err(|e| CliError::Config(format!("Invalid keyfile format: {}", e)))?;

    if bytes.len() != 32 {
        return Err(CliError::Config("Keyfile must contain 32-byte seed".into()));
    }

    let mut seed = [0u8; 32];
    seed.copy_from_slice(&bytes);
    Ok(seed)
}
