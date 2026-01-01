/// Intent-based Tool Adapter Layer
///
/// This module implements the control plane between agents and external tools.
/// Core principle: Agents emit Intents, never raw actions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

/// Unique identifier for a job in the queue
pub type JobId = u64;

/// Unique identifier for a node (GPU, CPU, etc.)
pub type NodeId = String;

/// Intent that an agent emits (never a direct action)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SignedIntent {
    /// Globally unique identifier (prevents replay attacks)
    pub intent_id: String,

    /// Agent that originated this intent
    pub origin_agent: String,

    /// Type of operation (whitelist, not strings)
    pub intent_type: IntentType,

    /// Tool-specific input (validated against schema per intent_type)
    pub payload: serde_json::Value,

    /// Resource estimate (agent's honest guess)
    pub resource_estimate: ResourceSpec,

    /// Creation timestamp (staleness check)
    pub timestamp_millis: u64,

    /// Cryptographic signature (proof of origin)
    pub signature: String,

    /// Who can cancel this job mid-flight
    pub kill_authority: Vec<String>,
}

/// Whitelist of valid intent types (not strings)
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum IntentType {
    /// Sentinel: Fast inference for signal scoring
    InferSignalScore,

    /// Sentinel: Risk veto check
    VetoRiskAssessment,

    /// Trainer: Start training run
    StartTrainingRun,

    /// Trainer: Continue from checkpoint
    ResumeTrainingRun,

    /// Simulator: Generate market scenarios
    GenerateScenarios,

    /// Simulator: Backtest strategy on scenarios
    BacktestStrategy,

    /// Vision: Render chart/heatmap
    RenderChart,

    /// Vision: Generate report
    GenerateReport,

    /// Watchdog: Verify decision
    VerifyDecision,

    /// Watchdog: Replay execution
    ReplayExecution,

    /// Watchdog: Audit trail check
    AuditTrailCheck,
}

/// Resource specification for a job
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceSpec {
    /// VRAM required (MB)
    pub vram_mb: u32,

    /// Estimated duration (ms)
    pub duration_ms: u32,

    /// CPU cores needed
    pub cpu_cores: u8,

    /// Network throughput (MB/s)
    pub network_mbps: u16,

    /// Node role affinity
    pub required_node_role: NodeRole,
}

/// GPU/CPU node role (not generic "worker")
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum NodeRole {
    Sentinel,      // Fast inference, low latency
    Trainer,       // Training, mutation (stateful, long-lived)
    SimulationForge, // Market simulators, war games
    Vision,        // Chart/visualization rendering
    Watchdog,      // Verification, audit, determinism checks
}

/// Execution result from a tool
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Was execution successful?
    pub success: bool,

    /// Output bytes (tool-specific)
    pub output: Vec<u8>,

    /// Actual duration (ms)
    pub duration_ms: u32,

    /// Side effects (file writes, network calls, etc.)
    pub side_effects: Vec<SideEffect>,

    /// Cryptographic hash of output
    pub output_hash: [u8; 32],

    /// Determinism seed used
    pub determinism_seed: u64,

    /// Error message if failed
    pub error: Option<String>,
}

/// Side effect from tool execution (logged for audit)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SideEffect {
    FileWrite { path: String, bytes: usize },
    NetworkCall { endpoint: String, method: String },
    DatabaseWrite { table: String, records: usize },
    StateChange { entity: String, description: String },
}

/// Normalized result (canonical form for storage/consumption)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NormalizedResult {
    /// Original tool output
    pub raw_output: Vec<u8>,

    /// Hash of output (immutable proof)
    pub output_hash: [u8; 32],

    /// Determinism seed
    pub execution_seed: u64,

    /// Execution lineage (who asked, who executed, when)
    pub lineage: ExecutionLineage,

    /// All side effects logged
    pub side_effects: Vec<SideEffect>,

    /// When completed
    pub completed_at_millis: u64,

    /// Actual duration (ms)
    pub duration_ms: u32,
}

/// Execution lineage (provenance tracking)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecutionLineage {
    pub origin_agent: String,
    pub intent_id: String,
    pub intent_type: IntentType,
    pub tool_name: String,
    pub node_id: NodeId,
    pub node_role: NodeRole,
}

