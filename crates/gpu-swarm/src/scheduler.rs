//! Task scheduler for the GPU swarm

use crate::error::{SwarmError, SwarmResult};
use crate::node::{NodeId, NodeRegistry, NodeStatus, SwarmNode};
use crate::task::{Task, TaskExecution, TaskId, TaskPriority, TaskStatus};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, VecDeque};
use std::sync::{Arc, RwLock};

/// Wrapper for priority queue ordering
#[derive(Debug, Clone)]
struct PrioritizedTask {
    task: Task,
    enqueued_at: i64,
}

impl PartialEq for PrioritizedTask {
    fn eq(&self, other: &Self) -> bool {
        self.task.id == other.task.id
    }
}

impl Eq for PrioritizedTask {}

impl PartialOrd for PrioritizedTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PrioritizedTask {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher priority first
        match self.task.priority.cmp(&other.task.priority) {
            Ordering::Equal => {
                // Older tasks first (FIFO within same priority)
                other.enqueued_at.cmp(&self.enqueued_at)
            }
            other => other,
        }
    }
}

/// Scheduling algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulingStrategy {
    /// Round-robin across all available nodes
    RoundRobin,
    /// Least loaded node first
    LeastLoaded,
    /// Best fit based on GPU capabilities
    BestFit,
    /// Locality-aware (prefer same region)
    LocalityAware,
    /// Reputation-weighted random
    ReputationWeighted,
}

impl Default for SchedulingStrategy {
    fn default() -> Self {
        SchedulingStrategy::BestFit
    }
}

/// Task scheduler configuration
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// Scheduling strategy
    pub strategy: SchedulingStrategy,

    /// Maximum queue size
    pub max_queue_size: usize,

    /// Maximum tasks per node
    pub max_tasks_per_node: usize,

    /// Task timeout grace period (seconds)
    pub timeout_grace_secs: i64,

    /// Minimum reputation for assignment
    pub min_reputation: u32,

    /// Enable task stealing
    pub enable_task_stealing: bool,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            strategy: SchedulingStrategy::BestFit,
            max_queue_size: 10_000,
            max_tasks_per_node: 4,
            timeout_grace_secs: 30,
            min_reputation: 1000,
            enable_task_stealing: true,
        }
    }
}

/// Task scheduler for distributing work across nodes
pub struct TaskScheduler {
    /// Configuration
    config: SchedulerConfig,

    /// Priority queue for pending tasks
    pending_queue: BinaryHeap<PrioritizedTask>,

    /// Tasks currently being executed
    executing: HashMap<TaskId, TaskExecution>,

    /// Tasks by assigned node
    by_node: HashMap<NodeId, Vec<TaskId>>,

    /// Completed tasks (recent)
    completed: VecDeque<TaskExecution>,

    /// Round-robin index
    rr_index: usize,

    /// Task status tracking
    status: HashMap<TaskId, TaskStatus>,
}

impl TaskScheduler {
    /// Create a new scheduler
    pub fn new(config: SchedulerConfig) -> Self {
        Self {
            config,
            pending_queue: BinaryHeap::new(),
            executing: HashMap::new(),
            by_node: HashMap::new(),
            completed: VecDeque::with_capacity(1000),
            rr_index: 0,
            status: HashMap::new(),
        }
    }

    /// Submit a task to the queue
    pub fn submit(&mut self, task: Task) -> SwarmResult<TaskId> {
        if self.pending_queue.len() >= self.config.max_queue_size {
            return Err(SwarmError::QueueFull);
        }

        if task.is_expired() {
            return Err(SwarmError::TaskExpired(task.id));
        }

        let id = task.id;
        self.status.insert(id, TaskStatus::Pending);

        self.pending_queue.push(PrioritizedTask {
            task,
            enqueued_at: chrono::Utc::now().timestamp(),
        });

        Ok(id)
    }

    /// Get next task for a node
    pub fn next_task_for_node(
        &mut self,
        node: &SwarmNode,
        registry: &NodeRegistry,
    ) -> Option<Task> {
        // Check if node can accept more tasks
        let node_tasks = self.by_node.get(&node.id).map(|v| v.len()).unwrap_or(0);
        if node_tasks >= self.config.max_tasks_per_node {
            return None;
        }

        // Check node status
        if node.status != NodeStatus::Online {
            return None;
        }

        // Check reputation
        if node.metrics.reputation < self.config.min_reputation {
            return None;
        }

        // Find a suitable task
        self.find_task_for_node(node)
    }

