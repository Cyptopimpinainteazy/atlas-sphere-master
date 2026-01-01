//! Test utilities and helpers for GPU Swarm testing

use gpu_swarm::{
    config::SwarmConfig,
    node::{GpuCapabilities, GpuBackend, NodeStatus, NodeRegistry, SwarmNode},
    task::{Task, TaskType, TaskPriority, TaskStatus},
    protocol::{TaskAssignment, TaskResult},
    scheduler::{TaskScheduler, SchedulerConfig, SchedulingStrategy},
    verification::{ExecutionVerifier, VerificationConfig},
    warden::{Warden, WardenConfig, SwarmState, SwarmPillars, ComputeLane, ThreatLevel},
    crown::{Crown, CrownConfig},
    error::SwarmResult,
};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Test configuration for different test scenarios
#[derive(Debug, Clone)]
pub struct TestConfig {
    pub node_count: usize,
    pub task_count: usize,
    pub vram_gb: u64,
    pub scheduler_strategy: SchedulingStrategy,
    pub enable_verification: bool,
    pub enable_crown: bool,
    pub enable_warden: bool,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            node_count: 3,
            task_count: 10,
            vram_gb: 8,
            scheduler_strategy: SchedulingStrategy::RoundRobin,
            enable_verification: true,
            enable_crown: true,
            enable_warden: true,
        }
    }
}

/// Test fixture for swarm testing
pub struct SwarmTestFixture {
    pub config: SwarmConfig,
    pub nodes: Vec<SwarmNode>,
    pub registry: NodeRegistry,
    pub scheduler: TaskScheduler,
    pub verifier: Option<ExecutionVerifier>,
    pub warden: Option<Warden>,
    pub crown: Option<Crown>,
    pub test_start_time: Instant,
}

impl SwarmTestFixture {
    /// Create a new test fixture with default configuration
    pub fn new() -> Self {
        Self::with_config(TestConfig::default())
    }

    /// Create a new test fixture with custom configuration
    pub fn with_config(config: TestConfig) -> Self {
        let swarm_config = SwarmConfig::default();
        let mut nodes = Vec::new();
        let mut registry = NodeRegistry::new();
        
        // Create nodes
        for i in 0..config.node_count {
            let gpu = create_test_gpu(config.vram_gb);
            let node = SwarmNode::new(&swarm_config, gpu).expect("Failed to create node");
            nodes.push(node.clone());
            registry.register(node).expect("Failed to register node");
        }
        
        // Create scheduler
        let scheduler_config = SchedulerConfig {
            strategy: config.scheduler_strategy,
            max_queue_size: 1000,
            max_tasks_per_node: 10,
            timeout_grace_secs: 30,
            min_reputation: 1000,
            enable_task_stealing: true,
        };
        let scheduler = TaskScheduler::new(scheduler_config);
        
        // Create verifier
        let verifier = if config.enable_verification {
            let verifier_config = VerificationConfig {
                min_verifications: 2,
                consensus_threshold: 66,
                verification_timeout: 60,
                allow_partial: true,
                reexecution_rate: 10,
            };
            Some(ExecutionVerifier::new(verifier_config))
        } else {
            None
        };
        
        // Create warden
        let warden = if config.enable_warden {
            let warden_config = WardenConfig::default();
            Some(Warden::new(warden_config))
        } else {
            None
        };
        
        // Create crown
        let crown = if config.enable_crown {
            let crown_config = CrownConfig::default();
            Some(Crown::new(crown_config))
        } else {
            None
        };
        
        Self {
            config: swarm_config,
            nodes,
            registry,
            scheduler,
            verifier,
            warden,
            crown,
            test_start_time: Instant::now(),
        }
    }

