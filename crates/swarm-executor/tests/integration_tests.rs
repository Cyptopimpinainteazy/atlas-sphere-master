//! Integration Tests for Swarm Executor
//!
//! Tests the complete job lifecycle, agent consensus, and scheduler operations.

use swarm_executor::*;
use std::sync::Arc;

// ============================================================================
// TEST 1: AGENT JOB LIFECYCLE
// ============================================================================

#[tokio::test]
async fn test_agent_job_lifecycle() {
    let executor = Arc::new(SwarmExecutor::new());
    
    // Step 1: Register an agent
    let agent = Agent {
        id: AgentId("agent-lifecycle-1".to_string()),
        name: "Test Agent".to_string(),
        description: "For job lifecycle testing".to_string(),
        owner: "test-owner".to_string(),
        enabled: true,
        created_at: 0,
        capabilities: vec!["transfer".to_string(), "query".to_string()],
        max_concurrent_jobs: 10,
        current_jobs: 0,
    };
    
    let agent_id = executor.register_agent(agent.clone()).await.unwrap();
    assert_eq!(agent_id, AgentId("agent-lifecycle-1".to_string()));
    
    // Step 2: Verify agent is registered
    let retrieved = executor.get_agent(&agent_id).await.unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().name, "Test Agent");
    
    // Step 3: Submit a job
    let action = AgentAction::Transfer {
        from: "alice".to_string(),
        to: "bob".to_string(),
        amount: 1000,
        asset_id: 0,
    };
    
    let job_id = executor
        .submit_job(agent_id.clone(), action.clone(), JobPriority::Normal)
        .await
        .unwrap();
    
    // Step 4: Verify job is pending
    let job = executor.get_job(job_id).await.unwrap().unwrap();
    assert_eq!(job.status, JobStatus::Pending);
    assert_eq!(job.agent_id, agent_id);
    assert_eq!(job.priority, JobPriority::Normal);
    
    // Step 5: Execute the job
    executor.execute_job(job_id).await.unwrap();
    
    // Step 6: Verify job is completed
    let completed_job = executor.get_job(job_id).await.unwrap().unwrap();
    assert_eq!(completed_job.status, JobStatus::Completed);
    assert!(completed_job.result.is_some());
    
    // Step 7: Verify statistics updated
    let stats = executor.get_stats().await.unwrap();
    assert_eq!(stats.total_jobs, 1);
    assert_eq!(stats.completed, 1);
}

// ============================================================================
// TEST 2: MULTI-AGENT CONSENSUS VOTING
// ============================================================================

#[tokio::test]
async fn test_multi_agent_consensus_voting() {
    let executor = Arc::new(SwarmExecutor::new());
    let coordinator = AgentCoordinator::new(executor.clone());
    
    // Register 3 agents
    let agents = vec![
        ("agent-consensus-1", "Alice Agent"),
        ("agent-consensus-2", "Bob Agent"),
        ("agent-consensus-3", "Charlie Agent"),
    ];
    
    let agent_ids: Vec<_> = agents
        .iter()
        .map(|(id, name)| {
            let agent = Agent {
                id: AgentId(id.to_string()),
                name: name.to_string(),
                description: "Consensus voter".to_string(),
                owner: "test".to_string(),
                enabled: true,
                created_at: 0,
                capabilities: vec!["vote".to_string()],
                max_concurrent_jobs: 10,
                current_jobs: 0,
            };
            AgentId(id.to_string())
        })
        .collect();
    
    for (i, agent_id) in agent_ids.iter().enumerate() {
        let agent = Agent {
            id: agent_id.clone(),
            name: agents[i].1.to_string(),
            description: "Consensus voter".to_string(),
            owner: "test".to_string(),
            enabled: true,
            created_at: 0,
            capabilities: vec!["vote".to_string()],
            max_concurrent_jobs: 10,
            current_jobs: 0,
        };
        executor.register_agent(agent).await.unwrap();
    }
    
    // Agent 1 creates a proposal
    let proposal_id = coordinator
        .create_proposal(
            agent_ids[0].clone(),
            "Market Action".to_string(),
            "Execute token swap".to_string(),
            agent_ids.clone(),
        )
        .await
        .unwrap();
    
    assert_ne!(proposal_id, 0);
    
    // Verify proposal exists
    let proposal = coordinator.get_proposal(proposal_id).await.unwrap();
    assert!(proposal.is_some());
    let prop = proposal.unwrap();
    assert_eq!(prop.recipients.len(), 3);
    assert!(!prop.has_consensus());
    
    // Agent 1 votes YES
    coordinator
        .vote_on_proposal(proposal_id, agent_ids[0].clone(), VoteChoice::Yes)
        .await
        .unwrap();
    
    // Agent 2 votes YES
    coordinator
        .vote_on_proposal(proposal_id, agent_ids[1].clone(), VoteChoice::Yes)
        .await
        .unwrap();
    
    // Verify consensus reached (2/3 > 50%)
    let proposal = coordinator.get_proposal(proposal_id).await.unwrap().unwrap();
    assert!(proposal.has_consensus());
    
    let status = proposal.voting_status();
    assert_eq!(status.yes_votes, 2);
    assert_eq!(status.consensus_reached, true);
}

