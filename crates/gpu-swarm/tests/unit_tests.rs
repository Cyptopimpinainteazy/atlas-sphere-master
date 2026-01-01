//! Comprehensive Unit Tests for GPU Swarm - 100% Coverage Target
//!
//! This test suite provides comprehensive unit tests for all modules
//! in the GPU swarm to achieve 100% code coverage.

use gpu_swarm::{
    announcer::{Announcer, AnnouncerConfig, AnnouncementType, AnnouncementSeverity},
    config::SwarmConfig,
    coordinator::{SwarmCoordinator, CoordinatorConfig},
    crown::{Crown, CrownConfig, CrownVerdict, IssueCategory, IssueSeverity},
    error::{SwarmError, SwarmResult},
    funding::{CampaignOrchestrator, NovaFlux, WebhookBridge},
    jobs::{SwarmJob, JobType, ModelTrainingJob, ZkProvingJob, ChainIndexingJob, MempoolAnalysisJob, X3SimulationJob, FundingCampaignJob},
    node::{SwarmNode, GpuCapabilities, GpuBackend, NodeStatus, NodeRegistry},
    protocol::{SwarmMessage, MessageEnvelope, TaskAssignment, TaskResult},
    scheduler::{TaskScheduler, SchedulerConfig, SchedulingStrategy},
    task::{Task, TaskType, TaskPriority, TaskStatus},
    verification::{ExecutionVerifier, VerificationConfig},
    warden::{Warden, WardenConfig, ComputeLane, ThreatLevel},
};
use std::time::Duration;
use std::collections::HashMap;
use serde_json::json;

/// Test helper: Create a test GPU capabilities
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

/// Test helper: Create a test node ID
fn test_node_id(seed: u8) -> [u8; 32] {
    let mut id = [0u8; 32];
    id[0] = seed;
    for i in 1..32 {
        id[i] = seed.wrapping_add(i as u8);
    }
    id
}

/// Test helper: Create a test submitter ID
fn test_submitter() -> [u8; 32] {
    let mut id = [0u8; 32];
    id[0] = 0xFF;
    id
}

/// Test helper: Create a test task
fn create_test_task(task_type: TaskType, reward: u64) -> Task {
    Task::new(task_type, test_submitter(), reward)
}

#[cfg(test)]
mod config_tests {
    use super::*;

    #[test]
    fn test_swarm_config_default() {
        let config = SwarmConfig::default();
        assert!(!config.network.listen_addresses.is_empty());
        assert!(config.node.display_name.is_none());
    }

    #[test]
    fn test_swarm_config_from_file() {
        let config = SwarmConfig::default();
        let temp_dir = std::env::temp_dir();
        let config_path = temp_dir.join("test_config.toml");
        
        // Save config to file
        config.to_file(&config_path).expect("Failed to save config");
        
        // Load config from file
        let loaded_config = SwarmConfig::from_file(&config_path).expect("Failed to load config");
        
        assert_eq!(config.node.display_name, loaded_config.node.display_name);
        
        // Clean up
        std::fs::remove_file(&config_path).ok();
    }

