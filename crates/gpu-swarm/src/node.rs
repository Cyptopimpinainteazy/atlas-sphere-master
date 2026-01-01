//! GPU node definitions for the swarm

use crate::config::SwarmConfig;
use crate::error::{SwarmError, SwarmResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Unique identifier for a node (public key)
pub type NodeId = [u8; 32];

/// GPU backend types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GpuBackend {
    /// NVIDIA CUDA
    Cuda,
    /// OpenCL (cross-platform)
    OpenCL,
    /// Vulkan (cross-platform)
    Vulkan,
    /// Apple Metal
    Metal,
    /// WebGPU (WASM targets)
    WebGpu,
}

impl std::fmt::Display for GpuBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GpuBackend::Cuda => write!(f, "CUDA"),
            GpuBackend::OpenCL => write!(f, "OpenCL"),
            GpuBackend::Vulkan => write!(f, "Vulkan"),
            GpuBackend::Metal => write!(f, "Metal"),
            GpuBackend::WebGpu => write!(f, "WebGPU"),
        }
    }
}

/// GPU capabilities of a node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuCapabilities {
    /// Available GPU backends
    pub backends: Vec<GpuBackend>,

    /// GPU device name
    pub device_name: String,

    /// GPU vendor
    pub vendor: String,

    /// Total VRAM in bytes
    pub total_vram: u64,

    /// Available VRAM in bytes
    pub available_vram: u64,

    /// Number of compute units/SMs
    pub compute_units: u32,

    /// Maximum workgroup size
    pub max_workgroup_size: u32,

    /// Maximum concurrent threads
    pub max_threads: u32,

    /// Compute capability (CUDA-specific)
    pub compute_capability: Option<(u32, u32)>,

    /// Supports double precision
    pub supports_fp64: bool,

    /// Supports half precision
    pub supports_fp16: bool,

    /// Tensor core support
    pub supports_tensor_cores: bool,
}

impl GpuCapabilities {
    /// Check if this GPU meets minimum requirements
    pub fn meets_requirements(&self, min_vram: u64, required_backends: &[GpuBackend]) -> bool {
        if self.total_vram < min_vram {
            return false;
        }

        for backend in required_backends {
            if !self.backends.contains(backend) {
                return false;
            }
        }

        true
    }

    /// Estimate compute capacity (relative units)
    pub fn compute_capacity(&self) -> u64 {
        // Simple heuristic: compute_units * max_threads * vram_factor
        let vram_factor = (self.total_vram / (1 << 30)) as u64; // GB
        self.compute_units as u64 * self.max_threads as u64 * vram_factor.max(1)
    }
}

/// Node status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeStatus {
    /// Node is initializing
    Initializing,
    /// Node is online and ready
    Online,
    /// Node is executing tasks
    Busy,
    /// Node is temporarily unavailable
    Offline,
    /// Node has been slashed/banned
    Banned,
}

/// Node metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NodeMetrics {
    /// Total tasks completed
    pub tasks_completed: u64,

    /// Total tasks failed
    pub tasks_failed: u64,

    /// Total compute units processed
    pub compute_units_processed: u64,

    /// Average task latency in milliseconds
    pub avg_latency_ms: u64,

    /// Uptime percentage (0-100)
    pub uptime_percentage: f32,

    /// Reputation score (0-10000)
    pub reputation: u32,

    /// Total rewards earned
    pub total_rewards: u64,

    /// Last heartbeat timestamp
    pub last_heartbeat: i64,
}

impl NodeMetrics {
    /// Calculate success rate
    pub fn success_rate(&self) -> f64 {
        let total = self.tasks_completed + self.tasks_failed;
        if total == 0 {
            return 1.0;
        }
        self.tasks_completed as f64 / total as f64
    }

    /// Update reputation based on task result
    pub fn update_reputation(&mut self, success: bool, weight: u32) {
        let delta = if success {
            weight.min(100)
        } else {
            (weight * 2).min(200)
        };

        if success {
            self.reputation = self.reputation.saturating_add(delta).min(10000);
        } else {
            self.reputation = self.reputation.saturating_sub(delta);
        }
    }
}

/// A node in the GPU swarm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmNode {
    /// Node's public key (identity)
    pub id: NodeId,

    /// Node's P2P address (multiaddr)
    pub peer_address: String,

    /// Node's region/location
    pub region: String,

    /// GPU capabilities
    pub gpu: GpuCapabilities,

    /// Current status
    pub status: NodeStatus,

    /// Node metrics
    pub metrics: NodeMetrics,

    /// Staked amount
    pub stake: u64,

    /// Supported task types
    pub supported_tasks: Vec<String>,

    /// Node version
    pub version: String,

    /// Registration timestamp
    pub registered_at: i64,
}

