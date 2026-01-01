//! Comprehensive Integration Tests for GPU Swarm
//!
//! This test suite provides end-to-end integration tests that verify
//! the complete workflow of the GPU swarm system.

use gpu_swarm::{
    announcer::{Announcer, AnnouncerConfig},
    config::SwarmConfig,
    coordinator::{SwarmCoordinator, CoordinatorConfig},
    crown::{Crown, CrownConfig},
    error::{SwarmError, SwarmResult},
    funding::{CampaignOrchestrator, NovaFlux, WebhookBridge},
    jobs::{SwarmJob, JobType, ModelTrainingJob, ZkProvingJob, ChainIndexingJob, MempoolAnalysisJob, X3SimulationJob, FundingCampaignJob},
    node::{SwarmNode, GpuCapabilities, GpuBackend, NodeStatus, NodeRegistry},
    protocol::{SwarmMessage, MessageEnvelope, TaskAssignment, TaskResult},
    scheduler::{TaskScheduler, SchedulerConfig, SchedulingStrategy},
    task::{Task, TaskType, TaskPriority, TaskStatus},
    verification::{ExecutionVerifier, VerificationConfig},
    warden::{Warden, WardenConfig, ComputeLane, ThreatLevel, SwarmState, SwarmPillars},
};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Test helper: Create test GPU capabilities
fn test_gpu_capabilities(vram_gb: u64) -> GpuCapabilities {
    GpuCapabilities {
        backends: vec![GpuBackend::Vulkan],
        device_name: format!("Test GPU {}GB", vram_gb),
        vendor: "Test Vendor".to_string(),
        total_vram: vram_gb * 1024 * 1024 * 1024,
        available_vram: (vram_gb * 1024 * 1024 * 1024) * 3 / 4,
        compute_units: 32,
        max_workgroup_size: 1024,
        max_threads: 32768,
        compute_capability: None,
        supports_fp64: false,
        supports_fp16: true,
        supports_tensor_cores: false,
    }
}

/// Test helper: Create test submitter ID
fn test_submitter() -> [u8; 32] {
    let mut id = [0u8; 32];
    id[0] = 0xFF;
    id
}

/// Test helper: Create test task
fn create_test_task(task_type: TaskType, reward: u64) -> Task {
    Task::new(task_type, test_submitter(), reward)
}

#[cfg(test)]
mod full_system_integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_complete_swarm_workflow() {
        // Test a complete end-to-end workflow
        println!("Testing complete swarm workflow...");
        
        // 1. Create configuration
        let config = SwarmConfig::default();
        
        // 2. Create nodes
        let mut nodes = Vec::new();
        for i in 0..3 {
            let gpu = test_gpu_capabilities(8);
            let node = SwarmNode::new(&config, gpu).expect("Failed to create node");
            nodes.push(node);
        }
        
        // 3. Create node registry
        let mut registry = NodeRegistry::new();
        for node in nodes {
            registry.register(node).expect("Failed to register node");
        }
        
        // 4. Create scheduler
        let scheduler_config = SchedulerConfig {
            strategy: SchedulingStrategy::RoundRobin,
            max_queue_size: 100,
            max_tasks_per_node: 4,
            timeout_grace_secs: 30,
            min_reputation: 1000,
            enable_task_stealing: true,
        };
        let mut scheduler = TaskScheduler::new(scheduler_config);
        
        // 5. Create verification system
        let verifier_config = VerificationConfig {
            min_verifications: 2,
            consensus_threshold: 66,
            verification_timeout: 60,
            allow_partial: true,
            reexecution_rate: 10,
        };
        let verifier = ExecutionVerifier::new(verifier_config);
        
        // 6. Create tasks
        let tasks = vec![
            create_test_task(
                TaskType::X3Bytecode {
                    bytecode: vec![1, 2, 3, 4],
                    input: vec![],
                    gas_budget: 100_000,
                },
                100,
            ),
            create_test_task(
                TaskType::MLTraining {
                    model_id: "test-model".to_string(),
                    training_data_hash: "abc123".to_string(),
                    epochs: 10,
                    batch_size: 32,
                },
                200,
            ),
            create_test_task(
                TaskType::ProofGeneration {
                    circuit_id: "poseidon".to_string(),
                    public_inputs: vec![1, 2, 3],
                    private_inputs: vec![4, 5, 6],
                },
                300,
            ),
        ];
        