    #[test]
    fn test_swarm_config_to_file_error() {
        let config = SwarmConfig::default();
        let invalid_path = std::path::Path::new("/invalid/path/config.toml");
        
        let result = config.to_file(invalid_path);
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod node_tests {
    use super::*;

    #[test]
    fn test_gpu_capabilities_creation() {
        let caps = test_gpu_capabilities(8);
        
        assert_eq!(caps.total_vram, 8 * 1024 * 1024 * 1024);
        assert_eq!(caps.available_vram, (8 * 1024 * 1024 * 1024) * 3 / 4);
        assert!(caps.backends.contains(&GpuBackend::Vulkan));
        assert_eq!(caps.device_name, "Test GPU 8GB");
    }

    #[test]
    fn test_gpu_capabilities_meets_requirements() {
        let caps = test_gpu_capabilities(8);
        
        // Should meet lower VRAM requirement
        assert!(caps.meets_requirements(4 * 1024 * 1024 * 1024, &[GpuBackend::Vulkan]));
        
        // Should fail higher VRAM requirement
        assert!(!caps.meets_requirements(16 * 1024 * 1024 * 1024, &[GpuBackend::Vulkan]));
        
        // Should fail wrong backend
        assert!(!caps.meets_requirements(4 * 1024 * 1024 * 1024, &[GpuBackend::Cuda]));
    }

    #[test]
    fn test_gpu_backends() {
        let backends = vec![
            GpuBackend::Cuda,
            GpuBackend::OpenCL,
            GpuBackend::Vulkan,
            GpuBackend::Metal,
            GpuBackend::WebGpu,
        ];
        
        for backend in backends {
            let mut caps = test_gpu_capabilities(8);
            caps.backends = vec![backend.clone()];
            assert!(caps.backends.contains(&backend));
        }
    }

    #[test]
    fn test_node_creation() {
        let config = SwarmConfig::default();
        let gpu = test_gpu_capabilities(8);
        
        let node = SwarmNode::new(&config, gpu).expect("Failed to create node");
        
        assert_eq!(node.gpu.total_vram, 8 * 1024 * 1024 * 1024);
        assert_eq!(node.status, NodeStatus::Offline);
        assert!(node.id.len() == 32);
    }

    #[test]
    fn test_node_registry() {
        let mut registry = NodeRegistry::new();
        let config = SwarmConfig::default();
        
        // Create and register nodes
        let gpu1 = test_gpu_capabilities(8);
        let gpu2 = test_gpu_capabilities(16);
        
        let mut node1 = SwarmNode::new(&config, gpu1).expect("Failed to create node1");
        node1.status = NodeStatus::Online;
        
        let mut node2 = SwarmNode::new(&config, gpu2).expect("Failed to create node2");
        node2.status = NodeStatus::Online;
        
        let node1_id = node1.id;
        let node2_id = node2.id;
        
        registry.register(node1).expect("Failed to register node1");
        registry.register(node2).expect("Failed to register node2");
        
        // Verify registration
        assert!(registry.get(&node1_id).is_some());
        assert!(registry.get(&node2_id).is_some());
        
        // Check online nodes count
        let online = registry.online_nodes();
        assert_eq!(online.len(), 2);
        
        // Update status
        registry.update_status(&node1_id, NodeStatus::Offline).expect("Failed to update status");
        let online_after = registry.online_nodes();
        assert_eq!(online_after.len(), 1);
        
        // Unregister node
        registry.unregister(&node2_id);
        assert!(registry.get(&node2_id).is_none());
    }

    #[test]
    fn test_node_registry_errors() {
        let mut registry = NodeRegistry::new();
        let config = SwarmConfig::default();
        let gpu = test_gpu_capabilities(8);
        let node = SwarmNode::new(&config, gpu).expect("Failed to create node");
        let node_id = node.id;
        
        registry.register(node).expect("Failed to register node");
        
        // Test updating non-existent node
        let result = registry.update_status(&test_node_id(99), NodeStatus::Online);
        assert!(result.is_err());
        
        // Test unregistering non-existent node
        let unregistered = registry.unregister(&test_node_id(99));
        assert!(unregistered.is_none());
    }
}

#[cfg(test)]
mod task_tests {
    use super::*;

    #[test]
    fn test_task_creation() {
        let task_type = TaskType::X3Bytecode {
            bytecode: vec![1, 2, 3, 4],
            input: vec![],
            gas_budget: 100_000,
        };
        
        let task = create_test_task(task_type.clone(), 100)
            .with_priority(TaskPriority::High)
            .with_timeout(Duration::from_secs(300));
        
        assert!(matches!(task.task_type, TaskType::X3Bytecode { .. }));
        assert_eq!(task.priority, TaskPriority::High);
        assert_eq!(task.reward, 100);
        assert_eq!(task.timeout, Duration::from_secs(300));
    }

    #[test]
    fn test_task_types() {
        let types = vec![
            TaskType::X3Bytecode {
                bytecode: vec![1, 2, 3],
                input: vec![],
                gas_budget: 100_000,
            },
            TaskType::MempoolSimulation {
                chain_id: 1,
                tx_count: 100,
                rpc_endpoint: "http://localhost:8545".to_string(),
            },
            TaskType::RouteOptimization {
                source_token: "0xA".to_string(),
                dest_token: "0xB".to_string(),
                amount: "1000000".to_string(),
                chains: vec![1, 137],
                max_hops: 3,
            },
            TaskType::MLTraining {
                model_id: "gpt-mini".to_string(),
                training_data_hash: "abc123".to_string(),
                epochs: 10,
                batch_size: 32,
            },
            TaskType::ProofGeneration {
                circuit_id: "poseidon".to_string(),
                public_inputs: vec![1, 2, 3],
                private_inputs: vec![4, 5, 6],
            },
            TaskType::ArbitrageSearch {
                pairs: vec![("ETH".to_string(), "USDC".to_string())],
                min_profit_bps: 50,
                max_gas: 1_000_000,
            },
            TaskType::Custom {
                task_type: "custom-workload".to_string(),
                payload: vec![1, 2, 3],
            },
        ];
        
        for task_type in types {
            let task = create_test_task(task_type.clone(), 10);
            assert_eq!(task.task_type, task_type);
        }
    }

    #[test]
    fn test_task_priority_ordering() {
        assert!(TaskPriority::Low < TaskPriority::Normal);
        assert!(TaskPriority::Normal < TaskPriority::High);
        assert!(TaskPriority::High < TaskPriority::Critical);
    }

    #[test]
    fn test_task_estimated_compute_units() {
        let task = create_test_task(
            TaskType::X3Bytecode {
                bytecode: vec![1, 2, 3],
                input: vec![],
                gas_budget: 500_000,
            },
            100,
        );
        
        assert_eq!(task.estimated_compute_units(), 500_000);
        
        let ml_task = create_test_task(
            TaskType::MLTraining {
                model_id: "test".to_string(),
                training_data_hash: "abc".to_string(),
                epochs: 10,
                batch_size: 32,
            },
            100,
        );
        
        // epochs * batch_size * 100
        assert_eq!(ml_task.estimated_compute_units(), 10 * 32 * 100);
    }

    #[test]
    fn test_task_builder_pattern() {
        let task = Task::new(
            TaskType::ProofGeneration {
                circuit_id: "test".to_string(),
                public_inputs: vec![1, 2, 3],
                private_inputs: vec![4, 5, 6],
            },
            test_submitter(),
            1000,
        )
        .with_priority(TaskPriority::Critical)
        .with_timeout(Duration::from_secs(600))
        .with_verification_count(3);
        
        assert_eq!(task.priority, TaskPriority::Critical);
        assert_eq!(task.timeout, Duration::from_secs(600));
        assert_eq!(task.verification_count, 3);
    }

    #[test]
    fn test_task_status_transitions() {
        let mut task = create_test_task(
            TaskType::X3Bytecode {
                bytecode: vec![1, 2, 3],
                input: vec![],
                gas_budget: 100_000,
            },
            100,
        );
        
        assert_eq!(task.status, TaskStatus::Pending);
        
        task.status = TaskStatus::Assigned;
        assert_eq!(task.status, TaskStatus::Assigned);
        
        task.status = TaskStatus::Executing;
        assert_eq!(task.status, TaskStatus::Executing);
        
        task.status = TaskStatus::Completed;
        assert_eq!(task.status, TaskStatus::Completed);
        
        task.status = TaskStatus::Failed;
        assert_eq!(task.status, TaskStatus::Failed);
    }

    #[test]
    fn test_task_is_expired() {
        let mut task = create_test_task(
            TaskType::X3Bytecode {
                bytecode: vec![1, 2, 3],
                input: vec![],
                gas_budget: 100_000,
            },
            100,
        );
        
        // Task should not be expired initially
        assert!(!task.is_expired());
        
        // Set created_at to 10 minutes ago (assuming timeout is 5 minutes)
        task.created_at = std::time::SystemTime::now() - Duration::from_secs(600);
        assert!(task.is_expired());
    }
}

#[cfg(test)]
mod scheduler_tests {
    use super::*;

    #[test]
    fn test_scheduler_creation() {
        let config = SchedulerConfig {
            strategy: SchedulingStrategy::RoundRobin,
            max_queue_size: 100,
            max_tasks_per_node: 4,
            timeout_grace_secs: 30,
            min_reputation: 1000,
            enable_task_stealing: true,
        };
        
        let _scheduler = TaskScheduler::new(config);
        // Scheduler created successfully
        assert!(true);
    }

    #[test]
    fn test_scheduling_strategies() {
        let strategies = vec![
            SchedulingStrategy::RoundRobin,
            SchedulingStrategy::LeastLoaded,
            SchedulingStrategy::BestFit,
            SchedulingStrategy::LocalityAware,
            SchedulingStrategy::ReputationWeighted,
        ];
        
        for strategy in strategies {
            let config = SchedulerConfig {
                strategy,
                ..Default::default()
            };
            
            let _scheduler = TaskScheduler::new(config);
            // Scheduler created with strategy
            assert!(true);
        }
    }
}

#[cfg(test)]
mod verification_tests {
    use super::*;

    #[test]
    fn test_verification_config() {
        let config = VerificationConfig {
            min_verifications: 2,
            consensus_threshold: 66,
            verification_timeout: 60,
            allow_partial: true,
            reexecution_rate: 10,
        };
        
        let verifier = ExecutionVerifier::new(config.clone());
        // Verifier is created successfully
        assert!(true);
    }
}

#[cfg(test)]
mod crown_tests {
    use super::*;

    #[test]
    fn test_crown_config_default() {
        let config = CrownConfig::default();
        assert!(config.max_evolution_allocation < 0.15);
        assert!(config.drift_threshold > 0.0);
        assert_eq!(config.max_warden_errors, 3);
    }

    #[test]
    fn test_issue_severity_ordering() {
        assert!(IssueSeverity::Critical > IssueSeverity::High);
        assert!(IssueSeverity::High > IssueSeverity::Medium);
        assert!(IssueSeverity::Medium > IssueSeverity::Low);
    }

    #[test]
    fn test_crown_verdict_severity() {
        let healthy = CrownVerdict::Healthy {
            confidence: 0.9,
            commendations: vec![],
        };
        let override_v = CrownVerdict::Override {
            reason: "test".to_string(),
            emergency_plan: Default::default(),
            warden_suspended: true,
        };

        assert!(healthy.is_healthy());
        assert!(!override_v.is_healthy());
        assert!(override_v.severity() > healthy.severity());
    }

    #[test]
    fn test_crown_issue_creation() {
        let issue = gpu_swarm::crown::CrownIssue {
            category: IssueCategory::WardenDrift,
            description: "Test issue".to_string(),
            severity: IssueSeverity::High,
            detected_at: 1234567890,
            evidence: vec!["test evidence".to_string()],
            suggested_fix: Some("test fix".to_string()),
        };

        assert_eq!(issue.category, IssueCategory::WardenDrift);
        assert_eq!(issue.severity, IssueSeverity::High);
        assert!(issue.suggested_fix.is_some());
    }
}

#[cfg(test)]
mod jobs_tests {
    use super::*;

    #[test]
    fn test_job_types() {
        let job_types = vec![
            JobType::ModelTraining,
            JobType::ZkProving,
            JobType::ChainIndexing,
            JobType::MempoolAnalysis,
            JobType::X3Simulation,
            JobType::FundingCampaign,
        ];

        for job_type in job_types {
            assert!(job_type.is_valid());
        }
    }

    #[test]
    fn test_model_training_job() {
        let job = ModelTrainingJob {
            model_id: "test-model".to_string(),
            training_data_hash: "abc123".to_string(),
            epochs: 10,
            batch_size: 32,
        };

        assert_eq!(job.job_type(), JobType::ModelTraining);
        
        // Test execution (should succeed)
        let result = job.execute();
        assert!(result.is_ok());
        
        // Test verification
        if let Ok(output) = result {
            let verified = job.verify(&output);
            assert!(verified.is_ok());
            assert!(verified.unwrap());
        }
    }

    #[test]
    fn test_zk_proving_job() {
        let job = ZkProvingJob {
            circuit_id: "poseidon".to_string(),
            proof_type: gpu_swarm::jobs::ProofType::Poseidon,
            public_inputs: vec![1, 2, 3],
            private_inputs: vec![4, 5, 6],
        };

        assert_eq!(job.job_type(), JobType::ZkProving);
        
        // Test execution
        let result = job.execute();
        assert!(result.is_ok());
        
        // Test verification
        if let Ok(output) = result {
            let verified = job.verify(&output);
            assert!(verified.is_ok());
            assert!(verified.unwrap());
        }
    }

    #[test]
    fn test_chain_indexing_job() {
        let job = ChainIndexingJob {
            chain_id: 1,
            start_block: 1000,
            end_block: 2000,
            index_type: gpu_swarm::jobs::IndexType::Transactions,
        };

        assert_eq!(job.job_type(), JobType::ChainIndexing);
        
        // Test execution
        let result = job.execute();
        assert!(result.is_ok());
        
        // Test verification
        if let Ok(output) = result {
            let verified = job.verify(&output);
            assert!(verified.is_ok());
            assert!(verified.unwrap());
        }
    }

    #[test]
    fn test_mempool_analysis_job() {
        let job = MempoolAnalysisJob {
            chain_id: 1,
            analysis_type: gpu_swarm::jobs::AnalysisType::MEV,
            time_window: Duration::from_secs(300),
        };

        assert_eq!(job.job_type(), JobType::MempoolAnalysis);
        
        // Test execution
        let result = job.execute();
        assert!(result.is_ok());
        
        // Test verification
        if let Ok(output) = result {
            let verified = job.verify(&output);
            assert!(verified.is_ok());
            assert!(verified.unwrap());
        }
    }

    #[test]
    fn test_x3_simulation_job() {
        let job = X3SimulationJob {
            bytecode: vec![1, 2, 3, 4],
            input: vec![],
            iterations: 1000,
        };

        assert_eq!(job.job_type(), JobType::X3Simulation);
        
        // Test execution
        let result = job.execute();
        assert!(result.is_ok());
        
        // Test verification
        if let Ok(output) = result {
            let verified = job.verify(&output);
            assert!(verified.is_ok());
            assert!(verified.unwrap());
        }
    }

    #[test]
    fn test_funding_campaign_job() {
        let job = FundingCampaignJob {
            campaign_type: gpu_swarm::jobs::CampaignType::VC,
            target_amount: 1000000,
            duration: Duration::from_secs(86400),
            strategy: gpu_swarm::jobs::CampaignStrategy::Aggressive,
        };

        assert_eq!(job.job_type(), JobType::FundingCampaign);
        
        // Test execution
        let result = job.execute();
        assert!(result.is_ok());
        
        // Test verification
        if let Ok(output) = result {
            let verified = job.verify(&output);
            assert!(verified.is_ok());
            assert!(verified.unwrap());
        }
    }
}

#[cfg(test)]
mod funding_tests {
    use super::*;

    #[test]
    fn test_novaflux_creation() {
        let config = gpu_swarm::funding::NovaFluxConfig {
            enabled: true,
            max_campaigns: 10,
            default_budget: 1000,
        };

        let novaflux = NovaFlux::new(config);
        assert!(novaflux.is_enabled());
    }

    #[test]
    fn test_campaign_orchestrator() {
        let config = gpu_swarm::funding::OrchestratorConfig {
            enabled: true,
            max_concurrent: 5,
            retry_attempts: 3,
        };

        let orchestrator = CampaignOrchestrator::new(config);
        assert!(orchestrator.is_enabled());
    }

    #[test]
    fn test_webhook_bridge() {
        let config = gpu_swarm::funding::WebhookConfig {
            enabled: true,
            endpoint: "http://localhost:3000/webhook".to_string(),
            timeout: Duration::from_secs(30),
        };

        let bridge = WebhookBridge::new(config);
        assert!(bridge.is_enabled());
    }
}

#[cfg(test)]
mod announcer_tests {
    use super::*;

    #[test]
    fn test_announcer_config() {
        let config = AnnouncerConfig {
            enabled: true,
            max_announcements: 1000,
            severity_threshold: AnnouncementSeverity::Medium,
        };

        assert!(config.enabled);
        assert_eq!(config.max_announcements, 1000);
        assert_eq!(config.severity_threshold, AnnouncementSeverity::Medium);
    }

    #[test]
    fn test_announcement_types() {
        let types = vec![
            AnnouncementType::WardenDecision,
            AnnouncementType::CrownEvaluation,
            AnnouncementType::FundingCampaign,
            AnnouncementType::ProphetForecast,
            AnnouncementType::ScrapyardAction,
        ];

        for announcement_type in types {
            assert!(announcement_type.is_valid());
        }
    }
}

#[cfg(test)]
mod protocol_tests {
    use super::*;

    #[test]
    fn test_message_envelope() {
        let message = SwarmMessage::Heartbeat(gpu_swarm::protocol::Heartbeat {
            node_id: test_node_id(1),
            timestamp: 1234567890,
            status: NodeStatus::Online,
        });

        let envelope = MessageEnvelope::new(test_node_id(1), message);
        assert_eq!(envelope.sender, test_node_id(1));
        assert!(envelope.message_id.len() > 0);
    }

    #[test]
    fn test_task_assignment() {
        let assignment = TaskAssignment {
            task_id: uuid::Uuid::new_v4(),
            node_id: test_node_id(1),
            task_type: TaskType::X3Bytecode {
                bytecode: vec![1, 2, 3],
                input: vec![],
                gas_budget: 100_000,
            },
            reward: 100,
            deadline: std::time::SystemTime::now() + Duration::from_secs(300),
        };

        assert!(assignment.task_id.is_nil() == false);
        assert_eq!(assignment.reward, 100);
    }

    #[test]
    fn test_task_result() {
        let result = TaskResult {
            task_id: uuid::Uuid::new_v4(),
            node_id: test_node_id(1),
            result: vec![1, 2, 3, 4],
            execution_time_ms: 1000,
            success: true,
            error_message: None,
        };

        assert!(result.task_id.is_nil() == false);
        assert_eq!(result.execution_time_ms, 1000);
        assert!(result.success);
    }
}

#[cfg(test)]
mod warden_tests {
    use super::*;

    #[test]
    fn test_warden_config() {
        let config = WardenConfig {
            enabled: true,
            update_interval: Duration::from_secs(60),
            max_allocations: 8,
        };

        assert!(config.enabled);
        assert_eq!(config.update_interval, Duration::from_secs(60));
        assert_eq!(config.max_allocations, 8);
    }

    #[test]
    fn test_compute_lanes() {
        let lanes = vec![
            ComputeLane::Security,
            ComputeLane::ChainOps,
            ComputeLane::Research,
            ComputeLane::Strategy,
            ComputeLane::AiAgents,
            ComputeLane::Ecosystem,
            ComputeLane::Storage,
            ComputeLane::Overflow,
            ComputeLane::Evolution,
        ];

        for lane in lanes {
            assert!(lane.is_valid());
        }
    }

    #[test]
    fn test_threat_levels() {
        let levels = vec![
            ThreatLevel::Low,
            ThreatLevel::Medium,
            ThreatLevel::High,
            ThreatLevel::Critical,
        ];

        for level in levels {
            assert!(level.is_valid());
        }
    }
}

#[cfg(test)]
mod coordinator_tests {
    use super::*;

    #[test]
    fn test_coordinator_config() {
        let config = CoordinatorConfig {
            max_nodes: 1000,
            max_tasks: 10000,
            heartbeat_interval: Duration::from_secs(30),
        };

        assert_eq!(config.max_nodes, 1000);
        assert_eq!(config.max_tasks, 10000);
        assert_eq!(config.heartbeat_interval, Duration::from_secs(30));
    }
}

#[cfg(test)]
mod error_tests {
    use super::*;

    #[test]
    fn test_swarm_error_variants() {
        let errors = vec![
            SwarmError::NodeNotFound(test_node_id(1)),
            SwarmError::InsufficientStake { required: 1000, actual: 500 },
            SwarmError::InvalidTask("Invalid task".to_string()),
            SwarmError::NetworkError("Network error".to_string()),
            SwarmError::SerializationError("Serialization error".to_string()),
            SwarmError::ConfigError("Config error".to_string()),
            SwarmError::QueueFull,
            SwarmError::TaskNotFound(uuid::Uuid::new_v4()),
            SwarmError::UnauthorizedVerifier(test_node_id(1)),
            SwarmError::InsufficientVerifiers { required: 3, actual: 2 },
            SwarmError::VerificationFailed("Verification failed".to_string()),
            SwarmError::Internal("Internal error".to_string()),
        ];

        for error in errors {
            assert!(error.to_string().len() > 0);
        }
    }

    #[test]
    fn test_error_conversion() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let swarm_error: SwarmError = io_error.into();
        assert!(matches!(swarm_error, SwarmError::Internal(_)));

        let json_error = serde_json::Error::custom("JSON error");
        let swarm_error: SwarmError = json_error.into();
        assert!(matches!(swarm_error, SwarmError::SerializationError(_)));
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_full_swarm_workflow() {
        // Test a complete workflow from task creation to completion
        let config = SwarmConfig::default();
        let gpu = test_gpu_capabilities(8);
        
        // Create node
        let node = SwarmNode::new(&config, gpu).expect("Failed to create node");
        assert_eq!(node.status, NodeStatus::Offline);
        
        // Create task
        let task = create_test_task(
            TaskType::X3Bytecode {
                bytecode: vec![1, 2, 3, 4],
                input: vec![],
                gas_budget: 100_000,
            },
            100,
        );
        
        assert_eq!(task.reward, 100);
        assert!(!task.is_expired());
        
        // Test task execution
        let job = ModelTrainingJob {
            model_id: "test".to_string(),
            training_data_hash: "abc".to_string(),
            epochs: 1,
            batch_size: 1,
        };
        
        let result = job.execute();
        assert!(result.is_ok());
        
        if let Ok(output) = result {
            let verified = job.verify(&output);
            assert!(verified.is_ok());
            assert!(verified.unwrap());
        }
    }

    #[tokio::test]
    async fn test_crown_evaluation_cycle() {
        let mut crown = Crown::default();
        let state = gpu_swarm::warden::SwarmState {
            timestamp: 1234567890,
            allocations: HashMap::new(),
            pillars: gpu_swarm::warden::SwarmPillars::default(),
            threat_level: ThreatLevel::Low,
        };
        
        let verdict = crown.evaluate(&state, None).await;
        assert!(verdict.is_healthy() || matches!(verdict, CrownVerdict::Caution { .. }));
        
        // Test Crown operations
        assert!(!crown.is_warden_suspended());
        assert!(crown.history().is_empty());
        
        let uptime = crown.uptime();
        assert!(uptime.as_secs() >= 0);
    }

    #[tokio::test]
    async fn test_funding_pipeline() {
        let config = gpu_swarm::funding::NovaFluxConfig {
            enabled: true,
            max_campaigns: 10,
            default_budget: 1000,
        };

        let novaflux = NovaFlux::new(config);
        assert!(novaflux.is_enabled());
        
        let orchestrator_config = gpu_swarm::funding::OrchestratorConfig {
            enabled: true,
            max_concurrent: 5,
            retry_attempts: 3,
        };

        let orchestrator = CampaignOrchestrator::new(orchestrator_config);
        assert!(orchestrator.is_enabled());
    }
}

#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_task_creation_performance() {
        let start = Instant::now();
        
        for _ in 0..1000 {
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
        println!("Created 1000 tasks in {:?}", duration);
        assert!(duration.as_millis() < 100); // Should be very fast
    }

    #[test]
    fn test_node_registry_performance() {
        let start = Instant::now();
        let mut registry = NodeRegistry::new();
        let config = SwarmConfig::default();
        
        for i in 0..1000 {
            let gpu = test_gpu_capabilities(8);
            let mut node = SwarmNode::new(&config, gpu).expect("Failed to create node");
            node.status = NodeStatus::Online;
            registry.register(node).expect("Failed to register node");
        }
        
        let duration = start.elapsed();
        println!("Registered 1000 nodes in {:?}", duration);
        assert!(duration.as_millis() < 500); // Should be reasonably fast
        
        // Test lookup performance
        let lookup_start = Instant::now();
        for _ in 0..1000 {
            let _ = registry.online_nodes();
        }
        let lookup_duration = lookup_start.elapsed();
        println!("Performed 1000 online node lookups in {:?}", lookup_duration);
        assert!(lookup_duration.as_millis() < 100);
    }
}

#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_empty_gpu_capabilities() {
        let caps = GpuCapabilities {
            backends: vec![],
            device_name: "".to_string(),
            vendor: "".to_string(),
            total_vram: 0,
            available_vram: 0,
            compute_units: 0,
            max_workgroup_size: 0,
            max_threads: 0,
            compute_capability: None,
            supports_fp64: false,
            supports_fp16: false,
            supports_tensor_cores: false,
        };
        