impl SwarmNode {
    /// Create a new node from config
    pub fn new(config: &SwarmConfig, gpu: GpuCapabilities) -> SwarmResult<Self> {
        // Generate a random key for now - in production this would be loaded from keypair_path
        let mut public_key = [0u8; 32];
        getrandom::getrandom(&mut public_key).unwrap_or_default();

        // Extract port from listen address or use default
        let listen_port = config
            .network
            .listen_addresses
            .first()
            .and_then(|addr| {
                addr.split('/')
                    .find(|s| s.parse::<u16>().is_ok())
                    .and_then(|s| s.parse().ok())
            })
            .unwrap_or(30333);

        Ok(Self {
            id: public_key,
            peer_address: format!(
                "/ip4/0.0.0.0/tcp/{}/p2p/{}",
                listen_port,
                hex::encode(&public_key[..16])
            ),
            region: config
                .identity
                .region
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
            gpu,
            status: NodeStatus::Initializing,
            metrics: NodeMetrics::default(),
            stake: config.rewards.min_stake,
            supported_tasks: config.execution.accepted_task_types.clone(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            registered_at: chrono::Utc::now().timestamp(),
        })
    }

    /// Check if node can execute a task
    pub fn can_execute(&self, task_type: &str, required_compute_units: u64) -> bool {
        if self.status != NodeStatus::Online {
            return false;
        }

        if !self.supported_tasks.is_empty()
            && !self.supported_tasks.contains(&task_type.to_string())
        {
            return false;
        }

        // Check if enough VRAM available
        self.gpu.available_vram >= required_compute_units * 1024
    }

    /// Update node heartbeat
    pub fn heartbeat(&mut self) {
        self.metrics.last_heartbeat = chrono::Utc::now().timestamp();
    }

    /// Check if node is stale (no recent heartbeat)
    pub fn is_stale(&self, timeout_secs: i64) -> bool {
        let now = chrono::Utc::now().timestamp();
        now - self.metrics.last_heartbeat > timeout_secs
    }
}

/// Registry of all nodes in the swarm
pub struct NodeRegistry {
    /// All registered nodes
    nodes: HashMap<NodeId, SwarmNode>,

    /// Nodes by status
    by_status: HashMap<NodeStatus, Vec<NodeId>>,

    /// Nodes by region
    by_region: HashMap<String, Vec<NodeId>>,

    /// Node public keys for cryptographic verification
    pub_keys: HashMap<NodeId, Vec<u8>>,
}

impl NodeRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            by_status: HashMap::new(),
            by_region: HashMap::new(),
        }
    }

    /// Register a new node
    pub fn register(&mut self, node: SwarmNode) -> SwarmResult<()> {
        let id = node.id;
        let status = node.status;
        let region = node.region.clone();

        // Check minimum stake
        if node.stake < crate::MIN_NODE_STAKE {
            return Err(SwarmError::InsufficientStake {
                required: crate::MIN_NODE_STAKE,
                available: node.stake,
            });
        }

        self.nodes.insert(id, node);

        self.by_status.entry(status).or_default().push(id);
        self.by_region.entry(region).or_default().push(id);

        Ok(())
    }

    /// Unregister a node
    pub fn unregister(&mut self, id: &NodeId) -> Option<SwarmNode> {
        let node = self.nodes.remove(id)?;

        if let Some(ids) = self.by_status.get_mut(&node.status) {
            ids.retain(|i| i != id);
        }

        if let Some(ids) = self.by_region.get_mut(&node.region) {
            ids.retain(|i| i != id);
        }

        Some(node)
    }

    /// Get a node by ID
    pub fn get(&self, id: &NodeId) -> Option<&SwarmNode> {
        self.nodes.get(id)
    }

    /// Get a mutable reference to a node
    pub fn get_mut(&mut self, id: &NodeId) -> Option<&mut SwarmNode> {
        self.nodes.get_mut(id)
    }

    /// Update node status
    pub fn update_status(&mut self, id: &NodeId, new_status: NodeStatus) -> SwarmResult<()> {
        let node = self
            .nodes
            .get_mut(id)
            .ok_or(SwarmError::NodeNotFound(*id))?;
        let old_status = node.status;

        if old_status != new_status {
            node.status = new_status;

            // Update status index
            if let Some(ids) = self.by_status.get_mut(&old_status) {
                ids.retain(|i| i != id);
            }
            self.by_status.entry(new_status).or_default().push(*id);
        }

        Ok(())
    }

    /// Get all online nodes
    pub fn online_nodes(&self) -> Vec<&SwarmNode> {
        self.by_status
            .get(&NodeStatus::Online)
            .map(|ids| ids.iter().filter_map(|id| self.nodes.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get nodes by region
    pub fn nodes_in_region(&self, region: &str) -> Vec<&SwarmNode> {
        self.by_region
            .get(region)
            .map(|ids| ids.iter().filter_map(|id| self.nodes.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get total compute capacity
    pub fn total_compute_capacity(&self) -> u64 {
        self.online_nodes()
            .iter()
            .map(|n| n.gpu.compute_capacity())
            .sum()
    }

    /// Get node count by status
    pub fn count_by_status(&self) -> HashMap<NodeStatus, usize> {
        self.by_status
            .iter()
            .map(|(status, ids)| (*status, ids.len()))
            .collect()
    }

    /// Register a public key for a node
    pub fn register_public_key(&mut self, node_id: NodeId, pub_key: Vec<u8>) {
        self.pub_keys.insert(node_id, pub_key);
    }

    /// Get a node's public key
    pub fn get_public_key(&self, node_id: &NodeId) -> Option<&[u8]> {
        self.pub_keys.get(node_id).map(|k| k.as_slice())
    }
}

impl Default for NodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}