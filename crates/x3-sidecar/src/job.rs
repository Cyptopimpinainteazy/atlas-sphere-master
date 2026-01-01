//! Job management

use serde::{Deserialize, Serialize};
use std::collections::BinaryHeap;
use std::time::Instant;

/// Execution job
#[derive(Clone, Debug)]
pub struct Job {
    /// Unique job ID (32 bytes)
    pub id: [u8; 32],
    /// Bytecode
    pub bytecode: Vec<u8>,
    /// Input data
    pub input: Vec<u8>,
    /// Gas limit
    pub gas_limit: u64,
    /// Priority (higher = more urgent)
    pub priority: u8,
    /// Callback URL for completion notification
    pub callback_url: Option<String>,
    /// Submitted timestamp
    pub submitted_at: Instant,
    /// Started timestamp
    pub started_at: Option<Instant>,
}

impl Job {
    /// Create a new job
    pub fn new(id: [u8; 32], bytecode: Vec<u8>, input: Vec<u8>, gas_limit: u64) -> Self {
        Self {
            id,
            bytecode,
            input,
            gas_limit,
            priority: 5,
            callback_url: None,
            submitted_at: Instant::now(),
            started_at: None,
        }
    }
}

impl PartialEq for Job {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Job {}

impl PartialOrd for Job {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Job {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Higher priority first
        self.priority.cmp(&other.priority)
    }
}

/// Job queue with priority support
pub struct JobQueue {
    /// Priority queue of pending jobs
    heap: BinaryHeap<Job>,
    /// Statistics
    stats: QueueStats,
}

impl JobQueue {
    /// Create a new job queue
    pub fn new() -> Self {
        Self {
            heap: BinaryHeap::new(),
            stats: QueueStats::default(),
        }
    }

    /// Add a job to the queue
    pub fn push(&mut self, job: Job) {
        self.heap.push(job);
    }

    /// Pop the highest priority job
    pub fn pop(&mut self) -> Option<Job> {
        self.heap.pop()
    }

    /// Get number of pending jobs
    pub fn len(&self) -> usize {
        self.heap.len()
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }

    /// Clear the queue
    pub fn clear(&mut self) {
        self.heap.clear();
    }

    /// Remove a specific job by ID
    pub fn remove(&mut self, id: &[u8; 32]) -> bool {
        let before = self.heap.len();
        let jobs: Vec<Job> = self.heap.drain().filter(|j| j.id != *id).collect();
        self.heap = jobs.into_iter().collect();
        self.heap.len() < before
    }

    /// Iterate over jobs (for status checks)
    pub fn iter(&self) -> impl Iterator<Item = &Job> {
        self.heap.iter()
    }

    /// Get queue statistics
    pub fn stats(&self) -> QueueStats {
        QueueStats {
            pending: self.heap.len(),
            running: self.stats.running,
            completed: self.stats.completed,
            failed: self.stats.failed,
            avg_wait_time_ms: self.stats.avg_wait_time_ms,
        }
    }

    /// Record a completed job
    pub fn record_completed(&mut self, wait_time_ms: u64) {
        self.stats.completed += 1;
        // Update moving average
        let total = self.stats.completed + self.stats.failed;
        self.stats.avg_wait_time_ms =
            (self.stats.avg_wait_time_ms * (total - 1) + wait_time_ms) / total;
    }

    /// Record a failed job
    pub fn record_failed(&mut self) {
        self.stats.failed += 1;
    }
}

impl Default for JobQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// Queue statistics
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct QueueStats {
    pub pending: usize,
    pub running: usize,
    pub completed: u64,
    pub failed: u64,
    pub avg_wait_time_ms: u64,
}
