//! X3 Sidecar Main Entry Point

use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::Level;
use x3_sidecar::{init_logging, SidecarConfig, SidecarDaemon};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to configuration file
    #[arg(short, long, default_value = "sidecar.toml")]
    config: PathBuf,

    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "info")]
    log_level: String,

    /// RPC port
    #[arg(long, default_value = "9955")]
    rpc_port: u16,

    /// Metrics port
    #[arg(long, default_value = "9956")]
    metrics_port: u16,

    /// Chain RPC URL
    #[arg(long, default_value = "http://localhost:9944")]
    chain_rpc: String,

    /// Data directory
    #[arg(long, default_value = "./x3-sidecar-data")]
    data_dir: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Initialize logging
    let level = match args.log_level.as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };
    init_logging(level);

    println!(
        r#"
    ╔═══════════════════════════════════════════════════════════════╗
    ║                                                               ║
    ║     ██╗  ██╗██████╗     ███████╗██╗██████╗ ███████╗ ██████╗   ║
    ║     ╚██╗██╔╝╚════██╗    ██╔════╝██║██╔══██╗██╔════╝██╔════╝   ║
    ║      ╚███╔╝  █████╔╝    ███████╗██║██║  ██║█████╗  ██║        ║
    ║      ██╔██╗  ╚═══██╗    ╚════██║██║██║  ██║██╔══╝  ██║        ║
    ║     ██╔╝ ██╗██████╔╝    ███████║██║██████╔╝███████╗╚██████╗   ║
    ║     ╚═╝  ╚═╝╚═════╝     ╚══════╝╚═╝╚═════╝ ╚══════╝ ╚═════╝   ║
    ║                                                               ║
    ║         Atlas Sphere Swarm Execution Node v0.1.0              ║
    ║                                                               ║
    ╚═══════════════════════════════════════════════════════════════╝
    "#
    );

    // Load or create config
    let config = if args.config.exists() {
        SidecarConfig::load(&args.config)?
    } else {
        SidecarConfig {
            rpc_port: args.rpc_port,
            metrics_port: args.metrics_port,
            chain_rpc: args.chain_rpc,
            data_dir: args.data_dir,
            ..Default::default()
        }
    };

    tracing::info!("Configuration loaded");
    tracing::info!("  RPC Port: {}", config.rpc_port);
    tracing::info!("  Chain RPC: {}", config.chain_rpc);
    tracing::info!("  Data Dir: {:?}", config.data_dir);

    // Create and run daemon
    let daemon = Arc::new(SidecarDaemon::new(config)?);
    daemon.run().await?;

    Ok(())
}