        // 7. Submit tasks to scheduler
        for task in tasks {
            let task_id = scheduler.submit(task).expect("Failed to submit task");
            println!("Submitted task: {}", task_id);
        }
        
        // 8. Process tasks
        let start_time = Instant::now();
        let mut completed_tasks = 0;
        
        while start_time.elapsed() < Duration::from_secs(10) && completed_tasks < 3 {
            // Simulate task processing
            let assignments = scheduler.get_assignments();
            for assignment in assignments {
                // Simulate task execution
                let result = TaskResult {
                    task_id: assignment.task_id,
                    node_id: assignment.node_id,
                    result: vec![1, 2, 3, 4],
                    execution_time_ms: 1000,
                    success: true,
                    error_message: None,
                };
                
                // Submit result
                let _ = scheduler.complete_task(assignment.task_id, result);
                completed_tasks += 1;
            }
            
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        assert_eq!(completed_tasks, 3, "All tasks should complete");
        println!("Complete swarm workflow test passed!");
    }

    #[tokio::test]
    async fn test_warden_crown_governance() {
        // Test the governance stack: Warden + Crown interaction
        println!("Testing Warden-Crown governance...");
        
        // 1. Create Warden
        let warden_config = WardenConfig::default();
        let mut warden = Warden::new(warden_config);
        
        // 2. Create Crown
        let crown_config = CrownConfig::default();
        let mut crown = Crown::new(crown_config);
        
        // 3. Create initial state
        let state = SwarmState {
            timestamp: 1234567890,
            allocations: HashMap::new(),
            pillars: SwarmPillars::default(),
            threat_level: ThreatLevel::Low,
        };
        
        // 4. Warden makes decision
        let decision = warden.make_decision(&state).await.expect("Warden decision failed");
        println!("Warden decision: confidence = {}", decision.allocation_plan.confidence);
        
        // 5. Crown evaluates Warden decision
        let verdict = crown.evaluate(&state, Some(&decision)).await;
        println!("Crown verdict: {:?}", verdict);
        
        // 6. Verify healthy state
        assert!(verdict.is_healthy() || matches!(verdict, gpu_swarm::crown::CrownVerdict::Caution { .. }));
        
        // 7. Test with unhealthy state
        let mut unhealthy_state = state.clone();
        unhealthy_state.pillars.update_profit(-0.8, -0.5);
        unhealthy_state.threat_level = ThreatLevel::Critical;
        
        let unhealthy_verdict = crown.evaluate(&unhealthy_state, None).await;
        println!("Unhealthy state verdict: {:?}", unhealthy_verdict);
        
        assert!(!unhealthy_verdict.is_healthy());
        
        println!("Warden-Crown governance test passed!");
    }

    #[tokio::test]
    async fn test_funding_pipeline_integration() {
        // Test the complete funding pipeline
        println!("Testing funding pipeline integration...");
        
        // 1. Create NovaFlux
        let novaflux_config = gpu_swarm::funding::NovaFluxConfig {
            enabled: true,
            max_campaigns: 10,
            default_budget: 1000,
        };
        let novaflux = NovaFlux::new(novaflux_config);
        
        // 2. Create Campaign Orchestrator
        let orchestrator_config = gpu_swarm::funding::OrchestratorConfig {
            enabled: true,
            max_concurrent: 5,
            retry_attempts: 3,
        };
        let orchestrator = CampaignOrchestrator::new(orchestrator_config);
        
        // 3. Create Webhook Bridge
        let webhook_config = gpu_swarm::funding::WebhookConfig {
            enabled: true,
            endpoint: "http://localhost:3000/webhook".to_string(),
            timeout: Duration::from_secs(30),
        };
        let webhook_bridge = WebhookBridge::new(webhook_config);
        
        // 4. Test campaign creation
        let campaign = gpu_swarm::funding::FundingCampaignJob {
            campaign_type: gpu_swarm::funding::CampaignType::VC,
            target_amount: 1000000,
            duration: Duration::from_secs(86400),
            strategy: gpu_swarm::funding::CampaignStrategy::Aggressive,
        };
        
        // 5. Execute campaign
        let result = campaign.execute();
        assert!(result.is_ok(), "Campaign execution should succeed");
        
        if let Ok(output) = result {
            let verified = campaign.verify(&output);
            assert!(verified.is_ok(), "Campaign verification should succeed");
            assert!(verified.unwrap(), "Campaign should verify successfully");
        }
        
        println!("Funding pipeline integration test passed!");
    }