// ============================================================================
// TEST 3: BLOCK INDEXER + AGENT INTERACTION
// ============================================================================

#[tokio::test]
async fn test_indexer_agent_integration() {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    let executor = Arc::new(SwarmExecutor::new());
    
    // Register an agent
    let agent = Agent {
        id: AgentId("agent-indexer-1".to_string()),
        name: "Indexer Monitor".to_string(),
        description: "Monitors blocks for trigger events".to_string(),
        owner: "test".to_string(),
        enabled: true,
        created_at: 0,
        capabilities: vec!["execute".to_string()],
        max_concurrent_jobs: 10,
        current_jobs: 0,
    };
    
    executor.register_agent(agent).await.unwrap();
    
    // Simulate: New block arrives with events
    // This would normally come from BlockWatcher
    
    // Agent detects block event and submits job
    let action = AgentAction::QueryState {
        key: "block:42:events".to_string(),
    };
    
    let job_id = executor
        .submit_job(
            AgentId("agent-indexer-1".to_string()),
            action,
            JobPriority::High,
        )
        .await
        .unwrap();
    
    // Execute job
    executor.execute_job(job_id).await.unwrap();
    
    // Verify job completed (would record results in indexer)
    let job = executor.get_job(job_id).await.unwrap().unwrap();
    assert_eq!(job.status, JobStatus::Completed);
    
    // Agent successfully processed block event
    assert!(job.result.is_some());
}

// ============================================================================
// TEST 4: SCHEDULER FAIRNESS
// ============================================================================

#[tokio::test]
async fn test_scheduler_fairness() {
    use scheduler::*;
    
    let executor = Arc::new(SwarmExecutor::new());
    
    // Register 3 agents
    for i in 1..=3 {
        let agent = Agent {
            id: AgentId(format!("agent-fair-{}", i)),
            name: format!("Fair Agent {}", i),
            description: "Fairness test".to_string(),
            owner: "test".to_string(),
            enabled: true,
            created_at: 0,
            capabilities: vec!["execute".to_string()],
            max_concurrent_jobs: 100,
            current_jobs: 0,
        };
        executor.register_agent(agent).await.unwrap();
    }
    
    // Submit 6 jobs (2 per agent)
    for i in 1..=3 {
        for j in 1..=2 {
            executor
                .submit_job(
                    AgentId(format!("agent-fair-{}", i)),
                    AgentAction::EmitEvent {
                        event_name: format!("test-event-{}-{}", i, j),
                        data: vec![],
                    },
                    JobPriority::Normal,
                )
                .await
                .unwrap();
        }
    }
    
    // Create scheduler with fairness enabled
    let config = SchedulerConfig {
        poll_interval: std::time::Duration::from_secs(1),
        max_jobs_per_cycle: 10,
        job_timeout: std::time::Duration::from_secs(60),
        enable_fairness: true,
    };
    
    let scheduler = JobScheduler::new(executor.clone(), config);
    
    // Process jobs (should round-robin between agents)
    let count = scheduler.process_jobs().await.unwrap();
    assert_eq!(count, 6); // All 6 jobs processed
    
    // Verify stats
    let stats = scheduler.get_stats().await.unwrap();
    assert_eq!(stats.total_executed, 6);
    assert_eq!(stats.total_succeeded, 6);
    assert_eq!(stats.total_failed, 0);
}

