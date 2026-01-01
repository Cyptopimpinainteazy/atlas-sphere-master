//! Sidecar configuration

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// VM configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VmConfig {
    /// Maximum gas per execution
    pub max_gas: u64,
    /// Memory limit in bytes
    pub memory_limit: usize,
    /// Stack size limit
    pub stack_limit: usize,
    /// Enable JIT compilation
    pub jit_enabled: bool,
    /// JIT threshold (calls before JIT)
    pub jit_threshold: u32,
}

impl Default for VmConfig {
    fn default() -> Self {
        Self {
            max_gas: 10_000_000,
            memory_limit: 64 * 1024 * 1024, // 64 MB
            stack_limit: 1024,
            jit_enabled: false,
            jit_threshold: 100,
        }
    }
}

/// Sidecar configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SidecarConfig {
    /// RPC server port
    pub rpc_port: u16,
    /// Metrics port
    pub metrics_port: u16,
    /// Chain RPC URL
    pub chain_rpc: String,
    /// Data directory
    pub data_dir: PathBuf,
    /// Executor private key (hex encoded)
    pub executor_key: String,
    /// Maximum pending jobs
    pub max_pending_jobs: usize,
    /// VM configuration
    pub vm: VmConfig,
    /// Enable GPU acceleration
    pub gpu_enabled: bool,
    /// Worker threads
    pub worker_threads: usize,
    /// Receipt submission retry count
    pub submit_retries: u32,
    /// Heartbeat interval (seconds)
    pub heartbeat_interval: u64,
}

impl Default for SidecarConfig {
    fn default() -> Self {
        Self {
            rpc_port: 9955,
            metrics_port: 9956,
            chain_rpc: "http://localhost:9944".to_string(),
            data_dir: PathBuf::from("./x3-sidecar-data"),
            executor_key: "0000000000000000000000000000000000000000000000000000000000000001"
                .to_string(),
            max_pending_jobs: 1000,
            vm: VmConfig::default(),
            gpu_enabled: false,
            worker_threads: 4,
            submit_retries: 3,
            heartbeat_interval: 30,
        }
    }
}

impl SidecarConfig {
    /// Load configuration from file
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let config: SidecarConfig = toml::from_str(&contents)?;
        Ok(config)
    }

    /// Save configuration to file
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let contents = toml::to_string_pretty(self)?;
        std::fs::write(path, contents)?;
        Ok(())
    }
}