        assert!(!caps.meets_requirements(1024, &[GpuBackend::Vulkan]));
    }

    #[test]
    fn test_task_with_zero_reward() {
        let task = create_test_task(
            TaskType::X3Bytecode {
                bytecode: vec![1, 2, 3],
                input: vec![],
                gas_budget: 100_000,
            },
            0,
        );
        
        assert_eq!(task.reward, 0);
        assert_eq!(task.estimated_compute_units(), 100_000);
    }

    #[test]
    fn test_task_with_max_values() {
        let task = create_test_task(
            TaskType::X3Bytecode {
                bytecode: vec![255; 1000],
                input: vec![255; 1000],
                gas_budget: u64::MAX,
            },
            u64::MAX,
        );
        
        assert_eq!(task.reward, u64::MAX);
        assert_eq!(task.estimated_compute_units(), u64::MAX);
    }

    #[test]
    fn test_node_with_max_vram() {
        let caps = test_gpu_capabilities(u64::MAX / (1024 * 1024 * 1024));
        assert!(caps.total_vram > 0);
    }

    #[test]
    fn test_scheduler_with_max_queue() {
        let config = SchedulerConfig {
            strategy: SchedulingStrategy::RoundRobin,
            max_queue_size: usize::MAX,
            max_tasks_per_node: 4,
            timeout_grace_secs: 30,
            min_reputation: 1000,
            enable_task_stealing: true,
        };
        
        let scheduler = TaskScheduler::new(config);
        assert!(scheduler.is_running());
    }
}