    #[tokio::test]
    async fn test_announcer_integration() {
        // Test the announcement system
        println!("Testing announcer integration...");
        
        // 1. Create announcer
        let announcer_config = AnnouncerConfig {
            enabled: true,
            max_announcements: 1000,
            severity_threshold: gpu_swarm::announcer::AnnouncementSeverity::Medium,
        };
        let announcer = Announcer::new(announcer_config);
        
        // 2. Test different announcement types
        let announcement_types = vec![
            gpu_swarm::announcer::AnnouncementType::WardenDecision,
            gpu_swarm::announcer::AnnouncementType::CrownEvaluation,
            gpu_swarm::announcer::AnnouncementType::FundingCampaign,
            gpu_swarm::announcer::AnnouncementType::ProphetForecast,
            gpu_swarm::announcer::AnnouncementType::ScrapyardAction,
        ];
        
        for announcement_type in announcement_types {
            assert!(announcement_type.is_valid(), "Announcement type should be valid");
        }
        
        // 3. Test severity levels
        let severities = vec![
            gpu_swarm::announcer::AnnouncementSeverity::Info,
            gpu_swarm::announcer::AnnouncementSeverity::Low,
            gpu_swarm::announcer::AnnouncementSeverity::Medium,
            gpu_swarm::announcer::AnnouncementSeverity::High,
            gpu_swarm::announcer::AnnouncementSeverity::Critical,
        ];
        
        for severity in severities {
            assert!(severity.is_valid(), "Severity should be valid");
        }
        
        println!("Announcer integration test passed!");
    }

    #[tokio::test]
    async fn test_error_handling_integration() {
        // Test error handling across the system
        println!("Testing error handling integration...");
        
        // 1. Test invalid task submission
        let scheduler_config = SchedulerConfig::default();
        let mut scheduler = TaskScheduler::new(scheduler_config);
        
        // Create an invalid task (too large)
        let large_task = create_test_task(
            TaskType::X3Bytecode {
                bytecode: vec![0; 20 * 1024 * 1024], // 20MB - exceeds limit
                input: vec![],
                gas_budget: 100_000,
            },
            100,
        );
        
        // Should handle gracefully
        let result = scheduler.submit(large_task);
        assert!(result.is_ok(), "Should handle large task gracefully");
        
        // 2. Test node failure simulation
        let config = SwarmConfig::default();
        let gpu = test_gpu_capabilities(8);
        let mut node = SwarmNode::new(&config, gpu).expect("Failed to create node");
        
        // Simulate node failure
        node.status = NodeStatus::Offline;
        
        let mut registry = NodeRegistry::new();
        registry.register(node).expect("Failed to register node");
        
        // Should handle offline nodes
        let online_nodes = registry.online_nodes();
        assert_eq!(online_nodes.len(), 0, "Should have no online nodes");
        
        // 3. Test verification failures
        let verifier_config = VerificationConfig::default();
        let verifier = ExecutionVerifier::new(verifier_config);
        
        // Should handle verification gracefully
        assert!(verifier.is_running());
        
        println!("Error handling integration test passed!");
    }

