//! Node Public Key Infrastructure (PKI) Registry
//!
//! Stores and manages public keys for GPU nodes.
//! Allows the coordinator to look up node public keys when verifying signatures.

use std::collections::HashMap;

/// Public key information for a GPU node
#[derive(Debug, Clone)]
pub struct NodePublicKey {
    pub node_id: String,
    pub pubkey: Vec<u8>,
    pub registered_at: u64,
}

/// Registry for managing node public keys
#[derive(Debug)]
pub struct NodePKIRegistry {
    keys: HashMap<String, Vec<u8>>,
}

impl NodePKIRegistry {
    /// Create a new empty PKI registry
    pub fn new() -> Self {
        NodePKIRegistry {
            keys: HashMap::new(),
        }
    }

    /// Register a node's public key
    ///
    /// # Arguments
    /// * `node_id` - Unique identifier for the node
    /// * `pubkey` - Public key bytes (must be 65 bytes for secp256k1 uncompressed)
    ///
    /// # Returns
    /// `Ok(())` if registration successful, `Err(String)` otherwise
    ///
    /// # Errors
    /// - Invalid pubkey length (must be 65 bytes)
    /// - Node already registered (duplicate registration prevented)
    pub fn register_public_key(&mut self, node_id: String, pubkey: Vec<u8>) -> Result<(), String> {
        // Validate key format: 65 bytes for secp256k1 uncompressed
        if pubkey.len() != 65 {
            return Err(format!(
                "Invalid pubkey length: {}, expected 65",
                pubkey.len()
            ));
        }

        // Prevent duplicate registration
        if self.keys.contains_key(&node_id) {
            return Err(format!("Node {} already registered", node_id));
        }

        self.keys.insert(node_id, pubkey);
        Ok(())
    }

    /// Get the public key for a node
    ///
    /// # Arguments
    /// * `node_id` - Node identifier to look up
    ///
    /// # Returns
    /// `Some(Vec<u8>)` if node found, `None` otherwise
    pub fn get_public_key(&self, node_id: &str) -> Option<Vec<u8>> {
        self.keys.get(node_id).cloned()
    }

    /// Check if a node is registered
    ///
    /// # Arguments
    /// * `node_id` - Node identifier to check
    ///
    /// # Returns
    /// `true` if node is registered, `false` otherwise
    pub fn is_registered(&self, node_id: &str) -> bool {
        self.keys.contains_key(node_id)
    }

    /// Get the number of registered nodes
    pub fn node_count(&self) -> usize {
        self.keys.len()
    }

    /// Get all registered node IDs
    pub fn list_nodes(&self) -> Vec<String> {
        self.keys.keys().cloned().collect()
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

    fn create_valid_pubkey() -> Vec<u8> {
        // Valid 65-byte secp256k1 uncompressed public key
        // Format: 0x04 + X (32 bytes) + Y (32 bytes)
        let mut key = vec![0x04];
        key.extend_from_slice(&[0x00; 32]);
        key.extend_from_slice(&[0x01; 32]);
        key
    }

    #[test]
    fn test_register_valid_pubkey() {
        let mut registry = NodePKIRegistry::new();
        let pubkey = create_valid_pubkey();

        let result = registry.register_public_key("node_1".to_string(), pubkey.clone());

        assert!(result.is_ok());
        assert!(registry.is_registered("node_1"));
        assert_eq!(registry.get_public_key("node_1"), Some(pubkey));
    }

    #[test]
    fn test_register_invalid_pubkey_length() {
        let mut registry = NodePKIRegistry::new();
        let pubkey = vec![0x00; 32]; // Too short

        let result = registry.register_public_key("node_1".to_string(), pubkey);

        assert!(result.is_err());
        assert!(result
            .err()
            .unwrap()
            .contains("Invalid pubkey length"));
    }

    #[test]
    fn test_reject_duplicate_registration() {
        let mut registry = NodePKIRegistry::new();
        let pubkey = create_valid_pubkey();

        registry
            .register_public_key("node_1".to_string(), pubkey.clone())
            .unwrap();

        let result = registry.register_public_key("node_1".to_string(), pubkey);

        assert!(result.is_err());
        assert!(result.err().unwrap().contains("already registered"));
    }

    #[test]
    fn test_get_unregistered_node() {
        let registry = NodePKIRegistry::new();

        let result = registry.get_public_key("unknown");

        assert_eq!(result, None);
    }

    #[test]
    fn test_is_registered() {
        let mut registry = NodePKIRegistry::new();
        let pubkey = create_valid_pubkey();

        registry
            .register_public_key("node_1".to_string(), pubkey)
            .unwrap();

        assert!(registry.is_registered("node_1"));
        assert!(!registry.is_registered("node_2"));
    }

    #[test]
    fn test_multiple_nodes() {
        let mut registry = NodePKIRegistry::new();
        let pubkey1 = create_valid_pubkey();
        let pubkey2 = create_valid_pubkey();

        registry
            .register_public_key("node_1".to_string(), pubkey1.clone())
            .unwrap();
        registry
            .register_public_key("node_2".to_string(), pubkey2.clone())
            .unwrap();

        assert_eq!(registry.node_count(), 2);
        assert_eq!(registry.get_public_key("node_1"), Some(pubkey1));
        assert_eq!(registry.get_public_key("node_2"), Some(pubkey2));
    }

    #[test]
    fn test_list_nodes() {
        let mut registry = NodePKIRegistry::new();
        let pubkey = create_valid_pubkey();

        registry
            .register_public_key("node_1".to_string(), pubkey.clone())
            .unwrap();
        registry
            .register_public_key("node_2".to_string(), pubkey)
            .unwrap();

        let nodes = registry.list_nodes();
        assert_eq!(nodes.len(), 2);
        assert!(nodes.contains(&"node_1".to_string()));
        assert!(nodes.contains(&"node_2".to_string()));
    }

    #[test]
    fn test_default_constructor() {
        let registry = NodePKIRegistry::default();
        assert_eq!(registry.node_count(), 0);
    }
}