/// Intent Router: validates and enqueues intents
pub struct IntentRouter {
    /// Known agents (trust list)
    trusted_agents: Arc<HashMap<String, AgentTrust>>,

    /// Job queue reference (filled by router)
    job_queue: Arc<JobQueueSystem>,
}

/// Trust metadata for an agent
#[derive(Clone, Debug)]
pub struct AgentTrust {
    pub agent_id: String,
    pub public_key: String,
    pub authorized_intent_types: Vec<IntentType>,
}

#[derive(Error, Debug)]
pub enum RouterError {
    #[error("Unknown agent: {0}")]
    UnknownAgent(String),

    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Unauthorized intent type: {0:?}")]
    UnauthorizedIntentType(IntentType),

    #[error("Replay attack detected: {0}")]
    ReplayAttack(String),

    #[error("Malformed payload")]
    MalformedPayload,

    #[error("Stale intent")]
    StaleIntent,
}

impl IntentRouter {
    pub fn new(
        trusted_agents: HashMap<String, AgentTrust>,
        job_queue: Arc<JobQueueSystem>,
    ) -> Self {
        Self {
            trusted_agents: Arc::new(trusted_agents),
            job_queue,
        }
    }

    /// Validate and route an intent to the queue
    pub fn route_intent(&self, intent: SignedIntent) -> Result<JobId, RouterError> {
        // 1. Check if agent is known
        let agent_trust = self
            .trusted_agents
            .get(&intent.origin_agent)
            .ok_or_else(|| RouterError::UnknownAgent(intent.origin_agent.clone()))?;

        // 2. Verify signature (simplified)
        self.verify_signature(&intent, &agent_trust.public_key)?;

        // 3. Check if agent is authorized for this intent type
        if !agent_trust
            .authorized_intent_types
            .contains(&intent.intent_type)
        {
            return Err(RouterError::UnauthorizedIntentType(intent.intent_type));
        }

        // 4. Check for replay (intent_id must be unique)
        self.job_queue
            .check_replay(&intent.intent_id)
            .map_err(|_| RouterError::ReplayAttack(intent.intent_id.clone()))?;

        // 5. Check staleness (intent created within last 60s)
        let now_millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        if now_millis - intent.timestamp_millis > 60_000 {
            return Err(RouterError::StaleIntent);
        }

        // 6. Validate payload against schema for this intent_type
        self.validate_payload(&intent.intent_type, &intent.payload)?;

        // 7. Determine priority class
        let priority = determine_priority(&intent.intent_type);

        // 8. Allocate resources and enqueue
        let job_id = self.job_queue.enqueue(intent, priority)?;

        Ok(job_id)
    }

    fn verify_signature(
        &self,
        _intent: &SignedIntent,
        _public_key: &str,
    ) -> Result<(), RouterError> {
        // Simplified: in production, use cryptographic verification
        // For now, accept all
        Ok(())
    }

    fn validate_payload(
        &self,
        intent_type: &IntentType,
        payload: &serde_json::Value,
    ) -> Result<(), RouterError> {
        // Simplified schema validation per intent_type
        match intent_type {
            IntentType::InferSignalScore => {
                // Must have market_data, strategy_id
                if !payload.get("market_data").is_some() {
                    return Err(RouterError::MalformedPayload);
                }
                Ok(())
            }
            IntentType::VetoRiskAssessment => {
                // Must have risk_metrics
                if !payload.get("risk_metrics").is_some() {
                    return Err(RouterError::MalformedPayload);
                }
                Ok(())
            }
            // ... more types
            _ => Ok(()),
        }
    }
}

/// Determine priority class from intent type
fn determine_priority(intent_type: &IntentType) -> PriorityClass {
    match intent_type {
        IntentType::VetoRiskAssessment => PriorityClass::P0,
        IntentType::InferSignalScore => PriorityClass::P1,
        IntentType::BacktestStrategy => PriorityClass::P2,
        IntentType::StartTrainingRun | IntentType::ResumeTrainingRun => PriorityClass::P3,
        IntentType::RenderChart | IntentType::GenerateReport => PriorityClass::P4,
        IntentType::VerifyDecision | IntentType::ReplayExecution | IntentType::AuditTrailCheck => {
            PriorityClass::P0 // Watchdog always P0
        }
        IntentType::GenerateScenarios => PriorityClass::P3,
    }
}

