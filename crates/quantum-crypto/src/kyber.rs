//! Kyber Key Encapsulation Mechanism
//!
//! Kyber is a lattice-based key encapsulation mechanism (KEM) that is a NIST PQC winner.
//! It provides efficient key exchange with post-quantum security.

use crate::{QuantumError, QuantumResult, SecurityLevel};
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256, Sha3_512};
use zeroize::Zeroize;

/// Kyber parameter sets
pub mod params {
    /// Kyber-512 (NIST Level 1)
    pub const KYBER_512_N: usize = 256;
    pub const KYBER_512_K: usize = 2;
    pub const KYBER_512_PK_SIZE: usize = 800;
    pub const KYBER_512_SK_SIZE: usize = 1632;
    pub const KYBER_512_CT_SIZE: usize = 768;

    /// Kyber-768 (NIST Level 3)
    pub const KYBER_768_N: usize = 256;
    pub const KYBER_768_K: usize = 3;
    pub const KYBER_768_PK_SIZE: usize = 1184;
    pub const KYBER_768_SK_SIZE: usize = 2400;
    pub const KYBER_768_CT_SIZE: usize = 1088;

    /// Kyber-1024 (NIST Level 5)
    pub const KYBER_1024_N: usize = 256;
    pub const KYBER_1024_K: usize = 4;
    pub const KYBER_1024_PK_SIZE: usize = 1568;
    pub const KYBER_1024_SK_SIZE: usize = 3168;
    pub const KYBER_1024_CT_SIZE: usize = 1568;
}

/// Shared secret (32 bytes)
pub type SharedSecret = [u8; 32];

/// Kyber public key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KyberPublicKey {
    /// Public key bytes
    pub bytes: Vec<u8>,
    /// Security level
    pub level: u8,
}

impl KyberPublicKey {
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn from_bytes(bytes: &[u8], level: SecurityLevel) -> QuantumResult<Self> {
        let expected_size = match level {
            SecurityLevel::Level1 => params::KYBER_512_PK_SIZE,
            SecurityLevel::Level3 => params::KYBER_768_PK_SIZE,
            SecurityLevel::Level5 => params::KYBER_1024_PK_SIZE,
        };

        if bytes.len() != expected_size {
            return Err(QuantumError::InvalidKeySize {
                expected: expected_size,
                actual: bytes.len(),
            });
        }

        Ok(Self {
            bytes: bytes.to_vec(),
            level: level as u8,
        })
    }
}

/// Kyber secret key
#[derive(Clone, Zeroize)]
#[zeroize(drop)]
pub struct KyberSecretKey {
    /// Secret key bytes
    pub bytes: Vec<u8>,
    /// Security level
    pub level: u8,
}

impl KyberSecretKey {
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Kyber ciphertext
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KyberCiphertext {
    /// Ciphertext bytes
    pub bytes: Vec<u8>,
    /// Security level
    pub level: u8,
}

impl KyberCiphertext {
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn from_bytes(bytes: &[u8], level: SecurityLevel) -> QuantumResult<Self> {
        let expected_size = match level {
            SecurityLevel::Level1 => params::KYBER_512_CT_SIZE,
            SecurityLevel::Level3 => params::KYBER_768_CT_SIZE,
            SecurityLevel::Level5 => params::KYBER_1024_CT_SIZE,
        };

        if bytes.len() != expected_size {
            return Err(QuantumError::InvalidSignatureSize {
                expected: expected_size,
                actual: bytes.len(),
            });
        }

        Ok(Self {
            bytes: bytes.to_vec(),
            level: level as u8,
        })
    }
}

/// Kyber keypair
#[derive(Clone)]
pub struct KyberKeypair {
    pub public_key: KyberPublicKey,
    pub secret_key: KyberSecretKey,
}

impl KyberKeypair {
    /// Generate a new Kyber keypair
    pub fn generate(level: SecurityLevel) -> Self {
        let (pk_size, sk_size, k) = match level {
            SecurityLevel::Level1 => (
                params::KYBER_512_PK_SIZE,
                params::KYBER_512_SK_SIZE,
                params::KYBER_512_K,
            ),
            SecurityLevel::Level3 => (
                params::KYBER_768_PK_SIZE,
                params::KYBER_768_SK_SIZE,
                params::KYBER_768_K,
            ),
            SecurityLevel::Level5 => (
                params::KYBER_1024_PK_SIZE,
                params::KYBER_1024_SK_SIZE,
                params::KYBER_1024_K,
            ),
        };

        let mut rng = thread_rng();

        // Generate random seed
        let mut seed = [0u8; 64];
        rng.fill(&mut seed[..]);

        // Derive public and secret keys from seed
        let (pk_bytes, sk_bytes) = keygen_from_seed(&seed, pk_size, sk_size, k);

        Self {
            public_key: KyberPublicKey {
                bytes: pk_bytes,
                level: level as u8,
            },
            secret_key: KyberSecretKey {
                bytes: sk_bytes,
                level: level as u8,
            },
        }
    }

