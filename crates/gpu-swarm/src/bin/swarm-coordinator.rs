//! GPU Swarm Coordinator Binary
//!
//! Run the central coordinator that manages the GPU swarm network.

use gpu_swarm::{
    config::SwarmConfig,
    coordinator::{CoordinatorConfig, SwarmCoordinator},
    network::{NetworkConfig, NetworkManager},
    node::NodeId,
};
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tracing::info!("Starting GPU Swarm Coordinator v{}", env!("CARGO_PKG_VERSION"));

    // Load or create configuration
    let config_path = PathBuf::from("coordinator-config.toml");
    let config = if config_path.exists() {
        tracing::info!("Loading config from {:?}", config_path);
        SwarmConfig::from_file(&config_path)?
    } else {
        tracing::info!("Using default configuration");
        SwarmConfig::default()
    };

    // Generate coordinator ID
    let coordinator_id: NodeId = {
        let mut id = [0u8; 32];
        if let Err(e) = getrandom::getrandom(&mut id) {
            eprintln!("Failed to generate random ID: {}", e);
            std::process::exit(1);
        }
        id
    };
    tracing::info!("Coordinator ID: {}", hex::encode(&coordinator_id[..16]));

    // Create network manager
    let net_config = NetworkConfig::default();
    let mut network = NetworkManager::new(net_config)?;

    // Start network
    network.start().await?;
    tracing::info!("Network listening on port {}", 9100);

    // Create coordinator
    let coord_config = CoordinatorConfig::default();
    let (mut coordinator, _message_tx, mut events) = SwarmCoordinator::new(coord_config, coordinator_id);

    // Spawn event handler
    let event_task = tokio::spawn(async move {
        while let Ok(event) = events.recv().await {
            tracing::info!("Coordinator event: {:?}", event);
            // Handle events here
        }
    });

    // Start coordinator
    tracing::info!("Coordinator ready. Press Ctrl+C to stop.");
    
    // Run coordinator in background
    let run_task = tokio::spawn(async move {
        if let Err(e) = coordinator.start().await {
            tracing::error!("Coordinator error: {}", e);
        }
    });

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;

    tracing::info!("Shutting down...");
    network.stop();

    // Cancel background tasks
    event_task.abort();
    run_task.abort();

    Ok(())
}
