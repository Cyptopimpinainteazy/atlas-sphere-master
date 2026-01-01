//! Extended BLAKE3 Hash Functions
//!
//! Quantum-resistant hash functions based on BLAKE3 with extended output support.
//! Provides various hash lengths and domain separation for security.

use blake3::Hasher;
use serde::{Deserialize, Serialize};

/// Domain separator for different use cases
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashDomain {
    /// Generic hashing
    Generic,
    /// Key derivation
    KeyDerivation,
    /// Message authentication
    MessageAuth,
    /// Commitment scheme
    Commitment,
    /// Merkle tree
    MerkleTree,
    /// Random oracle
    RandomOracle,
    /// Address derivation
    AddressDerivation,
    /// Transaction hashing
    Transaction,
    /// Block hashing
    Block,
    /// State commitment
    StateCommitment,
}

impl HashDomain {
    fn as_bytes(&self) -> &[u8] {
        match self {
            HashDomain::Generic => b"ATLAS_HASH_GENERIC",
            HashDomain::KeyDerivation => b"ATLAS_HASH_KDF",
            HashDomain::MessageAuth => b"ATLAS_HASH_MAC",
            HashDomain::Commitment => b"ATLAS_HASH_COMMIT",
            HashDomain::MerkleTree => b"ATLAS_HASH_MERKLE",
            HashDomain::RandomOracle => b"ATLAS_HASH_RO",
            HashDomain::AddressDerivation => b"ATLAS_HASH_ADDR",
            HashDomain::Transaction => b"ATLAS_HASH_TX",
            HashDomain::Block => b"ATLAS_HASH_BLOCK",
            HashDomain::StateCommitment => b"ATLAS_HASH_STATE",
        }
    }
}

/// 256-bit hash output
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Hash256(pub [u8; 32]);

impl Hash256 {
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

impl Default for Hash256 {
    fn default() -> Self {
        Self([0u8; 32])
    }
}

impl AsRef<[u8]> for Hash256 {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// 512-bit hash output
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Hash512(pub [u8; 64]);

impl Hash512 {
    pub fn as_bytes(&self) -> &[u8; 64] {
        &self.0
    }

    pub fn from_bytes(bytes: [u8; 64]) -> Self {
        Self(bytes)
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

impl Serialize for Hash512 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for Hash512 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let bytes = hex::decode(&s).map_err(serde::de::Error::custom)?;
        if bytes.len() != 64 {
            return Err(serde::de::Error::custom("Invalid hash length"));
        }
        let mut arr = [0u8; 64];
        arr.copy_from_slice(&bytes);
        Ok(Hash512(arr))
    }
}

impl Default for Hash512 {
    fn default() -> Self {
        Self([0u8; 64])
    }
}

impl AsRef<[u8]> for Hash512 {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

/// Extended BLAKE3 hasher with domain separation
pub struct Blake3Extended {
    hasher: Hasher,
}

impl Default for Blake3Extended {
    fn default() -> Self {
        Self::new()
    }
}

impl Blake3Extended {
    /// Create a new hasher with no domain
    pub fn new() -> Self {
        Self {
            hasher: Hasher::new(),
        }
    }

    /// Create a new hasher with domain separation
    pub fn with_domain(domain: HashDomain) -> Self {
        let mut hasher = Hasher::new();
        hasher.update(domain.as_bytes());
        hasher.update(&[0u8]); // Separator
        Self { hasher }
    }

    /// Create a keyed hasher (for MAC)
    pub fn keyed(key: &[u8; 32]) -> Self {
        Self {
            hasher: Hasher::new_keyed(key),
        }
    }

    /// Create a hasher for key derivation
    pub fn derive_key(context: &str) -> Self {
        Self {
            hasher: Hasher::new_derive_key(context),
        }
    }

    /// Update the hasher with data
    pub fn update(&mut self, data: &[u8]) -> &mut Self {
        self.hasher.update(data);
        self
    }

    /// Finalize and return 256-bit hash
    pub fn finalize_256(self) -> Hash256 {
        let hash = self.hasher.finalize();
        Hash256(*hash.as_bytes())
    }

    /// Finalize and return 512-bit hash
    pub fn finalize_512(self) -> Hash512 {
        let mut output = [0u8; 64];
        let mut reader = self.hasher.finalize_xof();
        reader.fill(&mut output);
        Hash512(output)
    }

