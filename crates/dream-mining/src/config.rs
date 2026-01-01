//! Configuration for Dream Mining

use serde::{Deserialize, Serialize};

/// Dream Mining configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DreamConfig {
    /// Whether Dream Mining is enabled
    pub enabled: bool,

    /// Start hour for mining schedule (0-23, local time)
    pub schedule_start: u32,

    /// End hour for mining schedule (0-23, local time)
    pub schedule_end: u32,

    /// Maximum GPU temperature (Celsius) before throttling
    pub max_gpu_temp: f32,

    /// Maximum GPU usage percentage
    pub max_gpu_usage: f32,

    /// Maximum CPU usage percentage
    pub max_cpu_usage: f32,

    /// Whether to mine on battery power
    pub mine_on_battery: bool,

    /// Minimum battery level to continue mining
    pub min_battery_level: f32,

    /// Idle time (seconds) before starting mining
    pub idle_threshold_secs: u64,

    /// Time between checking user activity
    pub activity_check_interval_secs: u64,

    /// Path to store task state
    pub state_file: Option<String>,

    /// Task priorities
    pub task_priorities: TaskPriorities,
}

impl Default for DreamConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            schedule_start: 23, // 11 PM
            schedule_end: 7,    // 7 AM
            max_gpu_temp: 75.0,
            max_gpu_usage: 80.0,
            max_cpu_usage: 50.0,
            mine_on_battery: false,
            min_battery_level: 20.0,
            idle_threshold_secs: 300, // 5 minutes
            activity_check_interval_secs: 30,
            state_file: None,
            task_priorities: TaskPriorities::default(),
        }
    }
}

/// Task priority configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPriorities {
    /// Priority for model training tasks (1-100)
    pub model_training: u8,

    /// Priority for route optimization tasks (1-100)
    pub route_optimization: u8,

    /// Priority for ZK proof generation tasks (1-100)
    pub zk_proof_generation: u8,

    /// Priority for index building tasks (1-100)
    pub index_building: u8,

    /// Priority for network analysis tasks (1-100)
    pub network_analysis: u8,
}

impl Default for TaskPriorities {
    fn default() -> Self {
        Self {
            model_training: 50,
            route_optimization: 70,
            zk_proof_generation: 60,
            index_building: 40,
            network_analysis: 30,
        }
    }
}

impl DreamConfig {
    /// Create a builder for configuration
    pub fn builder() -> DreamConfigBuilder {
        DreamConfigBuilder::default()
    }

    /// Load configuration from a file
    pub fn load(path: &str) -> crate::DreamResult<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to a file
    pub fn save(&self, path: &str) -> crate::DreamResult<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

/// Builder for Dream Mining configuration
#[derive(Default)]
pub struct DreamConfigBuilder {
    config: DreamConfig,
}

impl DreamConfigBuilder {
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.config.enabled = enabled;
        self
    }

    pub fn schedule(mut self, start: u32, end: u32) -> Self {
        self.config.schedule_start = start;
        self.config.schedule_end = end;
        self
    }

    pub fn max_gpu_temp(mut self, temp: f32) -> Self {
        self.config.max_gpu_temp = temp;
        self
    }

    pub fn max_gpu_usage(mut self, usage: f32) -> Self {
        self.config.max_gpu_usage = usage;
        self
    }

    pub fn mine_on_battery(mut self, enabled: bool) -> Self {
        self.config.mine_on_battery = enabled;
        self
    }

    pub fn idle_threshold(mut self, secs: u64) -> Self {
        self.config.idle_threshold_secs = secs;
        self
    }

    pub fn build(self) -> DreamConfig {
        self.config
    }
}
