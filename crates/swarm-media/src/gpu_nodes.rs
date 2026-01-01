//! GPU Node Management Service
//!
//! Handles registration, heartbeat, and discovery of GPU nodes
//! that execute media jobs on the swarm.

use crate::tool_adapter::{GpuNodeCapabilities, ToolType};
use chrono::Utc;
use std::collections::HashMap;
use uuid::Uuid;

/// Configuration for the node manager
#[derive(Debug, Clone)]
pub struct NodeManagerConfig {
    /// Heartbeat timeout in seconds (node marked offline if no heartbeat)
    pub heartbeat_timeout_secs: i64,
    /// Maximum VRAM the node can have (sanity check)
    pub max_vram_gb: u32,
    /// Minimum VRAM required to participate
    pub min_vram_gb: u32,
}

impl Default for NodeManagerConfig {
    fn default() -> Self {
        Self {
            heartbeat_timeout_secs: 300, // 5 minutes
            max_vram_gb: 256,            // No GPUs bigger than 256GB
            min_vram_gb: 4,              // Minimum 4GB VRAM to participate
        }
    }
}

/// Node status in the network
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeStatus {
    /// Node is online and responsive
    Online,
    /// Node has not heartbeat recently (still might recover)
    Stale,
    /// Node marked offline
    Offline,
}

/// GPU node registration with timestamps
#[derive(Debug, Clone)]
pub struct RegisteredNode {
    pub capabilities: GpuNodeCapabilities,
    pub status: NodeStatus,
    pub registered_at: i64,
    pub last_heartbeat: i64,
    pub total_jobs_assigned: u64,
    pub total_jobs_completed: u64,
    pub total_jobs_failed: u64,
}

/// Manages all GPU nodes in the network
pub struct GpuNodeManager {
    nodes: HashMap<Uuid, RegisteredNode>,
    config: NodeManagerConfig,
}

impl GpuNodeManager {
    /// Create a new node manager
    pub fn new(config: NodeManagerConfig) -> Self {
        Self {
            nodes: HashMap::new(),
            config,
        }
    }

    /// Register a new GPU node or update existing one
    pub fn register_node(&mut self, capabilities: GpuNodeCapabilities) -> Result<(), String> {
        // Validate capabilities
        if capabilities.vram_gb > self.config.max_vram_gb {
            return Err(format!(
                "VRAM {} exceeds maximum {}",
                capabilities.vram_gb, self.config.max_vram_gb
            ));
        }

        if capabilities.vram_gb < self.config.min_vram_gb {
            return Err(format!(
                "VRAM {} below minimum {}",
                capabilities.vram_gb, self.config.min_vram_gb
            ));
        }

        if capabilities.supported_tools.is_empty() {
            return Err("Node must support at least one tool".to_string());
        }

        let now = Utc::now().timestamp();
        let registered = RegisteredNode {
            capabilities,
            status: NodeStatus::Online,
            registered_at: now,
            last_heartbeat: now,
            total_jobs_assigned: 0,
            total_jobs_completed: 0,
            total_jobs_failed: 0,
        };

        self.nodes.insert(registered.capabilities.node_id, registered);
        Ok(())
    }

    /// Heartbeat from a node (keep-alive + update VRAM)
    pub fn heartbeat(&mut self, node_id: Uuid, available_vram_gb: u32) -> Result<(), String> {
        if let Some(node) = self.nodes.get_mut(&node_id) {
            node.last_heartbeat = Utc::now().timestamp();
            node.capabilities.available_vram_gb = available_vram_gb;
            node.capabilities.online = true;

            // Transition from Stale back to Online if needed
            if node.status == NodeStatus::Stale {
                node.status = NodeStatus::Online;
            }

            Ok(())
        } else {
            Err(format!("Node {} not registered", node_id))
        }
    }

    /// Check for stale/offline nodes and update their status
    pub fn refresh_node_status(&mut self) {
        let now = Utc::now().timestamp();
        let timeout = self.config.heartbeat_timeout_secs;

        for node in self.nodes.values_mut() {
            let time_since_heartbeat = now - node.last_heartbeat;

            match node.status {
                NodeStatus::Online => {
                    if time_since_heartbeat > timeout {
                        node.status = NodeStatus::Stale;
                        node.capabilities.online = false;
                    }
                }
                NodeStatus::Stale => {
                    if time_since_heartbeat > timeout * 2 {
                        node.status = NodeStatus::Offline;
                        node.capabilities.online = false;
                    }
                }
                NodeStatus::Offline => {
                    // Manual reactivation needed
                }
            }
        }
    }

    /// Get a node by ID
    pub fn get_node(&self, node_id: Uuid) -> Option<RegisteredNode> {
        self.nodes.get(&node_id).cloned()
    }

    /// Get all online nodes that support a given tool
    pub fn get_nodes_for_tool(&mut self, tool_type: &ToolType) -> Vec<RegisteredNode> {
        self.refresh_node_status();
        self.nodes
            .values()
            .filter(|n| n.status == NodeStatus::Online && n.capabilities.supported_tools.contains(tool_type))
            .cloned()
            .collect()
    }

