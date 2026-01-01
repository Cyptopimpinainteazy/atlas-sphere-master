//! Quantum-Resistant Hash Functions
//!
//! Wrapper module providing a unified interface for quantum-resistant hashing.
//! Uses BLAKE3 internally with additional security features.

use crate::blake3ext::{self, Hash256, Hash512, HashDomain};
use serde::{Deserialize, Serialize};

/// Quantum-resistant hash configuration
#[derive(Debug, Clone, Copy)]
pub struct HashConfig {
    /// Use double hashing for extra security
    pub double_hash: bool,
    /// Include length prefix
    pub length_prefix: bool,
    /// Add timestamp for uniqueness
    pub timestamp: bool,
}

impl Default for HashConfig {
    fn default() -> Self {
        Self {
            double_hash: false,
            length_prefix: true,
            timestamp: false,
        }
    }
}

/// Quantum-resistant hash with additional metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantumHash {
    /// The hash value
    pub value: Hash256,
    /// Hash algorithm identifier
    pub algorithm: HashAlgorithm,
    /// Security level
    pub security_bits: u16,
}

impl QuantumHash {
    pub fn as_bytes(&self) -> &[u8; 32] {
        self.value.as_bytes()
    }

    pub fn to_hex(&self) -> String {
        self.value.to_hex()
    }
}

/// Available hash algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HashAlgorithm {
    /// BLAKE3 (256-bit)
    Blake3_256,
    /// BLAKE3 (512-bit)
    Blake3_512,
    /// Double BLAKE3 for extra security
    Blake3Double,
    /// BLAKE3 with SHA3 finalization
    Blake3Sha3Hybrid,
}

impl HashAlgorithm {
    pub fn security_bits(&self) -> u16 {
        match self {
            HashAlgorithm::Blake3_256 => 128,
            HashAlgorithm::Blake3_512 => 256,
            HashAlgorithm::Blake3Double => 256,
            HashAlgorithm::Blake3Sha3Hybrid => 256,
        }
    }
}

/// Main quantum-resistant hasher
pub struct QuantumHasher {
    config: HashConfig,
    algorithm: HashAlgorithm,
    data: Vec<u8>,
}

impl QuantumHasher {
    /// Create a new hasher with default configuration
    pub fn new() -> Self {
        Self {
            config: HashConfig::default(),
            algorithm: HashAlgorithm::Blake3_256,
            data: Vec::new(),
        }
    }

    /// Create a hasher with specific algorithm
    pub fn with_algorithm(algorithm: HashAlgorithm) -> Self {
        Self {
            config: HashConfig::default(),
            algorithm,
            data: Vec::new(),
        }
    }

    /// Create a hasher with custom configuration
    pub fn with_config(config: HashConfig) -> Self {
        Self {
            config,
            algorithm: HashAlgorithm::Blake3_256,
            data: Vec::new(),
        }
    }

    /// Set the hash algorithm
    pub fn algorithm(mut self, algorithm: HashAlgorithm) -> Self {
        self.algorithm = algorithm;
        self
    }

    /// Enable double hashing
    pub fn double_hash(mut self, enable: bool) -> Self {
        self.config.double_hash = enable;
        self
    }

    /// Update with data
    pub fn update(&mut self, data: &[u8]) -> &mut Self {
        if self.config.length_prefix {
            self.data
                .extend_from_slice(&(data.len() as u64).to_le_bytes());
        }
        self.data.extend_from_slice(data);
        self
    }

    /// Chain update
    pub fn chain_update(mut self, data: &[u8]) -> Self {
        self.update(data);
        self
    }

    /// Finalize and return hash
    pub fn finalize(self) -> QuantumHash {
        let hash = match self.algorithm {
            HashAlgorithm::Blake3_256 => {
                let h = blake3ext::hash_256(&self.data);
                if self.config.double_hash {
                    blake3ext::hash_256(h.as_bytes())
                } else {
                    h
                }
            }
            HashAlgorithm::Blake3_512 => {
                let h = blake3ext::hash_512(&self.data);
                // Truncate to 256 bits for consistent output
                let mut out = [0u8; 32];
                out.copy_from_slice(&h.0[..32]);
                Hash256(out)
            }
            HashAlgorithm::Blake3Double => {
                let h1 = blake3ext::hash_256(&self.data);
                blake3ext::hash_256(h1.as_bytes())
            }
            HashAlgorithm::Blake3Sha3Hybrid => {
                use sha3::{Digest, Sha3_256};
                let h1 = blake3ext::hash_256(&self.data);
                let mut sha3 = Sha3_256::new();
                sha3.update(h1.as_bytes());
                let h2 = sha3.finalize();
                Hash256(h2.into())
            }
        };

        QuantumHash {
            value: hash,
            algorithm: self.algorithm,
            security_bits: self.algorithm.security_bits(),
        }
    }
}

