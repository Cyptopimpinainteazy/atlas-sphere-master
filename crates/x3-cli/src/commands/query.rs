//! Query commands for blockchain state.

use crate::error::Result;
use crate::project::Project;
use atlas_sdk::AtlasClient;
use clap::{Args, Subcommand};
use colored::Colorize;

#[derive(Args)]
pub struct QueryArgs {
    #[command(subcommand)]
    pub command: QueryCommands,
}

#[derive(Subcommand)]
pub enum QueryCommands {
    /// Get block information
    Block(BlockArgs),

    /// Query account balance
    Balance(BalanceArgs),

    /// Call contract function (read-only)
    Call(CallArgs),

    /// Get chain info
    Chain(ChainArgs),
}

#[derive(Args)]
pub struct BlockArgs {
    /// Block number (latest if omitted)
    pub number: Option<u64>,

    /// Network to query
    #[arg(short, long)]
    pub network: Option<String>,
}

#[derive(Args)]
pub struct BalanceArgs {
    /// Account address
    pub address: String,

    /// Asset ID (0 for native token)
    #[arg(short, long, default_value = "0")]
    pub asset_id: u32,

    /// Network to query
    #[arg(short, long)]
    pub network: Option<String>,
}

#[derive(Args)]
pub struct CallArgs {
    /// Contract address
    pub address: String,

    /// Calldata (hex)
    pub data: String,

    /// Network to query
    #[arg(short, long)]
    pub network: Option<String>,
}

#[derive(Args)]
pub struct ChainArgs {
    /// Network to query
    #[arg(short, long)]
    pub network: Option<String>,
}

pub async fn execute(args: QueryArgs) -> Result<()> {
    match args.command {
        QueryCommands::Block(a) => execute_block(a).await,
        QueryCommands::Balance(a) => execute_balance(a).await,
        QueryCommands::Call(a) => execute_call(a).await,
        QueryCommands::Chain(a) => execute_chain(a).await,
    }
}

async fn execute_block(args: BlockArgs) -> Result<()> {
    let endpoint = get_endpoint(args.network.as_deref());

    println!("{} Querying block...", "→".blue());

    let client = AtlasClient::connect(&endpoint).await?;

    let hash = match args.number {
        Some(num) => {
            println!("  Block number: {}", num);
            client.block_hash(num).await?
        }
        None => {
            println!("  Latest block");
            client.latest_block_hash().await?
        }
    };

    let header = client.block_header(Some(hash)).await?;

    println!();
    println!("{}", "Block Information".bold());
    println!("{}", "─".repeat(50));
    println!("  Number: {}", header.number);
    println!("  Hash: 0x{}", hex::encode(&hash.0[..16]));
    println!("  Parent: 0x{}", hex::encode(&header.parent_hash.0[..16]));
    println!(
        "  State Root: 0x{}",
        hex::encode(&header.state_root.0[..16])
    );
    println!(
        "  Extrinsics: 0x{}",
        hex::encode(&header.extrinsics_root.0[..16])
    );

    Ok(())
}

async fn execute_balance(args: BalanceArgs) -> Result<()> {
    let endpoint = get_endpoint(args.network.as_deref());

    println!("{} Querying balance...", "→".blue());
    println!("  Address: {}", args.address);
    println!("  Asset ID: {}", args.asset_id);

    let client = AtlasClient::connect(&endpoint).await?;

    let balance = if args.asset_id == 0 {
        client.native_balance(&args.address).await?
    } else {
        client.balance(&args.address, args.asset_id).await?
    };

    println!();
    println!(
        "  Balance: {} {}",
        atlas_sdk::utils::format_balance(balance, 12),
        if args.asset_id == 0 { "X3" } else { "tokens" }
    );

    // Try to get more account info
    if let Ok(info) = client.account_info(&args.address).await {
        println!("  Nonce: {}", info.nonce);
        println!(
            "  Native Balance: {}",
            atlas_sdk::utils::format_balance(info.native_balance, 12)
        );
        println!("  Authorized: {}", info.is_authorized);
    }

    Ok(())
}

async fn execute_call(args: CallArgs) -> Result<()> {
    let endpoint = get_endpoint(args.network.as_deref());

    println!("{} Calling contract...", "→".blue());
    println!("  Address: {}", args.address);

    let data = hex::decode(args.data.trim_start_matches("0x"))
        .map_err(|e| crate::error::CliError::Config(format!("Invalid hex: {}", e)))?;

    let client = AtlasClient::connect(&endpoint).await?;
    let result = client.evm_call(&args.address, &data).await?;

    println!();
    println!("  Result: 0x{}", hex::encode(&result).green());

    // Try to decode if it looks like a common type
    if result.len() == 32 {
        // Check if high bytes are zero (likely a small number)
        if result[..24].iter().all(|&b| b == 0) {
            let mut val_bytes = [0u8; 8];
            val_bytes.copy_from_slice(&result[24..]);
            let value = u64::from_be_bytes(val_bytes);
            println!("  As uint64: {}", value);
        }
    }

    Ok(())
}

async fn execute_chain(args: ChainArgs) -> Result<()> {
    let endpoint = get_endpoint(args.network.as_deref());

    println!("{} Querying chain info...", "→".blue());

    let client = AtlasClient::connect(&endpoint).await?;

    let chain = client.chain_name().await?;
    let node = client.node_name().await?;
    let version = client.node_version().await?;
    let latest = client.latest_block_hash().await?;
    let finalized = client.finalized_block_hash().await?;

    println!();
    println!("{}", "Chain Information".bold());
    println!("{}", "─".repeat(50));
    println!("  Chain: {}", chain);
    println!("  Node: {}", node);
    println!("  Version: {}", version);
    println!("  Latest Block: 0x{}", hex::encode(&latest.0[..8]));
    println!("  Finalized: 0x{}", hex::encode(&finalized.0[..8]));

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
