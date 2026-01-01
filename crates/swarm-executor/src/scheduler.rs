//! Job Scheduler - Intelligent Job Processing & Execution Pipeline
//!
//! Provides:
//! - Periodic job execution
//! - Workload balancing
//! - Agent availability management
//! - Execution guarantees

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use super::*;

/// Job scheduler configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    /// Polling interval for checking pending jobs
    pub poll_interval: Duration,
    /// Maximum jobs to execute per cycle
    pub max_jobs_per_cycle: u32,
    /// Max time to wait for job execution
    pub job_timeout: Duration,
    /// Enable fairness (round-robin between agents)
    pub enable_fairness: bool,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_secs(6), // Substrate block time
            max_jobs_per_cycle: 10,
            job_timeout: Duration::from_secs(60),
            enable_fairness: true,
        }
    }
}

/// Builder for scheduler configuration
pub struct SchedulerBuilder {
    config: SchedulerConfig,
}

impl SchedulerBuilder {
    pub fn new() -> Self {
        Self {
            config: SchedulerConfig::default(),
        }
    }

    pub fn poll_interval(mut self, duration: Duration) -> Self {
        self.config.poll_interval = duration;
        self
    }

    pub fn max_jobs_per_cycle(mut self, count: u32) -> Self {
        self.config.max_jobs_per_cycle = count;
        self
    }

    pub fn job_timeout(mut self, duration: Duration) -> Self {
        self.config.job_timeout = duration;
        self
    }

    pub fn enable_fairness(mut self, enabled: bool) -> Self {
        self.config.enable_fairness = enabled;
        self
    }

    pub fn build(self) -> SchedulerConfig {
        self.config
    }
}

impl Default for SchedulerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Job scheduler service
pub struct JobScheduler {
    executor: Arc<SwarmExecutor>,
    config: SchedulerConfig,
    last_executed_agent: Arc<RwLock<Option<AgentId>>>,
    execution_stats: Arc<RwLock<ExecutionStats>>,
}

/// Execution statistics
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct ExecutionStats {
    pub total_executed: u64,
    pub total_succeeded: u64,
    pub total_failed: u64,
    pub average_execution_time_ms: u64,
}

impl JobScheduler {
    /// Create a new job scheduler
    pub fn new(executor: Arc<SwarmExecutor>, config: SchedulerConfig) -> Self {
        Self {
            executor,
            config,
            last_executed_agent: Arc::new(RwLock::new(None)),
            execution_stats: Arc::new(RwLock::new(ExecutionStats::default())),
        }
    }

    /// Start the scheduler (runs indefinitely)
    pub async fn start(&self) -> Result<(), String> {
        loop {
            tokio::time::sleep(self.config.poll_interval).await;
            
            match self.process_jobs().await {
                Ok(count) => {
                    if count > 0 {
                        println!("✓ Processed {} jobs", count);
                    }
                }
                Err(e) => {
                    eprintln!("⚠️ Scheduler error: {}", e);
                }
            }
        }
    }

    /// Process pending jobs (can be called manually)
    pub async fn process_jobs(&self) -> Result<u32, String> {
        let pending = self.executor.list_pending_jobs().await?;
        
        if pending.is_empty() {
            return Ok(0);
        }

        let jobs_to_process = std::cmp::min(
            pending.len() as u32,
            self.config.max_jobs_per_cycle,
        );

        let mut executed = 0;
        for job in pending.iter().take(jobs_to_process as usize) {
            if self.execute_job(job.id).await.is_ok() {
                executed += 1;
            }
        }

        Ok(executed)
    }

    /// Execute a single job
    async fn execute_job(&self, job_id: JobId) -> Result<(), String> {
        let start = std::time::Instant::now();

        // Execute the job
        match self.executor.execute_job(job_id).await {
            Ok(_result) => {
                let elapsed = start.elapsed().as_millis() as u64;
                
                let mut stats = self.execution_stats.write().await;
                stats.total_executed += 1;
                stats.total_succeeded += 1;
                stats.average_execution_time_ms = 
                    (stats.average_execution_time_ms + elapsed) / 2;

                Ok(())
            }
            Err(e) => {
                let mut stats = self.execution_stats.write().await;
                stats.total_executed += 1;
                stats.total_failed += 1;

                Err(e)
            }
        }
    }

    /// Get next executable job (respecting fairness)
    pub async fn get_next_job(&self) -> Result<Option<Job>, String> {
        let pending = self.executor.list_pending_jobs().await?;
        
        if pending.is_empty() {
            return Ok(None);
        }

        if !self.config.enable_fairness {
            return Ok(pending.first().cloned());
        }

        // Fairness: rotate through agents
        let last_agent = self.last_executed_agent.read().await.clone();
        
        let next_job = if let Some(last_agent) = last_agent {
            // Find next job from different agent
            pending
                .iter()
                .find(|j| j.agent_id != last_agent)
                .cloned()
                .or_else(|| pending.first().cloned())
        } else {
            pending.first().cloned()
        };

        if let Some(job) = &next_job {
            let mut last = self.last_executed_agent.write().await;
            *last = Some(job.agent_id.clone());
        }

        Ok(next_job)
    }

    /// Get execution statistics
    pub async fn get_stats(&self) -> Result<ExecutionStats, String> {
        let stats = self.execution_stats.read().await;
        Ok(*stats)
    }

    /// Reset statistics
    pub async fn reset_stats(&self) -> Result<(), String> {
        let mut stats = self.execution_stats.write().await;
        *stats = ExecutionStats::default();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler_builder() {
        let config = SchedulerBuilder::new()
            .poll_interval(Duration::from_secs(3))
            .max_jobs_per_cycle(20)
            .enable_fairness(false)
            .build();

        assert_eq!(config.poll_interval, Duration::from_secs(3));
        assert_eq!(config.max_jobs_per_cycle, 20);
        assert!(!config.enable_fairness);
    }

    #[tokio::test]
    async fn test_process_jobs() {
        let executor = Arc::new(SwarmExecutor::new());
        let scheduler = JobScheduler::new(executor.clone(), SchedulerConfig::default());

        // Register an agent
        let agent = Agent {
            id: AgentId("agent-1".to_string()),
            name: "Test Agent".to_string(),
            description: "A test agent".to_string(),
            owner: "owner-1".to_string(),
            enabled: true,
            created_at: 0,
            capabilities: vec![],
            max_concurrent_jobs: 10,
            current_jobs: 0,
        };

        executor.register_agent(agent).await.unwrap();

        // Submit a job
        executor
            .submit_job(
                AgentId("agent-1".to_string()),
                AgentAction::EmitEvent {
                    event_name: "test".to_string(),
                    data: vec![],
                },
                JobPriority::Normal,
            )
            .await
            .unwrap();

        // Process jobs
        let count = scheduler.process_jobs().await.unwrap();
        assert_eq!(count, 1);

        // Check stats
        let stats = scheduler.get_stats().await.unwrap();
        assert_eq!(stats.total_executed, 1);
        assert_eq!(stats.total_succeeded, 1);
    }
}
