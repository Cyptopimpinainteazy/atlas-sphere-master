//! Execution Receipt Generation with Ed25519 Signing

use crate::executor::ExecutionResult;
use crate::state::StateManager;
use anyhow;
use blake2::{Blake2s256, Digest};
use ed25519_dalek::{Signature, Signer, SigningKey};
use serde::{Deserialize, Serialize};

/// Execution receipt
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionReceipt {
    /// Job ID
    pub job_id: [u8; 32],
    /// Input hash (Blake2s256)
    pub input_hash: [u8; 32],
    /// Output hash
    pub output_hash: [u8; 32],
    /// Pre-state root
    pub pre_state_root: [u8; 32],
    /// Post-state root
    pub post_state_root: [u8; 32],
    /// Gas used
    pub gas_used: u64,
    /// Success flag
    pub success: bool,
    /// Executor public key
    pub executor_pubkey: [u8; 32],
    /// Signature (Ed25519) - stored as hex string for serde compatibility
    #[serde(with = "signature_hex")]
    pub signature: [u8; 64],
    /// Timestamp
    pub timestamp: u64,
    /// Log hashes
    pub log_hashes: Vec<[u8; 32]>,
}

/// Serde helper for [u8; 64]
mod signature_hex {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(data: &[u8; 64], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        hex::encode(data).serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 64], D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let bytes = hex::decode(&s).map_err(serde::de::Error::custom)?;
        if bytes.len() != 64 {
            return Err(serde::de::Error::custom("expected 64 bytes for signature"));
        }
        let mut arr = [0u8; 64];
        arr.copy_from_slice(&bytes);
        Ok(arr)
    }
}

impl ExecutionReceipt {
    /// Encode receipt for hashing/signing
    pub fn encode_for_signing(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(256);

        data.extend_from_slice(&self.job_id);
        data.extend_from_slice(&self.input_hash);
        data.extend_from_slice(&self.output_hash);
        data.extend_from_slice(&self.pre_state_root);
        data.extend_from_slice(&self.post_state_root);
        data.extend_from_slice(&self.gas_used.to_le_bytes());
        data.push(if self.success { 1 } else { 0 });
        data.extend_from_slice(&self.executor_pubkey);
        data.extend_from_slice(&self.timestamp.to_le_bytes());

        for log_hash in &self.log_hashes {
            data.extend_from_slice(log_hash);
        }

        data
    }

    /// Compute receipt hash
    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = Blake2s256::new();
        hasher.update(self.encode_for_signing());
        hasher.finalize().into()
    }

    /// Encode receipt for RPC submission
    pub fn encode(&self) -> Vec<u8> {
        let mut data = self.encode_for_signing();
        data.extend_from_slice(&self.signature);
        data
    }
}

/// Receipt generator
pub struct ReceiptGenerator {
    signing_key: SigningKey,
    pubkey: [u8; 32],
}