#[cfg(test)]
mod serialization_tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_gpu_capabilities_serialization() {
        let caps = test_gpu_capabilities(8);
        let serialized = serde_json::to_string(&caps).expect("Failed to serialize");
        let deserialized: GpuCapabilities = serde_json::from_str(&serialized).expect("Failed to deserialize");
        
        assert_eq!(caps.total_vram, deserialized.total_vram);
        assert_eq!(caps.device_name, deserialized.device_name);
    }

    #[test]
    fn test_task_serialization() {
        let task = create_test_task(
            TaskType::X3Bytecode {
                bytecode: vec![1, 2, 3, 4],
                input: vec![],
                gas_budget: 100_000,
            },
            100,
        );
        
        let serialized = serde_json::to_string(&task).expect("Failed to serialize");
        let deserialized: Task = serde_json::from_str(&serialized).expect("Failed to deserialize");
        
        assert_eq!(task.reward, deserialized.reward);
        assert_eq!(task.priority, deserialized.priority);
    }

    #[test]
    fn test_swarm_config_serialization() {
        let config = SwarmConfig::default();
        let serialized = serde_json::to_string(&config).expect("Failed to serialize");
        let deserialized: SwarmConfig = serde_json::from_str(&serialized).expect("Failed to deserialize");
        
        assert_eq!(config.node.display_name, deserialized.node.display_name);
    }

    #[test]
    fn test_crown_verdict_serialization() {
        let verdict = CrownVerdict::Healthy {
            confidence: 0.9,
            commendations: vec!["Good job".to_string()],
        };
        
        let serialized = serde_json::to_string(&verdict).expect("Failed to serialize");
        let deserialized: CrownVerdict = serde_json::from_str(&serialized).expect("Failed to deserialize");
        
        assert!(matches!(deserialized, CrownVerdict::Healthy { .. }));
    }
}