impl Default for QuantumHasher {
    fn default() -> Self {
        Self::new()
    }
}

/// Quick hash function
pub fn quantum_hash(data: &[u8]) -> QuantumHash {
    QuantumHasher::new().chain_update(data).finalize()
}

/// Hash with specific algorithm
pub fn quantum_hash_with(algorithm: HashAlgorithm, data: &[u8]) -> QuantumHash {
    QuantumHasher::with_algorithm(algorithm)
        .chain_update(data)
        .finalize()
}

/// Hash multiple inputs
pub fn quantum_hash_many(inputs: &[&[u8]]) -> QuantumHash {
    let mut hasher = QuantumHasher::new();
    for input in inputs {
        hasher.update(input);
    }
    hasher.finalize()
}

/// Verify a hash matches expected data
pub fn verify_hash(data: &[u8], expected: &QuantumHash) -> bool {
    let computed = quantum_hash_with(expected.algorithm, data);
    constant_time_compare(computed.value.as_bytes(), expected.value.as_bytes())
}

/// Constant-time comparison
fn constant_time_compare(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Hash chain for building Merkle-like structures
pub struct HashChain {
    current: Hash256,
    count: u64,
}

impl HashChain {
    /// Start a new chain with initial data
    pub fn new(initial: &[u8]) -> Self {
        Self {
            current: blake3ext::hash_256(initial),
            count: 1,
        }
    }

    /// Add data to the chain
    pub fn add(&mut self, data: &[u8]) -> &mut Self {
        let mut input = Vec::with_capacity(32 + 8 + data.len());
        input.extend_from_slice(self.current.as_bytes());
        input.extend_from_slice(&self.count.to_le_bytes());
        input.extend_from_slice(data);
        self.current = blake3ext::hash_256(&input);
        self.count += 1;
        self
    }

    /// Get current chain hash
    pub fn current(&self) -> Hash256 {
        self.current
    }

    /// Get chain length
    pub fn length(&self) -> u64 {
        self.count
    }

    /// Finalize the chain
    pub fn finalize(self) -> (Hash256, u64) {
        (self.current, self.count)
    }
}

/// Hash-based random number generator
pub struct HashRng {
    state: Hash512,
    counter: u64,
}

impl HashRng {
    /// Create from seed
    pub fn from_seed(seed: &[u8]) -> Self {
        Self {
            state: blake3ext::hash_512(seed),
            counter: 0,
        }
    }

    /// Generate next 32 bytes
    pub fn next_bytes(&mut self) -> [u8; 32] {
        let mut input = Vec::with_capacity(64 + 8);
        input.extend_from_slice(self.state.as_bytes());
        input.extend_from_slice(&self.counter.to_le_bytes());

        let output = blake3ext::hash_256(&input);

        // Update state
        let mut state_input = Vec::with_capacity(64 + 32);
        state_input.extend_from_slice(self.state.as_bytes());
        state_input.extend_from_slice(output.as_bytes());
        self.state = blake3ext::hash_512(&state_input);
        self.counter += 1;

        *output.as_bytes()
    }

    /// Generate next u64
    pub fn next_u64(&mut self) -> u64 {
        let bytes = self.next_bytes();
        u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantum_hash() {
        let data = b"Test quantum hash";
        let hash = quantum_hash(data);

        assert_eq!(hash.value.0.len(), 32);
        assert_eq!(hash.security_bits, 128);
    }

    #[test]
    fn test_hash_algorithms() {
        let data = b"Test data";

        let h1 = quantum_hash_with(HashAlgorithm::Blake3_256, data);
        let h2 = quantum_hash_with(HashAlgorithm::Blake3Double, data);
        let h3 = quantum_hash_with(HashAlgorithm::Blake3Sha3Hybrid, data);

        // Different algorithms should produce different hashes
        assert_ne!(h1.value, h2.value);
        assert_ne!(h2.value, h3.value);
    }

    #[test]
    fn test_verify_hash() {
        let data = b"Important data";
        let hash = quantum_hash(data);

        assert!(verify_hash(data, &hash));
        assert!(!verify_hash(b"Wrong data", &hash));
    }

    #[test]
    fn test_hash_chain() {
        let mut chain = HashChain::new(b"Genesis");
        chain.add(b"Block 1").add(b"Block 2").add(b"Block 3");

        let (hash, count) = chain.finalize();
        assert_eq!(count, 4);
        assert_eq!(hash.0.len(), 32);
    }

    #[test]
    fn test_hash_rng() {
        let mut rng = HashRng::from_seed(b"Seed for RNG");

        let r1 = rng.next_bytes();
        let r2 = rng.next_bytes();
        let r3 = rng.next_bytes();

        // Should produce different outputs
        assert_ne!(r1, r2);
        assert_ne!(r2, r3);
    }
}
