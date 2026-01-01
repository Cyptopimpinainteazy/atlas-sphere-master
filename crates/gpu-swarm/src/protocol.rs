//! P2P protocol definitions for the GPU swarm

use crate::node::{GpuCapabilities, NodeId, NodeMetrics};
use crate::task::{Task, TaskExecution, TaskId, TaskStatus};
use serde::{Deserialize, Serialize};

/// Ed25519 signature wrapper (64 bytes)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Signature(pub [u8; 64]);

impl Default for Signature {
    fn default() -> Self {
        Signature([0u8; 64])
    }
}

impl Serialize for Signature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for Signature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct SignatureVisitor;

        impl<'de> serde::de::Visitor<'de> for SignatureVisitor {
            type Value = Signature;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("64 bytes")
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if v.len() != 64 {
                    return Err(E::invalid_length(v.len(), &self));
                }
                let mut arr = [0u8; 64];
                arr.copy_from_slice(v);
                Ok(Signature(arr))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::SeqAccess<'de>,
            {
                let mut arr = [0u8; 64];
                for (i, byte) in arr.iter_mut().enumerate() {
                    *byte = seq
                        .next_element()?
                        .ok_or_else(|| serde::de::Error::invalid_length(i, &self))?;
                }
                Ok(Signature(arr))
            }
        }

        deserializer.deserialize_bytes(SignatureVisitor)
    }
}

/// Protocol message types for swarm communication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SwarmMessage {
    // === Node Registration ===
    /// Request to join the swarm
    JoinRequest(JoinRequest),

    /// Response to join request
    JoinResponse(JoinResponse),

    /// Node leaving the swarm gracefully
    LeaveNotification(LeaveNotification),

    // === Heartbeat ===
    /// Periodic heartbeat from node
    Heartbeat(Heartbeat),

    /// Heartbeat acknowledgment
    HeartbeatAck(HeartbeatAck),

    // === Task Management ===
    /// New task submitted
    TaskSubmission(TaskSubmission),

    /// Task assigned to node(s)
    TaskAssignment(TaskAssignment),

    /// Task execution started
    TaskStarted(TaskStarted),

    /// Task execution result
    TaskResult(TaskResult),

    /// Task verification request
    VerificationRequest(VerificationRequest),

    /// Task verification result
    VerificationResult(VerificationResult),

    // === Coordination ===
    /// Request for available capacity
    CapacityQuery(CapacityQuery),

    /// Response with available capacity
    CapacityResponse(CapacityResponse),

    /// Task cancellation
    TaskCancellation(TaskCancellation),

    /// Slash notification (bad behavior detected)
    SlashNotification(SlashNotification),

    // === Gossip ===
    /// Node status update (gossiped)
    NodeStatusGossip(NodeStatusGossip),

    /// Task queue status (gossiped)
    TaskQueueGossip(TaskQueueGossip),
}