impl ReceiptGenerator {
    /// Create new receipt generator from private key bytes
    pub fn new(private_key: &[u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(private_key);
        let pubkey = signing_key.verifying_key().to_bytes();

        Self {
            signing_key,
            pubkey,
        }
    }

    /// Create from hex-encoded private key string
    pub fn from_hex(hex_key: &str) -> anyhow::Result<Self> {
        let bytes = hex::decode(hex_key.trim_start_matches("0x"))?;
        if bytes.len() != 32 {
            return Err(anyhow::anyhow!("Invalid key length: expected 32 bytes"));
        }
        let mut key = [0u8; 32];
        key.copy_from_slice(&bytes);
        Ok(Self::new(&key))
    }

    /// Generate receipt from execution result
    pub fn generate(
        &self,
        job_id: [u8; 32],
        input: &[u8],
        result: &ExecutionResult,
        pre_state: &StateManager,
        post_state: &StateManager,
    ) -> ExecutionReceipt {
        // Hash input
        let input_hash: [u8; 32] = {
            let mut hasher = Blake2s256::new();
            hasher.update(input);
            hasher.finalize().into()
        };

        // Hash output
        let output_hash: [u8; 32] = {
            let mut hasher = Blake2s256::new();
            hasher.update(&result.return_data);
            hasher.finalize().into()
        };

        // Hash logs
        let log_hashes: Vec<[u8; 32]> = result
            .logs
            .iter()
            .map(|log| {
                let mut hasher = Blake2s256::new();
                for topic in &log.topics {
                    hasher.update(topic);
                }
                hasher.update(&log.data);
                hasher.finalize().into()
            })
            .collect();

        // Create receipt (unsigned)
        let mut receipt = ExecutionReceipt {
            job_id,
            input_hash,
            output_hash,
            pre_state_root: pre_state.root(),
            post_state_root: post_state.root(),
            gas_used: result.gas_used,
            success: result.success,
            executor_pubkey: self.pubkey,
            signature: [0u8; 64],
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            log_hashes,
        };

        // Sign receipt
        let message = receipt.encode_for_signing();
        let signature = self.signing_key.sign(&message);
        receipt.signature = signature.to_bytes();

        receipt
    }

    /// Get executor public key
    pub fn pubkey(&self) -> [u8; 32] {
        self.pubkey
    }
}

/// Receipt verifier for on-chain verification
pub struct ReceiptVerifier;

impl ReceiptVerifier {
    /// Verify receipt signature
    pub fn verify(receipt: &ExecutionReceipt) -> bool {
        use ed25519_dalek::VerifyingKey;

        let Ok(verifying_key) = VerifyingKey::from_bytes(&receipt.executor_pubkey) else {
            return false;
        };

        let signature = Signature::from_bytes(&receipt.signature);
        let message = receipt.encode_for_signing();
        verifying_key.verify_strict(&message, &signature).is_ok()
    }
}

/// Receipt batch for multi-receipt submission
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReceiptBatch {
    pub receipts: Vec<ExecutionReceipt>,
    pub batch_root: [u8; 32],
}

impl ReceiptBatch {
    /// Create a new batch from receipts
    pub fn new(receipts: Vec<ExecutionReceipt>) -> Self {
        // Compute batch root (Merkle root of receipt hashes)
        let hashes: Vec<[u8; 32]> = receipts.iter().map(|r| r.hash()).collect();
        let batch_root = Self::merkle_root(&hashes);

        Self {
            receipts,
            batch_root,
        }
    }

    fn merkle_root(hashes: &[[u8; 32]]) -> [u8; 32] {
        if hashes.is_empty() {
            return [0u8; 32];
        }

        let mut current = hashes.to_vec();

        while current.len() > 1 {
            let mut next = Vec::new();
            for chunk in current.chunks(2) {
                let mut hasher = Blake2s256::new();
                hasher.update(chunk[0]);
                if chunk.len() > 1 {
                    hasher.update(chunk[1]);
                } else {
                    hasher.update(chunk[0]);
                }
                next.push(hasher.finalize().into());
            }
            current = next;
        }

        current[0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::ExecutionLog;

    #[test]
    fn test_receipt_generation_and_verification() {
        let private_key = [1u8; 32];
        let generator = ReceiptGenerator::new(&private_key);

        let job_id = [2u8; 32];
        let input = b"test input";
        let result = ExecutionResult {
            success: true,
            gas_used: 100,
            return_data: b"output".to_vec(),
            logs: vec![ExecutionLog {
                topics: vec![[3u8; 32]],
                data: b"log data".to_vec(),
            }],
            error: None,
        };

        let pre_state = StateManager::new();
        let mut post_state = StateManager::new();
        post_state.set(b"key", b"value");

        let receipt = generator.generate(job_id, input, &result, &pre_state, &post_state);

        // Verify signature
        assert!(ReceiptVerifier::verify(&receipt));
        assert_eq!(receipt.success, true);
        assert_eq!(receipt.gas_used, 100);
        assert_eq!(receipt.job_id, job_id);
    }

    #[test]
    fn test_tampered_receipt_fails_verification() {
        let private_key = [1u8; 32];
        let generator = ReceiptGenerator::new(&private_key);

        let mut receipt = generator.generate(
            [2u8; 32],
            b"input",
            &ExecutionResult {
                success: true,
                gas_used: 100,
                return_data: vec![],
                logs: vec![],
                error: None,
            },
            &StateManager::new(),
            &StateManager::new(),
        );

        // Tamper with receipt
        receipt.gas_used = 999;

        // Verification should fail
        assert!(!ReceiptVerifier::verify(&receipt));
    }

    #[test]
    fn test_receipt_batch() {
        let private_key = [1u8; 32];
        let generator = ReceiptGenerator::new(&private_key);

        let receipts: Vec<_> = (0..3)
            .map(|i| {
                generator.generate(
                    [i as u8; 32],
                    b"input",
                    &ExecutionResult {
                        success: true,
                        gas_used: 100,
                        return_data: vec![],
                        logs: vec![],
                        error: None,
                    },
                    &StateManager::new(),
                    &StateManager::new(),
                )
            })
            .collect();

        let batch = ReceiptBatch::new(receipts);
        assert_eq!(batch.receipts.len(), 3);
        assert_ne!(batch.batch_root, [0u8; 32]);
    }
}
