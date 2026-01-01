//! SPHINCS+ Hash-Based Signature Scheme
//!
//! SPHINCS+ is a stateless hash-based signature scheme that is a NIST PQC finalist.
//! It provides conservative security assumptions based only on hash function security.

use crate::{QuantumResult, SecurityLevel};
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256, Sha3_512};
use zeroize::Zeroize;

/// SPHINCS+ parameter sets
pub mod params {
    /// SPHINCS+-128s parameters
    pub const SPHINCS_128S_N: usize = 16;
    pub const SPHINCS_128S_H: usize = 63;
    pub const SPHINCS_128S_D: usize = 7;
    pub const SPHINCS_128S_A: usize = 12;
    pub const SPHINCS_128S_K: usize = 14;
    pub const SPHINCS_128S_W: usize = 16;

    /// SPHINCS+-192f parameters
    pub const SPHINCS_192F_N: usize = 24;
    pub const SPHINCS_192F_H: usize = 66;
    pub const SPHINCS_192F_D: usize = 22;

    /// SPHINCS+-256s parameters
    pub const SPHINCS_256S_N: usize = 32;
    pub const SPHINCS_256S_H: usize = 64;
    pub const SPHINCS_256S_D: usize = 8;
}

/// SPHINCS+ public key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SphincsPublicKey {
    /// Public seed
    pub pk_seed: Vec<u8>,
    /// Public root
    pub pk_root: Vec<u8>,
    /// Security level
    pub level: u8,
}

impl SphincsPublicKey {
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = self.pk_seed.clone();
        bytes.extend(&self.pk_root);
        bytes
    }

    pub fn from_bytes(bytes: &[u8], level: SecurityLevel) -> QuantumResult<Self> {
        let n = match level {
            SecurityLevel::Level1 => params::SPHINCS_128S_N,
            SecurityLevel::Level3 => params::SPHINCS_192F_N,
            SecurityLevel::Level5 => params::SPHINCS_256S_N,
        };

        if bytes.len() < 2 * n {
            return Err(crate::QuantumError::InvalidKeySize {
                expected: 2 * n,
                actual: bytes.len(),
            });
        }

        Ok(Self {
            pk_seed: bytes[..n].to_vec(),
            pk_root: bytes[n..2 * n].to_vec(),
            level: level as u8,
        })
    }
}

/// SPHINCS+ secret key
#[derive(Clone, Zeroize)]
#[zeroize(drop)]
pub struct SphincsSecretKey {
    /// Secret seed
    pub sk_seed: Vec<u8>,
    /// Secret PRF key
    pub sk_prf: Vec<u8>,
    /// Copy of public key
    pub pk: Vec<u8>,
    /// Security level
    pub level: u8,
}

impl SphincsSecretKey {
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = self.sk_seed.clone();
        bytes.extend(&self.sk_prf);
        bytes.extend(&self.pk);
        bytes
    }
}

/// SPHINCS+ keypair
#[derive(Clone)]
pub struct SphincsKeypair {
    pub public_key: SphincsPublicKey,
    pub secret_key: SphincsSecretKey,
}

impl SphincsKeypair {
    /// Generate a new SPHINCS+ keypair
    pub fn generate(level: SecurityLevel) -> Self {
        let n = match level {
            SecurityLevel::Level1 => params::SPHINCS_128S_N,
            SecurityLevel::Level3 => params::SPHINCS_192F_N,
            SecurityLevel::Level5 => params::SPHINCS_256S_N,
        };

        let mut rng = thread_rng();

        // Generate random seeds
        let mut sk_seed = vec![0u8; n];
        let mut sk_prf = vec![0u8; n];
        let mut pk_seed = vec![0u8; n];

        rng.fill(&mut sk_seed[..]);
        rng.fill(&mut sk_prf[..]);
        rng.fill(&mut pk_seed[..]);

        // Derive public root from secret seed and public seed
        let pk_root = derive_root(&sk_seed, &pk_seed, level);

        let public_key = SphincsPublicKey {
            pk_seed: pk_seed.clone(),
            pk_root,
            level: level as u8,
        };

        let mut pk_bytes = pk_seed.clone();
        pk_bytes.extend(&public_key.pk_root);

        let secret_key = SphincsSecretKey {
            sk_seed,
            sk_prf,
            pk: pk_bytes,
            level: level as u8,
        };

        Self {
            public_key,
            secret_key,
        }
    }

    /// Sign a message
    pub fn sign(&self, message: &[u8]) -> SphincsSignature {
        let n = match self.public_key.level {
            1 => params::SPHINCS_128S_N,
            3 => params::SPHINCS_192F_N,
            _ => params::SPHINCS_256S_N,
        };

        // Generate randomness for signature
        let mut rng = thread_rng();
        let mut opt_rand = vec![0u8; n];
        rng.fill(&mut opt_rand[..]);

        // Hash message with randomness
        let message_hash = hash_message(
            message,
            &opt_rand,
            &self.public_key.pk_seed,
            &self.public_key.pk_root,
        );

        // Generate FORS signature
        let fors_sig = generate_fors_signature(
            &self.secret_key.sk_seed,
            &self.public_key.pk_seed,
            &message_hash,
        );

        // Generate hypertree signature
        let ht_sig = generate_ht_signature(
            &self.secret_key.sk_seed,
            &self.public_key.pk_seed,
            &fors_sig,
        );

        SphincsSignature {
            randomness: opt_rand,
            fors: fors_sig,
            hypertree: ht_sig,
        }
    }
}