// === Node Registration Messages ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinRequest {
    /// Node's public key
    pub node_id: NodeId,

    /// Node's P2P listening address
    pub peer_address: String,

    /// Node's region
    pub region: String,

    /// GPU capabilities
    pub gpu_capabilities: GpuCapabilities,

    /// Stake amount
    pub stake: u64,

    /// Supported task types
    pub supported_tasks: Vec<String>,

    /// Node software version
    pub version: String,

    /// Signature over the request
    pub signature: Signature,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinResponse {
    /// Whether join was accepted
    pub accepted: bool,

    /// Rejection reason if not accepted
    pub reason: Option<String>,

    /// List of bootstrap peers to connect to
    pub bootstrap_peers: Vec<String>,

    /// Current epoch number
    pub current_epoch: u64,

    /// Coordinator's signature
    pub signature: Signature,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaveNotification {
    /// Node leaving
    pub node_id: NodeId,

    /// Reason for leaving
    pub reason: String,

    /// Signature
    pub signature: Signature,
}

// === Heartbeat Messages ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heartbeat {
    /// Node ID
    pub node_id: NodeId,

    /// Current timestamp
    pub timestamp: i64,

    /// Current metrics
    pub metrics: NodeMetrics,

    /// Number of tasks in queue
    pub queue_depth: u32,

    /// Available VRAM
    pub available_vram: u64,

    /// Signature
    pub signature: Signature,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatAck {
    /// Timestamp echoed back
    pub timestamp: i64,

    /// Any pending tasks for this node
    pub pending_tasks: Vec<TaskId>,
}

// === Task Messages ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSubmission {
    /// The task to execute
    pub task: Task,

    /// Submitter's signature
    pub signature: Signature,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskAssignment {
    /// Task ID
    pub task_id: TaskId,

    /// Primary executor node
    pub primary_executor: NodeId,

    /// Verification nodes
    pub verifiers: Vec<NodeId>,

    /// Assignment timestamp
    pub assigned_at: i64,

    /// Coordinator's signature
    pub signature: Signature,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStarted {
    /// Task ID
    pub task_id: TaskId,

    /// Executor node
    pub executor: NodeId,

    /// Start timestamp
    pub started_at: i64,

    /// Estimated completion time
    pub estimated_completion: i64,

    /// Signature
    pub signature: Signature,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// Task ID
    pub task_id: TaskId,

    /// Executor node
    pub executor: NodeId,

    /// Whether task succeeded
    pub success: bool,

    /// Result data (serialized)
    pub result_data: Vec<u8>,

    /// Hash of result data
    pub result_hash: [u8; 32],

    /// Compute units consumed
    pub compute_units: u64,

    /// Execution time in milliseconds
    pub execution_time_ms: u64,

    /// GPU state proof (for verification)
    pub execution_proof: ExecutionProof,

    /// Error message if failed
    pub error: Option<String>,

    /// Signature over result
    pub signature: Signature,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationRequest {
    /// Task ID to verify
    pub task_id: TaskId,

    /// Original task data
    pub task: Task,

    /// Result to verify
    pub result: TaskResult,

    /// Verifier node
    pub verifier: NodeId,

    /// Deadline for verification
    pub deadline: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    /// Task ID
    pub task_id: TaskId,

    /// Verifier node
    pub verifier: NodeId,

    /// Whether result is valid
    pub valid: bool,

    /// Verifier's result hash (should match)
    pub computed_hash: [u8; 32],

    /// Discrepancy details if invalid
    pub discrepancy: Option<String>,

    /// Signature
    pub signature: Signature,
}

// === Execution Proof ===

/// Proof of correct GPU execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionProof {
    /// GPU device fingerprint
    pub device_fingerprint: [u8; 32],

    /// Input state hash
    pub input_hash: [u8; 32],

    /// Output state hash
    pub output_hash: [u8; 32],

    /// Intermediate state checkpoints
    pub checkpoints: Vec<StateCheckpoint>,

    /// Execution trace (compressed)
    pub trace: Vec<u8>,

    /// Nonce used for randomness
    pub nonce: [u8; 16],

    /// Timestamp
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateCheckpoint {
    /// Checkpoint index
    pub index: u32,

    /// State hash at checkpoint
    pub state_hash: [u8; 32],

    /// Compute units consumed up to this point
    pub compute_units: u64,
}

impl ExecutionProof {
    /// Create a new empty proof
    pub fn new(input_hash: [u8; 32]) -> Self {
        Self {
            device_fingerprint: [0u8; 32],
            input_hash,
            output_hash: [0u8; 32],
            checkpoints: Vec::new(),
            trace: Vec::new(),
            nonce: [0u8; 16],
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    /// Add a checkpoint
    pub fn add_checkpoint(&mut self, state_hash: [u8; 32], compute_units: u64) {
        self.checkpoints.push(StateCheckpoint {
            index: self.checkpoints.len() as u32,
            state_hash,
            compute_units,
        });
    }

    /// Finalize the proof with output
    pub fn finalize(&mut self, output_hash: [u8; 32]) {
        self.output_hash = output_hash;
    }

    /// Verify proof structure (not cryptographic verification)
    pub fn is_valid_structure(&self) -> bool {
        // Input hash must be set
        if self.input_hash == [0u8; 32] {
            return false;
        }

        // Must have at least one checkpoint
        if self.checkpoints.is_empty() {
            return false;
        }

        // Checkpoints must be in order
        for i in 1..self.checkpoints.len() {
            if self.checkpoints[i].index <= self.checkpoints[i - 1].index {
                return false;
            }
            if self.checkpoints[i].compute_units < self.checkpoints[i - 1].compute_units {
                return false;
            }
        }

        true
    }
}

// === Coordination Messages ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityQuery {
    /// Requester node
    pub requester: NodeId,

    /// Required GPU capabilities
    pub required_capabilities: Vec<String>,

    /// Minimum available VRAM
    pub min_vram: u64,

    /// Preferred region
    pub preferred_region: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityResponse {
    /// Responding node
    pub node_id: NodeId,

    /// Available compute capacity
    pub available_capacity: u64,

    /// Current queue depth
    pub queue_depth: u32,

    /// Estimated wait time in seconds
    pub estimated_wait: u64,

    /// Region
    pub region: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCancellation {
    /// Task to cancel
    pub task_id: TaskId,

    /// Canceller (must be submitter)
    pub canceller: NodeId,

    /// Reason
    pub reason: String,

    /// Signature
    pub signature: Signature,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashNotification {
    /// Node being slashed
    pub node_id: NodeId,

    /// Slash amount
    pub slash_amount: u64,

    /// Reason for slash
    pub reason: String,

    /// Evidence (task results, etc.)
    pub evidence: Vec<u8>,

    /// Coordinator signature
    pub signature: Signature,
}

// === Gossip Messages ===

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeStatusGossip {
    /// Node that changed status
    pub node_id: NodeId,

    /// New status
    pub status: crate::node::NodeStatus,

    /// Timestamp
    pub timestamp: i64,

    /// Number of hops this gossip has traveled
    pub hops: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskQueueGossip {
    /// Total tasks in queue
    pub total_tasks: u64,

    /// Tasks by priority
    pub by_priority: [(u8, u64); 4],

    /// Average wait time
    pub avg_wait_time: u64,

    /// Timestamp
    pub timestamp: i64,
}

/// Message envelope for transport
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEnvelope {
    /// Protocol version
    pub version: u8,

    /// Sender node ID
    pub sender: NodeId,

    /// Message ID (for deduplication)
    pub message_id: [u8; 16],

    /// The actual message
    pub message: SwarmMessage,

    /// Timestamp
    pub timestamp: i64,
}

impl MessageEnvelope {
    /// Create a new envelope
    pub fn new(sender: NodeId, message: SwarmMessage) -> Self {
        let mut message_id = [0u8; 16];
        getrandom::getrandom(&mut message_id).unwrap_or_default();

        Self {
            version: crate::PROTOCOL_VERSION as u8,
            sender,
            message_id,
            message,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(self)
    }

    /// Deserialize from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(data)
    }
}