// ============================================================================
// TEST 5: ERROR RECOVERY
// ============================================================================

#[tokio::test]
async fn test_error_recovery() {
    let executor = Arc::new(SwarmExecutor::new());
    
    // Try to submit job for non-existent agent
    let result = executor
        .submit_job(
            AgentId("nonexistent-agent".to_string()),
            AgentAction::EmitEvent {
                event_name: "test".to_string(),
                data: vec![],
            },
            JobPriority::Normal,
        )
        .await;
    
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Agent not found");
    
    // Register agent and try again
    let agent = Agent {
        id: AgentId("agent-recovery-1".to_string()),
        name: "Recovery Agent".to_string(),
        description: "Error recovery test".to_string(),
        owner: "test".to_string(),
        enabled: false, // Disabled!
        created_at: 0,
        capabilities: vec![],
        max_concurrent_jobs: 10,
        current_jobs: 0,
    };
    
    executor.register_agent(agent).await.unwrap();
    
    // Try to submit job for disabled agent
    let result = executor
        .submit_job(
            AgentId("agent-recovery-1".to_string()),
            AgentAction::EmitEvent {
                event_name: "test".to_string(),
                data: vec![],
            },
            JobPriority::Normal,
        )
        .await;
    
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Agent is disabled");
}

// ============================================================================
// TEST 6: RPC SWARM ENDPOINTS SIMULATION
// ============================================================================

#[tokio::test]
async fn test_rpc_swarm_integration() {
    let executor = Arc::new(SwarmExecutor::new());
    let coordinator = AgentCoordinator::new(executor.clone());
    
    // Simulate RPC: submitJob
    let agent = Agent {
        id: AgentId("agent-rpc-1".to_string()),
        name: "RPC Test Agent".to_string(),
        description: "For RPC testing".to_string(),
        owner: "test".to_string(),
        enabled: true,
        created_at: 0,
        capabilities: vec!["execute".to_string()],
        max_concurrent_jobs: 10,
        current_jobs: 0,
    };
    
    executor.register_agent(agent).await.unwrap();
    
    // Simulate RPC call: submitJob
    let job_id = executor
        .submit_job(
            AgentId("agent-rpc-1".to_string()),
            AgentAction::Transfer {
                from: "alice".to_string(),
                to: "bob".to_string(),
                amount: 100,
                asset_id: 0,
            },
            JobPriority::High,
        )
        .await
        .unwrap();
    
    // Simulate RPC call: getJob
    let job = executor.get_job(job_id).await.unwrap();
    assert!(job.is_some());
    assert_eq!(job.unwrap().status, JobStatus::Pending);
    
    // Simulate RPC call: listPendingJobs
    let pending = executor.list_pending_jobs().await.unwrap();
    assert_eq!(pending.len(), 1);
    
    // Simulate RPC call: getAgent
    let agent = executor
        .get_agent(&AgentId("agent-rpc-1".to_string()))
        .await
        .unwrap();
    assert!(agent.is_some());
    
    // Simulate RPC call: getStats
    let stats = executor.get_stats().await.unwrap();
    assert_eq!(stats.total_agents, 1);
    assert_eq!(stats.total_jobs, 1);
    
    // Simulate RPC call: createProposal
    let proposal_id = coordinator
        .create_proposal(
            AgentId("agent-rpc-1".to_string()),
            "Test Proposal".to_string(),
            "Test description".to_string(),
            vec![AgentId("agent-rpc-1".to_string())],
        )
        .await
        .unwrap();
    
    assert!(proposal_id > 0);
    
    // Simulate RPC call: voteOnProposal
    coordinator
        .vote_on_proposal(proposal_id, AgentId("agent-rpc-1".to_string()), VoteChoice::Yes)
        .await
        .unwrap();
    
    // Verify vote recorded
    let proposal = coordinator.get_proposal(proposal_id).await.unwrap().unwrap();
    assert!(proposal.has_consensus());
}