    /// Schedule tasks across available nodes
    pub fn schedule_batch(&mut self, registry: &NodeRegistry) -> Vec<(TaskId, NodeId)> {
        let mut assignments = Vec::new();

        let online_nodes: Vec<_> = registry
            .online_nodes()
            .iter()
            .filter(|n| n.metrics.reputation >= self.config.min_reputation)
            .copied()
            .collect();

        if online_nodes.is_empty() {
            return assignments;
        }

        match self.config.strategy {
            SchedulingStrategy::RoundRobin => {
                assignments = self.schedule_round_robin(&online_nodes);
            }
            SchedulingStrategy::LeastLoaded => {
                assignments = self.schedule_least_loaded(&online_nodes);
            }
            SchedulingStrategy::BestFit => {
                assignments = self.schedule_best_fit(&online_nodes);
            }
            SchedulingStrategy::LocalityAware => {
                assignments = self.schedule_locality_aware(&online_nodes);
            }
            SchedulingStrategy::ReputationWeighted => {
                assignments = self.schedule_reputation_weighted(&online_nodes);
            }
        }

        assignments
    }

    /// Assign a task to a node
    pub fn assign(&mut self, task_id: TaskId, node_id: NodeId) -> SwarmResult<()> {
        // Verify task exists and is pending
        let status = self
            .status
            .get(&task_id)
            .ok_or(SwarmError::TaskNotFound(task_id))?;
        if *status != TaskStatus::Pending {
            return Err(SwarmError::InvalidTaskState {
                expected: TaskStatus::Pending,
                actual: *status,
            });
        }

        // Update status
        self.status.insert(task_id, TaskStatus::Assigned);

        // Track by node
        self.by_node.entry(node_id).or_default().push(task_id);

        // Create execution record
        self.executing.insert(
            task_id,
            TaskExecution {
                task_id,
                executor_node: node_id,
                started_at: chrono::Utc::now().timestamp(),
                completed_at: None,
                status: TaskStatus::Assigned,
                compute_units_used: 0,
                result_hash: None,
                error: None,
            },
        );

        Ok(())
    }

    /// Mark task as started
    pub fn mark_started(&mut self, task_id: TaskId) -> SwarmResult<()> {
        self.update_execution_status(task_id, TaskStatus::Executing)
    }

    /// Mark task as completed
    pub fn mark_completed(
        &mut self,
        task_id: TaskId,
        result_hash: [u8; 32],
        compute_units: u64,
    ) -> SwarmResult<()> {
        let execution = self
            .executing
            .get_mut(&task_id)
            .ok_or(SwarmError::TaskNotFound(task_id))?;

        execution.status = TaskStatus::Completed;
        execution.completed_at = Some(chrono::Utc::now().timestamp());
        execution.result_hash = Some(result_hash);
        execution.compute_units_used = compute_units;

        self.status.insert(task_id, TaskStatus::Completed);

        // Move to completed queue
        let execution = self
            .executing
            .remove(&task_id)
            .ok_or(SwarmError::TaskNotFound(task_id))?;
        let node_id = execution.executor_node;
        self.add_to_completed(execution);

        // Remove from node tracking
        self.remove_from_node(&node_id, task_id);

        Ok(())
    }

    /// Mark task as failed
    pub fn mark_failed(&mut self, task_id: TaskId, error: String) -> SwarmResult<()> {
        let execution = self
            .executing
            .get_mut(&task_id)
            .ok_or(SwarmError::TaskNotFound(task_id))?;

        execution.status = TaskStatus::Failed;
        execution.completed_at = Some(chrono::Utc::now().timestamp());
        execution.error = Some(error);

        self.status.insert(task_id, TaskStatus::Failed);

        // Move to completed queue
        if let Some(execution) = self.executing.remove(&task_id) {
            let node_id = execution.executor_node;
            self.add_to_completed(execution);

            // Remove from node tracking
            self.remove_from_node(&node_id, task_id);
        }

        Ok(())
    }

    /// Cancel a task
    pub fn cancel(&mut self, task_id: TaskId) -> SwarmResult<()> {
        let status = self
            .status
            .get(&task_id)
            .ok_or(SwarmError::TaskNotFound(task_id))?;

        match status {
            TaskStatus::Pending => {
                // Remove from pending queue (expensive, but cancellation is rare)
                let tasks: Vec<_> = self.pending_queue.drain().collect();
                for pt in tasks {
                    if pt.task.id != task_id {
                        self.pending_queue.push(pt);
                    }
                }
            }
            TaskStatus::Assigned | TaskStatus::Executing => {
                if let Some(mut execution) = self.executing.remove(&task_id) {
                    execution.status = TaskStatus::Cancelled;
                    execution.completed_at = Some(chrono::Utc::now().timestamp());
                    self.remove_from_node(&execution.executor_node, task_id);
                    self.add_to_completed(execution);
                }
            }
            _ => {
                return Err(SwarmError::InvalidTaskState {
                    expected: TaskStatus::Pending,
                    actual: *status,
                });
            }
        }

        self.status.insert(task_id, TaskStatus::Cancelled);
        Ok(())
    }

