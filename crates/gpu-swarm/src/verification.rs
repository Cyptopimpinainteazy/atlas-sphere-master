//! Verification system for GPU execution proofs

use crate::error::{SwarmError, SwarmResult};
use crate::node::NodeId;
use crate::protocol::{ExecutionProof, TaskResult, VerificationResult};
use crate::task::{Task, TaskId};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Verification threshold configuration
#[derive(Debug, Clone)]
pub struct VerificationConfig {
    /// Minimum verifications required
    pub min_verifications: u8,

    /// Consensus threshold (percentage)
    pub consensus_threshold: u8,

    /// Maximum time for verification (seconds)
    pub verification_timeout: u64,

    /// Allow partial verification for non-critical tasks
    pub allow_partial: bool,

    /// Re-execution sampling rate (1 in N)
    pub reexecution_rate: u32,
}

impl Default for VerificationConfig {
    fn default() -> Self {
        Self {
            min_verifications: 2,
            consensus_threshold: 66,
            verification_timeout: 60,
            allow_partial: false,
            reexecution_rate: 10,
        }
    }
}

/// Verification request tracking
#[derive(Debug, Clone)]
struct PendingVerification {
    task: Task,
    result: TaskResult,
    verifiers: Vec<NodeId>,
    responses: HashMap<NodeId, VerificationResult>,
    started_at: i64,
}

/// Verification verdict
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Verdict {
    /// Result is valid
    Valid,
    /// Result is invalid
    Invalid,
    /// Inconclusive (not enough verifications)
    Inconclusive,
    /// Timed out waiting for verifications
    TimedOut,
}

/// Summary of verification process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationSummary {
    /// Task ID
    pub task_id: TaskId,

    /// Final verdict
    pub verdict: Verdict,

    /// Number of valid votes
    pub valid_votes: u8,

    /// Number of invalid votes
    pub invalid_votes: u8,

    /// Consensus result hash (if valid)
    pub consensus_hash: Option<[u8; 32]>,

    /// Nodes that voted valid
    pub valid_voters: Vec<NodeId>,

    /// Nodes that voted invalid
    pub invalid_voters: Vec<NodeId>,

    /// Verification duration (seconds)
    pub duration: i64,
}

/// Verifier for GPU execution results
pub struct ExecutionVerifier {
    /// Configuration
    config: VerificationConfig,

    /// Pending verifications
    pending: HashMap<TaskId, PendingVerification>,

    /// Completed verifications
    completed: HashMap<TaskId, VerificationSummary>,
}

impl ExecutionVerifier {
    /// Create a new verifier
    pub fn new(config: VerificationConfig) -> Self {
        Self {
            config,
            pending: HashMap::new(),
            completed: HashMap::new(),
        }
    }

    /// Start verification for a task result
    pub fn start_verification(
        &mut self,
        task: Task,
        result: TaskResult,
        verifiers: Vec<NodeId>,
    ) -> SwarmResult<()> {
        if verifiers.len() < self.config.min_verifications as usize {
            return Err(SwarmError::InsufficientVerifiers {
                required: self.config.min_verifications,
                available: verifiers.len() as u8,
            });
        }

        let task_id = task.id;

        self.pending.insert(
            task_id,
            PendingVerification {
                task,
                result,
                verifiers,
                responses: HashMap::new(),
                started_at: chrono::Utc::now().timestamp(),
            },
        );

        Ok(())
    }

    /// Submit a verification response
    pub fn submit_verification(
        &mut self,
        task_id: TaskId,
        response: VerificationResult,
    ) -> SwarmResult<Option<VerificationSummary>> {
        let pending = self
            .pending
            .get_mut(&task_id)
            .ok_or(SwarmError::TaskNotFound(task_id))?;

        // Verify the response is from an authorized verifier
        if !pending.verifiers.contains(&response.verifier) {
            return Err(SwarmError::UnauthorizedVerifier(response.verifier));
        }

        // Store the response
        pending.responses.insert(response.verifier, response);

        // Check if we have enough responses
        if pending.responses.len() >= self.config.min_verifications as usize {
            return self.finalize_verification(task_id);
        }

        Ok(None)
    }

    /// Finalize verification and produce verdict
    fn finalize_verification(
        &mut self,
        task_id: TaskId,
    ) -> SwarmResult<Option<VerificationSummary>> {
        let pending = self
            .pending
            .remove(&task_id)
            .ok_or(SwarmError::TaskNotFound(task_id))?;

        let mut valid_voters = Vec::new();
        let mut invalid_voters = Vec::new();
        let mut hash_votes: HashMap<[u8; 32], u8> = HashMap::new();

        for (node_id, response) in &pending.responses {
            if response.valid {
                valid_voters.push(*node_id);
                *hash_votes.entry(response.computed_hash).or_insert(0) += 1;
            } else {
                invalid_voters.push(*node_id);
            }
        }

        let total_votes = pending.responses.len() as u8;
        let valid_votes = valid_voters.len() as u8;
        let invalid_votes = invalid_voters.len() as u8;

        // Calculate consensus
        let valid_percentage = (valid_votes as u16 * 100) / total_votes as u16;
        let consensus_reached = valid_percentage >= self.config.consensus_threshold as u16;

        // Find consensus hash
        let consensus_hash = if consensus_reached {
            hash_votes
                .into_iter()
                .max_by_key(|(_, count)| *count)
                .filter(|(_, count)| {
                    let hash_percentage = (*count as u16 * 100) / valid_votes as u16;
                    hash_percentage >= self.config.consensus_threshold as u16
                })
                .map(|(hash, _)| hash)
        } else {
            None
        };

        let verdict = if consensus_reached && consensus_hash.is_some() {
            // Verify consensus hash matches original result
            if consensus_hash == Some(pending.result.result_hash) {
                Verdict::Valid
            } else {
                Verdict::Invalid
            }
        } else if !self.config.allow_partial {
            Verdict::Invalid
        } else if valid_votes > invalid_votes {
            Verdict::Inconclusive
        } else {
            Verdict::Invalid
        };

        let now = chrono::Utc::now().timestamp();
        let summary = VerificationSummary {
            task_id,
            verdict,
            valid_votes,
            invalid_votes,
            consensus_hash,
            valid_voters,
            invalid_voters,
            duration: now - pending.started_at,
        };

        self.completed.insert(task_id, summary.clone());

        Ok(Some(summary))
    }

