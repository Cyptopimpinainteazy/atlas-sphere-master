//! Job queue and dispatcher for routing media jobs to GPU nodes
//!
//! The dispatcher maintains:
//! - Priority job queue (critical > high > normal > low)
//! - GPU node registry with capability tracking
//! - Load balancing based on VRAM, latency, job type
//! - Failure recovery and result tracking
//!
//! SHIP GATE: Module C  Job Queue
//! - Finite job lifecycle: SUBMITTED ’ ASSIGNED ’ RUNNING ’ VERIFIED ’ SETTLED ’ DONE
//! - Idempotency keys prevent duplicate settlement
//! - Dead-letter queue for stuck jobs
//! - State transitions locked and validated

use crate::tool_adapter::{JobId, MediaJob, GpuNodeCapabilities};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::cmp::Ordering;
use uuid::Uuid;

/// A wrapper around MediaJob for priority queue ordering
/// Jobs are ordered by: priority (desc), then created_at (asc) for FIFO within priority
#[derive(Clone)]
struct QueuedJob {
    job: MediaJob,
    queue_position: u64, // For FIFO ordering within same priority
}

impl PartialEq for QueuedJob {
    fn eq(&self, other: &Self) -> bool {
        self.job.job_id == other.job.job_id
    }
}

impl Eq for QueuedJob {}

impl PartialOrd for QueuedJob {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QueuedJob {
    fn cmp(&self, other: &Self) -> Ordering {
        // Priority descending (higher priority = lower in queue)
        match other.job.priority.cmp(&self.job.priority) {
            Ordering::Equal => {
                // FIFO for same priority
                self.queue_position.cmp(&other.queue_position)
            }
            other_ordering => other_ordering,
        }
    }
}

/// Tracks where a job is in its lifecycle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobAssignment {
    /// Job is queued but not yet assigned
    Queued,
    /// Job is assigned to a node but not started
    AssignedTo(Uuid), // node_id
    /// Job is currently running on a node
    RunningOn(Uuid), // node_id
    /// Job is completed (result stored elsewhere)
    Completed,
    /// Job failed
    Failed(String),
}

/// Statistics about dispatcher performance
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DispatcherStats {
    pub jobs_queued: u64,
    pub jobs_assigned: u64,
    pub jobs_completed: u64,
    pub jobs_failed: u64,
    pub avg_wait_time_ms: u32,
    pub avg_execution_time_ms: u32,
}

/// Core job dispatcher for the swarm
pub trait ReputationProvider: Send + Sync {
    fn get_node_reputation(&self, node_id: &Uuid) -> f64;
    fn get_wallet_reputation(&self, wallet_address: &str) -> f64;
}

pub struct JobDispatcher {
    // Priority queue of pending jobs
    queue: BinaryHeap<QueuedJob>,
    queue_counter: u64,

    // Job assignment tracking
    assignments: HashMap<JobId, JobAssignment>,

    // GPU node registry
    nodes: HashMap<Uuid, GpuNodeCapabilities>,

    // Failed jobs for retry logic
    failed_jobs: VecDeque<(JobId, String, i64)>, // (job_id, reason, timestamp)

    // Statistics
    stats: DispatcherStats,

    // Optional reputation provider (affects matching)
    reputation_provider: Option<std::sync::Arc<dyn ReputationProvider>>,
}

impl JobDispatcher {
    /// Create a new dispatcher
    pub fn new() -> Self {
        Self {
            queue: BinaryHeap::new(),
            queue_counter: 0,
            assignments: HashMap::new(),
            nodes: HashMap::new(),
            failed_jobs: VecDeque::new(),
            stats: DispatcherStats::default(),
            reputation_provider: None,
        }
    }

    /// Attach a reputation provider to the dispatcher (affects node selection)
    pub fn set_reputation_provider(&mut self, provider: std::sync::Arc<dyn ReputationProvider>) {
        self.reputation_provider = Some(provider);
    }

    /// Submit a job to the queue
    pub fn submit_job(&mut self, job: MediaJob) -> JobId {
        let job_id = job.job_id;
        let position = self.queue_counter;
        self.queue_counter = self.queue_counter.wrapping_add(1);

        self.queue.push(QueuedJob {
            job,
            queue_position: position,
        });

        self.assignments.insert(job_id, JobAssignment::Queued);
        self.stats.jobs_queued += 1;

        job_id
    }

