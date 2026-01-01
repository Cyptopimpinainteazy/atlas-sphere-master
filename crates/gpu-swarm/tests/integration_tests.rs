//! GPU Swarm Integration Tests

use gpu_swarm::{
    config::SwarmConfig,
    error::SwarmResult,
    node::{GpuBackend, GpuCapabilities, NodeId, NodeRegistry, NodeStatus, SwarmNode},
    scheduler::{SchedulerConfig, SchedulingStrategy, TaskScheduler},
    task::{Task, TaskId, TaskPriority, TaskStatus, TaskType},
    verification::{ExecutionVerifier, VerificationConfig},
};
use std::time::Duration;
use uuid::Uuid;

/// Helper to create a test GPU capabilities struct
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

/// Helper to create a test node ID
fn test_node_id(seed: u8) -> NodeId {
    let mut id = [0u8; 32];
    id[0] = seed;
    for i in 1..32 {
        id[i] = seed.wrapping_add(i as u8);
    }
    id
}

/// Helper to create a test submitter ID
fn test_submitter() -> [u8; 32] {
    let mut id = [0u8; 32];
    id[0] = 0xFF;
    id
}

/// Helper to create a test task
fn create_test_task(task_type: TaskType, reward: u64) -> Task {
    Task::new(task_type, test_submitter(), reward)
}

#[test]
fn test_node_creation() {
    let config = SwarmConfig::default();
    let gpu = test_gpu_capabilities(8);
    
    let node = SwarmNode::new(&config, gpu.clone()).expect("Failed to create node");
    
    assert_eq!(node.gpu.total_vram, 8 * 1024 * 1024 * 1024);
    assert!(node.gpu.backends.contains(&GpuBackend::Vulkan));
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
}

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
}

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
fn test_gpu_capabilities_requirements() {
    let caps = test_gpu_capabilities(8);
    
    // Should meet requirements with lower VRAM
    assert!(caps.meets_requirements(4 * 1024 * 1024 * 1024, &[GpuBackend::Vulkan]));
    
    // Should fail with higher VRAM requirement
    assert!(!caps.meets_requirements(16 * 1024 * 1024 * 1024, &[GpuBackend::Vulkan]));
    
    // Should fail with wrong backend
    assert!(!caps.meets_requirements(4 * 1024 * 1024 * 1024, &[GpuBackend::Cuda]));
}

#[test]
fn test_task_priority_ordering() {
    assert!(TaskPriority::Low < TaskPriority::Normal);
    assert!(TaskPriority::Normal < TaskPriority::High);
    assert!(TaskPriority::High < TaskPriority::Critical);
}

#[test]
fn test_node_metrics_success_rate() {
    use gpu_swarm::node::NodeMetrics;
    
    let mut metrics = NodeMetrics::default();
    
    // Initial success rate should be 1.0 (no tasks)
    assert_eq!(metrics.success_rate(), 1.0);
    
    // After some tasks
    metrics.tasks_completed = 90;
    metrics.tasks_failed = 10;
    
    assert!((metrics.success_rate() - 0.9).abs() < 0.001);
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

#[test]
fn test_swarm_config_default() {
    let config = SwarmConfig::default();
    
    // Should have default values
    assert!(!config.network.listen_addresses.is_empty());
}

#[tokio::test]
async fn test_task_builder_pattern() {
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
