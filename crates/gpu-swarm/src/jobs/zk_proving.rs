//! ZK Proving Job
//!
//! Generates zero-knowledge proofs for verifiable compute:
//! - Strategy execution proofs (prove PnL without revealing strategy)
//! - Cross-chain state proofs
//! - Batch verification aggregation
//! - Recursive proof composition

use crate::error::{SwarmError, SwarmResult};
use crate::jobs::{JobOutput, JobType, SwarmJob};
use crate::task::TaskPriority;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// ZK proving system type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProofSystem {
    /// Groth16 (trusted setup, constant size proofs)
    Groth16,
    /// PLONK (universal setup)
    Plonk,
    /// STARK (no trusted setup, larger proofs)
    Stark,
    /// Halo2 (recursive proofs)
    Halo2,
    /// Nova (folding scheme)
    Nova,
}

impl Default for ProofSystem {
    fn default() -> Self {
        Self::Plonk
    }
}

/// Configuration for ZK proving job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZkConfig {
    /// Proof system to use
    pub proof_system: ProofSystem,
    /// Circuit size (number of constraints)
    pub circuit_size: usize,
    /// Enable recursive composition
    pub recursive: bool,
    /// Batch size for aggregation
    pub batch_size: usize,
    /// Security parameter (bits)
    pub security_bits: u32,
    /// GPU acceleration enabled
    pub gpu_accelerated: bool,
    /// Memory limit (MB)
    pub memory_limit_mb: u32,
}

impl Default for ZkConfig {
    fn default() -> Self {
        Self {
            proof_system: ProofSystem::Plonk,
            circuit_size: 1 << 16, // 64K constraints
            recursive: false,
            batch_size: 1,
            security_bits: 128,
            gpu_accelerated: true,
            memory_limit_mb: 8192,
        }
    }
}

/// Type of proof to generate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProofType {
    /// Prove strategy execution produced claimed PnL
    StrategyExecution {
        strategy_hash: [u8; 32],
        claimed_pnl: f64,
        trade_count: usize,
    },
    /// Prove state transition on a chain
    StateTransition {
        chain_id: u64,
        pre_state_root: [u8; 32],
        post_state_root: [u8; 32],
        transactions: Vec<[u8; 32]>,
    },
    /// Aggregate multiple proofs into one
    Aggregation { proof_hashes: Vec<[u8; 32]> },
    /// Prove membership in Merkle tree
    MerkleMembership {
        root: [u8; 32],
        leaf: [u8; 32],
        path: Vec<[u8; 32]>,
    },
    /// Custom circuit proof
    Custom {
        circuit_id: String,
        public_inputs: Vec<u8>,
        private_inputs: Vec<u8>,
    },
}

/// Witness data for proof generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Witness {
    /// Public inputs (visible in proof)
    pub public_inputs: Vec<Vec<u8>>,
    /// Private inputs (hidden by proof)
    pub private_inputs: Vec<Vec<u8>>,
}

/// Generated ZK proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZkProof {
    /// Proof ID
    pub id: [u8; 32],
    /// Proof system used
    pub system: ProofSystem,
    /// Proof bytes
    pub proof_bytes: Vec<u8>,
    /// Public inputs
    pub public_inputs: Vec<Vec<u8>>,
    /// Proof size (bytes)
    pub proof_size: usize,
    /// Generation time (ms)
    pub generation_time_ms: u64,
    /// Verification time estimate (ms)
    pub verification_time_ms: u64,
    /// Is valid (self-verified)
    pub is_valid: bool,
}

/// Result from ZK proving job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZkProvingResult {
    /// Generated proofs
    pub proofs: Vec<ZkProof>,
    /// Total constraints proved
    pub total_constraints: usize,
    /// Total generation time (ms)
    pub total_time_ms: u64,
    /// Memory used (MB)
    pub memory_used_mb: u32,
    /// GPU utilized
    pub gpu_utilized: bool,
    /// Result hash for verification
    pub result_hash: [u8; 32],
}

/// ZK Proving Job
pub struct ZkProvingJob {
    pub config: ZkConfig,
    /// Proofs to generate
    pub proof_requests: Vec<ProofType>,
    /// Witness data for each proof
    pub witnesses: Vec<Witness>,
}

impl ZkProvingJob {
    pub fn new(config: ZkConfig) -> Self {
        Self {
            config,
            proof_requests: Vec::new(),
            witnesses: Vec::new(),
        }
    }

    pub fn with_proof_request(mut self, request: ProofType, witness: Witness) -> Self {
        self.proof_requests.push(request);
        self.witnesses.push(witness);
        self
    }

