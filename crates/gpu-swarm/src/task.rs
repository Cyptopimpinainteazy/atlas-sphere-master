//! Task definitions for the GPU swarm

use serde::{Deserialize, Serialize};
use std::time::Duration;
use uuid::Uuid;

/// Unique identifier for a task
pub type TaskId = Uuid;

/// Task priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum TaskPriority {
    /// Low priority - processed when resources are idle
    Low = 0,
    /// Normal priority - standard processing
    Normal = 1,
    /// High priority - processed before normal tasks
    High = 2,
    /// Critical priority - immediate processing
    Critical = 3,
}

impl Default for TaskPriority {
    fn default() -> Self {
        TaskPriority::Normal
    }
}

/// Types of tasks the swarm can execute
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskType {
    /// Execute X3 bytecode on GPU
    X3Bytecode {
        /// The bytecode to execute
        bytecode: Vec<u8>,
        /// Input data for the program
        input: Vec<u8>,
        /// Gas budget for execution
        gas_budget: u64,
    },

    /// Mempool simulation - scan and simulate pending transactions
    MempoolSimulation {
        /// Chain ID to simulate
        chain_id: u64,
        /// Number of transactions to simulate
        tx_count: u32,
        /// RPC endpoint for mempool data
        rpc_endpoint: String,
    },

    /// Route optimization - find optimal swap routes
    RouteOptimization {
        /// Source token address
        source_token: String,
        /// Destination token address
        dest_token: String,
        /// Amount to swap
        amount: String,
        /// Chains to consider
        chains: Vec<u64>,
        /// Maximum hops
        max_hops: u8,
    },

    /// ML training task
    MLTraining {
        /// Model identifier
        model_id: String,
        /// Training data hash (fetched from storage)
        training_data_hash: String,
        /// Number of epochs
        epochs: u32,
        /// Batch size
        batch_size: u32,
    },

    /// Proof generation (ZK)
    ProofGeneration {
        /// Circuit identifier
        circuit_id: String,
        /// Public inputs
        public_inputs: Vec<u8>,
        /// Private inputs (encrypted)
        private_inputs: Vec<u8>,
    },

    /// Arbitrage search
    ArbitrageSearch {
        /// Token pairs to monitor
        pairs: Vec<(String, String)>,
        /// Minimum profit threshold (basis points)
        min_profit_bps: u32,
        /// Maximum gas willing to spend
        max_gas: u64,
    },

    /// Custom task with raw payload
    Custom {
        /// Task type identifier
        task_type: String,
        /// Serialized payload
        payload: Vec<u8>,
    },
}

/// A task to be executed by the swarm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique task identifier
    pub id: TaskId,

    /// Task type and payload
    pub task_type: TaskType,

    /// Task priority
    pub priority: TaskPriority,

    /// Submitter's public key
    pub submitter: [u8; 32],

    /// Reward amount (in X3 tokens)
    pub reward: u64,

    /// Maximum execution time
    pub timeout: Duration,

    /// Number of verification nodes required
    pub verification_count: u8,

    /// Task creation timestamp
    pub created_at: i64,

    /// Deadline (optional, 0 = no deadline)
    pub deadline: i64,

    /// Metadata
    pub metadata: TaskMetadata,
}

/// Task metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskMetadata {
    /// Human-readable description
    pub description: Option<String>,

    /// Tags for categorization
    pub tags: Vec<String>,

    /// Required GPU capabilities
    pub required_capabilities: Vec<String>,

    /// Preferred regions (for latency-sensitive tasks)
    pub preferred_regions: Vec<String>,

    /// Minimum node reputation score
    pub min_reputation: Option<u32>,
}

impl Task {
    /// Create a new task
    pub fn new(task_type: TaskType, submitter: [u8; 32], reward: u64) -> Self {
        Self {
            id: Uuid::new_v4(),
            task_type,
            priority: TaskPriority::Normal,
            submitter,
            reward,
            timeout: Duration::from_secs(300),
            verification_count: 2,
            created_at: chrono::Utc::now().timestamp(),
            deadline: 0,
            metadata: TaskMetadata::default(),
        }
    }

    /// Set task priority
    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set task timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set verification count
    pub fn with_verification_count(mut self, count: u8) -> Self {
        self.verification_count = count;
        self
    }

    /// Set deadline
    pub fn with_deadline(mut self, deadline: i64) -> Self {
        self.deadline = deadline;
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, metadata: TaskMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    /// Check if task is expired
    pub fn is_expired(&self) -> bool {
        if self.deadline == 0 {
            return false;
        }
        chrono::Utc::now().timestamp() > self.deadline
    }

    /// Estimate compute units needed
    pub fn estimated_compute_units(&self) -> u64 {
        match &self.task_type {
            TaskType::X3Bytecode { gas_budget, .. } => *gas_budget,
            TaskType::MempoolSimulation { tx_count, .. } => *tx_count as u64 * 1000,
            TaskType::RouteOptimization {
                chains, max_hops, ..
            } => chains.len() as u64 * *max_hops as u64 * 10000,
            TaskType::MLTraining {
                epochs, batch_size, ..
            } => *epochs as u64 * *batch_size as u64 * 100,
            TaskType::ProofGeneration { .. } => 1_000_000,
            TaskType::ArbitrageSearch { pairs, .. } => pairs.len() as u64 * 5000,
            TaskType::Custom { payload, .. } => payload.len() as u64,
        }
    }
}

/// Status of a task in the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Task is waiting in queue
    Pending,
    /// Task has been assigned to nodes
    Assigned,
    /// Task is currently executing
    Executing,
    /// Task is being verified
    Verifying,
    /// Task completed successfully
    Completed,
    /// Task failed
    Failed,
    /// Task was cancelled
    Cancelled,
    /// Task timed out
    TimedOut,
}

/// Record of a task's execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecution {
    /// Task ID
    pub task_id: TaskId,

    /// Node that executed the task
    pub executor_node: [u8; 32],

    /// Execution start time
    pub started_at: i64,

    /// Execution end time
    pub completed_at: Option<i64>,

    /// Task status
    pub status: TaskStatus,

    /// Compute units consumed
    pub compute_units_used: u64,

    /// Result hash (for verification)
    pub result_hash: Option<[u8; 32]>,

    /// Error message (if failed)
    pub error: Option<String>,
}
