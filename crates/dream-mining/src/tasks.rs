//! Task definitions for Dream Mining

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// A task to be executed during Dream Mining
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DreamTask {
    /// Unique task identifier
    pub id: Uuid,

    /// Task type and parameters
    pub task_type: TaskType,

    /// Task priority (higher = more important)
    pub priority: TaskPriority,

    /// Estimated duration in seconds
    pub estimated_duration_secs: u64,

    /// Task status
    pub status: TaskStatus,

    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Last update timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,

    /// Progress (0.0 - 1.0)
    pub progress: f64,

    /// Number of retry attempts
    pub retries: u32,

    /// Maximum retries allowed
    pub max_retries: u32,
}

impl DreamTask {
    /// Create a new task
    pub fn new(task_type: TaskType, priority: TaskPriority) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4(),
            estimated_duration_secs: task_type.estimated_duration(),
            task_type,
            priority,
            status: TaskStatus::Pending,
            created_at: now,
            updated_at: now,
            progress: 0.0,
            retries: 0,
            max_retries: 3,
        }
    }
}

/// Task types with their specific parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskType {
    /// Train AI models for Evolution Core
    ModelTraining(ModelTrainingTask),

    /// Pre-compute optimal swap routes
    RouteOptimization(RouteOptimizationTask),

    /// Generate zero-knowledge proofs in batch
    ZkProofGeneration(ZkProofTask),

    /// Build search indexes
    IndexBuilding(IndexBuildTask),

    /// Analyze blockchain network
    NetworkAnalysis(NetworkAnalysisTask),
}

impl TaskType {
    /// Get estimated duration for this task type
    pub fn estimated_duration(&self) -> u64 {
        match self {
            TaskType::ModelTraining(t) => t.epochs as u64 * 60, // 1 min per epoch
            TaskType::RouteOptimization(t) => t.pairs.len() as u64 * 5,
            TaskType::ZkProofGeneration(t) => t.proof_count as u64 * 30,
            TaskType::IndexBuilding(_) => 120,
            TaskType::NetworkAnalysis(t) => t.chains.len() as u64 * 60,
        }
    }
}

/// Model training task parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelTrainingTask {
    /// Name of the model to train
    pub model_name: String,

    /// Training data path or identifier
    pub training_data: String,

    /// Number of training epochs
    pub epochs: u32,

    /// Batch size
    pub batch_size: u32,

    /// Learning rate
    pub learning_rate: f32,

    /// Model checkpoint path
    pub checkpoint_path: Option<String>,
}

/// Route optimization task parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteOptimizationTask {
    /// Token pairs to optimize routes for
    pub pairs: Vec<TokenPair>,

    /// Maximum hops in route
    pub max_hops: u8,

    /// Minimum liquidity threshold
    pub min_liquidity: u128,

    /// Chains to include
    pub chains: Vec<String>,
}

/// Token pair for route optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    pub token_in: String,
    pub token_out: String,
}

/// ZK proof generation task parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZkProofTask {
    /// Number of proofs to generate
    pub proof_count: u32,

    /// Proof type
    pub proof_type: ZkProofType,

    /// Circuit identifier
    pub circuit_id: String,

    /// Public inputs
    pub public_inputs: Vec<Vec<u8>>,
}

/// Types of ZK proofs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ZkProofType {
    /// Groth16 proof
    Groth16,
    /// PLONK proof
    Plonk,
    /// STARK proof
    Stark,
    /// Bulletproofs
    Bulletproofs,
}

/// Index building task parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexBuildTask {
    /// Name of the index to build
    pub index_name: String,

    /// Data source
    pub data_source: String,

    /// Fields to index
    pub fields: Vec<String>,

    /// Whether to rebuild from scratch
    pub rebuild: bool,
}

/// Network analysis task parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkAnalysisTask {
    /// Chains to analyze
    pub chains: Vec<String>,

    /// Analysis types to perform
    pub analysis_types: Vec<AnalysisType>,

    /// Time range for analysis (hours)
    pub time_range_hours: u32,
}

/// Types of network analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnalysisType {
    /// Analyze liquidity distribution
    LiquidityDistribution,
    /// Analyze trading patterns
    TradingPatterns,
    /// Analyze MEV activity
    MevActivity,
    /// Analyze bridge activity
    BridgeActivity,
    /// Analyze gas prices
    GasPrices,
}

/// Task priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TaskPriority {
    /// Low priority - execute when nothing else available
    Low = 1,
    /// Normal priority - standard execution order
    Normal = 2,
    /// High priority - execute before normal tasks
    High = 3,
    /// Critical priority - execute immediately
    Critical = 4,
}

/// Task execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Task is queued and waiting
    Pending,
    /// Task is currently executing
    Running,
    /// Task completed successfully
    Completed,
    /// Task failed
    Failed(String),
    /// Task was cancelled
    Cancelled,
    /// Task was paused (can be resumed)
    Paused,
}

/// Result of task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskResult {
    /// Task completed successfully
    Completed {
        output: String,
        metrics: HashMap<String, f64>,
    },

    /// Task was paused
    Paused { progress: f64 },

    /// Task failed
    Failed { error: String, recoverable: bool },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task = DreamTask::new(
            TaskType::ModelTraining(ModelTrainingTask {
                model_name: "test_model".to_string(),
                training_data: "data/train".to_string(),
                epochs: 10,
                batch_size: 32,
                learning_rate: 0.001,
                checkpoint_path: None,
            }),
            TaskPriority::Normal,
        );

        assert_eq!(task.status, TaskStatus::Pending);
        assert_eq!(task.progress, 0.0);
    }

    #[test]
    fn test_priority_ordering() {
        assert!(TaskPriority::Critical > TaskPriority::High);
        assert!(TaskPriority::High > TaskPriority::Normal);
        assert!(TaskPriority::Normal > TaskPriority::Low);
    }
}