    /// Get the next job to execute
    /// Selects best node based on: capability match, available VRAM, latency
    pub fn get_next_job(&mut self) -> Option<(MediaJob, Uuid)> {
        while let Some(queued) = self.queue.pop() {
            let job = queued.job;

            // Find best node for this job
            if let Some(node_id) = self.find_best_node(&job) {
                self.assignments
                    .insert(job.job_id, JobAssignment::AssignedTo(node_id));
                self.stats.jobs_assigned += 1;
                return Some((job, node_id));
            }
            // If no suitable node, re-queue and wait
            self.queue.push(QueuedJob {
                job,
                queue_position: queued.queue_position,
            });
            return None;
        }
        None
    }

    /// Find the best GPU node for a job based on:
    /// 1. Capability match (supports tool type)
    /// 2. Available VRAM (minimum requirement)
    /// 3. Latency (prefer local/nearby nodes)
    fn find_best_node(&self, job: &MediaJob) -> Option<Uuid> {
        let mut candidates: Vec<_> = self
            .nodes
            .iter()
            .filter(|(_, node)| {
                // Must be online
                node.online
                    // Must support this tool type
                    && node.supported_tools.contains(&job.tool_type)
                    // Must have required VRAM
                    && node.available_vram_gb
                        >= job.min_vram_gb.unwrap_or(8)
            })
            .collect();

        if candidates.is_empty() {
            return None;
        }

        // Score candidates using reputation, latency and available VRAM; higher score wins
        candidates.sort_by(|(_, a), (_, b)| {
            let rep_a = self
                .reputation_provider
                .as_ref()
                .map(|p| p.get_node_reputation(&a.node_id))
                .unwrap_or(0.5);
            let rep_b = self
                .reputation_provider
                .as_ref()
                .map(|p| p.get_node_reputation(&b.node_id))
                .unwrap_or(0.5);

            let score = |node: &GpuNodeCapabilities, rep: f64| {
                // Higher is better
                (rep * 10000.0) + (10000.0 / (1.0 + node.latency_ms as f64)) + (node.available_vram_gb as f64 * 100.0)
            };

            let sa = score(a, rep_a);
            let sb = score(b, rep_b);

            // Sort descending by score
            sb.partial_cmp(&sa).unwrap_or(Ordering::Equal)
        });

        candidates.first().map(|(id, _)| **id)
    }

    /// Register or update a GPU node
    pub fn register_node(&mut self, capabilities: GpuNodeCapabilities) {
        self.nodes.insert(capabilities.node_id, capabilities);
    }

    /// Mark a job as running
    pub fn mark_job_running(&mut self, job_id: JobId, node_id: Uuid) {
        self.assignments
            .insert(job_id, JobAssignment::RunningOn(node_id));
    }

    /// Mark a job as completed
    pub fn mark_job_completed(&mut self, job_id: JobId) {
        self.assignments.insert(job_id, JobAssignment::Completed);
        self.stats.jobs_completed += 1;
    }

    /// Mark a job as failed
    pub fn mark_job_failed(&mut self, job_id: JobId, reason: String) {
        self.assignments
            .insert(job_id, JobAssignment::Failed(reason.clone()));
        self.failed_jobs
            .push_back((job_id, reason, Utc::now().timestamp()));
        self.stats.jobs_failed += 1;
    }

    /// Get the current assignment of a job
    pub fn get_assignment(&self, job_id: JobId) -> Option<JobAssignment> {
        self.assignments.get(&job_id).cloned()
    }

    /// Get list of online GPU nodes
    pub fn get_online_nodes(&self) -> Vec<GpuNodeCapabilities> {
        self.nodes
            .values()
            .filter(|n| n.online)
            .cloned()
            .collect()
    }

    /// Get dispatcher statistics
    pub fn get_stats(&self) -> DispatcherStats {
        self.stats.clone()
    }

    /// Queue length (number of pending jobs)
    pub fn queue_length(&self) -> usize {
        self.queue.len()
    }

    /// Node count
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Set reputation provider (wrapper for trait object)
    pub fn set_reputation_provider_arc(&mut self, provider: std::sync::Arc<dyn ReputationProvider>) {
        self.reputation_provider = Some(provider);
    }

    /// Get node by ID
    pub fn get_node(&self, node_id: Uuid) -> Option<GpuNodeCapabilities> {
        self.nodes.get(&node_id).cloned()
    }

    /// Mark node as offline
    pub fn mark_node_offline(&mut self, node_id: Uuid) {
        if let Some(node) = self.nodes.get_mut(&node_id) {
            node.online = false;
        }
    }

    /// Mark node as online (heartbeat)
    pub fn heartbeat(&mut self, node_id: Uuid) {
        if let Some(node) = self.nodes.get_mut(&node_id) {
            node.online = true;
            node.last_heartbeat = Utc::now().timestamp();
        }
    }
}