    /// Generate proof for a single request
    fn generate_proof(&self, request: &ProofType, witness: &Witness) -> SwarmResult<ZkProof> {
        use std::time::Instant;

        let start = Instant::now();

        // In production, this would call actual ZK libraries (bellman, halo2, etc.)
        // For now, generate mock proofs

        let proof_id = match request {
            ProofType::StrategyExecution { strategy_hash, .. } => *strategy_hash,
            ProofType::StateTransition {
                post_state_root, ..
            } => *post_state_root,
            ProofType::Aggregation { proof_hashes } => {
                let mut hasher = blake3::Hasher::new();
                for hash in proof_hashes {
                    hasher.update(hash);
                }
                hasher.finalize().into()
            }
            ProofType::MerkleMembership { root, .. } => *root,
            ProofType::Custom { circuit_id, .. } => blake3::hash(circuit_id.as_bytes()).into(),
        };

        // Mock proof generation based on system
        let (proof_bytes, proof_size) = match self.config.proof_system {
            ProofSystem::Groth16 => {
                // Groth16: ~200 bytes constant size
                let proof = vec![0u8; 192];
                (proof.clone(), proof.len())
            }
            ProofSystem::Plonk => {
                // PLONK: ~500-1000 bytes
                let proof = vec![0u8; 768];
                (proof.clone(), proof.len())
            }
            ProofSystem::Stark => {
                // STARK: larger, ~50-100KB
                let proof = vec![0u8; 65536];
                (proof.clone(), proof.len())
            }
            ProofSystem::Halo2 => {
                // Halo2: ~500 bytes
                let proof = vec![0u8; 512];
                (proof.clone(), proof.len())
            }
            ProofSystem::Nova => {
                // Nova: ~1KB accumulator
                let proof = vec![0u8; 1024];
                (proof.clone(), proof.len())
            }
        };

        let generation_time_ms = start.elapsed().as_millis() as u64;

        // Estimate verification time based on proof system
        let verification_time_ms = match self.config.proof_system {
            ProofSystem::Groth16 => 5, // Constant time
            ProofSystem::Plonk => 10,  // Linear in public inputs
            ProofSystem::Stark => 50,  // Logarithmic in circuit
            ProofSystem::Halo2 => 15,  // Moderate
            ProofSystem::Nova => 20,   // Accumulator check
        };

        Ok(ZkProof {
            id: proof_id,
            system: self.config.proof_system,
            proof_bytes,
            public_inputs: witness.public_inputs.clone(),
            proof_size,
            generation_time_ms,
            verification_time_ms,
            is_valid: true,
        })
    }

    /// Execute full ZK proving job
    fn run_proving(&self) -> SwarmResult<ZkProvingResult> {
        use std::time::Instant;

        let start = Instant::now();
        let mut proofs = Vec::new();
        let mut total_constraints = 0;

        for (request, witness) in self.proof_requests.iter().zip(self.witnesses.iter()) {
            let proof = self.generate_proof(request, witness)?;
            proofs.push(proof);
            total_constraints += self.config.circuit_size;
        }

        // Handle batching if configured
        if self.config.batch_size > 1 && proofs.len() > 1 {
            // In production, aggregate proofs
            // For now, just track that aggregation would happen
        }

        // Calculate result hash
        let mut hasher = blake3::Hasher::new();
        for proof in &proofs {
            hasher.update(&proof.id);
            hasher.update(&proof.proof_bytes);
        }
        let result_hash: [u8; 32] = hasher.finalize().into();

        Ok(ZkProvingResult {
            proofs,
            total_constraints,
            total_time_ms: start.elapsed().as_millis() as u64,
            memory_used_mb: (total_constraints / 1000) as u32,
            gpu_utilized: self.config.gpu_accelerated,
            result_hash,
        })
    }
}

impl SwarmJob for ZkProvingJob {
    fn job_type(&self) -> JobType {
        JobType::ZkProving
    }

    fn compute_units(&self) -> u64 {
        // ZK proving is expensive: ~1000 CU per million constraints
        let constraint_millions =
            (self.config.circuit_size * self.proof_requests.len()) / 1_000_000;
        (constraint_millions.max(1) * 1000) as u64
    }

    fn timeout(&self) -> Duration {
        // ZK proofs can take a while
        Duration::from_secs(600) // 10 minutes
    }

    fn execute(&self) -> SwarmResult<JobOutput> {
        let result = self.run_proving()?;
        Ok(JobOutput::ZkProving(result))
    }

