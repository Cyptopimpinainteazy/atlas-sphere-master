/// Network Bootstrapping and P2P Configuration
///
/// Handles peer discovery, bootstrap node configuration, and network protocol setup.

use sc_network::{
	multiaddr::Multiaddr,
	NetworkService,
};
use std::sync::Arc;

/// Bootstrap configuration for Atlas Sphere network
#[derive(Clone, Debug)]
pub struct BootstrapConfig {
	/// Bootstrap node addresses
	pub bootstrap_nodes: Vec<Multiaddr>,
	/// Enable peer discovery
	pub enable_discovery: bool,
	/// Enable mDNS for local peer discovery
	pub enable_mdns: bool,
	/// Enable Kademlia DHT
	pub enable_kad: bool,
	/// Maximum incoming connections
	pub max_incoming_connections: u32,
	/// Maximum outgoing connections
	pub max_outgoing_connections: u32,
}

impl Default for BootstrapConfig {
	fn default() -> Self {
		Self {
			bootstrap_nodes: vec![
				// Local development nodes
				"/ip4/127.0.0.1/tcp/30333".parse().unwrap_or_else(|_| "/ip4/127.0.0.1/tcp/30333".parse().unwrap()),
			],
			enable_discovery: true,
			enable_mdns: true,
			enable_kad: true,
			max_incoming_connections: 100,
			max_outgoing_connections: 50,
		}
	}
}

impl BootstrapConfig {
	/// Create mainnet bootstrap configuration
	pub fn mainnet() -> Self {
		Self {
			bootstrap_nodes: vec![
				// Mainnet bootstrap nodes would go here
				"/dns4/bootstrap1.atlas-sphere.io/tcp/30333".parse().ok(),
				"/dns4/bootstrap2.atlas-sphere.io/tcp/30333".parse().ok(),
				"/dns4/bootstrap3.atlas-sphere.io/tcp/30333".parse().ok(),
			]
			.into_iter()
			.filter_map(|x| x)
			.collect(),
			enable_discovery: true,
			enable_mdns: false, // Disabled on mainnet
			enable_kad: true,
			max_incoming_connections: 200,
			max_outgoing_connections: 100,
		}
	}

	/// Create testnet bootstrap configuration
	pub fn testnet() -> Self {
		Self {
			bootstrap_nodes: vec![
				"/dns4/testnet-bootstrap.atlas-sphere.io/tcp/30333".parse().ok(),
			]
			.into_iter()
			.filter_map(|x| x)
			.collect(),
			enable_discovery: true,
			enable_mdns: true,
			enable_kad: true,
			max_incoming_connections: 150,
			max_outgoing_connections: 75,
		}
	}

	/// Create development bootstrap configuration
	pub fn development() -> Self {
		Self::default()
	}
}

/// Network protocol information
#[derive(Clone, Debug)]
pub struct ProtocolInfo {
	/// Protocol name
	pub name: String,
	/// Protocol version
	pub version: u32,
	/// Encoded genesis hash
	pub genesis_hash: [u8; 32],
	/// Fork ID
	pub fork_id: Option<String>,
}

impl ProtocolInfo {
	/// Create new protocol info for Atlas Sphere
	pub fn new(genesis_hash: [u8; 32]) -> Self {
		Self {
			name: "atlas-sphere".to_string(),
			version: 1,
			genesis_hash,
			fork_id: None,
		}
	}
}

/// Network peer information
#[derive(Clone, Debug)]
pub struct PeerInfo {
	/// Peer ID
	pub id: String,
	/// Peer addresses
	pub addresses: Vec<Multiaddr>,
	/// Connected since (unix timestamp)
	pub connected_since: u64,
	/// Best block number known by peer
	pub best_block: u32,
	/// Peer role (validator, light, full)
	pub role: PeerRole,
}

/// Peer role in the network
#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub enum PeerRole {
	/// Full archive node
	Full,
	/// Light client
	Light,
	/// Authority/Validator
	Authority,
}

impl std::fmt::Display for PeerRole {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			PeerRole::Full => write!(f, "Full"),
			PeerRole::Light => write!(f, "Light"),
			PeerRole::Authority => write!(f, "Authority"),
		}
	}
}

/// Network statistics
#[derive(Clone, Debug)]
pub struct NetworkStatistics {
	/// Number of connected peers
	pub connected_peers: u32,
	/// Inbound bandwidth (bytes/sec)
	pub inbound_bandwidth: u64,
	/// Outbound bandwidth (bytes/sec)
	pub outbound_bandwidth: u64,
	/// Average peer latency (ms)
	pub avg_latency: u64,
	/// Best block synchronized
	pub best_block: u32,
	/// Best block hash
	pub best_hash: String,
	/// Number of pending transactions
	pub pending_transactions: u32,
}

/// Network bootstrap manager
pub struct BootstrapManager {
	config: BootstrapConfig,
	protocol_info: ProtocolInfo,
	connected_peers: Vec<PeerInfo>,
}

impl BootstrapManager {
	/// Create new bootstrap manager
	pub fn new(config: BootstrapConfig, protocol_info: ProtocolInfo) -> Self {
		Self {
			config,
			protocol_info,
			connected_peers: Vec::new(),
		}
	}

	/// Get bootstrap nodes
	pub fn bootstrap_nodes(&self) -> &[Multiaddr] {
		&self.config.bootstrap_nodes
	}

	/// Add a connected peer
	pub fn add_peer(&mut self, peer: PeerInfo) {
		self.connected_peers.push(peer);
	}

	/// Get connected peers
	pub fn connected_peers(&self) -> &[PeerInfo] {
		&self.connected_peers
	}

	/// Get protocol info
	pub fn protocol_info(&self) -> &ProtocolInfo {
		&self.protocol_info
	}

	/// Check if peer discovery is enabled
	pub fn discovery_enabled(&self) -> bool {
		self.config.enable_discovery
	}

	/// Check if mDNS is enabled
	pub fn mdns_enabled(&self) -> bool {
		self.config.enable_mdns
	}

	/// Check if Kademlia DHT is enabled
	pub fn kad_enabled(&self) -> bool {
		self.config.enable_kad
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_bootstrap_config_mainnet() {
		let config = BootstrapConfig::mainnet();
		assert!(!config.bootstrap_nodes.is_empty());
		assert!(!config.enable_mdns);
		assert!(config.enable_discovery);
	}

	#[test]
	fn test_bootstrap_config_testnet() {
		let config = BootstrapConfig::testnet();
		assert!(!config.bootstrap_nodes.is_empty());
		assert!(config.enable_mdns);
	}

	#[test]
	fn test_peer_role_display() {
		assert_eq!(format!("{}", PeerRole::Full), "Full");
		assert_eq!(format!("{}", PeerRole::Authority), "Authority");
	}
}