    /// Create test tasks
    pub fn create_test_tasks(&self, count: usize) -> Vec<Task> {
        let mut tasks = Vec::new();
        for i in 0..count {
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
        tasks
    }

    /// Submit tasks to scheduler
    pub fn submit_tasks(&mut self, tasks: Vec<Task>) -> Vec<Uuid> {
        let mut task_ids = Vec::new();
        for task in tasks {
            let task_id = self.scheduler.submit(task).expect("Failed to submit task");
            task_ids.push(task_id);
        }
        task_ids
    }

    /// Complete tasks
    pub fn complete_tasks(&mut self, task_ids: Vec<Uuid>) {
        for task_id in task_ids {
            let result = TaskResult {
                task_id,
                node_id: [0; 32], // Use first node
                result: vec![1, 2, 3, 4],
                execution_time_ms: 100,
                success: true,
                error_message: None,
            };
            let _ = self.scheduler.complete_task(task_id, result);
        }
    }

    /// Get test duration
    pub fn test_duration(&self) -> Duration {
        self.test_start_time.elapsed()
    }

    /// Reset test start time
    pub fn reset_timer(&mut self) {
        self.test_start_time = Instant::now();
    }
}

/// Helper to create test GPU capabilities
pub fn create_test_gpu(vram_gb: u64) -> GpuCapabilities {
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

/// Helper to create test submitter ID
pub fn create_test_submitter() -> [u8; 32] {
    let mut id = [0u8; 32];
    id[0] = 0xFF;
    id
}

/// Helper to create test task
pub fn create_test_task(task_type: TaskType, reward: u64) -> Task {
    Task::new(task_type, create_test_submitter(), reward)
}

/// Helper to create test task assignment
pub fn create_test_assignment(task_id: Uuid, node_id: [u8; 32]) -> TaskAssignment {
    TaskAssignment {
        task_id,
        node_id,
        task_type: TaskType::X3Bytecode {
            bytecode: vec![1, 2, 3, 4],
            input: vec![],
            gas_budget: 100_000,
        },
        reward: 100,
        deadline: std::time::SystemTime::now() + Duration::from_secs(300),
    }
}

/// Helper to create test task result
pub fn create_test_result(task_id: Uuid, node_id: [u8; 32]) -> TaskResult {
    TaskResult {
        task_id,
        node_id,
        result: vec![1, 2, 3, 4],
        execution_time_ms: 1000,
        success: true,
        error_message: None,
    }
}

/// Helper to create test swarm state
pub fn create_test_state() -> SwarmState {
    SwarmState {
        timestamp: 1234567890,
        allocations: HashMap::new(),
        pillars: SwarmPillars::default(),
        threat_level: ThreatLevel::Low,
    }
}

/// Helper to create test state with allocations
pub fn create_test_state_with_allocations(allocations: HashMap<ComputeLane, f64>) -> SwarmState {
    SwarmState {
        timestamp: 1234567890,
        allocations,
        pillars: SwarmPillars::default(),
        threat_level: ThreatLevel::Low,
    }
}

/// Performance test helper
pub struct PerformanceTest {
    pub name: String,
    pub start_time: Instant,
    pub operations: u64,
}

impl PerformanceTest {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            start_time: Instant::now(),
            operations: 0,
        }
    }

    pub fn start(&mut self) {
        self.start_time = Instant::now();
    }

    pub fn increment(&mut self) {
        self.operations += 1;
    }

    pub fn end(&self) -> Duration {
        self.start_time.elapsed()
    }

    pub fn report(&self) {
        let duration = self.end();
        let ops_per_sec = self.operations as f64 / duration.as_secs_f64();
        println!(
            "{}: {} operations in {:?} ({:.2} ops/sec)",
            self.name,
            self.operations,
            duration,
            ops_per_sec
        );
    }
}

/// Memory usage tracking (placeholder implementation)
pub struct MemoryTracker {
    pub start_memory: f64,
    pub current_memory: f64,
}

impl MemoryTracker {
    pub fn new() -> Self {
        Self {
            start_memory: get_memory_usage(),
            current_memory: get_memory_usage(),
        }
    }

    pub fn update(&mut self) {
        self.current_memory = get_memory_usage();
    }

    pub fn memory_increase(&self) -> f64 {
        self.current_memory - self.start_memory
    }

    pub fn report(&self) {
        let increase = self.memory_increase();
        println!(
            "Memory usage: {:.2} MB (increase: {:.2} MB)",
            self.current_memory,
            increase
        );
    }
}

/// Test data generator
pub struct TestDataGenerator {
    pub counter: u64,
}

impl TestDataGenerator {
    pub fn new() -> Self {
        Self { counter: 0 }
    }

