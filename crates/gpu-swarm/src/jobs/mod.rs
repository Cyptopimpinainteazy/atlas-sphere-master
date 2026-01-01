//! GPU Swarm Job Executors
//!
//! Specialized job types for distributed GPU compute:
//! - X3 Strategy Simulation
//! - MEV Discovery
//! - ZK Proof Generation
//! - ML Model Training
//! - Mempool Analysis
//! - Chain Indexing
//! - Funding Campaigns (Prophet-timed outreach)

pub mod chain_indexing;
pub mod funding_campaign;
pub mod mempool_analysis;
pub mod mev_discovery;
pub mod model_training;
pub mod x3_simulation;
pub mod zk_proving;

pub use chain_indexing::{ChainIndexingJob, ChainIndexingResult, IndexingConfig};
pub use funding_campaign::{
    CampaignType, FundingCampaignConfig, FundingCampaignJob, FundingCampaignResult,
    GeneratedContent, LlmEngine, PersonalizationLevel, Prospect,
};
pub use mempool_analysis::{MempoolAnalysisJob, MempoolAnalysisResult, MempoolConfig, PendingTx};
pub use mev_discovery::{MevConfig, MevDiscoveryJob, MevDiscoveryResult, MevOpportunity, MevType};
pub use model_training::{ModelTrainingJob, ModelType, TrainingConfig, TrainingResult};
pub use x3_simulation::{SimConfig, SimulationResult, StrategyCandidate, X3SimulationJob};
pub use zk_proving::{ProofType, ZkConfig, ZkProof, ZkProvingJob, ZkProvingResult};

use crate::error::SwarmResult;
use crate::task::{TaskId, TaskPriority};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Common trait for all GPU swarm jobs
pub trait SwarmJob: Send + Sync {
    /// Job type identifier
    fn job_type(&self) -> JobType;

    /// Estimated compute units required
    fn compute_units(&self) -> u64;

    /// Maximum execution time
    fn timeout(&self) -> Duration;

    /// Execute the job and return result
    fn execute(&self) -> SwarmResult<JobOutput>;

    /// Verify the job result
    fn verify(&self, result: &JobOutput) -> SwarmResult<bool>;

    /// Get priority for scheduling
    fn priority(&self) -> TaskPriority {
        TaskPriority::Normal
    }

    /// Whether this job requires GPU acceleration
    fn requires_gpu(&self) -> bool {
        true
    }

    /// Minimum VRAM required (in MB)
    fn min_vram_mb(&self) -> u32 {
        512
    }
}

/// Type of GPU swarm job
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum JobType {
    /// X3 strategy simulation and evolution
    X3Simulation,
    /// MEV opportunity discovery
    MevDiscovery,
    /// Zero-knowledge proof generation
    ZkProving,
    /// ML model training
    ModelTraining,
    /// Mempool analysis
    MempoolAnalysis,
    /// Chain indexing
    ChainIndexing,
    /// Funding campaign (VC outreach, social, grants)
    FundingCampaign,
}

impl JobType {
    /// Get base reward multiplier for this job type
    pub fn reward_multiplier(&self) -> f64 {
        match self {
            JobType::X3Simulation => 1.0,
            JobType::MevDiscovery => 2.0, // Higher value
            JobType::ZkProving => 3.0,    // Most compute-intensive
            JobType::ModelTraining => 1.5,
            JobType::MempoolAnalysis => 0.8,
            JobType::ChainIndexing => 0.5,
            JobType::FundingCampaign => 0.3, // Low compute, high strategic value
        }
    }

    /// Get default timeout for this job type
    pub fn default_timeout(&self) -> Duration {
        match self {
            JobType::X3Simulation => Duration::from_secs(60),
            JobType::MevDiscovery => Duration::from_secs(30),
            JobType::ZkProving => Duration::from_secs(300),
            JobType::ModelTraining => Duration::from_secs(600),
            JobType::MempoolAnalysis => Duration::from_secs(15),
            JobType::ChainIndexing => Duration::from_secs(120),
            JobType::FundingCampaign => Duration::from_secs(120),
        }
    }
}

/// Output from job execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobOutput {
    X3Simulation(SimulationResult),
    MevDiscovery(MevDiscoveryResult),
    ZkProving(ZkProvingResult),
    ModelTraining(TrainingResult),
    MempoolAnalysis(MempoolAnalysisResult),
    ChainIndexing(ChainIndexingResult),
    FundingCampaign(FundingCampaignResult),
}

impl JobOutput {
    /// Get the job type for this output
    pub fn job_type(&self) -> JobType {
        match self {
            JobOutput::X3Simulation(_) => JobType::X3Simulation,
            JobOutput::MevDiscovery(_) => JobType::MevDiscovery,
            JobOutput::ZkProving(_) => JobType::ZkProving,
            JobOutput::ModelTraining(_) => JobType::ModelTraining,
            JobOutput::MempoolAnalysis(_) => JobType::MempoolAnalysis,
            JobOutput::ChainIndexing(_) => JobType::ChainIndexing,
            JobOutput::FundingCampaign(_) => JobType::FundingCampaign,
        }
    }

    /// Get compute units consumed
    pub fn compute_units(&self) -> u64 {
        match self {
            JobOutput::X3Simulation(r) => r.compute_units,
            JobOutput::MevDiscovery(_) => 0, // Calculated dynamically
            JobOutput::ZkProving(_) => 0,
            JobOutput::ModelTraining(_) => 0,
            JobOutput::MempoolAnalysis(_) => 0,
            JobOutput::ChainIndexing(_) => 0,
            JobOutput::FundingCampaign(r) => r.compute_units,
        }
    }
}

/// Job submission with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSubmission {
    /// Unique job ID
    pub job_id: TaskId,
    /// Job type
    pub job_type: JobType,
    /// Serialized job payload
    pub payload: Vec<u8>,
    /// Submitter account
    pub submitter: [u8; 32],
    /// Maximum fee willing to pay
    pub max_fee: u64,
    /// Submission timestamp
    pub submitted_at: u64,
    /// Required result count (for redundancy)
    pub redundancy: u8,
}

/// Job completion receipt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobReceipt {
    /// Job ID
    pub job_id: TaskId,
    /// Executor node ID
    pub executor: [u8; 32],
    /// Execution result hash
    pub result_hash: [u8; 32],
    /// Compute units consumed
    pub compute_units: u64,
    /// Execution duration (ms)
    pub duration_ms: u64,
    /// Fee charged
    pub fee: u64,
    /// Executor signature (Ed25519)
    #[serde(with = "serde_bytes")]
    pub signature: Vec<u8>,
}

/// Job queue statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JobQueueStats {
    pub pending_jobs: usize,
    pub running_jobs: usize,
    pub completed_jobs: usize,
    pub failed_jobs: usize,
    pub total_compute_units: u64,
    pub avg_wait_time_ms: u64,
    pub avg_execution_time_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_type_rewards() {
        assert!(JobType::ZkProving.reward_multiplier() > JobType::X3Simulation.reward_multiplier());
        assert!(
            JobType::MevDiscovery.reward_multiplier() > JobType::ChainIndexing.reward_multiplier()
        );
    }

    #[test]
    fn test_job_type_timeouts() {
        assert!(JobType::ZkProving.default_timeout() > JobType::MempoolAnalysis.default_timeout());
    }
}