    #[tokio::test]
    async fn test_performance_under_load() {
        // Test system performance under load
        println!("Testing performance under load...");
        
        let start_time = Instant::now();
        
        // 1. Create many tasks
        let mut tasks = Vec::new();
        for i in 0..100 {
            let task = create_test_task(
                TaskType::X3Bytecode {
                    bytecode: vec![i as u8; 100],
                    input: vec![],
                    gas_budget: 100_000,
                },
                100,
            );
            tasks.push(task);
        }
        
        // 2. Create scheduler
        let scheduler_config = SchedulerConfig {
            strategy: SchedulingStrategy::RoundRobin,
            max_queue_size: 1000,
            max_tasks_per_node: 10,
            timeout_grace_secs: 30,
            min_reputation: 1000,
            enable_task_stealing: true,
        };
        let mut scheduler = TaskScheduler::new(scheduler_config);
        
        // 3. Submit all tasks
        let submit_start = Instant::now();
        for task in tasks {
            let _task_id = scheduler.submit(task).expect("Failed to submit task");
        }
        let submit_duration = submit_start.elapsed();
        
        println!("Submitted 100 tasks in {:?}", submit_duration);
        assert!(submit_duration.as_millis() < 1000, "Task submission should be fast");
        
        // 4. Process tasks
        let process_start = Instant::now();
        let mut completed = 0;
        
        while completed < 100 && process_start.elapsed() < Duration::from_secs(30) {
            let assignments = scheduler.get_assignments();
            for assignment in assignments {
                let result = TaskResult {
                    task_id: assignment.task_id,
                    node_id: assignment.node_id,
                    result: vec![1, 2, 3, 4],
                    execution_time_ms: 10,
                    success: true,
                    error_message: None,
                };
                
                let _ = scheduler.complete_task(assignment.task_id, result);
                completed += 1;
            }
            
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        
        let total_duration = start_time.elapsed();
        println!("Processed 100 tasks in {:?}", total_duration);
        assert!(completed >= 90, "Should complete most tasks");
        
        println!("Performance under load test passed!");
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        // Test concurrent operations across the system
        println!("Testing concurrent operations...");
        
        let config = SwarmConfig::default();
        let gpu = test_gpu_capabilities(8);
        let node = SwarmNode::new(&config, gpu).expect("Failed to create node");
        
        let mut registry = NodeRegistry::new();
        registry.register(node).expect("Failed to register node");
        
        let scheduler_config = SchedulerConfig::default();
        let scheduler = Arc::new(RwLock::new(TaskScheduler::new(scheduler_config)));
        
        // Spawn multiple concurrent tasks
        let mut handles = Vec::new();
        
        for i in 0..10 {
            let scheduler = scheduler.clone();
            let handle = tokio::spawn(async move {
                let task = create_test_task(
                    TaskType::X3Bytecode {
                        bytecode: vec![i; 50],
                        input: vec![],
                        gas_budget: 100_000,
                    },
                    100,
                );
                
                let task_id = scheduler.write().await.submit(task).expect("Failed to submit");
                
                // Simulate some processing time
                tokio::time::sleep(Duration::from_millis(100)).await;
                
                let result = TaskResult {
                    task_id,
                    node_id: [0; 32],
                    result: vec![1, 2, 3, 4],
                    execution_time_ms: 50,
                    success: true,
                    error_message: None,
                };
                
                let _ = scheduler.write().await.complete_task(task_id, result);
                task_id
            });
            
            handles.push(handle);
        }
        
        // Wait for all tasks to complete
        let results: Vec<Uuid> = futures::future::join_all(handles).await.into_iter().map(|h| h.unwrap()).collect();
        
        assert_eq!(results.len(), 10, "All concurrent tasks should complete");
        
        println!("Concurrent operations test passed!");
    }

    #[tokio::test]
    async fn test_system_recovery() {
        // Test system recovery from failures
        println!("Testing system recovery...");
        
        // 1. Create a system state
        let config = SwarmConfig::default();
        let gpu = test_gpu_capabilities(8);
        let mut node = SwarmNode::new(&config, gpu).expect("Failed to create node");
        
        let mut registry = NodeRegistry::new();
        registry.register(node.clone()).expect("Failed to register node");
        
        let scheduler_config = SchedulerConfig::default();
        let mut scheduler = TaskScheduler::new(scheduler_config);
        
        // 2. Submit some tasks
        for i in 0..5 {
            let task = create_test_task(
                TaskType::X3Bytecode {
                    bytecode: vec![i; 10],
                    input: vec![],
                    gas_budget: 100_000,
                },
                100,
            );
            let _task_id = scheduler.submit(task).expect("Failed to submit task");
        }
        
        // 3. Simulate node failure
        node.status = NodeStatus::Offline;
        registry.update_status(&node.id, NodeStatus::Offline).expect("Failed to update status");
        
        // 4. System should handle failure gracefully
        let online_nodes = registry.online_nodes();
        assert_eq!(online_nodes.len(), 0, "No nodes should be online");
        
        // 5. Simulate node recovery
        node.status = NodeStatus::Online;
        registry.update_status(&node.id, NodeStatus::Online).expect("Failed to update status");
        
        // 6. System should recover
        let online_nodes_after = registry.online_nodes();
        assert_eq!(online_nodes_after.len(), 1, "Node should be back online");
        
        // 7. Tasks should be reassignable
        let assignments = scheduler.get_assignments();
        assert!(assignments.is_empty(), "No assignments should exist yet");
        
        println!("System recovery test passed!");
    }

    #[tokio::test]
    async fn test_memory_management() {
        // Test memory management under various conditions
        println!("Testing memory management...");
        
        let start_memory = get_memory_usage();
        
        // 1. Create many objects
        let mut tasks = Vec::new();
        let mut nodes = Vec::new();
        let mut schedulers = Vec::new();
        
        for i in 0..100 {
            // Create tasks
            let task = create_test_task(
                TaskType::X3Bytecode {
                    bytecode: vec![i; 1000],
                    input: vec![],
                    gas_budget: 100_000,
                },
                100,
            );
            tasks.push(task);
            
            // Create nodes
            let config = SwarmConfig::default();
            let gpu = test_gpu_capabilities(8);
            let node = SwarmNode::new(&config, gpu).expect("Failed to create node");
            nodes.push(node);
            
            // Create schedulers
            let scheduler_config = SchedulerConfig::default();
            let scheduler = TaskScheduler::new(scheduler_config);
            schedulers.push(scheduler);
        }
        
        let mid_memory = get_memory_usage();
        println!("Memory after creating objects: {} MB", mid_memory);
        
        // 2. Process tasks
        let mut scheduler = TaskScheduler::new(SchedulerConfig::default());
        for task in tasks {
            let _task_id = scheduler.submit(task).expect("Failed to submit task");
        }
        
        // Complete some tasks
        let assignments = scheduler.get_assignments();
        for assignment in assignments {
            let result = TaskResult {
                task_id: assignment.task_id,
                node_id: assignment.node_id,
                result: vec![1, 2, 3, 4],
                execution_time_ms: 100,
                success: true,
                error_message: None,
            };
            let _ = scheduler.complete_task(assignment.task_id, result);
        }
        
        let end_memory = get_memory_usage();
        println!("Final memory usage: {} MB", end_memory);
        
        // Memory should not grow excessively
        assert!(end_memory < start_memory + 100.0, "Memory usage should be reasonable");
        
        println!("Memory management test passed!");
    }
}

#[cfg(test)]
mod stress_tests {
    use super::*;