    /// Check for timed out tasks
    pub fn check_timeouts(&mut self) -> Vec<TaskId> {
        let now = chrono::Utc::now().timestamp();
        let grace = self.config.timeout_grace_secs;

        let mut timed_out = Vec::new();

        // Check pending queue
        let tasks: Vec<_> = self.pending_queue.drain().collect();
        for pt in tasks {
            if pt.task.is_expired() {
                timed_out.push(pt.task.id);
                self.status.insert(pt.task.id, TaskStatus::TimedOut);
            } else {
                self.pending_queue.push(pt);
            }
        }

        // Check executing tasks
        let executing_ids: Vec<_> = self.executing.keys().copied().collect();
        for task_id in executing_ids {
            if let Some(execution) = self.executing.get(&task_id) {
                // Get task timeout from somewhere (would need task data)
                // For now, use a default timeout check
                if now - execution.started_at > 600 + grace {
                    timed_out.push(task_id);
                    self.mark_failed(task_id, "Task timed out".to_string()).ok();
                }
            }
        }

        timed_out
    }

    /// Get task status
    pub fn get_status(&self, task_id: TaskId) -> Option<TaskStatus> {
        self.status.get(&task_id).copied()
    }

    /// Get queue statistics
    pub fn stats(&self) -> SchedulerStats {
        SchedulerStats {
            pending_count: self.pending_queue.len(),
            executing_count: self.executing.len(),
            completed_count: self.completed.len(),
            nodes_with_tasks: self.by_node.len(),
            avg_tasks_per_node: if self.by_node.is_empty() {
                0.0
            } else {
                self.executing.len() as f64 / self.by_node.len() as f64
            },
        }
    }

    // === Private Helper Methods ===

    fn find_task_for_node(&mut self, node: &SwarmNode) -> Option<Task> {
        // Find first suitable task from queue
        let mut suitable = None;
        let mut remaining = Vec::new();

        while let Some(pt) = self.pending_queue.pop() {
            let task_type_str = format!("{:?}", pt.task.task_type);
            if node.can_execute(&task_type_str, pt.task.estimated_compute_units()) {
                suitable = Some(pt.task);
                break;
            } else {
                remaining.push(pt);
            }
        }

        // Put back unsuitable tasks
        for pt in remaining {
            self.pending_queue.push(pt);
        }

        suitable
    }

    fn schedule_round_robin(&mut self, nodes: &[&SwarmNode]) -> Vec<(TaskId, NodeId)> {
        let mut assignments = Vec::new();

        while let Some(pt) = self.pending_queue.pop() {
            // Find next available node
            let start_idx = self.rr_index;
            loop {
                let node = nodes[self.rr_index % nodes.len()];
                self.rr_index = (self.rr_index + 1) % nodes.len();

                let node_tasks = self.by_node.get(&node.id).map(|v| v.len()).unwrap_or(0);
                if node_tasks < self.config.max_tasks_per_node {
                    assignments.push((pt.task.id, node.id));
                    break;
                }

                if self.rr_index == start_idx {
                    // No available nodes, put task back
                    self.pending_queue.push(pt);
                    return assignments;
                }
            }
        }

        assignments
    }

    fn schedule_least_loaded(&mut self, nodes: &[&SwarmNode]) -> Vec<(TaskId, NodeId)> {
        let mut assignments = Vec::new();

        while let Some(pt) = self.pending_queue.pop() {
            // Find least loaded node
            let mut best_node = None;
            let mut min_load = usize::MAX;

            for node in nodes {
                let load = self.by_node.get(&node.id).map(|v| v.len()).unwrap_or(0);
                if load < self.config.max_tasks_per_node && load < min_load {
                    min_load = load;
                    best_node = Some(node.id);
                }
            }

            match best_node {
                Some(node_id) => {
                    assignments.push((pt.task.id, node_id));
                }
                None => {
                    self.pending_queue.push(pt);
                    break;
                }
            }
        }

        assignments
    }

