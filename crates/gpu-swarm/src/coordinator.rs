//! Swarm coordinator - central task management and orchestration

use crate::config::SwarmConfig;
use crate::error::{SwarmError, SwarmResult};
use crate::node::{NodeId, NodeRegistry, SwarmNode, NodeStatus, GpuCapabilities};
use crate::protocol::*;
use crate::scheduler::{TaskScheduler, SchedulerConfig, SchedulerStats, SchedulingStrategy};
use crate::task::{Task, TaskId, TaskStatus};
use crate::verification::{ExecutionVerifier, VerificationConfig, VerificationSummary, Verdict};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock, broadcast};

/// Events emitted by the coordinator
#[derive(Debug, Clone)]
pub enum CoordinatorEvent {
    /// A node joined the swarm
    NodeJoined { node_id: NodeId, region: String },
    
    /// A node left the swarm
    NodeLeft { node_id: NodeId, reason: String },
    
    /// A task was submitted
    TaskSubmitted { task_id: TaskId, priority: u8 },
    
    /// A task was assigned to a node
    TaskAssigned { task_id: TaskId, node_id: NodeId },
    
    /// A task completed
    TaskCompleted { task_id: TaskId, success: bool },
    
    /// Verification completed
    VerificationCompleted { task_id: TaskId, verdict: Verdict },
    
    /// A node was slashed
    NodeSlashed { node_id: NodeId, amount: u64, reason: String },
    
    /// Metrics updated
    MetricsUpdated(CoordinatorMetrics),
}

/// Coordinator metrics
#[derive(Debug, Clone, Default)]
pub struct CoordinatorMetrics {
    /// Total nodes registered
    pub total_nodes: usize,
    
    /// Online nodes
    pub online_nodes: usize,
    
    /// Tasks in queue
    pub queued_tasks: usize,
    
    /// Tasks executing
    pub executing_tasks: usize,
    
    /// Tasks completed (last hour)
    pub completed_tasks_hour: u64,
    
    /// Total compute capacity
    pub total_compute_capacity: u64,
    
    /// Average task latency (ms)
    pub avg_task_latency_ms: u64,
    
    /// Total rewards distributed (last hour)
    pub rewards_distributed_hour: u64,
}

/// Coordinator configuration
#[derive(Debug, Clone)]
pub struct CoordinatorConfig {
    /// Node heartbeat timeout (seconds)
    pub heartbeat_timeout_secs: u64,
    
    /// Task timeout (seconds)
    pub task_timeout_secs: u64,
    
    /// Maximum concurrent tasks
    pub max_concurrent_tasks: usize,
    
    /// Minimum stake required
    pub min_stake: u64,
    
    /// Slashing percentage for misbehavior
    pub slash_percentage: u8,
    
    /// Reward pool distribution interval (seconds)
    pub reward_interval_secs: u64,
    
    /// Scheduler configuration
    pub scheduler: SchedulerConfig,
    
    /// Verifier configuration
    pub verifier: VerificationConfig,
}

impl Default for CoordinatorConfig {
    fn default() -> Self {
        Self {
            heartbeat_timeout_secs: 60,
            task_timeout_secs: 300,
            max_concurrent_tasks: 1000,
            min_stake: crate::MIN_NODE_STAKE,
            slash_percentage: 10,
            reward_interval_secs: 3600,
            scheduler: SchedulerConfig::default(),
            verifier: VerificationConfig::default(),
        }
    }
}

/// The swarm coordinator - manages nodes, tasks, and verification
pub struct SwarmCoordinator {
    /// Configuration
    config: CoordinatorConfig,
    
    /// Node registry
    nodes: Arc<RwLock<NodeRegistry>>,
    
    /// Task scheduler
    scheduler: Arc<RwLock<TaskScheduler>>,
    
    /// Execution verifier
    verifier: Arc<RwLock<ExecutionVerifier>>,
    
