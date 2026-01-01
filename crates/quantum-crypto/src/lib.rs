//! Quantum-Resistant Cryptography for Atlas Sphere
//!
//! This crate provides post-quantum cryptographic primitives to future-proof
//! Atlas Sphere against quantum computing attacks.
//!
//! # Algorithms
//!
//! - **SPHINCS+**: Stateless hash-based signatures (NIST PQC finalist)
//! - **Kyber**: Lattice-based key encapsulation mechanism
//! - **Dilithium**: Lattice-based digital signatures
//! - **BLAKE3+**: Extended hash functions for quantum resistance
//!
//! # Security Levels
//!
//! | Algorithm | Classical | Post-Quantum |
//! |-----------|-----------|--------------|
//! | SPHINCS+  | 256-bit   | 128-bit      |
//! | Kyber-768 | 192-bit   | 192-bit      |
//! | Dilithium3| 192-bit   | 192-bit      |
//!
//! # Usage
//!
//! ```ignore
//! use quantum_crypto::{sphincs, kyber, dilithium};
//!
//! // Generate quantum-resistant keys
//! let (pk, sk) = sphincs::keygen();
//!
//! // Sign a message
//! let signature = sphincs::sign(&sk, message);
//!
//! // Verify signature
//! assert!(sphincs::verify(&pk, message, &signature));
//! ```

#![allow(dead_code)]
#![allow(unused_variables)]

pub mod blake3ext;
pub mod dilithium;
pub mod error;
pub mod hash;
pub mod kyber;
pub mod sphincs;
pub mod types;

pub use error::{QuantumError, QuantumResult};
pub use types::*;

/// Quantum Crypto version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Security level for quantum-resistant operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityLevel {
    /// NIST Level 1 (AES-128 equivalent)
    Level1,
    /// NIST Level 3 (AES-192 equivalent)
    Level3,
    /// NIST Level 5 (AES-256 equivalent)
    Level5,
}

impl Default for SecurityLevel {
    fn default() -> Self {
        SecurityLevel::Level3
    }
}

/// Combined quantum-resistant keypair
pub struct QuantumKeypair {
    /// SPHINCS+ keypair for signatures
    pub sphincs: sphincs::SphincsKeypair,
    /// Kyber keypair for key encapsulation
    pub kyber: kyber::KyberKeypair,
    /// Dilithium keypair for fast signatures
    pub dilithium: dilithium::DilithiumKeypair,
}

impl QuantumKeypair {
    /// Generate a new quantum-resistant keypair
    pub fn generate(level: SecurityLevel) -> Self {
        Self {
            sphincs: sphincs::SphincsKeypair::generate(level),
            kyber: kyber::KyberKeypair::generate(level),
            dilithium: dilithium::DilithiumKeypair::generate(level),
        }
    }

    /// Sign a message using SPHINCS+ (most secure, larger signatures)
    pub fn sign_sphincs(&self, message: &[u8]) -> sphincs::SphincsSignature {
        self.sphincs.sign(message)
    }

    /// Sign a message using Dilithium (faster, smaller signatures)
    pub fn sign_dilithium(&self, message: &[u8]) -> dilithium::DilithiumSignature {
        self.dilithium.sign(message)
    }

    /// Encapsulate a shared secret using Kyber
    pub fn encapsulate(
        &self,
        recipient_pk: &kyber::KyberPublicKey,
    ) -> (kyber::KyberCiphertext, kyber::SharedSecret) {
        kyber::encapsulate(recipient_pk)
    }

    /// Decapsulate a shared secret using Kyber
    pub fn decapsulate(
        &self,
        ciphertext: &kyber::KyberCiphertext,
    ) -> QuantumResult<kyber::SharedSecret> {
        self.kyber.decapsulate(ciphertext)
    }
}

/// Quantum-resistant hash function wrapper
pub fn quantum_hash(data: &[u8]) -> hash::QuantumHash {
    hash::quantum_hash(data)
}

/// Quantum-resistant address derivation
pub fn derive_quantum_address(pubkey: &[u8]) -> [u8; 32] {
    let hash = blake3::hash(pubkey);
    let mut address = [0u8; 32];
    address.copy_from_slice(hash.as_bytes());
    address
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let keypair = QuantumKeypair::generate(SecurityLevel::Level3);
        assert!(!keypair.sphincs.public_key.as_bytes().is_empty());
    }

    #[test]
    fn test_quantum_hash() {
        let data = b"test data";
        let hash1 = quantum_hash(data);
        let hash2 = quantum_hash(data);
        assert_eq!(hash1, hash2);
    }
}
