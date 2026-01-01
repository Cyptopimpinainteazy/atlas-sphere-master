//! Funding Bridge - NovaFlux Integration & n8n Orchestration
//!
//! This module bridges the GPU swarm with the funding-automator tooling,
//! providing:
//! - NovaFlux AI persona for content generation
//! - n8n webhook dispatch for workflow automation
//! - Campaign orchestration and scheduling
//! - Response tracking and optimization

pub mod novaflux;
pub mod orchestrator;
pub mod webhook;

pub use novaflux::{ContentTone, NovaFlux, NovaFluxConfig, SocialScript};
pub use orchestrator::{CampaignOrchestrator, CampaignSchedule, OrchestratorConfig};
pub use webhook::{WebhookBridge, WebhookConfig, WebhookPayload, WebhookResult};