/// Priority class (hard-coded, meaningful)
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PriorityClass {
    P0 = 0, // Survival (risk veto, kill switches)
    P1 = 1, // Live execution (trades, route selection)
    P2 = 2, // Intelligence (scoring, signals)
    P3 = 3, // Evolution (training, mutation)
    P4 = 4, // Cosmetics (charts, dashboards)
}

impl PriorityClass {
    /// SLA timeout (ms) for this priority class
    pub fn timeout_ms(&self) -> u64 {
        match self {
            PriorityClass::P0 => 100,
            PriorityClass::P1 => 500,
            PriorityClass::P2 => 5_000,
            PriorityClass::P3 => 60_000,
            PriorityClass::P4 => 300_000,
        }
    }

    /// Max retries for this priority class
    pub fn max_retries(&self) -> u32 {
        match self {
            PriorityClass::P0 => 3,
            PriorityClass::P1 => 2,
            PriorityClass::P2 | PriorityClass::P3 | PriorityClass::P4 => 1,
        }
    }
}

/// Job queue system (holds P0-P4 queues)
pub struct JobQueueSystem {
    // Queue storage (simplified: in production, use persistent storage)
    queues: Arc<std::sync::Mutex<JobQueues>>,

    // Seen intent IDs (replay detection)
    seen_intents: Arc<std::sync::Mutex<std::collections::HashSet<String>>>,
}

struct JobQueues {
    p0: Vec<QueuedJob>,
    p1: Vec<QueuedJob>,
    p2: Vec<QueuedJob>,
    p3: Vec<QueuedJob>,
    p4: Vec<QueuedJob>,
    next_job_id: JobId,
}

/// Job in the queue
#[derive(Clone, Debug)]
pub struct QueuedJob {
    pub job_id: JobId,
    pub intent: SignedIntent,
    pub priority: PriorityClass,
    pub assigned_node: Option<NodeId>,
    pub status: JobStatus,
    pub enqueued_at_millis: u64,
    pub started_at_millis: Option<u64>,
    pub completed_at_millis: Option<u64>,
    pub result: Option<NormalizedResult>,
    pub dead_letter: Option<DeadLetterEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed { reason: String },
    DeadLettered,
    Cancelled,
    TimedOut,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeadLetterEntry {
    pub error: String,
    pub last_node: Option<NodeId>,
    pub retry_count: u32,
}

#[derive(Error, Debug)]
pub enum QueueError {
    #[error("Replay attack detected")]
    ReplayDetected,

    #[error("Failed to enqueue: {0}")]
    EnqueueFailed(String),

    #[error("Job not found: {0}")]
    JobNotFound(JobId),
}

impl JobQueueSystem {
    pub fn new() -> Self {
        Self {
            queues: Arc::new(std::sync::Mutex::new(JobQueues {
                p0: Vec::new(),
                p1: Vec::new(),
                p2: Vec::new(),
                p3: Vec::new(),
                p4: Vec::new(),
                next_job_id: 1,
            })),
            seen_intents: Arc::new(std::sync::Mutex::new(std::collections::HashSet::new())),
        }
    }

    /// Check if intent_id has been seen before (replay detection)
    pub fn check_replay(&self, intent_id: &str) -> Result<(), QueueError> {
        let mut seen = self.seen_intents.lock().unwrap();
        if seen.contains(intent_id) {
            return Err(QueueError::ReplayDetected);
        }
        seen.insert(intent_id.to_string());
        Ok(())
    }

