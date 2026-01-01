//! Configuration for the GPU swarm

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Main configuration for a swarm node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmConfig {
    /// Node identity configuration
    pub identity: IdentityConfig,

    /// Network configuration
    pub network: NetworkConfig,

    /// GPU configuration
    pub gpu: GpuConfig,

    /// Task execution configuration
    pub execution: ExecutionConfig,

    /// Reward configuration
    pub rewards: RewardConfig,
}

impl Default for SwarmConfig {
    fn default() -> Self {
        Self {
            identity: IdentityConfig::default(),
            network: NetworkConfig::default(),
            gpu: GpuConfig::default(),
            execution: ExecutionConfig::default(),
            rewards: RewardConfig::default(),
        }
    }
}

/// Node identity configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityConfig {
    /// Path to the node's keypair file
    pub keypair_path: PathBuf,

    /// Node's display name (optional)
    pub display_name: Option<String>,

    /// Node's region (for locality-aware scheduling)
    pub region: Option<String>,
}

impl Default for IdentityConfig {
    fn default() -> Self {
        Self {
            keypair_path: PathBuf::from("~/.x3-swarm/node.key"),
            display_name: None,
            region: None,
        }
    }
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Listen addresses for P2P
    pub listen_addresses: Vec<String>,

    /// Bootstrap nodes to connect to
    pub bootstrap_nodes: Vec<String>,

    /// Coordinator endpoint (for centralized scheduling mode)
    pub coordinator_endpoint: Option<String>,

    /// Maximum number of peer connections
    pub max_peers: usize,

    /// Enable mDNS for local discovery
    pub enable_mdns: bool,

    /// Enable Kademlia DHT
    pub enable_dht: bool,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            listen_addresses: vec![
                "/ip4/0.0.0.0/tcp/30333".to_string(),
                "/ip4/0.0.0.0/udp/30333/quic-v1".to_string(),
            ],
            bootstrap_nodes: vec![
                // Default testnet bootstrap nodes
                "/dns4/swarm1.testnet.atlas-sphere.io/tcp/30333/p2p/12D3KooWxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx".to_string(),
            ],
            coordinator_endpoint: Some("https://coordinator.atlas-sphere.io".to_string()),
            max_peers: 50,
            enable_mdns: true,
            enable_dht: true,
        }
    }
}

/// GPU configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuConfig {
    /// Enable CUDA backend
    pub enable_cuda: bool,

    /// Enable OpenCL backend
    pub enable_opencl: bool,

    /// Enable Vulkan compute backend
    pub enable_vulkan: bool,

    /// GPU device indices to use (empty = all available)
    pub device_indices: Vec<u32>,

    /// Maximum memory usage per GPU (MB, 0 = unlimited)
    pub max_memory_mb: u64,

    /// GPU power limit percentage (0-100, 0 = no limit)
    pub power_limit_percent: u8,
}

impl Default for GpuConfig {
    fn default() -> Self {
        Self {
            enable_cuda: true,
            enable_opencl: false,
            enable_vulkan: false,
            device_indices: vec![],
            max_memory_mb: 0,
            power_limit_percent: 0,
        }
    }
}

/// Task execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    /// Maximum concurrent tasks
    pub max_concurrent_tasks: usize,

    /// Task timeout
    #[serde(with = "humantime_serde")]
    pub task_timeout: Duration,

    /// Enable sandboxed execution
    pub sandbox_enabled: bool,

    /// Maximum task payload size (bytes)
    pub max_payload_size: usize,

    /// Task types this node will accept
    pub accepted_task_types: Vec<String>,

    /// Minimum reward threshold (don't accept tasks below this)
    pub min_reward_threshold: u64,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 4,
            task_timeout: Duration::from_secs(300),
            sandbox_enabled: true,
            max_payload_size: 16 * 1024 * 1024, // 16 MB
            accepted_task_types: vec![
                "x3_bytecode".to_string(),
                "mempool_sim".to_string(),
                "route_optimize".to_string(),
            ],
            min_reward_threshold: 0,
        }
    }
}

/// Reward configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardConfig {
    /// Wallet address to receive rewards
    pub reward_address: Option<String>,

    /// Minimum stake amount required
    pub min_stake: u64,

    /// Auto-restake rewards
    pub auto_restake: bool,

    /// Claim threshold (auto-claim when rewards exceed this)
    pub claim_threshold: u64,
}

impl Default for RewardConfig {
    fn default() -> Self {
        Self {
            reward_address: None,
            min_stake: 1000,
            auto_restake: false,
            claim_threshold: 100,
        }
    }
}

impl SwarmConfig {
    /// Load configuration from a TOML file
    pub fn from_file(path: &std::path::Path) -> Result<Self, crate::SwarmError> {
        let content = std::fs::read_to_string(path)?;
        toml::from_str(&content).map_err(|e| crate::SwarmError::ConfigError(e.to_string()))
    }

    /// Save configuration to a TOML file
    pub fn to_file(&self, path: &std::path::Path) -> Result<(), crate::SwarmError> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| crate::SwarmError::ConfigError(e.to_string()))?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

// Add missing humantime_serde for Duration serialization
mod humantime_serde {
    use serde::{self, Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_secs())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}