    pub fn generate_task(&mut self) -> Task {
        let task = create_test_task(
            TaskType::X3Bytecode {
                bytecode: vec![self.counter as u8; 100],
                input: vec![],
                gas_budget: 100_000,
            },
            100,
        );
        self.counter += 1;
        task
    }

    pub fn generate_tasks(&mut self, count: usize) -> Vec<Task> {
        (0..count).map(|_| self.generate_task()).collect()
    }

    pub fn generate_node(&mut self, vram_gb: u64) -> SwarmNode {
        let config = SwarmConfig::default();
        let gpu = create_test_gpu(vram_gb);
        SwarmNode::new(&config, gpu).expect("Failed to create node")
    }

    pub fn generate_nodes(&mut self, count: usize, vram_gb: u64) -> Vec<SwarmNode> {
        (0..count).map(|_| self.generate_node(vram_gb)).collect()
    }
}

/// Test assertion helpers
pub mod assertions {
    use super::*;

    pub fn assert_task_completed(task: &Task) {
        assert_eq!(task.status, TaskStatus::Completed, "Task should be completed");
    }

    pub fn assert_task_failed(task: &Task) {
        assert_eq!(task.status, TaskStatus::Failed, "Task should be failed");
    }

    pub fn assert_node_online(node: &SwarmNode) {
        assert_eq!(node.status, NodeStatus::Online, "Node should be online");
    }

    pub fn assert_node_offline(node: &SwarmNode) {
        assert_eq!(node.status, NodeStatus::Offline, "Node should be offline");
    }

    pub fn assert_scheduler_running(scheduler: &TaskScheduler) {
        assert!(scheduler.is_running(), "Scheduler should be running");
    }

    pub fn assert_verifier_running(verifier: &ExecutionVerifier) {
        assert!(verifier.is_running(), "Verifier should be running");
    }

    pub fn assert_warden_enabled(warden: &Warden) {
        assert!(warden.is_enabled(), "Warden should be enabled");
    }

    pub fn assert_crown_healthy(crown: &Crown) {
        assert!(!crown.is_warden_suspended(), "Crown should not have warden suspended");
    }
}

/// Test scenario builders
pub mod scenarios {
    use super::*;

    /// Build a healthy swarm scenario
    pub fn build_healthy_swarm() -> SwarmTestFixture {
        let config = TestConfig {
            node_count: 5,
            task_count: 20,
            vram_gb: 16,
            scheduler_strategy: SchedulingStrategy::RoundRobin,
            enable_verification: true,
            enable_crown: true,
            enable_warden: true,
        };
        SwarmTestFixture::with_config(config)
    }

    /// Build a stressed swarm scenario
    pub fn build_stressed_swarm() -> SwarmTestFixture {
        let config = TestConfig {
            node_count: 2,
            task_count: 100,
            vram_gb: 4,
            scheduler_strategy: SchedulingStrategy::LeastLoaded,
            enable_verification: true,
            enable_crown: true,
            enable_warden: true,
        };
        SwarmTestFixture::with_config(config)
    }

    /// Build a minimal swarm scenario
    pub fn build_minimal_swarm() -> SwarmTestFixture {
        let config = TestConfig {
            node_count: 1,
            task_count: 1,
            vram_gb: 8,
            scheduler_strategy: SchedulingStrategy::RoundRobin,
            enable_verification: false,
            enable_crown: false,
            enable_warden: false,
        };
        SwarmTestFixture::with_config(config)
    }

    /// Build a failure scenario
    pub fn build_failure_scenario() -> SwarmTestFixture {
        let config = TestConfig {
            node_count: 3,
            task_count: 10,
            vram_gb: 8,
            scheduler_strategy: SchedulingStrategy::RoundRobin,
            enable_verification: true,
            enable_crown: true,
            enable_warden: true,
        };
        let mut fixture = SwarmTestFixture::with_config(config);
        
        // Simulate node failures
        for node in fixture.nodes.iter_mut() {
            node.status = NodeStatus::Offline;
        }
        
        fixture
    }
}