    /// Event broadcaster
    event_tx: broadcast::Sender<CoordinatorEvent>,
    
    /// Message queue (incoming)
    message_rx: mpsc::Receiver<MessageEnvelope>,
    
    /// Message sender (for responses)
    message_tx: mpsc::Sender<MessageEnvelope>,
    
    /// Coordinator's identity
    coordinator_id: NodeId,
    
    /// Metrics
    metrics: Arc<RwLock<CoordinatorMetrics>>,
    
    /// Running flag
    running: Arc<RwLock<bool>>,
}

impl SwarmCoordinator {
    /// Create a new coordinator
    pub fn new(
        config: CoordinatorConfig,
        coordinator_id: NodeId,
    ) -> (Self, mpsc::Sender<MessageEnvelope>, broadcast::Receiver<CoordinatorEvent>) {
        let (message_tx, message_rx) = mpsc::channel(1000);
        let (event_tx, event_rx) = broadcast::channel(100);
        
        let coordinator = Self {
            config: config.clone(),
            nodes: Arc::new(RwLock::new(NodeRegistry::new())),
            scheduler: Arc::new(RwLock::new(TaskScheduler::new(config.scheduler))),
            verifier: Arc::new(RwLock::new(ExecutionVerifier::new(config.verifier))),
            event_tx,
            message_rx,
            message_tx: message_tx.clone(),
            coordinator_id,
            metrics: Arc::new(RwLock::new(CoordinatorMetrics::default())),
            running: Arc::new(RwLock::new(false)),
        };
        
        (coordinator, message_tx, event_rx)
    }
    
    /// Start the coordinator
    pub async fn start(&mut self) -> SwarmResult<()> {
        {
            let mut running = self.running.write().await;
            *running = true;
        }
        
        // Start background tasks
        self.spawn_heartbeat_monitor();
        self.spawn_task_scheduler();
        self.spawn_timeout_checker();
        self.spawn_metrics_updater();
        
        // Main message processing loop
        self.run_message_loop().await
    }
    
    /// Stop the coordinator
    pub async fn stop(&self) {
        let mut running = self.running.write().await;
        *running = false;
    }
    
    /// Process incoming messages
    async fn run_message_loop(&mut self) -> SwarmResult<()> {
        while *self.running.read().await {
            match tokio::time::timeout(
                std::time::Duration::from_secs(1),
                self.message_rx.recv(),
            ).await {
                Ok(Some(envelope)) => {
                    if let Err(e) = self.handle_message(envelope).await {
                        tracing::warn!("Error handling message: {:?}", e);
                    }
                }
                Ok(None) => break, // Channel closed
                Err(_) => continue, // Timeout, check running flag
            }
        }
        Ok(())
    }
    
    /// Handle an incoming message
    async fn handle_message(&self, envelope: MessageEnvelope) -> SwarmResult<()> {
        match envelope.message {
            SwarmMessage::JoinRequest(req) => self.handle_join_request(req).await,
            SwarmMessage::LeaveNotification(notif) => self.handle_leave(notif).await,
            SwarmMessage::Heartbeat(hb) => self.handle_heartbeat(hb).await,
            SwarmMessage::TaskSubmission(sub) => self.handle_task_submission(sub).await,
            SwarmMessage::TaskStarted(started) => self.handle_task_started(started).await,
            SwarmMessage::TaskResult(result) => self.handle_task_result(result).await,
            SwarmMessage::VerificationResult(ver) => self.handle_verification_result(ver).await,
            _ => Ok(()), // Ignore other message types
        }
    }
    