    /// Enqueue an intent
    pub fn enqueue(&self, intent: SignedIntent, priority: PriorityClass) -> Result<JobId, QueueError> {
        let mut queues = self.queues.lock().unwrap();
        let job_id = queues.next_job_id;
        queues.next_job_id += 1;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let job = QueuedJob {
            job_id,
            intent,
            priority,
            assigned_node: None,
            status: JobStatus::Queued,
            enqueued_at_millis: now,
            started_at_millis: None,
            completed_at_millis: None,
            result: None,
            dead_letter: None,
        };

        match priority {
            PriorityClass::P0 => queues.p0.push(job),
            PriorityClass::P1 => queues.p1.push(job),
            PriorityClass::P2 => queues.p2.push(job),
            PriorityClass::P3 => queues.p3.push(job),
            PriorityClass::P4 => queues.p4.push(job),
        }

        Ok(job_id)
    }

    /// Get queue depth per priority
    pub fn queue_depths(&self) -> HashMap<PriorityClass, usize> {
        let queues = self.queues.lock().unwrap();
        [
            (PriorityClass::P0, queues.p0.len()),
            (PriorityClass::P1, queues.p1.len()),
            (PriorityClass::P2, queues.p2.len()),
            (PriorityClass::P3, queues.p3.len()),
            (PriorityClass::P4, queues.p4.len()),
        ]
        .iter()
        .cloned()
        .collect()
    }

    /// Next job to execute (weighted fair scheduling, P0 preemption)
    pub fn next_job(&self) -> Option<QueuedJob> {
        let mut queues = self.queues.lock().unwrap();

        // P0 always runs first
        if let Some(job) = queues.p0.first_mut() {
            let mut j = job.clone();
            j.status = JobStatus::Running;
            return Some(j);
        }

        // Then P1
        if let Some(job) = queues.p1.first_mut() {
            let mut j = job.clone();
            j.status = JobStatus::Running;
            return Some(j);
        }

        // Then P2 (if P1 queue depth < 10% of capacity)
        if queues.p1.len() < 10 {
            if let Some(job) = queues.p2.first_mut() {
                let mut j = job.clone();
                j.status = JobStatus::Running;
                return Some(j);
            }
        }

        // Then P3 (if P1-P2 nearly empty)
        if queues.p1.is_empty() && queues.p2.len() < 5 {
            if let Some(job) = queues.p3.first_mut() {
                let mut j = job.clone();
                j.status = JobStatus::Running;
                return Some(j);
            }
        }

        // Finally P4
        if queues.p3.is_empty() {
            if let Some(job) = queues.p4.first_mut() {
                let mut j = job.clone();
                j.status = JobStatus::Running;
                return Some(j);
            }
        }

        None
    }
}

impl Default for JobQueueSystem {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intent_router_validates_agent() {
        let mut agents = HashMap::new();
        agents.insert(
            "sentinel-1".to_string(),
            AgentTrust {
                agent_id: "sentinel-1".to_string(),
                public_key: "0xabcd".to_string(),
                authorized_intent_types: vec![IntentType::InferSignalScore],
            },
        );

        let queue = Arc::new(JobQueueSystem::new());
        let router = IntentRouter::new(agents, queue);

        let intent = SignedIntent {
            intent_id: "test-1".to_string(),
            origin_agent: "unknown-agent".to_string(),
            intent_type: IntentType::InferSignalScore,
            payload: serde_json::json!({"market_data": {}}),
            resource_estimate: ResourceSpec {
                vram_mb: 1024,
                duration_ms: 100,
                cpu_cores: 4,
                network_mbps: 100,
                required_node_role: NodeRole::Sentinel,
            },
            timestamp_millis: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            signature: "0x1234".to_string(),
            kill_authority: vec!["ops".to_string()],
        };

        let result = router.route_intent(intent);
        assert!(result.is_err());
    }

    #[test]
    fn test_priority_class_timeouts() {
        assert_eq!(PriorityClass::P0.timeout_ms(), 100);
        assert_eq!(PriorityClass::P1.timeout_ms(), 500);
        assert_eq!(PriorityClass::P2.timeout_ms(), 5_000);
        assert_eq!(PriorityClass::P3.timeout_ms(), 60_000);
        assert_eq!(PriorityClass::P4.timeout_ms(), 300_000);
    }

    #[test]
    fn test_priority_class_ordering() {
        assert!(PriorityClass::P0 < PriorityClass::P1);
        assert!(PriorityClass::P1 < PriorityClass::P2);
    }
}
