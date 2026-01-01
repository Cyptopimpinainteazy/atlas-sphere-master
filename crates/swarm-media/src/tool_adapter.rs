//! Tool Adapter abstraction layer for the swarm media orchestration system.
//! 
//! This module defines the core trait and types that allow the swarm to invoke
//! media generation tools without knowing their implementation details (local, cloud, or hybrid).
//!
//! Philosophy:
//! - Swarm never hardcodes a tool provider
//! - Adapters are pluggable and can be swapped at runtime
//! - Jobs are queued centrally and routed based on VRAM, latency, job type
//! - Results are cached and versioned for reproducibility

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Unique identifier for a media generation job
pub type JobId = Uuid;

/// Identifies the type of tool (LLM, image, video, audio, etc.)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ToolType {
    /// LLM inference (chat, completion, reasoning)
    LlmInference,
    /// Text generation (LLaMA, DeepSeek, etc.)
    TextGeneration,
    /// Image generation (Stable Diffusion, SDXL, etc.)
    ImageGeneration,
    /// Image to image (ControlNet, inpainting, etc.)
    ImageToImage,
    /// Video generation or processing
    VideoProcessing,
    /// Speech synthesis (TTS)
    TextToSpeech,
    /// Speech to text (Whisper)
    SpeechToText,
    /// Translation
    Translation,
    /// Localization (transcription + translation + subtitles)
    Localization,
    /// Diagram/infographic generation
    DiagramGeneration,
    /// Asset management operations
    AssetManagement,
    /// Generic or custom tool type
    Custom(String),
}

/// Job execution priority level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    /// Low priority, can be deferred
    Low = 0,
    /// Normal priority (default)
    Normal = 1,
    /// High priority, execute soon
    High = 2,
    /// Critical priority, execute immediately
    Critical = 3,
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Normal
    }
}

/// Job execution status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    /// Queued and waiting for assignment
    Queued,
    /// Assigned to a node but not yet started
    Assigned,
    /// Currently executing
    Running,
    /// Completed successfully
    Completed,
    /// Failed with error
    Failed(String),
    /// Cancelled by user
    Cancelled,
}

/// Metadata about a GPU node that can execute jobs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuNodeCapabilities {
    /// Unique node identifier
    pub node_id: Uuid,
    /// Human-readable node name
    pub name: String,
    /// Total VRAM in GB
    pub vram_gb: u32,
    /// Available VRAM in GB (dynamic)
    pub available_vram_gb: u32,
    /// Supported tool types
    pub supported_tools: Vec<ToolType>,
    /// Average latency in ms for job execution
    pub latency_ms: u32,
    /// Whether node is currently online
    pub online: bool,
    /// Last heartbeat timestamp (Unix seconds)
    pub last_heartbeat: i64,
    /// Number of jobs processed successfully
    pub jobs_completed: u64,
    /// Cumulative compute provided (in GPU-hours)
    pub compute_contributed: f64,
}

/// Parameters for a tool invocation
/// Maps directly to the tool's expected input format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParams {
    /// Raw JSON parameters for the tool
    pub params: serde_json::Value,
}

impl ToolParams {
    pub fn new(params: serde_json::Value) -> Self {
        Self { params }
    }

    pub fn from_json_str(json_str: &str) -> serde_json::Result<Self> {
        Ok(Self {
            params: serde_json::from_str(json_str)?,
        })
    }

    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.params.get(key)
    }
}

/// Result of a successful tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Unique job ID that produced this result
    pub job_id: JobId,
    /// The tool that was invoked
    pub tool_type: ToolType,
    /// Raw output data (format depends on tool)
    pub output: serde_json::Value,
    /// Execution time in milliseconds
    pub execution_time_ms: u32,
    /// Optional content hash for reproducibility/caching
    pub content_hash: Option<String>,
    /// GPU node that executed this job
    pub executed_by_node: Uuid,
}

/// Job request submitted to the swarm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaJob {
    pub job_id: JobId,
    pub tool_type: ToolType,
    pub params: ToolParams,
    pub priority: Priority,
    pub created_at: i64,
    /// Optional unique identifier for deduplication/caching
    pub request_hash: Option<String>,
    /// Desired VRAM minimum in GB
    pub min_vram_gb: Option<u32>,
}