    /// Check for timed out verifications
    pub fn check_timeouts(&mut self) -> Vec<VerificationSummary> {
        let now = chrono::Utc::now().timestamp();
        let timeout = self.config.verification_timeout as i64;

        let timed_out: Vec<_> = self
            .pending
            .iter()
            .filter(|(_, p)| now - p.started_at > timeout)
            .map(|(id, _)| *id)
            .collect();

        let mut summaries = Vec::new();

        for task_id in timed_out {
            if let Some(pending) = self.pending.remove(&task_id) {
                let summary = VerificationSummary {
                    task_id,
                    verdict: Verdict::TimedOut,
                    valid_votes: pending.responses.values().filter(|r| r.valid).count() as u8,
                    invalid_votes: pending.responses.values().filter(|r| !r.valid).count() as u8,
                    consensus_hash: None,
                    valid_voters: pending
                        .responses
                        .iter()
                        .filter(|(_, r)| r.valid)
                        .map(|(n, _)| *n)
                        .collect(),
                    invalid_voters: pending
                        .responses
                        .iter()
                        .filter(|(_, r)| !r.valid)
                        .map(|(n, _)| *n)
                        .collect(),
                    duration: now - pending.started_at,
                };

                self.completed.insert(task_id, summary.clone());
                summaries.push(summary);
            }
        }

        summaries
    }

    /// Get verification summary
    pub fn get_summary(&self, task_id: TaskId) -> Option<&VerificationSummary> {
        self.completed.get(&task_id)
    }

    /// Get pending verification status
    pub fn pending_status(&self, task_id: TaskId) -> Option<PendingVerificationStatus> {
        self.pending
            .get(&task_id)
            .map(|p| PendingVerificationStatus {
                task_id,
                total_verifiers: p.verifiers.len(),
                responses_received: p.responses.len(),
                started_at: p.started_at,
            })
    }
}

/// Status of a pending verification
#[derive(Debug, Clone)]
pub struct PendingVerificationStatus {
    pub task_id: TaskId,
    pub total_verifiers: usize,
    pub responses_received: usize,
    pub started_at: i64,
}

/// Utility functions for proof verification
pub mod utils {
    use super::*;

    /// Hash data using SHA-256
    pub fn hash_data(data: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize().into()
    }

    /// Verify execution proof structure
    pub fn verify_proof_structure(proof: &ExecutionProof) -> bool {
        proof.is_valid_structure()
    }

    /// Compare two results for equality
    pub fn results_match(a: &[u8], b: &[u8]) -> bool {
        if a.len() != b.len() {
            return false;
        }

        // Constant-time comparison to prevent timing attacks
        let mut diff = 0u8;
        for (x, y) in a.iter().zip(b.iter()) {
            diff |= x ^ y;
        }
        diff == 0
    }

    /// Generate verification challenge for a task
    pub fn generate_challenge(task: &Task, nonce: &[u8; 16]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(task.id.as_bytes());
        hasher.update(nonce);
        hasher.update(&task.created_at.to_le_bytes());
        hasher.finalize().into()
    }

    /// Verify a node's signature (placeholder - real impl would use ed25519)
    pub fn verify_signature(
        _public_key: &[u8; 32],
        _message: &[u8],
        _signature: &[u8; 64],
    ) -> bool {
        // TODO: Implement ed25519 verification
        true
    }
}

/// Deterministic re-execution verifier
pub struct ReexecutionVerifier {
    /// Configuration
    config: VerificationConfig,

    /// Tasks to re-execute
    pending_reexecution: Vec<(TaskId, Task, TaskResult)>,
}

impl ReexecutionVerifier {
    /// Create a new re-execution verifier
    pub fn new(config: VerificationConfig) -> Self {
        Self {
            config,
            pending_reexecution: Vec::new(),
        }
    }

    /// Queue a task for re-execution verification
    pub fn queue_for_reexecution(&mut self, task: Task, result: TaskResult) {
        // Randomly sample based on reexecution rate
        let should_reexecute = {
            let mut bytes = [0u8; 4];
            getrandom::getrandom(&mut bytes).unwrap_or_default();
            let random = u32::from_le_bytes(bytes);
            random % self.config.reexecution_rate == 0
        };

        if should_reexecute {
            self.pending_reexecution.push((task.id, task, result));
        }
    }

    /// Get next task to re-execute
    pub fn next_reexecution(&mut self) -> Option<(Task, TaskResult)> {
        self.pending_reexecution
            .pop()
            .map(|(_, task, result)| (task, result))
    }

    /// Verify re-execution result matches original
    pub fn verify_reexecution(&self, original: &TaskResult, reexecuted: &TaskResult) -> bool {
        // Compare result hashes
        utils::results_match(&original.result_hash, &reexecuted.result_hash)
    }
}
