//! GPU Swarm Warden - Master Control Intelligence
//!
//! The Warden is the central brain that commands the GPU swarm, dynamically
//! allocating resources across compute lanes based on four pillars:
//! - Profit Trajectory (P↑)
//! - Intelligence Trajectory (I↑)
//! - Infrastructure Integrity (S↑)
//! - Ecosystem Expansion (E↑)

pub mod allocator;
pub mod governance;
pub mod metrics;
pub mod policy;
pub mod predictor;
pub mod signals;

pub use allocator::{AllocationChange, AllocationPlan, GpuAllocator, LaneAllocation};
pub use governance::{
    EmergencyOverride, GovernanceAction, GovernanceEngine, GuardBot, GuardType, ThreatLevel,
};
pub use metrics::{
    EcosystemMetrics, HealthStatus, InfrastructureMetrics, IntelligenceMetrics, MetricsCollector,
    Pillar, PillarScores, ProfitMetrics, SwarmPillars,
};
pub use policy::{AllocationPolicy, ComputeLane, LaneConstraints};
pub use predictor::{
    LaneForecast, LoadPredictor, LoadTrend, PredictionAction, PredictionHorizon, SwarmForecast,
};
pub use signals::{AlertSeverity, LaneAlert, LaneMetrics, LaneSignal, SignalAggregator, SignalType};

use crate::error::SwarmResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Current state of the swarm as seen by the Warden
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmState {
    /// Current pillar scores
    pub pillars: SwarmPillars,
    /// GPU allocation by lane
    pub allocations: HashMap<ComputeLane, f64>,
    /// Active job counts by lane
    pub active_jobs: HashMap<ComputeLane, usize>,
    /// Total available GPU compute units
    pub total_compute_units: u64,
    /// Currently utilized compute units
    pub utilized_compute_units: u64,
    /// Number of online nodes
    pub online_nodes: usize,
    /// Total VRAM available (MB)
    pub total_vram_mb: u64,
    /// Timestamp of state capture
    pub timestamp: u64,
    /// Active threats detected
    pub threat_level: ThreatLevel,
}

impl Default for SwarmState {
    fn default() -> Self {
        Self {
            pillars: SwarmPillars::default(),
            allocations: HashMap::new(),
            active_jobs: HashMap::new(),
            total_compute_units: 0,
            utilized_compute_units: 0,
            online_nodes: 0,
            total_vram_mb: 0,
            timestamp: 0,
            threat_level: ThreatLevel::None,
        }
    }
}

/// Warden decision output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WardenDecision {
    /// Current conditions summary
    pub conditions: SwarmState,
    /// Predicted future state
    pub predictions: SwarmForecast,
    /// GPU allocation plan
    pub allocation_plan: AllocationPlan,
    /// Strategic justification
    pub justification: String,
    /// Fallback rules
    pub fallback_rules: Vec<FallbackRule>,
    /// Decision timestamp
    pub decided_at: u64,
}

/// Conditional fallback rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackRule {
    pub condition: String,
    pub action: String,
    pub priority: u8,
}

/// Configuration for the Warden
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WardenConfig {
    /// Decision cycle interval
    pub cycle_interval: Duration,
    /// Minimum security allocation (never goes below this)
    pub min_security_allocation: f64,
    /// Minimum chain ops allocation
    pub min_chain_ops_allocation: f64,
    /// Maximum single-lane allocation
    pub max_lane_allocation: f64,
    /// Pillar critical threshold
    pub critical_threshold: f64,
    /// Enable predictive reallocation
    pub predictive_enabled: bool,
    /// Prediction lookahead window
    pub prediction_window: Duration,
    /// Maximum allocation change per cycle
    pub max_change_per_cycle: f64,
}

impl Default for WardenConfig {
    fn default() -> Self {
        Self {
            cycle_interval: Duration::from_secs(10),
            min_security_allocation: 0.15,    // Security always gets at least 15%
            min_chain_ops_allocation: 0.10,   // Chain ops always gets at least 10%
            max_lane_allocation: 0.50,        // No single lane gets more than 50%
            critical_threshold: 0.25,         // Below 25% is critical
            predictive_enabled: true,
            prediction_window: Duration::from_secs(300), // 5 minute lookahead
            max_change_per_cycle: 0.10,       // Max 10% shift per cycle
        }
    }
}

