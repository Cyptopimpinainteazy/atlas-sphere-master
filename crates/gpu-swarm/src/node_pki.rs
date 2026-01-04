//! Node Public Key Infrastructure (PKI) Registry
//!
//! Stores and manages public keys for GPU nodes.
//! Allows the coordinator to look up node public keys when verifying signatures.
//!
//! # Key Format
//!
//! This registry stores *public key bytes* (opaque to the rest of the system) keyed by `NodeId`.
//! At this phase we accept both common node-identity formats:
//!
//! - **ed25519** public key (32 bytes)
//! - **secp256k1** public key
//!   - Compressed (33 bytes): `0x02|0x03 + 32 bytes`
//!   - Uncompressed (65 bytes): `0x04 + 64 bytes`
//!
//! The exact signature scheme used by the settlement layer can be finalized later; the registry
//! is intentionally format-flexible while still rejecting obviously invalid encodings.

use std::collections::HashMap;

use crate::node::NodeId;

/// Result type for PKI operations
pub type PkiResult<T> = Result<T, String>;

/// Public key information for a GPU node
#[derive(Debug, Clone)]
pub struct NodePublicKey {
    pub node_id: NodeId,
    pub pubkey: Vec<u8>,
    pub registered_at: u64,
}

/// Registry for managing node public keys
#[derive(Debug, Clone)]
pub struct NodePKIRegistry {
    pub_keys: HashMap<NodeId, Vec<u8>>,
}

impl NodePKIRegistry {
    /// Create a new empty PKI registry
    pub fn new() -> Self {
        NodePKIRegistry {
            pub_keys: HashMap::new(),
        }
    }

    /// Register a node's public key
    ///
    /// # Arguments
    /// * `node_id` - Unique identifier for the node
    /// * `pubkey` - Public key bytes (ed25519 32B, or secp256k1 33B/65B)
    ///
    /// # Returns
    /// `Ok(())` if registration successful, `Err(String)` otherwise
    ///
    /// # Errors
    /// - "Invalid node ID" - If node_id is all zeroes
    /// - "Empty public key" - If pubkey is empty
    /// - "Invalid public key encoding" - If pubkey does not look like an ed25519 or secp256k1 key
    /// - "Node already registered" - If node already registered
    pub fn register_public_key(&mut self, node_id: NodeId, pubkey: Vec<u8>) -> PkiResult<()> {
        if node_id == [0u8; 32] {
            return Err("Invalid node ID".to_string());
        }

        if pubkey.is_empty() {
            return Err("Empty public key".to_string());
        }

        validate_public_key_bytes(&pubkey)?;

        if self.pub_keys.contains_key(&node_id) {
            return Err("Node already registered".to_string());
        }

        self.pub_keys.insert(node_id, pubkey);
        Ok(())
    }

    /// Get the public key for a node
    ///
    /// # Arguments
    /// * `node_id` - Node identifier to look up
    ///
    /// # Returns
    /// `Some(Vec<u8>)` if node found, `None` otherwise
    pub fn get_public_key(&self, node_id: &NodeId) -> Option<Vec<u8>> {
        self.pub_keys.get(node_id).cloned()
    }

    /// Get the public key for a node, returning error if not found
    ///
    /// # Arguments
    /// * `node_id` - Node identifier to look up
    ///
    /// # Returns
    /// `Ok(Vec<u8>)` if node found, `Err(String)` otherwise
    pub fn get_public_key_or_error(&self, node_id: &NodeId) -> PkiResult<Vec<u8>> {
        self.pub_keys
            .get(node_id)
            .cloned()
            .ok_or_else(|| "Node not found".to_string())
    }

    /// Check if a node is registered
    ///
    /// # Arguments
    /// * `node_id` - Node identifier to check
    ///
    /// # Returns
    /// `true` if node is registered, `false` otherwise
    pub fn is_registered(&self, node_id: &NodeId) -> bool {
        self.pub_keys.contains_key(node_id)
    }

    /// Get the number of registered nodes
    pub fn node_count(&self) -> usize {
        self.pub_keys.len()
    }

    /// Get all registered node IDs
    pub fn list_nodes(&self) -> Vec<NodeId> {
        self.pub_keys.keys().copied().collect()
    }
}

fn validate_public_key_bytes(pubkey: &[u8]) -> PkiResult<()> {
    match pubkey.len() {
        // ed25519 public key
        32 => Ok(()),

        // secp256k1 compressed public key
        33 if matches!(pubkey[0], 0x02 | 0x03) => Ok(()),

        // secp256k1 uncompressed public key
        65 if pubkey[0] == 0x04 => Ok(()),

        33 | 65 => Err("Invalid public key encoding".to_string()),
        _ => Err(format!(
            "Invalid public key size: expected 32 (ed25519) or 33/65 (secp256k1), got {}",
            pubkey.len()
        )),
    }
}