// ============================================================================
// TEST 7: HIGH PRIORITY JOB EXECUTION
// ============================================================================

#[tokio::test]
async fn test_priority_job_execution() {
    use scheduler::*;
    
    let executor = Arc::new(SwarmExecutor::new());
    
    // Register agent
    let agent = Agent {
        id: AgentId("agent-priority-1".to_string()),
        name: "Priority Test Agent".to_string(),
        description: "Tests priority scheduling".to_string(),
        owner: "test".to_string(),
        enabled: true,
        created_at: 0,
        capabilities: vec!["execute".to_string()],
        max_concurrent_jobs: 100,
        current_jobs: 0,
    };
    
    executor.register_agent(agent).await.unwrap();
    
    // Submit jobs in reverse priority order
    let _low = executor
        .submit_job(
            AgentId("agent-priority-1".to_string()),
            AgentAction::EmitEvent {
                event_name: "low".to_string(),
                data: vec![],
            },
            JobPriority::Low,
        )
        .await
        .unwrap();
    
    let _normal = executor
        .submit_job(
            AgentId("agent-priority-1".to_string()),
            AgentAction::EmitEvent {
                event_name: "normal".to_string(),
                data: vec![],
            },
            JobPriority::Normal,
        )
        .await
        .unwrap();
    
    let _high = executor
        .submit_job(
            AgentId("agent-priority-1".to_string()),
            AgentAction::EmitEvent {
                event_name: "high".to_string(),
                data: vec![],
            },
            JobPriority::High,
        )
        .await
        .unwrap();
    
    // Get pending jobs (should be sorted by priority)
    let pending = executor.list_pending_jobs().await.unwrap();
    assert_eq!(pending.len(), 3);
    
    // Verify priority ordering (high → normal → low)
    assert_eq!(pending[0].priority, JobPriority::High);
    assert_eq!(pending[1].priority, JobPriority::Normal);
    assert_eq!(pending[2].priority, JobPriority::Low);
}

// ============================================================================
// TEST 8: MESSAGE ROUTING BETWEEN AGENTS
// ============================================================================

#[tokio::test]
async fn test_agent_message_routing() {
    let executor = Arc::new(SwarmExecutor::new());
    let coordinator = AgentCoordinator::new(executor.clone());
    
    // Register agents
    for i in 1..=2 {
        let agent = Agent {
            id: AgentId(format!("agent-message-{}", i)),
            name: format!("Message Agent {}", i),
            description: "Message routing test".to_string(),
            owner: "test".to_string(),
            enabled: true,
            created_at: 0,
            capabilities: vec!["communicate".to_string()],
            max_concurrent_jobs: 10,
            current_jobs: 0,
        };
        executor.register_agent(agent).await.unwrap();
    }
    
    // Agent 1 sends data request to Agent 2
    let message = Message::DataRequest {
        from: AgentId("agent-message-1".to_string()),
        to: AgentId("agent-message-2".to_string()),
        query: "balance".to_string(),
    };
    
    coordinator.send_message(message).await.unwrap();
    
    // Agent 2 receives message
    let messages = coordinator
        .get_messages(&AgentId("agent-message-2".to_string()))
        .await
        .unwrap();
    
    assert_eq!(messages.len(), 1);
    
    // Agent 2 responds
    let response = Message::DataResponse {
        from: AgentId("agent-message-2".to_string()),
        to: AgentId("agent-message-1".to_string()),
        data: vec![1, 2, 3],
    };
    
    coordinator.send_message(response).await.unwrap();
    
    // Agent 1 receives response
    let messages = coordinator
        .get_messages(&AgentId("agent-message-1".to_string()))
        .await
        .unwrap();
    
    assert_eq!(messages.len(), 1);
}