    /// Finalize and return arbitrary-length hash
    pub fn finalize_extended(self, output: &mut [u8]) {
        let mut reader = self.hasher.finalize_xof();
        reader.fill(output);
    }
}

/// Quick hash functions

/// Hash data to 256 bits
pub fn hash_256(data: &[u8]) -> Hash256 {
    let hash = blake3::hash(data);
    Hash256(*hash.as_bytes())
}

/// Hash data to 256 bits with domain separation
pub fn hash_256_domain(domain: HashDomain, data: &[u8]) -> Hash256 {
    let mut hasher = Blake3Extended::with_domain(domain);
    hasher.update(data);
    hasher.finalize_256()
}

/// Hash data to 512 bits
pub fn hash_512(data: &[u8]) -> Hash512 {
    let mut hasher = Blake3Extended::new();
    hasher.update(data);
    hasher.finalize_512()
}

/// Hash data to 512 bits with domain separation
pub fn hash_512_domain(domain: HashDomain, data: &[u8]) -> Hash512 {
    let mut hasher = Blake3Extended::with_domain(domain);
    hasher.update(data);
    hasher.finalize_512()
}

/// Hash multiple inputs
pub fn hash_many(inputs: &[&[u8]]) -> Hash256 {
    let mut hasher = Blake3Extended::new();
    for input in inputs {
        hasher.update(&(input.len() as u64).to_le_bytes());
        hasher.update(input);
    }
    hasher.finalize_256()
}

/// Keyed hash (MAC)
pub fn keyed_hash(key: &[u8; 32], data: &[u8]) -> Hash256 {
    let mut hasher = Blake3Extended::keyed(key);
    hasher.update(data);
    hasher.finalize_256()
}

/// Key derivation function
pub fn derive_key(context: &str, key_material: &[u8], output: &mut [u8]) {
    let mut hasher = Blake3Extended::derive_key(context);
    hasher.update(key_material);
    hasher.finalize_extended(output);
}

/// Derive a 256-bit key
pub fn derive_key_256(context: &str, key_material: &[u8]) -> Hash256 {
    let mut output = [0u8; 32];
    derive_key(context, key_material, &mut output);
    Hash256(output)
}

/// Merkle tree node hash
pub fn merkle_node(left: &Hash256, right: &Hash256) -> Hash256 {
    let mut hasher = Blake3Extended::with_domain(HashDomain::MerkleTree);
    hasher.update(left.as_bytes());
    hasher.update(right.as_bytes());
    hasher.finalize_256()
}

/// Merkle tree leaf hash
pub fn merkle_leaf(data: &[u8]) -> Hash256 {
    let mut hasher = Blake3Extended::with_domain(HashDomain::MerkleTree);
    hasher.update(&[0u8]); // Leaf prefix
    hasher.update(data);
    hasher.finalize_256()
}

/// Commitment scheme: commit to data
pub fn commit(data: &[u8], randomness: &[u8; 32]) -> Hash256 {
    let mut hasher = Blake3Extended::with_domain(HashDomain::Commitment);
    hasher.update(randomness);
    hasher.update(data);
    hasher.finalize_256()
}

/// Transaction hash
pub fn hash_transaction(tx_data: &[u8]) -> Hash256 {
    hash_256_domain(HashDomain::Transaction, tx_data)
}

/// Block hash
pub fn hash_block(block_data: &[u8]) -> Hash256 {
    hash_256_domain(HashDomain::Block, block_data)
}

/// Address derivation from public key
pub fn derive_address(public_key: &[u8]) -> [u8; 20] {
    let hash = hash_256_domain(HashDomain::AddressDerivation, public_key);
    let mut address = [0u8; 20];
    address.copy_from_slice(&hash.0[12..32]);
    address
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_256() {
        let data = b"Hello, Atlas Sphere!";
        let hash = hash_256(data);
        assert_eq!(hash.0.len(), 32);

        // Same input should produce same output
        let hash2 = hash_256(data);
        assert_eq!(hash, hash2);

        // Different input should produce different output
        let hash3 = hash_256(b"Different data");
        assert_ne!(hash, hash3);
    }

    #[test]
    fn test_domain_separation() {
        let data = b"Same data";

        let h1 = hash_256_domain(HashDomain::Generic, data);
        let h2 = hash_256_domain(HashDomain::Transaction, data);
        let h3 = hash_256_domain(HashDomain::Block, data);

        // Different domains should produce different hashes
        assert_ne!(h1, h2);
        assert_ne!(h2, h3);
        assert_ne!(h1, h3);
    }

    #[test]
    fn test_keyed_hash() {
        let key = [1u8; 32];
        let data = b"Message to authenticate";

        let mac1 = keyed_hash(&key, data);
        let mac2 = keyed_hash(&key, data);
        assert_eq!(mac1, mac2);

        let different_key = [2u8; 32];
        let mac3 = keyed_hash(&different_key, data);
        assert_ne!(mac1, mac3);
    }

    #[test]
    fn test_merkle_tree() {
        let leaf1 = merkle_leaf(b"Data 1");
        let leaf2 = merkle_leaf(b"Data 2");
        let leaf3 = merkle_leaf(b"Data 3");
        let leaf4 = merkle_leaf(b"Data 4");

        let node1 = merkle_node(&leaf1, &leaf2);
        let node2 = merkle_node(&leaf3, &leaf4);
        let root = merkle_node(&node1, &node2);

        assert_eq!(root.0.len(), 32);
    }

    #[test]
    fn test_commitment() {
        let data = b"Secret data";
        let randomness = [42u8; 32];

        let c1 = commit(data, &randomness);
        let c2 = commit(data, &randomness);
        assert_eq!(c1, c2);

        let different_randomness = [43u8; 32];
        let c3 = commit(data, &different_randomness);
        assert_ne!(c1, c3);
    }

    #[test]
    fn test_key_derivation() {
        let context = "atlas-sphere/key-derivation/v1";
        let key_material = b"Master secret key material";

        let key1 = derive_key_256(context, key_material);
        let key2 = derive_key_256(context, key_material);
        assert_eq!(key1, key2);

        let key3 = derive_key_256("different/context", key_material);
        assert_ne!(key1, key3);
    }
}