    fn schedule_best_fit(&mut self, nodes: &[&SwarmNode]) -> Vec<(TaskId, NodeId)> {
        let mut assignments = Vec::new();

        while let Some(pt) = self.pending_queue.pop() {
            let required_compute = pt.task.estimated_compute_units();

            // Find best fitting node (closest available capacity)
            let mut best_node = None;
            let mut best_score = f64::MAX;

            for node in nodes {
                let load = self.by_node.get(&node.id).map(|v| v.len()).unwrap_or(0);
                if load >= self.config.max_tasks_per_node {
                    continue;
                }

                let capacity = node.gpu.compute_capacity();
                let score = (capacity as f64 - required_compute as f64).abs();

                // Prefer nodes with matching capabilities
                let task_type_str = format!("{:?}", pt.task.task_type);
                if node.can_execute(&task_type_str, required_compute) && score < best_score {
                    best_score = score;
                    best_node = Some(node.id);
                }
            }

            match best_node {
                Some(node_id) => {
                    assignments.push((pt.task.id, node_id));
                }
                None => {
                    self.pending_queue.push(pt);
                    break;
                }
            }
        }

        assignments
    }

    fn schedule_locality_aware(&mut self, nodes: &[&SwarmNode]) -> Vec<(TaskId, NodeId)> {
        let mut assignments = Vec::new();

        while let Some(pt) = self.pending_queue.pop() {
            let preferred_regions = &pt.task.metadata.preferred_regions;

            // Try preferred regions first
            let mut best_node = None;

            if !preferred_regions.is_empty() {
                for region in preferred_regions {
                    for node in nodes {
                        if node.region == *region {
                            let load = self.by_node.get(&node.id).map(|v| v.len()).unwrap_or(0);
                            if load < self.config.max_tasks_per_node {
                                best_node = Some(node.id);
                                break;
                            }
                        }
                    }
                    if best_node.is_some() {
                        break;
                    }
                }
            }

            // Fall back to any available node
            if best_node.is_none() {
                for node in nodes {
                    let load = self.by_node.get(&node.id).map(|v| v.len()).unwrap_or(0);
                    if load < self.config.max_tasks_per_node {
                        best_node = Some(node.id);
                        break;
                    }
                }
            }

            match best_node {
                Some(node_id) => {
                    assignments.push((pt.task.id, node_id));
                }
                None => {
                    self.pending_queue.push(pt);
                    break;
                }
            }
        }

        assignments
    }

    fn schedule_reputation_weighted(&mut self, nodes: &[&SwarmNode]) -> Vec<(TaskId, NodeId)> {
        use rand::prelude::*;

        let mut assignments = Vec::new();
        let mut rng = rand::thread_rng();

        while let Some(pt) = self.pending_queue.pop() {
            // Build weighted list of available nodes
            let available: Vec<_> = nodes
                .iter()
                .filter(|n| {
                    let load = self.by_node.get(&n.id).map(|v| v.len()).unwrap_or(0);
                    load < self.config.max_tasks_per_node
                })
                .map(|n| (n.id, n.metrics.reputation as f64))
                .collect();

            if available.is_empty() {
                self.pending_queue.push(pt);
                break;
            }

            // Weighted random selection
            let total_weight: f64 = available.iter().map(|(_, w)| w).sum();
            let mut choice = rng.gen::<f64>() * total_weight;

            let mut selected = available[0].0;
            for (node_id, weight) in &available {
                choice -= weight;
                if choice <= 0.0 {
                    selected = *node_id;
                    break;
                }
            }

            assignments.push((pt.task.id, selected));
        }

        assignments
    }

    fn update_execution_status(
        &mut self,
        task_id: TaskId,
        new_status: TaskStatus,
    ) -> SwarmResult<()> {
        let execution = self
            .executing
            .get_mut(&task_id)
            .ok_or(SwarmError::TaskNotFound(task_id))?;

        execution.status = new_status;
        self.status.insert(task_id, new_status);

        Ok(())
    }

    fn add_to_completed(&mut self, execution: TaskExecution) {
        self.completed.push_back(execution);

        // Keep only recent completions
        while self.completed.len() > 1000 {
            self.completed.pop_front();
        }
    }

    fn remove_from_node(&mut self, node_id: &NodeId, task_id: TaskId) {
        if let Some(tasks) = self.by_node.get_mut(node_id) {
            tasks.retain(|id| *id != task_id);
            if tasks.is_empty() {
                self.by_node.remove(node_id);
            }
        }
    }
}

/// Scheduler statistics
#[derive(Debug, Clone)]
pub struct SchedulerStats {
    pub pending_count: usize,
    pub executing_count: usize,
    pub completed_count: usize,
    pub nodes_with_tasks: usize,
    pub avg_tasks_per_node: f64,
}