/// Async test helpers
pub mod async_helpers {
    use super::*;
    use std::future::Future;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    /// Timeout wrapper for async operations
    pub async fn with_timeout<F, T>(future: F, timeout: Duration) -> Result<T, &'static str>
    where
        F: Future<Output = T>,
    {
        let start = Instant::now();
        tokio::select! {
            result = future => Ok(result),
            _ = tokio::time::sleep(timeout) => Err("Operation timed out"),
        }
    }

    /// Retry wrapper for operations that might fail
    pub async fn with_retry<F, T, E>(
        mut operation: F,
        max_retries: u32,
        delay: Duration,
    ) -> Result<T, E>
    where
        F: FnMut() -> Pin<Box<dyn Future<Output = Result<T, E>> + Send>>,
        E: std::fmt::Display,
    {
        let mut last_error = None;
        
        for attempt in 0..=max_retries {
            match operation().await {
                Ok(result) => return Ok(result),
                Err(error) => {
                    last_error = Some(error);
                    if attempt < max_retries {
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }
        
        Err(last_error.unwrap())
    }
}

/// Benchmark utilities
pub mod benchmarks {
    use super::*;
    use std::time::Instant;

    /// Benchmark a function
    pub fn benchmark<F, T>(name: &str, iterations: u64, mut function: F) -> Duration
    where
        F: FnMut() -> T,
    {
        let start = Instant::now();
        
        for _ in 0..iterations {
            let _ = function();
        }
        
        let duration = start.elapsed();
        println!(
            "{}: {} iterations in {:?} (avg: {:?} per iteration)",
            name,
            iterations,
            duration,
            duration / iterations as u32
        );
        
        duration
    }

    /// Benchmark with setup and teardown
    pub fn benchmark_with_setup<F, S, T, U>(
        name: &str,
        iterations: u64,
        setup: S,
        mut function: F,
        _teardown: T,
    ) -> Duration
    where
        S: Fn() -> U,
        F: FnMut(&U) -> (),
        T: Fn(U) -> (),
    {
        let setup_result = setup();
        let start = Instant::now();
        
        for _ in 0..iterations {
            function(&setup_result);
        }
        
        let duration = start.elapsed();
        println!(
            "{}: {} iterations in {:?} (avg: {:?} per iteration)",
            name,
            iterations,
            duration,
            duration / iterations as u32
        );
        
        duration
    }
}

// Helper function to get memory usage (placeholder)
fn get_memory_usage() -> f64 {
    // In a real implementation, this would use system APIs
    // For testing, we'll just return a dummy value
    0.0
}

#[cfg(test)]
mod test_utils_tests {
    use super::*;

    #[test]
    fn test_test_fixture_creation() {
        let fixture = SwarmTestFixture::new();
        assert_eq!(fixture.nodes.len(), 3);
        assert_eq!(fixture.registry.online_nodes().len(), 3);
        assert!(fixture.scheduler.is_running());
    }

    #[test]
    fn test_test_data_generator() {
        let mut generator = TestDataGenerator::new();
        
        let task1 = generator.generate_task();
        let task2 = generator.generate_task();
        
        assert_ne!(task1.id, task2.id);
        assert_eq!(task1.reward, 100);
        assert_eq!(task2.reward, 100);
    }

    #[test]
    fn test_performance_test() {
        let mut perf_test = PerformanceTest::new("test");
        perf_test.start();
        
        // Simulate some operations
        for _ in 0..100 {
            perf_test.increment();
        }
        
        let duration = perf_test.end();
        assert!(duration.as_millis() > 0);
        
        perf_test.report();
    }

    #[test]
    fn test_memory_tracker() {
        let mut tracker = MemoryTracker::new();
        tracker.update();
        
        let increase = tracker.memory_increase();
        assert!(increase >= 0.0);
        
        tracker.report();
    }

    #[test]
    fn test_scenarios() {
        let healthy = scenarios::build_healthy_swarm();
        assert_eq!(healthy.nodes.len(), 5);
        
        let minimal = scenarios::build_minimal_swarm();
        assert_eq!(minimal.nodes.len(), 1);
        
        let failure = scenarios::build_failure_scenario();
        assert_eq!(failure.nodes.len(), 3);
        // All nodes should be offline in failure scenario
        for node in failure.nodes {
            assert_eq!(node.status, NodeStatus::Offline);
        }
    }
}