    fn verify(&self, result: &JobOutput) -> SwarmResult<bool> {
        match result {
            JobOutput::ZkProving(zk_result) => {
                // Verify result hash
                let mut hasher = blake3::Hasher::new();
                for proof in &zk_result.proofs {
                    hasher.update(&proof.id);
                    hasher.update(&proof.proof_bytes);
                }
                let expected_hash: [u8; 32] = hasher.finalize().into();

                // Also check all proofs are self-verified
                let all_valid = zk_result.proofs.iter().all(|p| p.is_valid);

                Ok(expected_hash == zk_result.result_hash && all_valid)
            }
            _ => Err(SwarmError::InvalidResult("Wrong result type".into())),
        }
    }

    fn priority(&self) -> TaskPriority {
        TaskPriority::High
    }

    fn requires_gpu(&self) -> bool {
        self.config.gpu_accelerated
    }

    fn min_vram_mb(&self) -> u32 {
        // ZK proving needs significant VRAM
        4096 // 4GB minimum
    }
}

/// Builder for strategy execution proofs
pub struct StrategyProofBuilder {
    strategy_bytecode: Vec<u8>,
    trades: Vec<TradeRecord>,
    market_data: Vec<u8>,
}

/// Trade record for proof generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeRecord {
    pub timestamp: u64,
    pub action: String,
    pub asset: String,
    pub amount: f64,
    pub price: f64,
    pub pnl: f64,
}

impl StrategyProofBuilder {
    pub fn new(bytecode: Vec<u8>) -> Self {
        Self {
            strategy_bytecode: bytecode,
            trades: Vec::new(),
            market_data: Vec::new(),
        }
    }

    pub fn with_trades(mut self, trades: Vec<TradeRecord>) -> Self {
        self.trades = trades;
        self
    }

    pub fn build(self) -> (ProofType, Witness) {
        let strategy_hash: [u8; 32] = blake3::hash(&self.strategy_bytecode).into();
        let claimed_pnl: f64 = self.trades.iter().map(|t| t.pnl).sum();

        let proof_type = ProofType::StrategyExecution {
            strategy_hash,
            claimed_pnl,
            trade_count: self.trades.len(),
        };

        let witness = Witness {
            public_inputs: vec![strategy_hash.to_vec(), claimed_pnl.to_le_bytes().to_vec()],
            private_inputs: vec![
                self.strategy_bytecode,
                bincode::serialize(&self.trades).unwrap_or_default(),
            ],
        };

        (proof_type, witness)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zk_config_default() {
        let config = ZkConfig::default();
        assert_eq!(config.proof_system, ProofSystem::Plonk);
        assert_eq!(config.security_bits, 128);
    }

    #[test]
    fn test_strategy_proof_builder() {
        let bytecode = vec![0x20, 0x64, 0x00, 0x00, 0x30, 0x01];
        let trades = vec![
            TradeRecord {
                timestamp: 1000,
                action: "buy".to_string(),
                asset: "ETH".to_string(),
                amount: 1.0,
                price: 2000.0,
                pnl: 0.0,
            },
            TradeRecord {
                timestamp: 2000,
                action: "sell".to_string(),
                asset: "ETH".to_string(),
                amount: 1.0,
                price: 2100.0,
                pnl: 100.0,
            },
        ];

        let (proof_type, witness) = StrategyProofBuilder::new(bytecode)
            .with_trades(trades)
            .build();

        if let ProofType::StrategyExecution { claimed_pnl, .. } = proof_type {
            assert_eq!(claimed_pnl, 100.0);
        }

        assert_eq!(witness.public_inputs.len(), 2);
    }

    #[test]
    fn test_zk_proof_generation() {
        let config = ZkConfig::default();
        let (proof_type, witness) = StrategyProofBuilder::new(vec![0x20, 0x64])
            .with_trades(vec![])
            .build();

        let job = ZkProvingJob::new(config).with_proof_request(proof_type, witness);

        let result = job.execute().unwrap();

        if let JobOutput::ZkProving(zk_result) = result {
            assert_eq!(zk_result.proofs.len(), 1);
            assert!(zk_result.proofs[0].is_valid);
        }
    }

    #[test]
    fn test_proof_systems_sizes() {
        for system in [
            ProofSystem::Groth16,
            ProofSystem::Plonk,
            ProofSystem::Stark,
            ProofSystem::Halo2,
            ProofSystem::Nova,
        ] {
            let config = ZkConfig {
                proof_system: system,
                ..Default::default()
            };

            let job = ZkProvingJob::new(config).with_proof_request(
                ProofType::MerkleMembership {
                    root: [0u8; 32],
                    leaf: [1u8; 32],
                    path: vec![[2u8; 32]],
                },
                Witness {
                    public_inputs: vec![vec![0u8; 32]],
                    private_inputs: vec![],
                },
            );

            let result = job.execute().unwrap();
            if let JobOutput::ZkProving(zk_result) = result {
                assert!(zk_result.proofs[0].proof_size > 0);
            }
        }
    }
}