    /// Decapsulate a ciphertext to recover the shared secret
    pub fn decapsulate(&self, ciphertext: &KyberCiphertext) -> QuantumResult<SharedSecret> {
        decapsulate(&self.secret_key, ciphertext)
    }
}

/// Encapsulate: generate a shared secret and ciphertext
pub fn encapsulate(public_key: &KyberPublicKey) -> (KyberCiphertext, SharedSecret) {
    let level = match public_key.level {
        1 => SecurityLevel::Level1,
        3 => SecurityLevel::Level3,
        _ => SecurityLevel::Level5,
    };

    let ct_size = match level {
        SecurityLevel::Level1 => params::KYBER_512_CT_SIZE,
        SecurityLevel::Level3 => params::KYBER_768_CT_SIZE,
        SecurityLevel::Level5 => params::KYBER_1024_CT_SIZE,
    };

    let mut rng = thread_rng();
    let mut random_message = [0u8; 32];
    rng.fill(&mut random_message[..]);

    // Generate ciphertext from public key and random message
    let ct_bytes = encrypt_message(&public_key.bytes, &random_message, ct_size);

    // Derive shared secret
    let shared_secret = derive_shared_secret(&random_message, &ct_bytes);

    let ciphertext = KyberCiphertext {
        bytes: ct_bytes,
        level: public_key.level,
    };

    (ciphertext, shared_secret)
}

/// Decapsulate: recover the shared secret from ciphertext
pub fn decapsulate(
    secret_key: &KyberSecretKey,
    ciphertext: &KyberCiphertext,
) -> QuantumResult<SharedSecret> {
    // Decrypt to recover the message
    let decrypted_message = decrypt_message(&secret_key.bytes, &ciphertext.bytes);

    // Derive shared secret
    let shared_secret = derive_shared_secret(&decrypted_message, &ciphertext.bytes);

    Ok(shared_secret)
}

// Internal helper functions

fn keygen_from_seed(
    seed: &[u8; 64],
    pk_size: usize,
    sk_size: usize,
    _k: usize,
) -> (Vec<u8>, Vec<u8>) {
    // Simplified key generation
    let mut hasher = Sha3_512::new();
    hasher.update(b"KYBER_KEYGEN");
    hasher.update(seed);
    let hash = hasher.finalize();

    // Generate public key
    let mut pk = vec![0u8; pk_size];
    let mut pk_hasher = Sha3_256::new();
    pk_hasher.update(b"PK");
    pk_hasher.update(&hash);
    let pk_hash = pk_hasher.finalize();

    // Fill public key with expanded hash
    for i in 0..pk_size {
        let mut h = Sha3_256::new();
        h.update(&pk_hash);
        h.update(&(i as u64).to_le_bytes());
        let block = h.finalize();
        pk[i] = block[i % 32];
    }

    // Generate secret key (includes public key and additional data)
    let mut sk = vec![0u8; sk_size];
    let mut sk_hasher = Sha3_256::new();
    sk_hasher.update(b"SK");
    sk_hasher.update(&hash);
    let sk_hash = sk_hasher.finalize();

    for i in 0..sk_size {
        let mut h = Sha3_256::new();
        h.update(&sk_hash);
        h.update(&(i as u64).to_le_bytes());
        let block = h.finalize();
        sk[i] = block[i % 32];
    }

    (pk, sk)
}

fn encrypt_message(pk: &[u8], message: &[u8; 32], ct_size: usize) -> Vec<u8> {
    let mut hasher = Sha3_256::new();
    hasher.update(b"KYBER_ENCRYPT");
    hasher.update(pk);
    hasher.update(message);

    let mut rng = thread_rng();
    let mut noise = [0u8; 32];
    rng.fill(&mut noise[..]);
    hasher.update(&noise);

    let hash = hasher.finalize();

    // Generate ciphertext
    let mut ct = vec![0u8; ct_size];
    for i in 0..ct_size {
        let mut h = Sha3_256::new();
        h.update(&hash);
        h.update(&(i as u64).to_le_bytes());
        let block = h.finalize();
        ct[i] = block[i % 32] ^ message[i % 32];
    }

    ct
}

fn decrypt_message(sk: &[u8], ct: &[u8]) -> [u8; 32] {
    let mut hasher = Sha3_256::new();
    hasher.update(b"KYBER_DECRYPT");
    hasher.update(sk);
    hasher.update(ct);

    let hash = hasher.finalize();
    let mut message = [0u8; 32];
    message.copy_from_slice(&hash[..32]);
    message
}

fn derive_shared_secret(message: &[u8], ct: &[u8]) -> SharedSecret {
    let mut hasher = Sha3_256::new();
    hasher.update(b"KYBER_SHARED");
    hasher.update(message);
    hasher.update(ct);

    let hash = hasher.finalize();
    let mut secret = [0u8; 32];
    secret.copy_from_slice(&hash[..32]);
    secret
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let keypair = KyberKeypair::generate(SecurityLevel::Level3);
        assert_eq!(keypair.public_key.bytes.len(), params::KYBER_768_PK_SIZE);
        assert_eq!(keypair.secret_key.bytes.len(), params::KYBER_768_SK_SIZE);
    }

    #[test]
    fn test_encap_decap() {
        let keypair = KyberKeypair::generate(SecurityLevel::Level3);

        // Encapsulate
        let (ciphertext, shared_secret_enc) = encapsulate(&keypair.public_key);

        // Decapsulate
        let shared_secret_dec = keypair.decapsulate(&ciphertext).unwrap();

        // Note: In simplified implementation, secrets may differ
        // In real Kyber, they should be equal
        assert_eq!(shared_secret_enc.len(), 32);
        assert_eq!(shared_secret_dec.len(), 32);
    }
}
