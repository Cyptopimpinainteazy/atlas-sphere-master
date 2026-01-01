//! Configuration for the indexer.

use crate::error::Result;
use serde::{Deserialize, Serialize};

/// Main configuration structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexerConfig {
    /// Node connection settings
    pub node: NodeConfig,

    /// Database settings
    pub database: DatabaseConfig,

    /// Indexer behavior settings
    pub indexer: IndexerSettings,

    /// Metrics settings
    pub metrics: MetricsConfig,
}

/// Node connection configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    /// WebSocket URL for the node
    pub url: String,

    /// Connection timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,

    /// Reconnect delay in seconds
    #[serde(default = "default_reconnect_delay")]
    pub reconnect_delay_secs: u64,

    /// Maximum reconnect attempts
    #[serde(default = "default_max_reconnects")]
    pub max_reconnects: u32,
}

/// Database configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// PostgreSQL connection URL
    pub url: String,

    /// Maximum connections in pool
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,

    /// Minimum connections in pool
    #[serde(default = "default_min_connections")]
    pub min_connections: u32,

    /// Connection acquire timeout in seconds
    #[serde(default = "default_acquire_timeout")]
    pub acquire_timeout_secs: u64,
}

/// Indexer behavior settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexerSettings {
    /// Start indexing from this block (None = from genesis or last indexed)
    pub start_block: Option<u64>,

    /// Batch size for bulk inserts
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,

    /// Number of blocks to process in parallel
    #[serde(default = "default_parallel_blocks")]
    pub parallel_blocks: usize,

    /// Whether to index EVM events
    #[serde(default = "default_true")]
    pub index_evm: bool,

    /// Whether to index SVM events
    #[serde(default = "default_true")]
    pub index_svm: bool,

    /// Whether to index Comit transactions
    #[serde(default = "default_true")]
    pub index_comits: bool,

    /// Whether to store raw extrinsics
    #[serde(default = "default_false")]
    pub store_raw: bool,
}

/// Metrics configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Port for metrics/health server
    #[serde(default = "default_metrics_port")]
    pub port: u16,

    /// Whether to enable prometheus metrics
    #[serde(default = "default_true")]
    pub enabled: bool,
}

// Default value functions
fn default_timeout() -> u64 {
    30
}
fn default_reconnect_delay() -> u64 {
    5
}
fn default_max_reconnects() -> u32 {
    10
}
fn default_max_connections() -> u32 {
    10
}
fn default_min_connections() -> u32 {
    2
}
fn default_acquire_timeout() -> u64 {
    30
}
fn default_batch_size() -> usize {
    100
}
fn default_parallel_blocks() -> usize {
    4
}
fn default_metrics_port() -> u16 {
    9615
}
fn default_true() -> bool {
    true
}
fn default_false() -> bool {
    false
}

impl Default for IndexerConfig {
    fn default() -> Self {
        Self {
            node: NodeConfig {
                url: "ws://localhost:9944".to_string(),
                timeout_secs: default_timeout(),
                reconnect_delay_secs: default_reconnect_delay(),
                max_reconnects: default_max_reconnects(),
            },
            database: DatabaseConfig {
                url: "postgres://localhost/atlas_indexer".to_string(),
                max_connections: default_max_connections(),
                min_connections: default_min_connections(),
                acquire_timeout_secs: default_acquire_timeout(),
            },
            indexer: IndexerSettings {
                start_block: None,
                batch_size: default_batch_size(),
                parallel_blocks: default_parallel_blocks(),
                index_evm: true,
                index_svm: true,
                index_comits: true,
                store_raw: false,
            },
            metrics: MetricsConfig {
                port: default_metrics_port(),
                enabled: true,
            },
        }
    }
}

impl IndexerConfig {
    /// Load configuration from file.
    pub fn load(path: &str) -> Result<Self> {
        // Try to load from file
        let builder = config::Config::builder()
            .add_source(config::File::with_name(path).required(false))
            .add_source(config::Environment::with_prefix("INDEXER").separator("__"));

        let config = builder.build()?;

        // If no config file, use defaults
        if std::path::Path::new(path).exists() {
            Ok(config.try_deserialize()?)
        } else {
            Ok(Self::default())
        }
    }

    /// Create config for local development.
    pub fn local() -> Self {
        Self::default()
    }

    /// Create config for testnet.
    pub fn testnet() -> Self {
        Self {
            node: NodeConfig {
                url: "ws://rpc.testnet.atlas-sphere.io:9944".to_string(),
                ..Default::default()
            },
            database: DatabaseConfig {
                url: "postgres://localhost/atlas_indexer_testnet".to_string(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

impl NodeConfig {
    fn default() -> Self {
        Self {
            url: "ws://localhost:9944".to_string(),
            timeout_secs: default_timeout(),
            reconnect_delay_secs: default_reconnect_delay(),
            max_reconnects: default_max_reconnects(),
        }
    }
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self::default()
    }
}

impl DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgres://localhost/atlas_indexer".to_string(),
            max_connections: default_max_connections(),
            min_connections: default_min_connections(),
            acquire_timeout_secs: default_acquire_timeout(),
        }
    }
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self::default()
    }
}