    /// Handle join request
    async fn handle_join_request(&self, req: JoinRequest) -> SwarmResult<()> {
        // Verify minimum stake
        if req.stake < self.config.min_stake {
            let response = JoinResponse {
                accepted: false,
                reason: Some(format!(
                    "Insufficient stake: {} required, {} provided",
                    self.config.min_stake, req.stake
                )),
                bootstrap_peers: Vec::new(),
                current_epoch: 0,
                signature: Signature::default(), // TODO: Sign
            };
            self.send_response(req.node_id, SwarmMessage::JoinResponse(response)).await?;
            return Ok(());
        }
        
        // Create node
        let node = SwarmNode {
            id: req.node_id,
            peer_address: req.peer_address,
            region: req.region.clone(),
            gpu: req.gpu_capabilities,
            status: NodeStatus::Online,
            metrics: Default::default(),
            stake: req.stake,
            supported_tasks: req.supported_tasks,
            version: req.version,
            registered_at: chrono::Utc::now().timestamp(),
        };
        
        // Register node
        {
            let mut nodes = self.nodes.write().await;
            nodes.register(node)?;
        }
        
        // Get bootstrap peers
        let bootstrap_peers = {
            let nodes = self.nodes.read().await;
            nodes.online_nodes()
                .iter()
                .take(10)
                .map(|n| n.peer_address.clone())
                .collect()
        };
        
        // Send response
        let response = JoinResponse {
            accepted: true,
            reason: None,
            bootstrap_peers,
            current_epoch: 0, // TODO: Track epoch
            signature: Signature::default(), // TODO: Sign
        };
        self.send_response(req.node_id, SwarmMessage::JoinResponse(response)).await?;
        
        // Emit event
        self.emit_event(CoordinatorEvent::NodeJoined {
            node_id: req.node_id,
            region: req.region,
        });
        
        Ok(())
    }
    
    /// Handle leave notification
    async fn handle_leave(&self, notif: LeaveNotification) -> SwarmResult<()> {
        {
            let mut nodes = self.nodes.write().await;
            nodes.unregister(&notif.node_id);
        }
        
        self.emit_event(CoordinatorEvent::NodeLeft {
            node_id: notif.node_id,
            reason: notif.reason,
        });
        
        Ok(())
    }
    
    /// Handle heartbeat
    async fn handle_heartbeat(&self, hb: Heartbeat) -> SwarmResult<()> {
        let mut nodes = self.nodes.write().await;
        
        if let Some(node) = nodes.get_mut(&hb.node_id) {
            node.heartbeat();
            node.metrics = hb.metrics;
            node.gpu.available_vram = hb.available_vram;
        }
        
        // Send ack with any pending tasks
        let scheduler = self.scheduler.read().await;
        let pending_tasks = Vec::new(); // TODO: Get pending tasks for this node
        
        let ack = HeartbeatAck {
            timestamp: hb.timestamp,
            pending_tasks,
        };
        
        drop(nodes);
        drop(scheduler);
        
        self.send_response(hb.node_id, SwarmMessage::HeartbeatAck(ack)).await
    }
    
    /// Handle task submission
    async fn handle_task_submission(&self, sub: TaskSubmission) -> SwarmResult<()> {
        let task_id = sub.task.id;
        let priority = sub.task.priority as u8;
        
        // Add to scheduler
        {
            let mut scheduler = self.scheduler.write().await;
            scheduler.submit(sub.task)?;
        }
        
        self.emit_event(CoordinatorEvent::TaskSubmitted { task_id, priority });
        
        // Trigger scheduling
        self.schedule_tasks().await?;
        
        Ok(())
    }
    
    /// Handle task started notification
    async fn handle_task_started(&self, started: TaskStarted) -> SwarmResult<()> {
        let mut scheduler = self.scheduler.write().await;
        scheduler.mark_started(started.task_id)?;
        Ok(())
    }
    
