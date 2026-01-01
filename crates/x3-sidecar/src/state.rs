//! State Manager with Merkle Tree

use blake2::{Blake2s256, Digest};
use std::collections::BTreeMap;

/// State manager for tracking execution state
pub struct StateManager {
    /// Current state tree
    tree: MerkleTree,
    /// Checkpoints for rollback
    checkpoints: Vec<MerkleTree>,
}

impl StateManager {
    /// Create new state manager
    pub fn new() -> Self {
        Self {
            tree: MerkleTree::new(),
            checkpoints: Vec::new(),
        }
    }

    /// Create a checkpoint
    pub fn checkpoint(&mut self) {
        self.checkpoints.push(self.tree.clone());
    }

    /// Rollback to last checkpoint
    pub fn rollback(&mut self) {
        if let Some(tree) = self.checkpoints.pop() {
            self.tree = tree;
        }
    }

    /// Get current state root
    pub fn root(&self) -> [u8; 32] {
        self.tree.root()
    }

    /// Set a key-value pair
    pub fn set(&mut self, key: &[u8], value: &[u8]) {
        self.tree.insert(key.to_vec(), value.to_vec());
    }

    /// Get a value by key
    pub fn get(&self, key: &[u8]) -> Option<&[u8]> {
        self.tree.get(key)
    }

    /// Generate Merkle proof for a key
    pub fn proof(&self, key: &[u8]) -> Option<Vec<[u8; 32]>> {
        self.tree.proof(key)
    }

    /// Verify a Merkle proof
    pub fn verify_proof(root: &[u8; 32], key: &[u8], value: &[u8], proof: &[[u8; 32]]) -> bool {
        MerkleTree::verify(root, key, value, proof)
    }
}

impl Default for StateManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple Merkle tree implementation
#[derive(Clone)]
struct MerkleTree {
    leaves: BTreeMap<Vec<u8>, Vec<u8>>,
    cached_root: Option<[u8; 32]>,
}

impl MerkleTree {
    fn new() -> Self {
        Self {
            leaves: BTreeMap::new(),
            cached_root: None,
        }
    }

    fn insert(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.leaves.insert(key, value);
        self.cached_root = None; // Invalidate cache
    }

    fn get(&self, key: &[u8]) -> Option<&[u8]> {
        self.leaves.get(key).map(|v| v.as_slice())
    }

    fn root(&self) -> [u8; 32] {
        if let Some(cached) = self.cached_root {
            return cached;
        }

        if self.leaves.is_empty() {
            return [0u8; 32];
        }

        // Hash all leaves
        let mut hashes: Vec<[u8; 32]> = self
            .leaves
            .iter()
            .map(|(k, v)| {
                let mut hasher = Blake2s256::new();
                hasher.update(k);
                hasher.update(v);
                hasher.finalize().into()
            })
            .collect();

        // Build tree
        while hashes.len() > 1 {
            let mut next_level = Vec::new();
            for chunk in hashes.chunks(2) {
                let mut hasher = Blake2s256::new();
                hasher.update(chunk[0]);
                if chunk.len() > 1 {
                    hasher.update(chunk[1]);
                } else {
                    hasher.update(chunk[0]); // Duplicate odd leaf
                }
                next_level.push(hasher.finalize().into());
            }
            hashes = next_level;
        }

        hashes[0]
    }

    fn proof(&self, key: &[u8]) -> Option<Vec<[u8; 32]>> {
        if !self.leaves.contains_key(key) {
            return None;
        }

        let keys: Vec<_> = self.leaves.keys().collect();
        let idx = keys.iter().position(|k| k.as_slice() == key)?;

        // Build proof path
        let mut proof = Vec::new();
        let mut hashes: Vec<[u8; 32]> = self
            .leaves
            .iter()
            .map(|(k, v)| {
                let mut hasher = Blake2s256::new();
                hasher.update(k);
                hasher.update(v);
                hasher.finalize().into()
            })
            .collect();

        let mut current_idx = idx;
        while hashes.len() > 1 {
            // Get sibling
            let sibling_idx = if current_idx % 2 == 0 {
                current_idx + 1
            } else {
                current_idx - 1
            };

            if sibling_idx < hashes.len() {
                proof.push(hashes[sibling_idx]);
            } else if !hashes.is_empty() {
                proof.push(hashes[hashes.len() - 1]);
            }

            // Move to parent level
            let mut next_level = Vec::new();
            for chunk in hashes.chunks(2) {
                let mut hasher = Blake2s256::new();
                hasher.update(chunk[0]);
                if chunk.len() > 1 {
                    hasher.update(chunk[1]);
                } else {
                    hasher.update(chunk[0]);
                }
                next_level.push(hasher.finalize().into());
            }
            hashes = next_level;
            current_idx /= 2;
        }

        Some(proof)
    }

    fn verify(root: &[u8; 32], key: &[u8], value: &[u8], proof: &[[u8; 32]]) -> bool {
        let mut hasher = Blake2s256::new();
        hasher.update(key);
        hasher.update(value);
        let mut current_hash: [u8; 32] = hasher.finalize().into();

        for sibling in proof {
            let mut hasher = Blake2s256::new();
            // Order matters for Merkle proof - simplified version
            if current_hash < *sibling {
                hasher.update(current_hash);
                hasher.update(sibling);
            } else {
                hasher.update(sibling);
                hasher.update(current_hash);
            }
            current_hash = hasher.finalize().into();
        }

        &current_hash == root
    }
}

/// State diff for tracking changes
#[derive(Clone, Debug, Default)]
pub struct StateDiff {
    pub changes: Vec<StateChange>,
}

/// Individual state change
#[derive(Clone, Debug)]
pub struct StateChange {
    pub key: Vec<u8>,
    pub old_value: Option<Vec<u8>>,
    pub new_value: Option<Vec<u8>>,
}

impl StateDiff {
    pub fn new() -> Self {
        Self {
            changes: Vec::new(),
        }
    }

    pub fn record_change(
        &mut self,
        key: Vec<u8>,
        old_value: Option<Vec<u8>>,
        new_value: Option<Vec<u8>>,
    ) {
        self.changes.push(StateChange {
            key,
            old_value,
            new_value,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_manager_basic() {
        let mut state = StateManager::new();

        // Set some values
        state.set(b"key1", b"value1");
        state.set(b"key2", b"value2");

        assert_eq!(state.get(b"key1"), Some(b"value1".as_slice()));
        assert_eq!(state.get(b"key2"), Some(b"value2".as_slice()));
        assert_eq!(state.get(b"key3"), None);
    }

    #[test]
    fn test_state_checkpoint_rollback() {
        let mut state = StateManager::new();

        state.set(b"key1", b"value1");
        let root_before = state.root();

        state.checkpoint();
        state.set(b"key1", b"value2");
        assert_eq!(state.get(b"key1"), Some(b"value2".as_slice()));

        state.rollback();
        assert_eq!(state.get(b"key1"), Some(b"value1".as_slice()));
        assert_eq!(state.root(), root_before);
    }

    #[test]
    fn test_merkle_root_consistency() {
        let mut state1 = StateManager::new();
        let mut state2 = StateManager::new();

        state1.set(b"a", b"1");
        state1.set(b"b", b"2");

        state2.set(b"a", b"1");
        state2.set(b"b", b"2");

        assert_eq!(state1.root(), state2.root());
    }
}
