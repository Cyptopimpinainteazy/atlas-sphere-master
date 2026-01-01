//! DNS Resolution Module for Swarm Media
//!
//! Provides DNS-based service discovery and resolution for
//! swarm network nodes and endpoints.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::sync::RwLock;
use thiserror::Error;

/// DNS-related errors
#[derive(Debug, Error)]
pub enum DnsError {
    #[error("Record not found: {0}")]
    NotFound(String),
    #[error("Invalid record format: {0}")]
    InvalidFormat(String),
    #[error("Lock poisoned")]
    LockPoisoned,
}

/// DNS record types supported by the swarm
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RecordType {
    /// IPv4 address record
    A,
    /// IPv6 address record
    AAAA,
    /// Canonical name record
    CNAME,
    /// Service record for node discovery
    SRV,
    /// Text record for metadata
    TXT,
}

/// A DNS record entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsRecord {
    pub name: String,
    pub record_type: RecordType,
    pub value: String,
    pub ttl: u32,
    pub priority: Option<u16>,
    pub weight: Option<u16>,
    pub port: Option<u16>,
}

impl DnsRecord {
    /// Create a new A record
    pub fn a_record(name: &str, ip: Ipv4Addr, ttl: u32) -> Self {
        Self {
            name: name.to_string(),
            record_type: RecordType::A,
            value: ip.to_string(),
            ttl,
            priority: None,
            weight: None,
            port: None,
        }
    }

    /// Create a new AAAA record
    pub fn aaaa_record(name: &str, ip: Ipv6Addr, ttl: u32) -> Self {
        Self {
            name: name.to_string(),
            record_type: RecordType::AAAA,
            value: ip.to_string(),
            ttl,
            priority: None,
            weight: None,
            port: None,
        }
    }

    /// Create a new SRV record
    pub fn srv_record(
        name: &str,
        target: &str,
        port: u16,
        priority: u16,
        weight: u16,
        ttl: u32,
    ) -> Self {
        Self {
            name: name.to_string(),
            record_type: RecordType::SRV,
            value: target.to_string(),
            ttl,
            priority: Some(priority),
            weight: Some(weight),
            port: Some(port),
        }
    }

    /// Create a new TXT record
    pub fn txt_record(name: &str, value: &str, ttl: u32) -> Self {
        Self {
            name: name.to_string(),
            record_type: RecordType::TXT,
            value: value.to_string(),
            ttl,
            priority: None,
            weight: None,
            port: None,
        }
    }
}

/// Swarm node information for DNS registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmNode {
    pub node_id: String,
    pub ip: IpAddr,
    pub rpc_port: u16,
    pub p2p_port: u16,
    pub capabilities: Vec<String>,
}

/// Simple in-memory DNS resolver for swarm services
pub struct SwarmDnsResolver {
    records: RwLock<HashMap<(String, RecordType), Vec<DnsRecord>>>,
    nodes: RwLock<HashMap<String, SwarmNode>>,
    domain_suffix: String,
}

impl Default for SwarmDnsResolver {
    fn default() -> Self {
        Self::new("swarm.atlas-sphere.local")
    }
}

impl SwarmDnsResolver {
    /// Create a new DNS resolver with custom domain suffix
    pub fn new(domain_suffix: &str) -> Self {
        Self {
            records: RwLock::new(HashMap::new()),
            nodes: RwLock::new(HashMap::new()),
            domain_suffix: domain_suffix.to_string(),
        }
    }

    /// Register a DNS record
    pub fn register(&self, record: DnsRecord) -> Result<(), DnsError> {
        let key = (record.name.clone(), record.record_type);
        let mut records = self.records.write().map_err(|_| DnsError::LockPoisoned)?;
        records.entry(key).or_insert_with(Vec::new).push(record);
        Ok(())
    }

    /// Remove all records for a name and type
    pub fn unregister(&self, name: &str, record_type: RecordType) -> Result<(), DnsError> {
        let key = (name.to_string(), record_type);
        let mut records = self.records.write().map_err(|_| DnsError::LockPoisoned)?;
        records.remove(&key);
        Ok(())
    }

    /// Resolve a hostname to records
    pub fn resolve(&self, name: &str, record_type: RecordType) -> Result<Vec<DnsRecord>, DnsError> {
        let key = (name.to_string(), record_type);
        let records = self.records.read().map_err(|_| DnsError::LockPoisoned)?;
        records
            .get(&key)
            .cloned()
            .ok_or_else(|| DnsError::NotFound(name.to_string()))
    }

    /// Resolve hostname to IP addresses
    pub fn resolve_ip(&self, name: &str) -> Option<IpAddr> {
        // Try A record first, then AAAA
        if let Ok(records) = self.resolve(name, RecordType::A) {
            if let Some(r) = records.first() {
                return r.value.parse().ok();
            }
        }
        if let Ok(records) = self.resolve(name, RecordType::AAAA) {
            if let Some(r) = records.first() {
                return r.value.parse().ok();
            }
        }
        None
    }