    /// Handle task result
    async fn handle_task_result(&self, result: TaskResult) -> SwarmResult<()> {
        let task_id = result.task_id;
        
        if result.success {
            // Start verification
            let verifiers = self.select_verifiers(&result.executor, 2).await?;
            
            // TODO: Get original task from scheduler
            // For now, we'll skip verification for the prototype
            
            {
                let mut scheduler = self.scheduler.write().await;
                scheduler.mark_completed(task_id, result.result_hash, result.compute_units)?;
            }
            
            self.emit_event(CoordinatorEvent::TaskCompleted {
                task_id,
                success: true,
            });
        } else {
            let mut scheduler = self.scheduler.write().await;
            scheduler.mark_failed(task_id, result.error.unwrap_or_default())?;
            
            self.emit_event(CoordinatorEvent::TaskCompleted {
                task_id,
                success: false,
            });
        }
        
        Ok(())
    }
    
    /// Handle verification result
    async fn handle_verification_result(&self, ver: VerificationResult) -> SwarmResult<()> {
        let mut verifier = self.verifier.write().await;
        
        if let Some(summary) = verifier.submit_verification(ver.task_id, ver)? {
            self.emit_event(CoordinatorEvent::VerificationCompleted {
                task_id: summary.task_id,
                verdict: summary.verdict,
            });
            
            // Handle slashing for invalid results
            if summary.verdict == Verdict::Invalid {
                self.slash_for_invalid_result(&summary).await?;
            }
        }
        
        Ok(())
    }
    
    /// Schedule pending tasks
    async fn schedule_tasks(&self) -> SwarmResult<()> {
        // Collect all assignments first
        let assignments = {
            let nodes = self.nodes.read().await;
            let mut scheduler = self.scheduler.write().await;
            scheduler.schedule_batch(&nodes)
        };
        
        // Process assignments outside of lock scope
        for (task_id, node_id) in assignments {
            {
                let mut scheduler = self.scheduler.write().await;
                scheduler.assign(task_id, node_id)?;
            }
            
            self.emit_event(CoordinatorEvent::TaskAssigned { task_id, node_id });
            
            // Send assignment to node
            let assignment = TaskAssignment {
                task_id,
                primary_executor: node_id,
                verifiers: Vec::new(), // TODO: Select verifiers
                assigned_at: chrono::Utc::now().timestamp(),
                signature: Signature::default(),
            };
            
            self.send_response(node_id, SwarmMessage::TaskAssignment(assignment)).await?;
        }
        
        Ok(())
    }
    
    /// Select verifier nodes
    async fn select_verifiers(&self, executor: &NodeId, count: usize) -> SwarmResult<Vec<NodeId>> {
        let nodes = self.nodes.read().await;
        
        let verifiers: Vec<_> = nodes.online_nodes()
            .iter()
            .filter(|n| n.id != *executor && n.metrics.reputation >= 5000)
            .take(count)
            .map(|n| n.id)
            .collect();
        
        Ok(verifiers)
    }
    
    /// Slash a node for invalid result
    async fn slash_for_invalid_result(&self, summary: &VerificationSummary) -> SwarmResult<()> {
        // Find the executor from invalid voters... this is simplified
        // In practice, we'd track which node produced the invalid result
        
        for node_id in &summary.invalid_voters {
            let mut nodes = self.nodes.write().await;
            if let Some(node) = nodes.get_mut(node_id) {
                let slash_amount = (node.stake * self.config.slash_percentage as u64) / 100;
                node.stake = node.stake.saturating_sub(slash_amount);
                
                self.emit_event(CoordinatorEvent::NodeSlashed {
                    node_id: *node_id,
                    amount: slash_amount,
                    reason: "Invalid execution result".to_string(),
                });
            }
        }
        
        Ok(())
    }
    
    /// Send a response to a node
    async fn send_response(&self, _target: NodeId, message: SwarmMessage) -> SwarmResult<()> {
        let envelope = MessageEnvelope::new(self.coordinator_id, message);
        
        // In practice, this would send via the network layer
        // For now, we just log it
        tracing::debug!("Sending message: {:?}", envelope);
        
        Ok(())
    }
    
