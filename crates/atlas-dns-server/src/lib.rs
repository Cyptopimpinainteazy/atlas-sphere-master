//! Atlas Sphere DNS Server - Core Library
//! 
//! Authoritative DNS server for the .x3 TLD with blockchain integration
//! Provides DNS resolution for Atlas Sphere ecosystem services

pub mod config;
pub mod error;
pub mod server;
pub mod domain;
pub mod blockchain;
pub mod api;
pub mod zone;
pub mod cache;
pub mod registry;

// Re-export commonly used types
pub use config::DnsConfig;
pub use error::{DnsError, DnsResult};
pub use domain::{DomainRecord, DnsRecordType, DnsRecord};
pub use blockchain::{BlockchainClient, DomainOwnership};

// Core types
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// DNS Server Statistics
#[derive(Debug, Clone, Default)]
pub struct DnsStats {
    pub queries_total: u64,
    pub queries_cached: u64,
    pub queries_authoritative: u64,
    pub domains_registered: u64,
    pub blockchain_verified: u64,
    pub uptime_seconds: u64,
}

/// Main DNS Server Instance
#[derive(Clone)]
pub struct AtlasDnsInstance {
    pub server: Arc<server::AtlasDnsServer>,
    pub stats: Arc<RwLock<DnsStats>>,
    pub config: Arc<DnsConfig>,
}

impl AtlasDnsInstance {
    /// Create new DNS instance
    pub async fn new(config: DnsConfig) -> DnsResult<Self> {
        let server = Arc::new(server::AtlasDnsServer::new(config.clone()).await?);
        let stats = Arc::new(RwLock::new(DnsStats::default()));
        let config = Arc::new(config);
        
        Ok(Self {
            server,
            stats,
            config,
        })
    }
    
    /// Get server statistics
    pub async fn get_stats(&self) -> DnsStats {
        self.stats.read().await.clone()
    }
    
    /// Update statistics
    pub async fn update_stats<F>(&self, updater: F)
    where
        F: FnOnce(&mut DnsStats),
    {
        let mut stats = self.stats.write().await;
        updater(&mut stats);
    }
    
    /// Start the DNS instance
    pub async fn start(self) -> DnsResult<()> {
        self.server.start().await?;
        Ok(())
    }
    
    /// Stop the DNS instance
    pub async fn stop(&self) -> DnsResult<()> {
        self.server.stop().await?;
        Ok(())
    }
}

/// Initialize DNS server with default configuration
pub async fn create_default_dns_server() -> DnsResult<AtlasDnsInstance> {
    let config = DnsConfig::default();
    AtlasDnsInstance::new(config).await
}

/// Get system information
pub fn get_system_info() -> HashMap<String, String> {
    let mut info = HashMap::new();
    info.insert("name".to_string(), NAME.to_string());
    info.insert("version".to_string(), VERSION.to_string());
    info.insert("rust_version".to_string(), 
                std::env::var("RUST_VERSION").unwrap_or_else(|_| "unknown".to_string()));
    info.insert("platform".to_string(), 
                std::env::consts::OS.to_string());
    info
}