impl Default for NodePKIRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node_id(seed: u8) -> NodeId {
        let mut id = [0u8; 32];
        id[0] = seed;
        for i in 1..32 {
            id[i] = seed.wrapping_add(i as u8);
        }
        id
    }

    fn ed25519_pubkey() -> Vec<u8> {
        vec![0x11; 32]
    }

    fn secp256k1_compressed(prefix: u8) -> Vec<u8> {
        let mut key = vec![prefix];
        key.extend_from_slice(&[0x22; 32]);
        key
    }

    fn secp256k1_uncompressed() -> Vec<u8> {
        let mut key = vec![0x04];
        key.extend_from_slice(&[0x33; 64]);
        key
    }

    #[test]
    fn test_register_valid_pubkey() {
        let mut registry = NodePKIRegistry::new();
        let id = node_id(1);
        let pubkey = secp256k1_compressed(0x02);

        let result = registry.register_public_key(id, pubkey.clone());

        assert!(result.is_ok());
        assert!(registry.is_registered(&id));
        assert_eq!(registry.get_public_key(&id).unwrap(), pubkey);
    }

    #[test]
    fn test_register_ed25519_pubkey() {
        let mut registry = NodePKIRegistry::new();
        let id = node_id(2);
        let pubkey = ed25519_pubkey();

        assert!(registry.register_public_key(id, pubkey.clone()).is_ok());
        assert_eq!(registry.get_public_key(&id).unwrap(), pubkey);
    }

    #[test]
    fn test_register_secp256k1_uncompressed_pubkey() {
        let mut registry = NodePKIRegistry::new();
        let id = node_id(3);
        let pubkey = secp256k1_uncompressed();

        assert!(registry.register_public_key(id, pubkey.clone()).is_ok());
        assert_eq!(registry.get_public_key(&id).unwrap(), pubkey);
    }

    #[test]
    fn test_register_invalid_pubkey_length() {
        let mut registry = NodePKIRegistry::new();
        let id = node_id(1);
        let pubkey = vec![0x02; 31];

        let result = registry.register_public_key(id, pubkey);

        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap()
            .contains("Invalid public key size"));
    }

    #[test]
    fn test_register_invalid_key_format() {
        let mut registry = NodePKIRegistry::new();
        let id = node_id(1);
        let pubkey = secp256k1_compressed(0x05);

        let result = registry.register_public_key(id, pubkey);

        assert!(result.is_err());
        assert!(result.err().unwrap().contains("Invalid public key encoding"));
    }

    #[test]
    fn test_reject_duplicate_registration() {
        let mut registry = NodePKIRegistry::new();
        let id = node_id(1);
        let pubkey = secp256k1_compressed(0x02);

        registry
            .register_public_key(id, pubkey.clone())
            .unwrap();

        let result = registry.register_public_key(id, pubkey);

        assert!(result.is_err());
        assert!(result.err().unwrap().contains("already registered"));
    }

    #[test]
    fn test_get_unregistered_node() {
        let registry = NodePKIRegistry::new();

        let result = registry.get_public_key(&node_id(9));

        assert_eq!(result, None);
    }

    #[test]
    fn test_get_unregistered_node_error() {
        let registry = NodePKIRegistry::new();

        let result = registry.get_public_key_or_error(&node_id(9));

        assert!(result.is_err());
        assert!(result.err().unwrap().contains("Node not found"));
    }

    #[test]
    fn test_is_registered() {
        let mut registry = NodePKIRegistry::new();
        let id1 = node_id(1);
        let id2 = node_id(2);
        let pubkey = secp256k1_compressed(0x02);

        registry.register_public_key(id1, pubkey).unwrap();

        assert!(registry.is_registered(&id1));
        assert!(!registry.is_registered(&id2));
    }

    #[test]
    fn test_multiple_nodes() {
        let mut registry = NodePKIRegistry::new();
        let id1 = node_id(1);
        let id2 = node_id(2);
        let pubkey1 = secp256k1_compressed(0x02);
        let pubkey2 = secp256k1_compressed(0x03);

        registry.register_public_key(id1, pubkey1.clone()).unwrap();
        registry.register_public_key(id2, pubkey2.clone()).unwrap();

        assert_eq!(registry.node_count(), 2);
        assert_eq!(registry.get_public_key(&id1).unwrap(), pubkey1);
        assert_eq!(registry.get_public_key(&id2).unwrap(), pubkey2);
    }

    #[test]
    fn test_list_nodes() {
        let mut registry = NodePKIRegistry::new();
        let id1 = node_id(1);
        let id2 = node_id(2);
        let pubkey = secp256k1_compressed(0x02);

        registry.register_public_key(id1, pubkey.clone()).unwrap();
        registry.register_public_key(id2, pubkey).unwrap();

        let nodes = registry.list_nodes();
        assert_eq!(nodes.len(), 2);
        assert!(nodes.contains(&id1));
        assert!(nodes.contains(&id2));
    }

    #[test]
    fn test_default_constructor() {
        let registry = NodePKIRegistry::default();
        assert_eq!(registry.node_count(), 0);
    }

    #[test]
    fn test_invalid_node_id() {
        let mut registry = NodePKIRegistry::new();
        let pubkey = secp256k1_compressed(0x02);

        let result = registry.register_public_key([0u8; 32], pubkey);

        assert!(result.is_err());
        assert!(result.err().unwrap().contains("Invalid node ID"));
    }

    #[test]
    fn test_empty_public_key() {
        let mut registry = NodePKIRegistry::new();

        let result = registry.register_public_key(node_id(1), vec![]);

        assert!(result.is_err());
        assert!(result.err().unwrap().contains("Empty public key"));
    }

    #[test]
    fn test_both_key_prefixes() {
        let mut registry = NodePKIRegistry::new();
        let pubkey_02 = secp256k1_compressed(0x02);
        let pubkey_03 = secp256k1_compressed(0x03);

        assert!(registry.register_public_key(node_id(1), pubkey_02).is_ok());
        assert!(registry.register_public_key(node_id(2), pubkey_03).is_ok());

        assert_eq!(registry.node_count(), 2);
    }
}
