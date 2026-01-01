//! Swarm Executor - Autonomous Agent Job Execution Engine
//!
//! This module provides:
//! - Job queue management
//! - Agent task scheduling
//! - State mutation execution
//! - Agent communication protocol

pub mod coordinator;
pub mod scheduler;

pub use coordinator::{AgentCoordinator, Message, Proposal, VoteChoice, AlertSeverity};
pub use scheduler::{JobScheduler, SchedulerConfig, SchedulerBuilder, ExecutionStats};

use serde::{Deserialize, Serialize};
use sp_core::H256;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

/// Unique job identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct JobId(pub u64);

/// Unique agent identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub String);

/// Job status lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    /// Queued but not yet executed
    Pending,
    /// Currently executing
    Executing,
    /// Successfully completed
    Completed,
    /// Failed with error
    Failed,
    /// Cancelled by user
    Cancelled,
}

/// Job priority for scheduling
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum JobPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

/// Represents a job to be executed by an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: JobId,
    pub agent_id: AgentId,
    pub priority: JobPriority,
    pub status: JobStatus,
    pub action: AgentAction,
    pub created_at: u64,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
    pub result: Option<JobResult>,
    pub error: Option<String>,
}

/// Types of actions agents can perform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentAction {
    /// Transfer assets between accounts
    Transfer {
        from: String,
        to: String,
        amount: u128,
        asset_id: u32,
    },
    /// Execute a smart contract
    ExecuteContract {
        contract_address: String,
        method: String,
        args: Vec<u8>,
    },
    /// Call an X3 DeFi operation
    CallX3Function {
        module: String,
        function: String,
        args: Vec<u8>,
    },
    /// Query chain state
    QueryState {
        key: String,
    },
    /// Emit an event
    EmitEvent {
        event_name: String,
        data: Vec<u8>,
    },
}

/// Result of job execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobResult {
    pub success: bool,
    pub output: Vec<u8>,
    pub state_changes: u32,
    pub gas_used: u64,
}

/// Agent capabilities and configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: AgentId,
    pub name: String,
    pub description: String,
    pub owner: String,
    pub enabled: bool,
    pub created_at: u64,
    pub capabilities: Vec<String>,
    pub max_concurrent_jobs: u32,
    pub current_jobs: u32,
}

/// Swarm executor for managing agents and their jobs
pub struct SwarmExecutor {
    jobs: Arc<RwLock<HashMap<JobId, Job>>>,
    agents: Arc<RwLock<HashMap<AgentId, Agent>>>,
    job_counter: Arc<RwLock<u64>>,
}

impl SwarmExecutor {
    /// Create a new swarm executor
    pub fn new() -> Self {
        Self {
            jobs: Arc::new(RwLock::new(HashMap::new())),
            agents: Arc::new(RwLock::new(HashMap::new())),
            job_counter: Arc::new(RwLock::new(0)),
        }
    }

    /// Register a new agent
    pub async fn register_agent(&self, agent: Agent) -> Result<AgentId, String> {
        let mut agents = self.agents.write().await;
        let agent_id = agent.id.clone();
        agents.insert(agent_id.clone(), agent);
        Ok(agent_id)
    }

    /// Get agent by ID
    pub async fn get_agent(&self, agent_id: &AgentId) -> Result<Option<Agent>, String> {
        let agents = self.agents.read().await;
        Ok(agents.get(agent_id).cloned())
    }

    /// List all agents
    pub async fn list_agents(&self) -> Result<Vec<Agent>, String> {
        let agents = self.agents.read().await;
        Ok(agents.values().cloned().collect())
    }

    /// Submit a job for execution
    pub async fn submit_job(
        &self,
        agent_id: AgentId,
        action: AgentAction,
        priority: JobPriority,
    ) -> Result<JobId, String> {
        // Verify agent exists
        let agents = self.agents.read().await;
        let agent = agents
            .get(&agent_id)
            .ok_or("Agent not found".to_string())?;

        if !agent.enabled {
            return Err("Agent is disabled".to_string());
        }

        // Check job capacity
        if agent.current_jobs >= agent.max_concurrent_jobs {
            return Err("Agent at max concurrent jobs".to_string());
        }

        drop(agents);

        // Create job
        let mut counter = self.job_counter.write().await;
        *counter += 1;
        let job_id = JobId(*counter);
        drop(counter);

        let now = current_timestamp();
        let job = Job {
            id: job_id,
            agent_id: agent_id.clone(),
            priority,
            status: JobStatus::Pending,
            action,
            created_at: now,
            started_at: None,
            completed_at: None,
            result: None,
            error: None,
        };

        let mut jobs = self.jobs.write().await;
        jobs.insert(job_id, job);

        Ok(job_id)
    }

    /// Get job by ID
    pub async fn get_job(&self, job_id: JobId) -> Result<Option<Job>, String> {
        let jobs = self.jobs.read().await;
        Ok(jobs.get(&job_id).cloned())
    }

    /// List all jobs for an agent
    pub async fn list_agent_jobs(&self, agent_id: &AgentId) -> Result<Vec<Job>, String> {
        let jobs = self.jobs.read().await;
        Ok(jobs
            .values()
            .filter(|j| j.agent_id == *agent_id)
            .cloned()
            .collect())
    }