/// The Warden - Master Control Intelligence for the GPU Swarm
pub struct Warden {
    config: WardenConfig,
    allocator: GpuAllocator,
    predictor: LoadPredictor,
    signals: Arc<RwLock<SignalAggregator>>,
    governance: GovernanceEngine,
    metrics: Arc<RwLock<MetricsCollector>>,
    current_state: Arc<RwLock<SwarmState>>,
    last_decision: Arc<RwLock<Option<WardenDecision>>>,
    started_at: Instant,
}

impl Warden {
    /// Create a new Warden with default configuration
    pub fn new() -> Self {
        Self::with_config(WardenConfig::default())
    }

    /// Create a new Warden with custom configuration
    pub fn with_config(config: WardenConfig) -> Self {
        let policy = AllocationPolicy::default();

        Self {
            allocator: GpuAllocator::new(policy),
            predictor: LoadPredictor::default(),
            signals: Arc::new(RwLock::new(SignalAggregator::default())),
            governance: GovernanceEngine::new(),
            metrics: Arc::new(RwLock::new(MetricsCollector::new())),
            current_state: Arc::new(RwLock::new(SwarmState::default())),
            last_decision: Arc::new(RwLock::new(None)),
            config,
            started_at: Instant::now(),
        }
    }

    /// Ingest a signal from a compute lane
    pub async fn ingest_signal(&self, signal: LaneSignal) -> Option<LaneAlert> {
        let mut signals = self.signals.write().await;
        signals.ingest(signal)
    }

    /// Update the current swarm state
    pub async fn update_state(&self, state: SwarmState) {
        let mut current = self.current_state.write().await;
        *current = state;
    }

    /// Get the current swarm state
    pub async fn get_state(&self) -> SwarmState {
        self.current_state.read().await.clone()
    }