    /// Emit an event
    fn emit_event(&self, event: CoordinatorEvent) {
        let _ = self.event_tx.send(event);
    }
    
    /// Spawn heartbeat monitor task
    fn spawn_heartbeat_monitor(&self) {
        let nodes = Arc::clone(&self.nodes);
        let timeout = self.config.heartbeat_timeout_secs;
        let running = Arc::clone(&self.running);
        let event_tx = self.event_tx.clone();
        
        tokio::spawn(async move {
            while *running.read().await {
                tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                
                let mut nodes_guard = nodes.write().await;
                let stale: Vec<_> = nodes_guard.online_nodes()
                    .iter()
                    .filter(|n| n.is_stale(timeout as i64))
                    .map(|n| n.id)
                    .collect();
                
                for node_id in stale {
                    nodes_guard.update_status(&node_id, NodeStatus::Offline).ok();
                    let _ = event_tx.send(CoordinatorEvent::NodeLeft {
                        node_id,
                        reason: "Heartbeat timeout".to_string(),
                    });
                }
            }
        });
    }
    
    /// Spawn task scheduler task
    fn spawn_task_scheduler(&self) {
        // Scheduling happens on-demand in this implementation
    }
    
    /// Spawn timeout checker task
    fn spawn_timeout_checker(&self) {
        let scheduler = Arc::clone(&self.scheduler);
        let verifier = Arc::clone(&self.verifier);
        let running = Arc::clone(&self.running);
        let event_tx = self.event_tx.clone();
        
        tokio::spawn(async move {
            while *running.read().await {
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                
                // Check scheduler timeouts
                {
                    let mut sched = scheduler.write().await;
                    let timed_out = sched.check_timeouts();
                    for task_id in timed_out {
                        let _ = event_tx.send(CoordinatorEvent::TaskCompleted {
                            task_id,
                            success: false,
                        });
                    }
                }
                
                // Check verification timeouts
                {
                    let mut ver = verifier.write().await;
                    let timed_out = ver.check_timeouts();
                    for summary in timed_out {
                        let _ = event_tx.send(CoordinatorEvent::VerificationCompleted {
                            task_id: summary.task_id,
                            verdict: summary.verdict,
                        });
                    }
                }
            }
        });
    }
    
    /// Spawn metrics updater task
    fn spawn_metrics_updater(&self) {
        let nodes = Arc::clone(&self.nodes);
        let scheduler = Arc::clone(&self.scheduler);
        let metrics = Arc::clone(&self.metrics);
        let running = Arc::clone(&self.running);
        let event_tx = self.event_tx.clone();
        
        tokio::spawn(async move {
            while *running.read().await {
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                
                let nodes_guard = nodes.read().await;
                let sched_guard = scheduler.read().await;
                let stats = sched_guard.stats();
                
                let online_count = nodes_guard.online_nodes().len();
                let total_capacity = nodes_guard.total_compute_capacity();
                let status_counts = nodes_guard.count_by_status();
                
                let new_metrics = CoordinatorMetrics {
                    total_nodes: status_counts.values().sum(),
                    online_nodes: online_count,
                    queued_tasks: stats.pending_count,
                    executing_tasks: stats.executing_count,
                    completed_tasks_hour: 0, // TODO: Track
                    total_compute_capacity: total_capacity,
                    avg_task_latency_ms: 0, // TODO: Track
                    rewards_distributed_hour: 0, // TODO: Track
                };
                
                {
                    let mut m = metrics.write().await;
                    *m = new_metrics.clone();
                }
                
                let _ = event_tx.send(CoordinatorEvent::MetricsUpdated(new_metrics));
            }
        });
    }
    
    /// Get current metrics
    pub async fn metrics(&self) -> CoordinatorMetrics {
        self.metrics.read().await.clone()
    }
    
    /// Get scheduler stats
    pub async fn scheduler_stats(&self) -> SchedulerStats {
        self.scheduler.read().await.stats()
    }
}