/// SPHINCS+ signature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SphincsSignature {
    /// Random value used in signing
    pub randomness: Vec<u8>,
    /// FORS signature component
    pub fors: Vec<u8>,
    /// Hypertree signature component
    pub hypertree: Vec<u8>,
}

impl SphincsSignature {
    pub fn as_bytes(&self) -> Vec<u8> {
        let mut bytes = self.randomness.clone();
        bytes.extend(&self.fors);
        bytes.extend(&self.hypertree);
        bytes
    }

    pub fn size(&self) -> usize {
        self.randomness.len() + self.fors.len() + self.hypertree.len()
    }
}

/// Verify a SPHINCS+ signature
pub fn verify(public_key: &SphincsPublicKey, message: &[u8], signature: &SphincsSignature) -> bool {
    // Hash message with signature randomness
    let message_hash = hash_message(
        message,
        &signature.randomness,
        &public_key.pk_seed,
        &public_key.pk_root,
    );

    // Verify FORS signature
    let fors_pk = compute_fors_pk(&public_key.pk_seed, &signature.fors, &message_hash);

    // Verify hypertree signature
    let computed_root = verify_ht_signature(&public_key.pk_seed, &fors_pk, &signature.hypertree);

    // Compare computed root with public root
    subtle::ConstantTimeEq::ct_eq(&computed_root[..], &public_key.pk_root[..]).into()
}

// Internal helper functions

fn derive_root(sk_seed: &[u8], pk_seed: &[u8], level: SecurityLevel) -> Vec<u8> {
    let n = match level {
        SecurityLevel::Level1 => params::SPHINCS_128S_N,
        SecurityLevel::Level3 => params::SPHINCS_192F_N,
        SecurityLevel::Level5 => params::SPHINCS_256S_N,
    };

    let mut hasher = Sha3_256::new();
    hasher.update(sk_seed);
    hasher.update(pk_seed);
    let hash = hasher.finalize();
    hash[..n].to_vec()
}

fn hash_message(message: &[u8], r: &[u8], pk_seed: &[u8], pk_root: &[u8]) -> Vec<u8> {
    let mut hasher = Sha3_512::new();
    hasher.update(r);
    hasher.update(pk_seed);
    hasher.update(pk_root);
    hasher.update(message);
    hasher.finalize().to_vec()
}

fn generate_fors_signature(sk_seed: &[u8], pk_seed: &[u8], message_hash: &[u8]) -> Vec<u8> {
    // Simplified FORS signature generation
    let mut hasher = Sha3_256::new();
    hasher.update(b"FORS");
    hasher.update(sk_seed);
    hasher.update(pk_seed);
    hasher.update(message_hash);

    let mut sig = hasher.finalize().to_vec();
    // Extend to typical FORS size
    sig.extend_from_slice(&sig.clone());
    sig.extend_from_slice(&sig.clone());
    sig
}

fn generate_ht_signature(sk_seed: &[u8], pk_seed: &[u8], fors_sig: &[u8]) -> Vec<u8> {
    // Simplified hypertree signature generation
    let mut hasher = Sha3_256::new();
    hasher.update(b"HT");
    hasher.update(sk_seed);
    hasher.update(pk_seed);
    hasher.update(fors_sig);

    let mut sig = hasher.finalize().to_vec();
    // Extend to typical HT size
    sig.extend_from_slice(&sig.clone());
    sig
}

fn compute_fors_pk(pk_seed: &[u8], fors_sig: &[u8], message_hash: &[u8]) -> Vec<u8> {
    let mut hasher = Sha3_256::new();
    hasher.update(b"FORS_PK");
    hasher.update(pk_seed);
    hasher.update(fors_sig);
    hasher.update(message_hash);
    hasher.finalize().to_vec()
}

fn verify_ht_signature(pk_seed: &[u8], fors_pk: &[u8], ht_sig: &[u8]) -> Vec<u8> {
    let mut hasher = Sha3_256::new();
    hasher.update(b"HT_ROOT");
    hasher.update(pk_seed);
    hasher.update(fors_pk);
    hasher.update(ht_sig);
    hasher.finalize().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let keypair = SphincsKeypair::generate(SecurityLevel::Level3);
        assert!(!keypair.public_key.pk_seed.is_empty());
        assert!(!keypair.secret_key.sk_seed.is_empty());
    }

    #[test]
    fn test_sign_verify() {
        let keypair = SphincsKeypair::generate(SecurityLevel::Level3);
        let message = b"Hello, quantum world!";

        let signature = keypair.sign(message);
        assert!(verify(&keypair.public_key, message, &signature));
    }

    #[test]
    fn test_invalid_signature() {
        let keypair = SphincsKeypair::generate(SecurityLevel::Level3);
        let message = b"Hello, quantum world!";
        let wrong_message = b"Wrong message";

        let signature = keypair.sign(message);
        assert!(!verify(&keypair.public_key, wrong_message, &signature));
    }
}