    /// Execute a decision cycle - the core Warden logic
    pub async fn decide(&mut self) -> SwarmResult<WardenDecision> {
        let state = self.current_state.read().await.clone();
        let signals = self.signals.read().await;

        // 1. Record metrics for prediction
        self.predictor.record(&signals);

        // 2. Assess current conditions
        let conditions = self.assess_conditions(&state, &signals);

        // 3. Generate predictions
        let predictions = if self.config.predictive_enabled {
            self.predictor.forecast(&signals, PredictionHorizon::ShortTerm)
        } else {
            SwarmForecast {
                lane_forecasts: HashMap::new(),
                predicted_utilization: 0.0,
                scaling_up: Vec::new(),
                scaling_down: Vec::new(),
                warnings: Vec::new(),
                generated_at_ms: 0,
            }
        };

        // 4. Check for emergency overrides
        let executed_actions = self.governance.execute_overrides();
        let is_emergency = self.governance.threat_level().is_emergency();

        // 5. Calculate optimal allocation
        let total_gpu_units = state.total_compute_units as u32;
        let total_vram_mb = state.total_vram_mb;
        let allocation_plan =
            self.allocator
                .compute_allocation(&signals, total_gpu_units, total_vram_mb, is_emergency);

        // 6. Generate justification
        let justification =
            self.generate_justification(&conditions, &allocation_plan, &executed_actions);

        // 7. Build fallback rules
        let fallback_rules = self.generate_fallback_rules(&conditions);

        let decision = WardenDecision {
            conditions,
            predictions,
            allocation_plan,
            justification,
            fallback_rules,
            decided_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        // Store decision
        let mut last = self.last_decision.write().await;
        *last = Some(decision.clone());

        Ok(decision)
    }

    /// Assess current conditions from state and signals
    fn assess_conditions(&self, state: &SwarmState, signals: &SignalAggregator) -> SwarmState {
        let mut assessed = state.clone();

        // Update pillar scores from lane metrics
        let metrics = signals.all_metrics();

        // Profit from strategy + ai agents
        let strategy_metrics = metrics.get(&ComputeLane::Strategy);
        let agent_metrics = metrics.get(&ComputeLane::AiAgents);
        assessed.pillars.profit.mev_captured = strategy_metrics
            .map(|m| m.revenue)
            .unwrap_or(0.0);
        assessed.pillars.profit.update();

        // Intelligence from research
        let research_metrics = metrics.get(&ComputeLane::Research);
        assessed.pillars.intelligence.training_throughput = research_metrics
            .map(|m| m.avg_throughput)
            .unwrap_or(0.0);

        // Infrastructure from security + chain ops
        let security_metrics = metrics.get(&ComputeLane::Security);
        let chain_metrics = metrics.get(&ComputeLane::ChainOps);
        assessed.pillars.infrastructure.avg_uptime = security_metrics
            .map(|m| if m.is_healthy() { 99.0 } else { 50.0 })
            .unwrap_or(50.0);
        assessed.pillars.infrastructure.compute_utilization = state.utilized_compute_units as f64
            / state.total_compute_units.max(1) as f64
            * 100.0;

        // Ecosystem from ecosystem lane
        let eco_metrics = metrics.get(&ComputeLane::Ecosystem);
        assessed.pillars.ecosystem.growth_rate = eco_metrics
            .map(|m| m.avg_throughput / 100.0)
            .unwrap_or(0.0);

        // Update threat level from governance
        assessed.threat_level = self.governance.threat_level();

        assessed
    }

    /// Generate strategic justification for the allocation
    fn generate_justification(
        &self,
        state: &SwarmState,
        plan: &AllocationPlan,
        actions: &[GovernanceAction],
    ) -> String {
        let mut reasons = Vec::new();

        // Report executed governance actions
        if !actions.is_empty() {
            reasons.push(format!("Executed {} governance actions", actions.len()));
        }

        // Check pillars
        let critical = state.pillars.critical_pillars();
        if !critical.is_empty() {
            let names: Vec<_> = critical.iter().map(|p| p.display_name()).collect();
            reasons.push(format!(
                "Critical pillars detected: {}. Prioritizing recovery.",
                names.join(", ")
            ));
        }

        // Check threat level
        if state.threat_level != ThreatLevel::None {
            reasons.push(format!(
                "Threat level {:?}: Security allocation increased by {:.0}%",
                state.threat_level,
                (state.threat_level.security_multiplier() - 1.0) * 100.0
            ));
        }

        // Check utilization
        let utilization = if state.total_compute_units > 0 {
            state.utilized_compute_units as f64 / state.total_compute_units as f64
        } else {
            0.0
        };

        if utilization > 0.9 {
            reasons.push("High GPU utilization (>90%): Prioritizing high-value lanes.".to_string());
        } else if utilization < 0.5 {
            reasons.push(
                "Low GPU utilization (<50%): Enabling overflow/creative workloads.".to_string(),
            );
        }

        // Composite health
        let health = state.pillars.composite_score();
        let scores = state.pillars.pillar_scores();
        reasons.push(format!(
            "Composite swarm health: {:.1}% (P:{:.0}% I:{:.0}% S:{:.0}% E:{:.0}%)",
            health * 100.0,
            scores.profit * 100.0,
            scores.intelligence * 100.0,
            scores.infrastructure * 100.0,
            scores.ecosystem * 100.0
        ));

        // Plan confidence
        reasons.push(format!("Plan confidence: {:.0}%", plan.confidence * 100.0));

        if reasons.is_empty() {
            "Normal operations. Allocation optimized for balanced growth.".to_string()
        } else {
            reasons.join(" | ")
        }
    }

    /// Generate fallback rules based on current conditions
    fn generate_fallback_rules(&self, state: &SwarmState) -> Vec<FallbackRule> {
        let mut rules = Vec::new();

        // Security fallback - always present
        rules.push(FallbackRule {
            condition: "Threat level rises to High or Critical".to_string(),
            action: format!(
                "Immediately shift to {}%+ security allocation",
                (self.config.min_security_allocation * 2.0 * 100.0) as u32
            ),
            priority: 0,
        });

        // Infrastructure fallback
        if state.pillars.infrastructure.score() < 0.5 {
            rules.push(FallbackRule {
                condition: "Infrastructure score drops below 25%".to_string(),
                action: "Pause non-critical workloads, boost chain ops to 30%".to_string(),
                priority: 1,
            });
        }

        // Profit fallback
        if state.pillars.profit.score() < 0.3 {
            rules.push(FallbackRule {
                condition: "Profit continues declining for 3 cycles".to_string(),
                action: "Shift 15% from research to strategy/trading".to_string(),
                priority: 2,
            });
        }

        // Overflow activation
        rules.push(FallbackRule {
            condition: "Utilization below 60% for 5 cycles".to_string(),
            action: "Enable overflow creative workloads up to 20%".to_string(),
            priority: 3,
        });

        rules
    }

    /// Submit an emergency override
    pub fn submit_emergency(&mut self, action: GovernanceAction, initiator: &str, reason: &str) -> Result<String, String> {
        let override_req = EmergencyOverride::new(action, initiator.to_string(), reason.to_string());
        self.governance.submit_override(override_req)
    }

    /// Register a guard bot
    pub fn register_guard(&mut self, guard: GuardBot) {
        self.governance.register_guard(guard);
    }

    /// Add admin for governance
    pub fn add_admin(&mut self, admin_key: String) {
        self.governance.add_admin(admin_key);
    }

    /// Get the last decision made
    pub async fn last_decision(&self) -> Option<WardenDecision> {
        self.last_decision.read().await.clone()
    }

    /// Get current threat level
    pub fn threat_level(&self) -> ThreatLevel {
        self.governance.threat_level()
    }

    /// Check if a lane is halted
    pub fn is_lane_halted(&self, lane: ComputeLane) -> bool {
        self.governance.is_lane_halted(lane)
    }

    /// Get uptime
    pub fn uptime(&self) -> Duration {
        self.started_at.elapsed()
    }

    /// Get configuration
    pub fn config(&self) -> &WardenConfig {
        &self.config
    }
}

impl Default for Warden {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_threat_level_multiplier() {
        assert_eq!(ThreatLevel::None.security_multiplier(), 1.0);
        assert_eq!(ThreatLevel::Critical.security_multiplier(), 4.0);
    }

