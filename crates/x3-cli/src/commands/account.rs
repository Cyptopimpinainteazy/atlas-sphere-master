//! Account management commands.

use crate::error::Result;
use crate::project::Project;
use atlas_sdk::{AtlasClient, Signer, Sr25519Signer};
use clap::{Args, Subcommand};
use colored::Colorize;

#[derive(Args)]
pub struct AccountArgs {
    #[command(subcommand)]
    pub command: AccountCommands,
}

#[derive(Subcommand)]
pub enum AccountCommands {
    /// Generate a new account
    New(NewAccountArgs),

    /// Get account balance
    Balance(BalanceArgs),

    /// Get account info
    Info(InfoArgs),

    /// Request testnet tokens from faucet
    Faucet(FaucetArgs),
}

#[derive(Args)]
pub struct NewAccountArgs {
    /// Output format (json, text)
    #[arg(short, long, default_value = "text")]
    pub format: String,
}

#[derive(Args)]
pub struct BalanceArgs {
    /// Account address
    pub address: String,

    /// Network to query
    #[arg(short, long)]
    pub network: Option<String>,

    /// Asset ID (0 = native)
    #[arg(long, default_value = "0")]
    pub asset: u32,
}

#[derive(Args)]
pub struct InfoArgs {
    /// Account address
    pub address: String,

    /// Network to query
    #[arg(short, long)]
    pub network: Option<String>,
}

#[derive(Args)]
pub struct FaucetArgs {
    /// Account address to fund
    pub address: String,

    /// Amount to request
    #[arg(long, default_value = "100")]
    pub amount: u64,
}

pub async fn execute(args: AccountArgs) -> Result<()> {
    match args.command {
        AccountCommands::New(a) => execute_new(a).await,
        AccountCommands::Balance(a) => execute_balance(a).await,
        AccountCommands::Info(a) => execute_info(a).await,
        AccountCommands::Faucet(a) => execute_faucet(a).await,
    }
}

async fn execute_new(args: NewAccountArgs) -> Result<()> {
    let signer = Sr25519Signer::generate();
    let account_id = signer.account_id();

    if args.format == "json" {
        let output = serde_json::json!({
            "address": format!("0x{}", hex::encode(account_id)),
            "public_key": format!("0x{}", hex::encode(account_id)),
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("{} Generated new account", "✓".green());
        println!();
        println!("  Address: 0x{}", hex::encode(account_id));
        println!();
        println!("{}", "⚠️  Save this information securely!".yellow());
        println!("  The private key is not recoverable.");
    }

    Ok(())
}

async fn execute_balance(args: BalanceArgs) -> Result<()> {
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

    println!("{} Querying balance...", "→".blue());

    let client = AtlasClient::connect(&endpoint).await?;
    let balance = client.balance(&args.address, args.asset).await?;

    let symbol = if args.asset == 0 { "ATLAS" } else { "tokens" };
    let formatted = format_balance(balance);

    println!();
    println!("  Address: {}", args.address);
    println!("  Balance: {} {}", formatted.green(), symbol);

    Ok(())
}

async fn execute_info(args: InfoArgs) -> Result<()> {
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

    println!("{} Querying account info...", "→".blue());

    let client = AtlasClient::connect(&endpoint).await?;

    // Get account info
    match client.account_info(&args.address).await {
        Ok(info) => {
            println!();
            println!("{}", "Account Information".bold());
            println!("{}", "─".repeat(40));
            println!("  Nonce: {}", info.nonce);
            println!("  Native Balance: {}", format_balance(info.native_balance));
            println!(
                "  Authorized: {}",
                if info.is_authorized {
                    "Yes".green()
                } else {
                    "No".yellow()
                }
            );

            if !info.asset_balances.is_empty() {
                println!();
                println!("  Asset Balances:");
                for (asset_id, balance) in &info.asset_balances {
                    println!(
                        "    {}: {}",
                        hex::encode(&asset_id.0[..4]),
                        format_balance(*balance)
                    );
                }
            }
        }
        Err(e) => {
            println!("{} Could not fetch account info: {}", "!".yellow(), e);

            // Try basic balance query
            if let Ok(balance) = client.native_balance(&args.address).await {
                println!();
                println!("  Native Balance: {}", format_balance(balance));
            }
        }
    }

    Ok(())
}

async fn execute_faucet(args: FaucetArgs) -> Result<()> {
    println!("{} Requesting tokens from faucet...", "→".blue());
    println!();
    println!("  Address: {}", args.address);
    println!("  Amount: {} ATLAS", args.amount);
    println!();

    // For now, just print instructions
    println!("{} Testnet faucet available at:", "ℹ".cyan());
    println!("  https://faucet.testnet.atlas-sphere.io");
    println!();
    println!("Enter your address there to receive testnet tokens.");

    Ok(())
}

fn format_balance(balance: u128) -> String {
    let whole = balance / 1_000_000_000_000_000_000;
    let frac = (balance % 1_000_000_000_000_000_000) / 1_000_000_000_000_000;

    if frac == 0 {
        format!("{}", whole)
    } else {
        format!("{}.{:03}", whole, frac)
    }
}