    /// Get all nodes
    pub fn get_all_nodes(&mut self) -> Vec<RegisteredNode> {
        self.refresh_node_status();
        self.nodes.values().cloned().collect()
    }

    /// Get network statistics
    pub fn get_stats(&mut self) -> NetworkStats {
        self.refresh_node_status();

        let online_count = self
            .nodes
            .values()
            .filter(|n| n.status == NodeStatus::Online)
            .count();
        let stale_count = self
            .nodes
            .values()
            .filter(|n| n.status == NodeStatus::Stale)
            .count();
        let offline_count = self
            .nodes
            .values()
            .filter(|n| n.status == NodeStatus::Offline)
            .count();

        let total_vram: u32 = self.nodes.values().map(|n| n.capabilities.vram_gb).sum();
        let available_vram: u32 = self
            .nodes
            .values()
            .filter(|n| n.status == NodeStatus::Online)
            .map(|n| n.capabilities.available_vram_gb)
            .sum();

        let total_jobs_completed: u64 = self
            .nodes
            .values()
            .map(|n| n.total_jobs_completed)
            .sum();

        NetworkStats {
            total_nodes: self.nodes.len(),
            online_nodes: online_count,
            stale_nodes: stale_count,
            offline_nodes: offline_count,
            total_vram_gb: total_vram,
            available_vram_gb: available_vram,
            total_jobs_completed,
        }
    }

    /// Mark node as completed a job
    pub fn record_job_completed(&mut self, node_id: Uuid) -> Result<(), String> {
        if let Some(node) = self.nodes.get_mut(&node_id) {
            node.total_jobs_completed += 1;
            node.total_jobs_assigned += 1;
            Ok(())
        } else {
            Err(format!("Node {} not found", node_id))
        }
    }

    /// Mark node as failed a job
    pub fn record_job_failed(&mut self, node_id: Uuid) -> Result<(), String> {
        if let Some(node) = self.nodes.get_mut(&node_id) {
            node.total_jobs_failed += 1;
            node.total_jobs_assigned += 1;
            Ok(())
        } else {
            Err(format!("Node {} not found", node_id))
        }
    }

    /// Deregister a node
    pub fn deregister_node(&mut self, node_id: Uuid) -> Result<(), String> {
        if self.nodes.remove(&node_id).is_some() {
            Ok(())
        } else {
            Err(format!("Node {} not found", node_id))
        }
    }
}

impl Default for GpuNodeManager {
    fn default() -> Self {
        Self::new(NodeManagerConfig::default())
    }
}

/// Network-wide statistics
#[derive(Debug, Clone)]
pub struct NetworkStats {
    pub total_nodes: usize,
    pub online_nodes: usize,
    pub stale_nodes: usize,
    pub offline_nodes: usize,
    pub total_vram_gb: u32,
    pub available_vram_gb: u32,
    pub total_jobs_completed: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_node(id: Uuid, vram: u32) -> GpuNodeCapabilities {
        GpuNodeCapabilities {
            node_id: id,
            name: format!("test-node-{}", id),
            vram_gb: vram,
            available_vram_gb: vram,
            supported_tools: vec![
                ToolType::TextGeneration,
                ToolType::ImageGeneration,
            ],
            latency_ms: 450,
            online: true,
            last_heartbeat: Utc::now().timestamp(),
            jobs_completed: 0,
            compute_contributed: 0.0,
        }
    }

    #[test]
    fn test_register_node() {
        let mut manager = GpuNodeManager::default();
        let node_id = Uuid::new_v4();
        let node = create_test_node(node_id, 24);

        assert!(manager.register_node(node).is_ok());
        assert!(manager.get_node(node_id).is_some());
    }

    #[test]
    fn test_register_invalid_vram() {
        let mut manager = GpuNodeManager::default();
        let node_id = Uuid::new_v4();
        let mut node = create_test_node(node_id, 256); // At max
        node.vram_gb = 300; // Exceeds max

        assert!(manager.register_node(node).is_err());
    }

    #[test]
    fn test_heartbeat() {
        let mut manager = GpuNodeManager::default();
        let node_id = Uuid::new_v4();
        let node = create_test_node(node_id, 24);

        manager.register_node(node).unwrap();
        assert!(manager.heartbeat(node_id, 20).is_ok());

        let node = manager.get_node(node_id).unwrap();
        assert_eq!(node.capabilities.available_vram_gb, 20);
    }

    #[test]
    fn test_network_stats() {
        let mut manager = GpuNodeManager::default();

        for _i in 0..3 {
            let node_id = Uuid::new_v4();
            let node = create_test_node(node_id, 24);
            manager.register_node(node).unwrap();
        }

        let stats = manager.get_stats();
        assert_eq!(stats.total_nodes, 3);
        assert_eq!(stats.online_nodes, 3);
        assert_eq!(stats.total_vram_gb, 72);
    }
}
