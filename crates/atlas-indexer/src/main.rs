//! Atlas Sphere Blockchain Indexer
//!
//! A high-performance indexer that listens to Atlas Sphere blockchain events,
//! processes blocks and transactions, and stores them in PostgreSQL for
//! efficient querying via the API Gateway.

use clap::Parser;
use tracing::{error, info};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod config;
mod db;
mod error;
mod indexer;
mod metrics;
mod models;
mod server;

use config::IndexerConfig;
use error::Result;

#[derive(Parser)]
#[command(name = "atlas-indexer")]
#[command(about = "Atlas Sphere blockchain indexer", long_about = None)]
struct Cli {
    /// Path to config file
    #[arg(short, long, default_value = "indexer.toml")]
    config: String,

    /// Database URL (overrides config)
    #[arg(long, env = "DATABASE_URL")]
    database_url: Option<String>,

    /// Node RPC URL (overrides config)
    #[arg(long, env = "NODE_URL")]
    node_url: Option<String>,

    /// Start from block number
    #[arg(long)]
    from_block: Option<u64>,

    /// Run database migrations
    #[arg(long)]
    migrate: bool,

    /// Log level
    #[arg(long, default_value = "info")]
    log_level: String,

    /// Metrics port
    #[arg(long, default_value = "9615")]
    metrics_port: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse CLI arguments
    let cli = Cli::parse();

    // Initialize logging
    init_logging(&cli.log_level);

    info!(
        "Starting Atlas Sphere Indexer v{}",
        env!("CARGO_PKG_VERSION")
    );

    // Load configuration
    let mut config = IndexerConfig::load(&cli.config)?;

    // Override with CLI args
    if let Some(url) = cli.database_url {
        config.database.url = url;
    }
    if let Some(url) = cli.node_url {
        config.node.url = url;
    }
    if let Some(block) = cli.from_block {
        config.indexer.start_block = Some(block);
    }
    config.metrics.port = cli.metrics_port;

    // Initialize database
    let db = db::Database::connect(&config.database).await?;

    // Run migrations if requested
    if cli.migrate {
        info!("Running database migrations...");
        db.migrate().await?;
        info!("Migrations complete");
    }

    // Initialize metrics
    let metrics = metrics::Metrics::new();

    // Start metrics/health server
    let server_handle = tokio::spawn(server::run(config.metrics.port, metrics.clone()));

    // Create and run indexer
    let indexer = indexer::Indexer::new(config, db, metrics).await?;

    // Handle shutdown gracefully
    let indexer_handle = tokio::spawn(async move { indexer.run().await });

    // Wait for shutdown signal
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Received shutdown signal");
        }
        result = indexer_handle => {
            match result {
                Ok(Ok(())) => info!("Indexer stopped"),
                Ok(Err(e)) => error!("Indexer error: {}", e),
                Err(e) => error!("Indexer task panic: {}", e),
            }
        }
        result = server_handle => {
            match result {
                Ok(Ok(())) => info!("Server stopped"),
                Ok(Err(e)) => error!("Server error: {}", e),
                Err(e) => error!("Server task panic: {}", e),
            }
        }
    }

    info!("Shutdown complete");
    Ok(())
}

fn init_logging(level: &str) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));

    tracing_subscriber::registry()
        .with(fmt::layer().with_target(true))
        .with(filter)
        .init();
}