impl MediaJob {
    pub fn new(tool_type: ToolType, params: ToolParams) -> Self {
        Self {
            job_id: Uuid::new_v4(),
            tool_type,
            params,
            priority: Priority::default(),
            created_at: chrono::Utc::now().timestamp(),
            request_hash: None,
            min_vram_gb: None,
        }
    }

    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_min_vram(mut self, vram_gb: u32) -> Self {
        self.min_vram_gb = Some(vram_gb);
        self
    }
}

/// Core trait that all tool implementations must satisfy
/// This is the abstraction layer that decouples the swarm from specific tools
#[async_trait]
pub trait ToolAdapter: Send + Sync {
    /// Get the tool type this adapter handles
    fn tool_type(&self) -> ToolType;

    /// Check if this adapter can handle a specific tool type
    fn supports(&self, tool_type: ToolType) -> bool {
        self.tool_type() == tool_type
    }

    /// Validate that parameters are correct before queuing
    async fn validate_params(&self, params: &ToolParams) -> Result<(), String>;

    /// Actually invoke the tool with the given parameters
    /// Returns the job ID for later status polling
    async fn invoke(&self, params: ToolParams) -> Result<JobId, String>;

    /// Get the current status of a job
    async fn get_status(&self, job_id: JobId) -> Result<JobStatus, String>;

    /// Retrieve the result of a completed job
    async fn get_result(&self, job_id: JobId) -> Result<ToolResult, String>;

    /// Cancel an in-flight job
    async fn cancel_job(&self, job_id: JobId) -> Result<(), String>;

    /// Get information about resource requirements for this tool
    fn resource_requirements(&self, _params: &ToolParams) -> ToolResourceReq {
        // Default: moderate VRAM, immediate execution
        ToolResourceReq {
            min_vram_gb: 8,
            preferred_latency_ms: 1000,
            supports_batching: false,
        }
    }
}

/// Resource requirements for a tool invocation
#[derive(Debug, Clone)]
pub struct ToolResourceReq {
    /// Minimum VRAM in GB required to execute this job
    pub min_vram_gb: u32,
    /// Preferred execution latency in milliseconds
    pub preferred_latency_ms: u32,
    /// Whether this tool supports batch execution
    pub supports_batching: bool,
}

/// The main orchestrator that manages all tools and routes jobs
pub struct ToolOrchestrator {
    adapters: HashMap<ToolType, Box<dyn ToolAdapter>>,
}

impl ToolOrchestrator {
    pub fn new() -> Self {
        Self {
            adapters: HashMap::new(),
        }
    }

    /// Register a tool adapter for a specific tool type
    pub fn register<T: ToolAdapter + 'static>(&mut self, adapter: T) {
        let tool_type = adapter.tool_type();
        self.adapters.insert(tool_type, Box::new(adapter));
    }

    /// Get the adapter for a specific tool type
    pub fn get_adapter(&self, tool_type: ToolType) -> Option<&dyn ToolAdapter> {
        self.adapters.get(&tool_type).map(|b| b.as_ref())
    }

    /// List all registered tool types
    pub fn list_tools(&self) -> Vec<ToolType> {
        self.adapters.keys().cloned().collect()
    }
}

impl Default for ToolOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_creation() {
        let params = ToolParams::new(serde_json::json!({"prompt": "test"}));
        let job = MediaJob::new(ToolType::TextGeneration, params);

        assert_eq!(job.tool_type, ToolType::TextGeneration);
        assert_eq!(job.priority, Priority::Normal);
        assert!(job.job_id.to_string().len() > 0);
    }

    #[test]
    fn test_priority_ordering() {
        assert!(Priority::Low < Priority::Normal);
        assert!(Priority::Normal < Priority::High);
        assert!(Priority::High < Priority::Critical);
    }

    #[test]
    fn test_gpu_node_capabilities() {
        let node = GpuNodeCapabilities {
            node_id: Uuid::new_v4(),
            name: "gpu-node-1".to_string(),
            vram_gb: 24,
            available_vram_gb: 20,
            supported_tools: vec![ToolType::ImageGeneration, ToolType::TextToSpeech],
            latency_ms: 450,
            online: true,
            last_heartbeat: chrono::Utc::now().timestamp(),
            jobs_completed: 42,
            compute_contributed: 12.5,
        };

        assert_eq!(node.vram_gb, 24);
        assert!(node.online);
    }
}