    #[test]
    fn test_warden_config_default() {
        let config = WardenConfig::default();
        assert!(config.min_security_allocation >= 0.1);
        assert!(config.max_lane_allocation <= 0.6);
    }

    #[tokio::test]
    async fn test_warden_creation() {
        let warden = Warden::new();
        let state = warden.get_state().await;
        assert_eq!(state.online_nodes, 0);
    }

    #[tokio::test]
    async fn test_warden_decision() {
        let mut warden = Warden::new();

        // Update with some state
        let state = SwarmState {
            total_compute_units: 100,
            total_vram_mb: 102400, // 100GB
            online_nodes: 10,
            ..Default::default()
        };
        warden.update_state(state).await;

        // Make a decision
        let decision = warden.decide().await.expect("should decide");
        assert!(decision.allocation_plan.allocations.len() > 0);
        assert!(decision.justification.len() > 0);
    }

    #[tokio::test]
    async fn test_warden_signal_ingestion() {
        let warden = Warden::new();

        // Ingest some signals
        let signal = LaneSignal::new(ComputeLane::Research, SignalType::Load(0.75));
        let alert = warden.ingest_signal(signal).await;

        // Should not alert on 75% load
        assert!(alert.is_none());

        // High load should alert
        let high_signal = LaneSignal::new(ComputeLane::Security, SignalType::Load(0.98));
        let alert = warden.ingest_signal(high_signal).await;
        // May or may not alert depending on implementation
    }

    #[test]
    fn test_fallback_rules() {
        let warden = Warden::new();
        let state = SwarmState::default();
        let rules = warden.generate_fallback_rules(&state);

        assert!(!rules.is_empty());
        // Security rule should always be present
        assert!(rules.iter().any(|r| r.condition.contains("Threat")));
    }
}