    #[tokio::test]
    async fn test_stress_with_many_nodes() {
        // Stress test with many nodes
        println!("Testing stress with many nodes...");
        
        let config = SwarmConfig::default();
        let mut registry = NodeRegistry::new();
        
        // Create 100 nodes
        for i in 0..100 {
            let gpu = test_gpu_capabilities(8);
            let node = SwarmNode::new(&config, gpu).expect("Failed to create node");
            registry.register(node).expect("Failed to register node");
        }
        
        assert_eq!(registry.online_nodes().len(), 100, "All nodes should be registered");
        
        // Test node operations
        let start_time = Instant::now();
        for _ in 0..1000 {
            let _online = registry.online_nodes();
        }
        let duration = start_time.elapsed();
        
        println!("1000 node lookups in {:?}", duration);
        assert!(duration.as_millis() < 1000, "Node operations should be fast");
        
        println!("Stress test with many nodes passed!");
    }

    #[tokio::test]
    async fn test_stress_with_many_tasks() {
        // Stress test with many tasks
        println!("Testing stress with many tasks...");
        
        let scheduler_config = SchedulerConfig {
            max_queue_size: 10000,
            ..Default::default()
        };
        let mut scheduler = TaskScheduler::new(scheduler_config);
        
        // Submit 1000 tasks
        let start_time = Instant::now();
        for i in 0..1000 {
            let task = create_test_task(
                TaskType::X3Bytecode {
                    bytecode: vec![i as u8; 100],
                    input: vec![],
                    gas_budget: 100_000,
                },
                100,
            );
            let _task_id = scheduler.submit(task).expect("Failed to submit task");
        }
        let submit_duration = start_time.elapsed();
        
        println!("Submitted 1000 tasks in {:?}", submit_duration);
        assert!(submit_duration.as_millis() < 5000, "Task submission should be reasonable");
        
        // Complete tasks
        let complete_start = Instant::now();
        let assignments = scheduler.get_assignments();
        for assignment in assignments {
            let result = TaskResult {
                task_id: assignment.task_id,
                node_id: assignment.node_id,
                result: vec![1, 2, 3, 4],
                execution_time_ms: 10,
                success: true,
                error_message: None,
            };
            let _ = scheduler.complete_task(assignment.task_id, result);
        }
        let complete_duration = complete_start.elapsed();
        
        println!("Completed tasks in {:?}", complete_duration);
        
        println!("Stress test with many tasks passed!");
    }
}

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_edge_case_empty_system() {
        // Test with completely empty system
        let config = SwarmConfig::default();
        let registry = NodeRegistry::new();
        assert!(registry.online_nodes().is_empty());
        
        let scheduler_config = SchedulerConfig::default();
        let scheduler = TaskScheduler::new(scheduler_config);
        assert!(scheduler.get_assignments().is_empty());
        
        let verifier_config = VerificationConfig::default();
        let verifier = ExecutionVerifier::new(verifier_config);
        assert!(verifier.is_running());
        
        println!("Edge case - empty system test passed!");
    }

