//! Swarm Media Infrastructure
//!
//! This crate provides the distributed GPU compute orchestration and media
//! generation capabilities for the Atlas Sphere ecosystem.
//!
//! ## Feature Flags
//!
//! - `std` (default): Enables full functionality including async IO, database,
//!   HTTP clients, and runtime components. Required for node operation.
//! - Without `std`: Provides minimal type definitions for wasm/no_std contexts.
//!
//! ## Core Components (std feature required)
//!
//! - **GPU Node Management**: Registration, heartbeat, reputation-based routing
//! - **Job Queue**: Priority-based job dispatcher with QoS guarantees
//! - **Tool Adapters**: LLM, Image, Video, TTS, Diagram, Localization
//! - **Asset Management**: Content-addressed storage with versioning
//! - **Marketing Agents**: Autonomous content creation and distribution

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

// ============================================================================
// Core Infrastructure (std-gated)
// ============================================================================

/// Tool adapter trait and common types
#[cfg(feature = "std")]
pub mod tool_adapter;

/// GPU node management and heartbeat system
#[cfg(feature = "std")]
pub mod gpu_nodes;

/// Priority-based job queue dispatcher
#[cfg(feature = "std")]
pub mod job_queue;

/// Adapter abstractions and factory
#[cfg(feature = "std")]
pub mod adapters;

// ============================================================================
// Media Generation Adapters (std-gated)
// ============================================================================

/// LLM inference adapter with cloud fallback
#[cfg(feature = "std")]
pub mod llm_adapter;

/// Stable Diffusion XL image generation
#[cfg(feature = "std")]
pub mod image_adapter;

/// FFmpeg-based video orchestration
#[cfg(feature = "std")]
pub mod video_adapter;

/// Text-to-speech with XTTS-v2/Piper/ElevenLabs
#[cfg(feature = "std")]
pub mod tts_adapter;

/// Diagram and chart generation with Vega-Lite/Mermaid
#[cfg(feature = "std")]
pub mod diagram_adapter;

/// Localization, transcription, and translation
#[cfg(feature = "std")]
pub mod localization;

// ============================================================================
// Asset & Content Management (std-gated)
// ============================================================================

/// Content-addressed asset storage with versioning
#[cfg(feature = "std")]
pub mod asset_manager;

/// Content repurposing pipeline
#[cfg(feature = "std")]
pub mod repurposing;

/// Intent conversion for repurposing
#[cfg(feature = "std")]
pub mod repurposing_intent_converter;

// ============================================================================
// Reputation, Rewards & Compensation (std-gated)
// ============================================================================

/// Reputation scoring and node ranking
#[cfg(feature = "std")]
pub mod reputation;

/// Reward distribution system
#[cfg(feature = "std")]
pub mod rewards;

/// Contributor management
#[cfg(feature = "std")]
pub mod contributor;

// ============================================================================
// Marketing & Governance (std-gated)
// ============================================================================

/// Autonomous marketing agents
#[cfg(feature = "std")]
pub mod marketing_agents;

/// Marketing governance and approvals
#[cfg(feature = "std")]
pub mod marketing_governance;

/// Media orchestration pipeline
#[cfg(feature = "std")]
pub mod media_orchestration;

/// Content cadence management
#[cfg(feature = "std")]
pub mod cadence;

// ============================================================================
// RPC & External Interfaces (std-gated)
// ============================================================================

/// Core RPC API endpoints
#[cfg(feature = "std")]
pub mod rpc_api;

/// Media-specific RPC methods
#[cfg(feature = "std")]
pub mod rpc_media;

/// DNS resolution for node discovery
#[cfg(feature = "std")]
pub mod dns;

// ============================================================================
// Re-exports for convenience (std-gated)
// ============================================================================

#[cfg(feature = "std")]
pub use tool_adapter::{ToolAdapter, ToolType, ToolParams, ToolResult, JobId, JobStatus};
#[cfg(feature = "std")]
pub use gpu_nodes::{GpuNodeManager, NodeStatus};
#[cfg(feature = "std")]
pub use job_queue::JobDispatcher;
#[cfg(feature = "std")]
pub use llm_adapter::{LlmAdapter, LlmModel};
#[cfg(feature = "std")]
pub use image_adapter::{ImageAdapter, ImageGenerationParams};
#[cfg(feature = "std")]
pub use video_adapter::{VideoAdapter, VideoGenerationParams};
#[cfg(feature = "std")]
pub use tts_adapter::{TtsParams, TtsModel, VoicePreset};
#[cfg(feature = "std")]
pub use diagram_adapter::{DiagramAdapter, DiagramParams, DiagramType};
#[cfg(feature = "std")]
pub use localization::{Language, WhisperModel, SubtitleFormat};
#[cfg(feature = "std")]
pub use asset_manager::{AssetType, AssetStatus};
#[cfg(feature = "std")]
pub use reputation::ReputationManager;
#[cfg(feature = "std")]
pub use marketing_agents::{MarketingAgent, AgentType};

// ============================================================================
// Tests (std-only)
// ============================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Verify all major types are exported
        let _job_id: JobId = uuid::Uuid::new_v4();
        let _status = JobStatus::Queued;
        let _tool_type = ToolType::LlmInference;
    }

    #[test]
    fn test_adapter_types() {
        // Verify adapter types exist
        let _llm_model = LlmModel::Llama70B;
        let _tts_model = TtsModel::XttsV2;
        let _diagram_type = DiagramType::LineChart;
        let _whisper_model = WhisperModel::Large;
    }
}