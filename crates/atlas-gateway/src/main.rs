//! Atlas Gateway - REST and GraphQL API for indexed blockchain data.

mod config;
mod db;
mod error;
mod graphql;
mod rest;

use crate::config::GatewayConfig;
use crate::db::Database;
use crate::graphql::create_schema;
use crate::rest::create_router;
use clap::Parser;
use std::net::SocketAddr;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Atlas Sphere API Gateway
#[derive(Parser, Debug)]
#[command(name = "atlas-gateway")]
#[command(about = "REST and GraphQL API gateway for Atlas Sphere")]
#[command(version)]
struct Args {
    /// Config file path
    #[arg(short, long, env = "GATEWAY_CONFIG")]
    config: Option<String>,

    /// HTTP server host
    #[arg(long, env = "GATEWAY_HOST", default_value = "127.0.0.1")]
    host: String,

    /// HTTP server port
    #[arg(short, long, env = "GATEWAY_PORT", default_value_t = 8080)]
    port: u16,

    /// Database URL
    #[arg(long, env = "DATABASE_URL")]
    database_url: Option<String>,

    /// Log level
    #[arg(long, env = "RUST_LOG", default_value = "info")]
    log_level: String,
}

fn init_logging(level: &str) {
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(level))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    init_logging(&args.log_level);

    info!("Atlas Gateway starting...");

    // Load configuration
    let config = match args.config {
        Some(path) => GatewayConfig::load(&path).expect("Failed to load config"),
        None => {
            let mut config = GatewayConfig::default();
            config.server.host = args.host;
            config.server.port = args.port;
            if let Some(url) = args.database_url {
                config.database.url = url;
            }
            config
        }
    };

    // Connect to database
    let db = match Database::connect(&config.database).await {
        Ok(db) => db,
        Err(e) => {
            error!("Failed to connect to database: {}", e);
            std::process::exit(1);
        }
    };

    info!("Database connected");

    // Create GraphQL schema
    let schema = create_schema(db.clone());

    // Create REST router
    let app = create_router(db, schema);

    // Start server
    let addr: SocketAddr = format!("{}:{}", config.server.host, config.server.port)
        .parse()
        .expect("Invalid address");

    info!("Server listening on http://{}", addr);
    info!("GraphQL endpoint: http://{}/graphql", addr);
    info!("GraphQL playground: http://{}/graphql/playground", addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    info!("Server shutdown complete");
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Shutdown signal received");
}