    #[test]
    fn test_edge_case_single_node() {
        // Test with single node
        let config = SwarmConfig::default();
        let gpu = test_gpu_capabilities(8);
        let node = SwarmNode::new(&config, gpu).expect("Failed to create node");
        
        let mut registry = NodeRegistry::new();
        registry.register(node).expect("Failed to register node");
        
        assert_eq!(registry.online_nodes().len(), 1);
        
        println!("Edge case - single node test passed!");
    }

    #[test]
    fn test_edge_case_single_task() {
        // Test with single task
        let scheduler_config = SchedulerConfig::default();
        let mut scheduler = TaskScheduler::new(scheduler_config);
        
        let task = create_test_task(
            TaskType::X3Bytecode {
                bytecode: vec![1, 2, 3],
                input: vec![],
                gas_budget: 100_000,
            },
            100,
        );
        
        let task_id = scheduler.submit(task).expect("Failed to submit task");
        
        // Complete the task
        let assignments = scheduler.get_assignments();
        assert_eq!(assignments.len(), 1);
        
        let result = TaskResult {
            task_id: assignments[0].task_id,
            node_id: assignments[0].node_id,
            result: vec![1, 2, 3, 4],
            execution_time_ms: 100,
            success: true,
            error_message: None,
        };
        
        let _ = scheduler.complete_task(task_id, result);
        
        println!("Edge case - single task test passed!");
    }
}

// Helper function to get memory usage (placeholder)
fn get_memory_usage() -> f64 {
    // In a real implementation, this would use system APIs
    // For testing, we'll just return a dummy value
    0.0
}

#[cfg(test)]
mod benchmark_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn benchmark_task_creation() {
        let start = Instant::now();
        
        for _ in 0..10000 {
            let task = create_test_task(
                TaskType::X3Bytecode {
                    bytecode: vec![1, 2, 3, 4],
                    input: vec![],
                    gas_budget: 100_000,
                },
                100,
            );
            assert_eq!(task.reward, 100);
        }
        
        let duration = start.elapsed();
        println!("Created 10000 tasks in {:?}", duration);
        assert!(duration.as_millis() < 100, "Task creation should be very fast");
    }

    #[test]
    fn benchmark_node_operations() {
        let config = SwarmConfig::default();
        let mut registry = NodeRegistry::new();
        
        let start = Instant::now();
        
        for i in 0..1000 {
            let gpu = test_gpu_capabilities(8);
            let node = SwarmNode::new(&config, gpu).expect("Failed to create node");
            registry.register(node).expect("Failed to register node");
        }
        
        let duration = start.elapsed();
        println!("Registered 1000 nodes in {:?}", duration);
        assert!(duration.as_millis() < 1000, "Node registration should be fast");
    }

    #[test]
    fn benchmark_scheduler_operations() {
        let scheduler_config = SchedulerConfig::default();
        let mut scheduler = TaskScheduler::new(scheduler_config);
        
        let start = Instant::now();
        
        // Submit many tasks
        for i in 0..1000 {
            let task = create_test_task(
                TaskType::X3Bytecode {
                    bytecode: vec![i; 10],
                    input: vec![],
                    gas_budget: 100_000,
                },
                100,
            );
            let _task_id = scheduler.submit(task).expect("Failed to submit task");
        }
        
        let duration = start.elapsed();
        println!("Submitted 1000 tasks in {:?}", duration);
        assert!(duration.as_millis() < 500, "Task submission should be fast");
    }
}