// ============================================================================
// TEST 9: JOB CANCELLATION
// ============================================================================

#[tokio::test]
async fn test_job_cancellation() {
    let executor = Arc::new(SwarmExecutor::new());
    
    // Register agent
    let agent = Agent {
        id: AgentId("agent-cancel-1".to_string()),
        name: "Cancel Test Agent".to_string(),
        description: "Job cancellation test".to_string(),
        owner: "test".to_string(),
        enabled: true,
        created_at: 0,
        capabilities: vec!["execute".to_string()],
        max_concurrent_jobs: 10,
        current_jobs: 0,
    };
    
    executor.register_agent(agent).await.unwrap();
    
    // Submit a job
    let job_id = executor
        .submit_job(
            AgentId("agent-cancel-1".to_string()),
            AgentAction::EmitEvent {
                event_name: "test".to_string(),
                data: vec![],
            },
            JobPriority::Normal,
        )
        .await
        .unwrap();
    
    // Verify job is pending
    let job = executor.get_job(job_id).await.unwrap().unwrap();
    assert_eq!(job.status, JobStatus::Pending);
    
    // Cancel the job
    executor.cancel_job(job_id).await.unwrap();
    
    // Verify job is cancelled
    let job = executor.get_job(job_id).await.unwrap().unwrap();
    assert_eq!(job.status, JobStatus::Cancelled);
    
    // Cannot cancel already-executed job
    executor.execute_job(job_id).await.unwrap();
    let result = executor.cancel_job(job_id).await;
    assert!(result.is_err());
}

// ============================================================================
// TEST 10: CONCURRENT OPERATIONS
// ============================================================================

#[tokio::test]
async fn test_concurrent_operations() {
    let executor = Arc::new(SwarmExecutor::new());
    
    // Register agents in parallel
    let mut handles = vec![];
    
    for i in 1..=5 {
        let exec = executor.clone();
        let handle = tokio::spawn(async move {
            let agent = Agent {
                id: AgentId(format!("agent-concurrent-{}", i)),
                name: format!("Concurrent Agent {}", i),
                description: "Concurrent test".to_string(),
                owner: "test".to_string(),
                enabled: true,
                created_at: 0,
                capabilities: vec!["execute".to_string()],
                max_concurrent_jobs: 10,
                current_jobs: 0,
            };
            
            exec.register_agent(agent).await.unwrap()
        });
        
        handles.push(handle);
    }
    
    // Wait for all registrations
    for handle in handles {
        let _ = handle.await;
    }
    
    // Submit jobs in parallel
    let mut job_handles = vec![];
    
    for i in 1..=5 {
        let exec = executor.clone();
        let handle = tokio::spawn(async move {
            exec.submit_job(
                AgentId(format!("agent-concurrent-{}", i)),
                AgentAction::EmitEvent {
                    event_name: format!("event-{}", i),
                    data: vec![],
                },
                JobPriority::Normal,
            )
            .await
            .unwrap()
        });
        
        job_handles.push(handle);
    }
    
    // Wait for all submissions
    for handle in job_handles {
        let _ = handle.await;
    }
    
    // Verify all jobs submitted
    let pending = executor.list_pending_jobs().await.unwrap();
    assert_eq!(pending.len(), 5);
    
    // Verify all agents registered
    let agents = executor.list_agents().await.unwrap();
    assert_eq!(agents.len(), 5);
}