    /// List pending jobs (ordered by priority)
    pub async fn list_pending_jobs(&self) -> Result<Vec<Job>, String> {
        let jobs = self.jobs.read().await;
        let mut pending: Vec<_> = jobs
            .values()
            .filter(|j| j.status == JobStatus::Pending)
            .cloned()
            .collect();
        
        // Sort by priority (highest first) then by creation time (oldest first)
        pending.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then_with(|| a.created_at.cmp(&b.created_at))
        });
        
        Ok(pending)
    }

    /// Execute a job
    pub async fn execute_job(&self, job_id: JobId) -> Result<JobResult, String> {
        let mut jobs = self.jobs.write().await;
        let job = jobs
            .get_mut(&job_id)
            .ok_or("Job not found".to_string())?;

        // Update status to executing
        job.status = JobStatus::Executing;
        job.started_at = Some(current_timestamp());

        // In production, this would actually execute the action
        // For now, return a dummy result
        let result = JobResult {
            success: true,
            output: vec![],
            state_changes: 0,
            gas_used: 0,
        };

        job.status = JobStatus::Completed;
        job.completed_at = Some(current_timestamp());
        job.result = Some(result.clone());

        Ok(result)
    }

    /// Cancel a job
    pub async fn cancel_job(&self, job_id: JobId) -> Result<(), String> {
        let mut jobs = self.jobs.write().await;
        let job = jobs
            .get_mut(&job_id)
            .ok_or("Job not found".to_string())?;

        if job.status != JobStatus::Pending {
            return Err("Can only cancel pending jobs".to_string());
        }

        job.status = JobStatus::Cancelled;
        Ok(())
    }

    /// Get job statistics
    pub async fn get_stats(&self) -> Result<SwarmStats, String> {
        let jobs = self.jobs.read().await;
        let agents = self.agents.read().await;

        let total_jobs = jobs.len() as u32;
        let pending = jobs
            .values()
            .filter(|j| j.status == JobStatus::Pending)
            .count() as u32;
        let executing = jobs
            .values()
            .filter(|j| j.status == JobStatus::Executing)
            .count() as u32;
        let completed = jobs
            .values()
            .filter(|j| j.status == JobStatus::Completed)
            .count() as u32;
        let failed = jobs
            .values()
            .filter(|j| j.status == JobStatus::Failed)
            .count() as u32;

        Ok(SwarmStats {
            total_agents: agents.len() as u32,
            active_agents: agents
                .values()
                .filter(|a| a.enabled && a.current_jobs > 0)
                .count() as u32,
            total_jobs,
            pending,
            executing,
            completed,
            failed,
        })
    }
}

impl Default for SwarmExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the swarm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmStats {
    pub total_agents: u32,
    pub active_agents: u32,
    pub total_jobs: u32,
    pub pending: u32,
    pub executing: u32,
    pub completed: u32,
    pub failed: u32,
}

/// Get current Unix timestamp
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_agent() {
        let executor = SwarmExecutor::new();
        
        let agent = Agent {
            id: AgentId("agent-1".to_string()),
            name: "Test Agent".to_string(),
            description: "A test agent".to_string(),
            owner: "owner-1".to_string(),
            enabled: true,
            created_at: current_timestamp(),
            capabilities: vec!["transfer".to_string()],
            max_concurrent_jobs: 10,
            current_jobs: 0,
        };
        
        let id = executor.register_agent(agent.clone()).await.unwrap();
        let retrieved = executor.get_agent(&id).await.unwrap();
        
        assert_eq!(retrieved, Some(agent));
    }

    #[tokio::test]
    async fn test_submit_job() {
        let executor = SwarmExecutor::new();
        
        let agent = Agent {
            id: AgentId("agent-1".to_string()),
            name: "Test Agent".to_string(),
            description: "A test agent".to_string(),
            owner: "owner-1".to_string(),
            enabled: true,
            created_at: current_timestamp(),
            capabilities: vec!["transfer".to_string()],
            max_concurrent_jobs: 10,
            current_jobs: 0,
        };
        
        executor.register_agent(agent).await.unwrap();
        
        let action = AgentAction::Transfer {
            from: "account-1".to_string(),
            to: "account-2".to_string(),
            amount: 1000,
            asset_id: 0,
        };
        
        let job_id = executor
            .submit_job(
                AgentId("agent-1".to_string()),
                action,
                JobPriority::Normal,
            )
            .await
            .unwrap();
        
        let job = executor.get_job(job_id).await.unwrap();
        assert!(job.is_some());
        assert_eq!(job.unwrap().status, JobStatus::Pending);
    }

    #[test]
    fn test_job_priority_ordering() {
        let j1 = Job {
            id: JobId(1),
            agent_id: AgentId("a".to_string()),
            priority: JobPriority::Low,
            status: JobStatus::Pending,
            action: AgentAction::EmitEvent {
                event_name: "test".to_string(),
                data: vec![],
            },
            created_at: 100,
            started_at: None,
            completed_at: None,
            result: None,
            error: None,
        };

        let j2 = Job {
            id: JobId(2),
            agent_id: AgentId("a".to_string()),
            priority: JobPriority::High,
            status: JobStatus::Pending,
            action: AgentAction::EmitEvent {
                event_name: "test".to_string(),
                data: vec![],
            },
            created_at: 50,
            started_at: None,
            completed_at: None,
            result: None,
            error: None,
        };

        assert!(j2.priority > j1.priority);
    }
}
