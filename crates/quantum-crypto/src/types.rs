//! Common types for Quantum Crypto

use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

/// Quantum-resistant public key (aggregated)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuantumPublicKey {
    /// SPHINCS+ public key bytes
    pub sphincs: Vec<u8>,
    /// Kyber public key bytes
    pub kyber: Vec<u8>,
    /// Dilithium public key bytes
    pub dilithium: Vec<u8>,
}

/// Quantum-resistant secret key (aggregated)
#[derive(Clone, Zeroize)]
#[zeroize(drop)]
pub struct QuantumSecretKey {
    /// SPHINCS+ secret key bytes
    pub sphincs: Vec<u8>,
    /// Kyber secret key bytes
    pub kyber: Vec<u8>,
    /// Dilithium secret key bytes
    pub dilithium: Vec<u8>,
}

/// Quantum signature type (can be either SPHINCS+ or Dilithium)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QuantumSignature {
    /// SPHINCS+ signature (larger, more conservative)
    Sphincs(Vec<u8>),
    /// Dilithium signature (smaller, faster)
    Dilithium(Vec<u8>),
    /// Hybrid signature (both for maximum security)
    Hybrid {
        sphincs: Vec<u8>,
        dilithium: Vec<u8>,
    },
}

/// Key encapsulation result
#[derive(Debug, Clone)]
pub struct EncapsulationResult {
    /// Ciphertext to send to recipient
    pub ciphertext: Vec<u8>,
    /// Shared secret (keep private)
    pub shared_secret: [u8; 32],
}

/// Signature type preference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignaturePreference {
    /// Use SPHINCS+ for maximum security
    Sphincs,
    /// Use Dilithium for speed
    Dilithium,
    /// Use both for hybrid security
    Hybrid,
}

impl Default for SignaturePreference {
    fn default() -> Self {
        SignaturePreference::Dilithium
    }
}

/// Quantum-resistant hash output
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct QuantumHashOutput(pub [u8; 64]);

impl Serialize for QuantumHashOutput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&hex::encode(self.0))
    }
}

impl<'de> Deserialize<'de> for QuantumHashOutput {
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
        Ok(QuantumHashOutput(arr))
    }
}

impl AsRef<[u8]> for QuantumHashOutput {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<[u8; 64]> for QuantumHashOutput {
    fn from(bytes: [u8; 64]) -> Self {
        Self(bytes)
    }
}