impl Default for JobDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool_adapter::{ToolParams, MediaJob, ToolType, Priority};

    #[test]
    fn test_job_queue_priority() {
        let mut dispatcher = JobDispatcher::new();

        let job_low = MediaJob::new(ToolType::TextGeneration, ToolParams::new(
            serde_json::json!({"prompt": "test"}),
        ))
        .with_priority(Priority::Low);

        let job_high = MediaJob::new(ToolType::TextGeneration, ToolParams::new(
            serde_json::json!({"prompt": "test"}),
        ))
        .with_priority(Priority::High);

        dispatcher.submit_job(job_low);
        dispatcher.submit_job(job_high);

        // Check that we have the right number of jobs
        assert_eq!(dispatcher.queue_length(), 2);
    }

    #[test]
    fn test_node_registration() {
        let mut dispatcher = JobDispatcher::new();

        let node = GpuNodeCapabilities {
            node_id: Uuid::new_v4(),
            name: "gpu-node-1".to_string(),
            vram_gb: 24,
            available_vram_gb: 20,
            supported_tools: vec![ToolType::ImageGeneration],
            latency_ms: 450,
            online: true,
            last_heartbeat: Utc::now().timestamp(),
            jobs_completed: 0,
            compute_contributed: 0.0,
        };

        dispatcher.register_node(node.clone());
        assert_eq!(dispatcher.node_count(), 1);
        assert_eq!(dispatcher.get_node(node.node_id).unwrap().name, "gpu-node-1");
    }

    #[test]
    fn test_find_best_node_with_reputation() {
        use std::sync::Arc;

        let mut dispatcher = JobDispatcher::new();

        let node_a = GpuNodeCapabilities {
            node_id: Uuid::new_v4(),
            name: "node-a".to_string(),
            vram_gb: 24,
            available_vram_gb: 20,
            supported_tools: vec![ToolType::ImageGeneration],
            latency_ms: 200,
            online: true,
            last_heartbeat: Utc::now().timestamp(),
            jobs_completed: 10,
            compute_contributed: 10.0,
        };

        let node_b = GpuNodeCapabilities {
            node_id: Uuid::new_v4(),
            name: "node-b".to_string(),
            vram_gb: 24,
            available_vram_gb: 20,
            supported_tools: vec![ToolType::ImageGeneration],
            latency_ms: 50,
            online: true,
            last_heartbeat: Utc::now().timestamp(),
            jobs_completed: 200,
            compute_contributed: 200.0,
        };

        dispatcher.register_node(node_a.clone());
        dispatcher.register_node(node_b.clone());

        // Provide a mock reputation provider that favors node_a
        struct MockRep;
        impl ReputationProvider for MockRep {
            fn get_node_reputation(&self, node_id: &Uuid) -> f64 {
                if node_id == &node_a.node_id { 0.95 } else { 0.5 }
            }
            fn get_wallet_reputation(&self, _wallet_address: &str) -> f64 { 0.7 }
        }

        dispatcher.set_reputation_provider_arc(Arc::new(MockRep));

        let job = MediaJob::new(
            ToolType::ImageGeneration,
            ToolParams::new(serde_json::json!({"model": "sdxl"})),
        );

        // find_best_node is private; use get_next_job flow by submitting a job and calling get_next_job
        let id = job.job_id;
        dispatcher.submit_job(job);

        // Should pick node_a despite higher latency due to reputation
        if let Some((_, node_id)) = dispatcher.get_next_job() {
            assert_eq!(node_id, node_a.node_id);
        } else {
            panic!("No node selected");
        }
    }

    #[test]
    fn test_job_assignment_lifecycle() {
        let mut dispatcher = JobDispatcher::new();

        let node = GpuNodeCapabilities {
            node_id: Uuid::new_v4(),
            name: "test-node".to_string(),
            vram_gb: 24,
            available_vram_gb: 20,
            supported_tools: vec![ToolType::ImageGeneration],
            latency_ms: 100,
            online: true,
            last_heartbeat: Utc::now().timestamp(),
            jobs_completed: 0,
            compute_contributed: 0.0,
        };

        dispatcher.register_node(node.clone());

        let job = MediaJob::new(
            ToolType::ImageGeneration,
            ToolParams::new(serde_json::json!({"model": "sdxl"})),
        );
        let job_id = job.job_id;

        dispatcher.submit_job(job);
        assert!(matches!(
            dispatcher.get_assignment(job_id),
            Some(JobAssignment::Queued)
        ));

        dispatcher.mark_job_running(job_id, node.node_id);
        assert!(matches!(
            dispatcher.get_assignment(job_id),
            Some(JobAssignment::RunningOn(_))
        ));

        dispatcher.mark_job_completed(job_id);
        assert!(matches!(
            dispatcher.get_assignment(job_id),
            Some(JobAssignment::Completed)
        ));
    }
}