    /// Register a swarm node with full DNS records
    pub fn register_node(&self, node: SwarmNode) -> Result<(), DnsError> {
        let hostname = format!("{}.{}", node.node_id, self.domain_suffix);

        // A/AAAA record based on IP type
        match node.ip {
            IpAddr::V4(ipv4) => {
                self.register(DnsRecord::a_record(&hostname, ipv4, 300))?;
            }
            IpAddr::V6(ipv6) => {
                self.register(DnsRecord::aaaa_record(&hostname, ipv6, 300))?;
            }
        }

        // SRV record for RPC service discovery
        self.register(DnsRecord::srv_record(
            &format!("_rpc._tcp.{}", self.domain_suffix),
            &hostname,
            node.rpc_port,
            10,
            10,
            300,
        ))?;

        // SRV record for P2P service discovery
        self.register(DnsRecord::srv_record(
            &format!("_p2p._tcp.{}", self.domain_suffix),
            &hostname,
            node.p2p_port,
            10,
            10,
            300,
        ))?;

        // TXT record for capabilities
        if !node.capabilities.is_empty() {
            self.register(DnsRecord::txt_record(
                &hostname,
                &node.capabilities.join(","),
                300,
            ))?;
        }

        // Store node info
        let mut nodes = self.nodes.write().map_err(|_| DnsError::LockPoisoned)?;
        nodes.insert(node.node_id.clone(), node);

        Ok(())
    }

    /// Unregister a swarm node
    pub fn unregister_node(&self, node_id: &str) -> Result<(), DnsError> {
        let hostname = format!("{}.{}", node_id, self.domain_suffix);

        self.unregister(&hostname, RecordType::A)?;
        self.unregister(&hostname, RecordType::AAAA)?;
        self.unregister(&hostname, RecordType::TXT)?;

        let mut nodes = self.nodes.write().map_err(|_| DnsError::LockPoisoned)?;
        nodes.remove(node_id);

        Ok(())
    }

    /// Get node info by ID
    pub fn get_node(&self, node_id: &str) -> Option<SwarmNode> {
        let nodes = self.nodes.read().ok()?;
        nodes.get(node_id).cloned()
    }

    /// Get all registered nodes
    pub fn list_nodes(&self) -> Vec<SwarmNode> {
        let nodes = self.nodes.read().ok();
        nodes
            .map(|n| n.values().cloned().collect())
            .unwrap_or_default()
    }

    /// Find nodes with specific capability
    pub fn find_by_capability(&self, capability: &str) -> Vec<SwarmNode> {
        self.list_nodes()
            .into_iter()
            .filter(|n| n.capabilities.iter().any(|c| c == capability))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_resolve() {
        let resolver = SwarmDnsResolver::default();
        let node = SwarmNode {
            node_id: "node1".to_string(),
            ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
            rpc_port: 9944,
            p2p_port: 30333,
            capabilities: vec!["gpu".to_string(), "compute".to_string()],
        };
        resolver.register_node(node).unwrap();

        let ip = resolver.resolve_ip("node1.swarm.atlas-sphere.local");
        assert_eq!(ip, Some(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100))));
    }

    #[test]
    fn test_list_nodes() {
        let resolver = SwarmDnsResolver::default();

        resolver
            .register_node(SwarmNode {
                node_id: "node1".to_string(),
                ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
                rpc_port: 9944,
                p2p_port: 30333,
                capabilities: vec![],
            })
            .unwrap();

        resolver
            .register_node(SwarmNode {
                node_id: "node2".to_string(),
                ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 101)),
                rpc_port: 9944,
                p2p_port: 30333,
                capabilities: vec![],
            })
            .unwrap();

        let nodes = resolver.list_nodes();
        assert_eq!(nodes.len(), 2);
    }

    #[test]
    fn test_find_by_capability() {
        let resolver = SwarmDnsResolver::default();

        resolver
            .register_node(SwarmNode {
                node_id: "gpu-node".to_string(),
                ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)),
                rpc_port: 9944,
                p2p_port: 30333,
                capabilities: vec!["gpu".to_string()],
            })
            .unwrap();

        resolver
            .register_node(SwarmNode {
                node_id: "cpu-node".to_string(),
                ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 101)),
                rpc_port: 9944,
                p2p_port: 30333,
                capabilities: vec!["cpu".to_string()],
            })
            .unwrap();

        let gpu_nodes = resolver.find_by_capability("gpu");
        assert_eq!(gpu_nodes.len(), 1);
        assert_eq!(gpu_nodes[0].node_id, "gpu-node");
    }

    #[test]
    fn test_srv_record() {
        let record = DnsRecord::srv_record("_rpc._tcp.example.com", "node1.example.com", 9944, 10, 5, 300);
        assert_eq!(record.record_type, RecordType::SRV);
        assert_eq!(record.port, Some(9944));
        assert_eq!(record.priority, Some(10));
        assert_eq!(record.weight, Some(5));
    }
}
