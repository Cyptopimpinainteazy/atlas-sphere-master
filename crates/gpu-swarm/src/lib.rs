//! GPU Swarm - Distributed GPU Compute Network for X3 Atlas Sphere
//!
//! This crate provides the core infrastructure for the X3 GPU Swarm, enabling:
//! - Distributed GPU compute across volunteer nodes
//! - Task scheduling and load balancing
//! - Deterministic execution verification
//! - Reward distribution for compute providers
//! - On-chain decision visibility (Block Announcer)
//! - Prophet-timed funding campaigns
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                     Swarm Coordinator                           │
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐ │
//! │  │ Task Queue  │  │ Scheduler   │  │ Verification Engine     │ │
//! │  └──────┬──────┘  └──────┬──────┘  └───────────┬─────────────┘ │
//! └─────────┼────────────────┼─────────────────────┼───────────────┘
//!           │                │                     │
//!     ┌─────▼─────┬──────────▼──────────┬──────────▼─────┐
//!     │           │                     │                │
//! ┌───▼───┐   ┌───▼───┐            ┌────▼────┐      ┌────▼────┐
//! │ Node  │   │ Node  │    ...     │  Node   │      │  Node   │
//! │ GPU:A │   │ GPU:B │            │  GPU:X  │      │  GPU:Y  │
//! └───────┘   └───────┘            └─────────┘      └─────────┘
//! ```
//!
//! # Governance Stack
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │  THE CROWN (Meta-Governor)                                      │
//! │  ├── Auditor: Chain health, profit flows, security monitoring   │
//! │  ├── Prophet: Market cycle forecasting, opportunity detection   │
//! │  └── Scrapyard: Failure recycling, knowledge extraction         │
//! └─────────────────────────────────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────────┐
//! │  THE WARDEN (GPU Allocator)                                     │
//! │  Four Pillars: Profit ↑ | Intelligence ↑ | Security ↑ | Eco ↑  │
//! └─────────────────────────────────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────────┐
//! │  BLOCK ANNOUNCER → On-Chain Events                              │
//! │  All decisions visible to anyone paying attention               │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Warden - Master Control Intelligence
//!
//! The Warden orchestrates GPU allocation across compute lanes, balancing:
//! - **Profit (P↑)**: MEV, trading, arbitrage revenue
//! - **Intelligence (I↑)**: AI research, model training
//! - **Infrastructure (S↑)**: Chain ops, security
//! - **Ecosystem (E↑)**: dApp support, community growth
//!
//! # Task Types
//!
//! - **X3 Bytecode Execution**: Run X3 MIR/JIT workloads on GPU
//! - **Mempool Simulation**: Parallel mempool scanning and simulation
//! - **Route Optimization**: Cross-chain arbitrage pathfinding
//! - **ML Training**: Reinforcement learning model training
//! - **Proof Generation**: ZK proof generation for verifiable compute
//! - **Funding Campaigns**: Prophet-timed VC/grant/social outreach
//!
//! # Security Model
//!
//! Nodes execute only sandboxed X3 bytecode with:
//! - No filesystem access
//! - No network access beyond swarm protocol
//! - No wallet/signing capabilities
//! - Deterministic execution verified by multiple nodes

pub mod announcer;
pub mod config;
pub mod coordinator;
pub mod crown;
pub mod error;
pub mod funding;
pub mod jobs;
pub mod network;
pub mod node;
pub mod node_pki;
pub mod protocol;
pub mod scheduler;
pub mod task;
pub mod verification;
pub mod warden;

// SHIP GATE: Module B — Watchdog + Timeout enforcement
pub mod watchdog;
pub mod timeout_enforcer;

pub use announcer::{
    AnnouncementPayload, AnnouncementSeverity, AnnouncementType, Announcer, AnnouncerConfig,
    AnnouncerStats, CrownAnnouncementPayload, FundingAnnouncementPayload,
    ProphetAnnouncementPayload, ScrapyardAnnouncementPayload, SwarmAnnouncement,
    WardenAnnouncementPayload,
};
pub use config::SwarmConfig;
pub use coordinator::{CoordinatorEvent, CoordinatorMetrics, SwarmCoordinator};
pub use crown::{
    AuditReport, AuditSeverity, Auditor, ChainHealthMetrics, Crown, CrownConfig, CrownEvaluation,
    CrownIssue, CrownVerdict, DisassemblyReport, EmergencyPlan, ForecastHorizon, IssueCategory,
    IssueSeverity, MarketCycle, MarketForecast, ProfitFlowMetrics, Prophet, QuarantineReason,
    RecycledKnowledge, Scrapyard, ScrapyardModule, ScrapyardVerdict, SecurityThreat,
    SwarmAnomalyType, ThreatForecast, VolatilityRegime,
};
pub use error::{SwarmError, SwarmResult};
pub use funding::{
    CampaignOrchestrator, CampaignSchedule, ContentTone, NovaFlux, NovaFluxConfig,
    OrchestratorConfig, SocialScript, WebhookBridge, WebhookConfig, WebhookPayload, WebhookResult,
};
pub use jobs::{
    CampaignType, ChainIndexingJob, FundingCampaignConfig, FundingCampaignJob,
    FundingCampaignResult, JobOutput, JobReceipt, JobSubmission, JobType, LlmEngine,
    MempoolAnalysisJob, MevDiscoveryJob, ModelTrainingJob, PersonalizationLevel, Prospect,
    SwarmJob, X3SimulationJob, ZkProvingJob,
};
pub use network::{NetworkConfig, NetworkEvent, NetworkManager, PeerId};
pub use node::{
    GpuBackend, GpuCapabilities, NodeId, NodeMetrics, NodeRegistry, NodeStatus, SwarmNode,
};
pub use node_pki::{NodePKIRegistry, NodePublicKey};
pub use protocol::{ExecutionProof, MessageEnvelope, SwarmMessage, TaskAssignment, TaskResult};
pub use scheduler::{SchedulerConfig, SchedulerStats, SchedulingStrategy, TaskScheduler};
pub use task::{Task, TaskExecution, TaskId, TaskMetadata, TaskPriority, TaskStatus, TaskType};
pub use verification::{ExecutionVerifier, Verdict, VerificationConfig, VerificationSummary};
pub use warden::{
    AllocationPlan, AllocationPolicy, ComputeLane, GovernanceAction, GovernanceEngine, GuardBot,
    GuardType, LaneAllocation, LaneSignal, LoadPredictor, MetricsCollector, SignalAggregator,
    SwarmPillars, SwarmState, ThreatLevel, Warden, WardenConfig, WardenDecision,
};

/// Current version of the swarm protocol
pub const PROTOCOL_VERSION: u32 = 1;

/// Maximum task payload size (16 MB)
pub const MAX_TASK_SIZE: usize = 16 * 1024 * 1024;

/// Default task timeout (5 minutes)
pub const DEFAULT_TASK_TIMEOUT_SECS: u64 = 300;

/// Minimum stake required to participate as a node (in X3 tokens)
pub const MIN_NODE_STAKE: u64 = 1000;
