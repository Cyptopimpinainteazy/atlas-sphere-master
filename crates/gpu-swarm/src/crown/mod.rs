//! The Crown - Meta-Governor for the GPU Swarm
//!
//! The Crown is the immune system that watches everything:
//! - Audits the Warden's decisions for drift, bias, and corruption
//! - Monitors chain health, profit flows, and security threats
//! - Prevents Evo Babies from gaming the system
//! - Overrides and recalibrates when thresholds are exceeded
//!
//! Hierarchy:
//! - Crown (brain stem) → regulates survival
//! - Warden (arms) → controls compute
//! - Auditor (eyes) → monitors everything
//! - Prophet (foresight) → predicts the future

pub mod auditor;
pub mod prophet;
pub mod scrapyard;

pub use auditor::{
    AuditReport, AuditSeverity, Auditor, ChainHealthMetrics, ProfitFlowMetrics, SecurityThreat,
    SwarmAnomalyType,
};
pub use prophet::{
    ForecastHorizon, MarketCycle, MarketForecast, Prophet, ThreatForecast, VolatilityRegime,
};
pub use scrapyard::{
    DisassemblyReport, QuarantineReason, RecycledKnowledge, Scrapyard, ScrapyardModule,
    ScrapyardVerdict,
};

use crate::warden::{
    AllocationPlan, ComputeLane, GovernanceAction, SwarmPillars, SwarmState, ThreatLevel, Warden,
    WardenDecision,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Crown evaluation cycle interval
pub const DEFAULT_CROWN_CYCLE_SECS: u64 = 300; // 5 minutes

/// Drift threshold before Crown intervenes
pub const DRIFT_THRESHOLD: f64 = 0.15;

/// Profit loss threshold before Crown intervenes (15% loss)
pub const PROFIT_LOSS_THRESHOLD: f64 = 0.15;

/// Maximum consecutive Warden errors before override
pub const MAX_WARDEN_ERRORS: u32 = 3;

/// Crown verdict on system state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CrownVerdict {
    /// System is healthy, Warden is performing well
    Healthy {
        confidence: f64,
        commendations: Vec<String>,
    },
    /// Minor issues detected, watching closely
    Caution {
        issues: Vec<CrownIssue>,
        recommendations: Vec<String>,
    },
    /// Significant problems, intervention recommended
    Warning {
        issues: Vec<CrownIssue>,
        required_actions: Vec<GovernanceAction>,
    },
    /// Critical failure, Crown takes over
    Override {
        reason: String,
        emergency_plan: EmergencyPlan,
        warden_suspended: bool,
    },
}

impl CrownVerdict {
    pub fn is_healthy(&self) -> bool {
        matches!(self, Self::Healthy { .. })
    }

    pub fn severity(&self) -> u8 {
        match self {
            Self::Healthy { .. } => 0,
            Self::Caution { .. } => 1,
            Self::Warning { .. } => 2,
            Self::Override { .. } => 3,
        }
    }
}

/// Issues detected by the Crown
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrownIssue {
    pub category: IssueCategory,
    pub description: String,
    pub severity: IssueSeverity,
    pub detected_at: u64,
    pub evidence: Vec<String>,
    pub suggested_fix: Option<String>,
}

/// Categories of issues the Crown can detect
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IssueCategory {
    /// Warden is drifting from optimal behavior
    WardenDrift,
    /// Warden is biased toward certain lanes
    AllocationBias,
    /// Profit is declining or negative
    ProfitDecline,
    /// Chain health is degrading
    ChainHealth,
    /// Security threats detected
    SecurityThreat,
    /// Evo Babies gaming the system
    EvolutionGaming,
    /// Resource exhaustion risk
    ResourceExhaustion,
    /// Mission creep detected
    MissionCreep,
    /// Model instability detected
    ModelInstability,
    /// Potential DoS on chain
    ChainStress,
}

/// Severity of detected issues
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum IssueSeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

/// Emergency plan when Crown takes over
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergencyPlan {
    /// Forced allocation overrides
    pub forced_allocations: HashMap<ComputeLane, f64>,
    /// Lanes to halt immediately
    pub halt_lanes: Vec<ComputeLane>,
    /// Modules to quarantine
    pub quarantine_modules: Vec<String>,
    /// Actions to execute
    pub actions: Vec<GovernanceAction>,
    /// Duration of emergency mode
    pub duration: Duration,
    /// Justification
    pub justification: String,
}

/// Historical record of Crown evaluations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrownEvaluation {
    pub timestamp: u64,
    pub verdict: CrownVerdict,
    pub warden_state: SwarmState,
    pub audit_report: AuditReport,
    pub prophet_forecast: Option<MarketForecast>,
    pub pillar_scores: SwarmPillars,
    pub actions_taken: Vec<GovernanceAction>,
}

/// Crown configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrownConfig {
    /// Evaluation cycle interval
    pub cycle_interval: Duration,
    /// Drift threshold before intervention
    pub drift_threshold: f64,
    /// Profit loss threshold
    pub profit_loss_threshold: f64,
    /// Max consecutive Warden errors
    pub max_warden_errors: u32,
    /// Enable Prophet forecasting
    pub prophet_enabled: bool,
    /// Enable Scrapyard recycling
    pub scrapyard_enabled: bool,
    /// Minimum healthy pillar score
    pub min_pillar_score: f64,
    /// Maximum single lane allocation
    pub max_lane_concentration: f64,
    /// Evolution lane max (prevent gaming)
    pub max_evolution_allocation: f64,
}

impl Default for CrownConfig {
    fn default() -> Self {
        Self {
            cycle_interval: Duration::from_secs(DEFAULT_CROWN_CYCLE_SECS),
            drift_threshold: DRIFT_THRESHOLD,
            profit_loss_threshold: PROFIT_LOSS_THRESHOLD,
            max_warden_errors: MAX_WARDEN_ERRORS,
            prophet_enabled: true,
            scrapyard_enabled: true,
            min_pillar_score: 0.25,
            max_lane_concentration: 0.45,
            max_evolution_allocation: 0.12, // Never let evolution dominate
        }
    }
}

/// The Crown - Meta-Governor watching everything
pub struct Crown {
    config: CrownConfig,
    auditor: Auditor,
    prophet: Prophet,
    scrapyard: Arc<RwLock<Scrapyard>>,
    /// Historical evaluations
    evaluation_history: VecDeque<CrownEvaluation>,
    /// Consecutive Warden errors
    warden_error_count: u32,
    /// Is Warden currently suspended?
    warden_suspended: bool,
    /// Last known good state
    last_good_state: Option<SwarmState>,
    /// Baseline allocations (for drift detection)
    baseline_allocations: HashMap<ComputeLane, f64>,
    /// Started at
    started_at: Instant,
}

impl Default for Crown {
    fn default() -> Self {
        Self::new(CrownConfig::default())
    }
}

impl Crown {
    /// Create new Crown with configuration
    pub fn new(config: CrownConfig) -> Self {
        Self {
            auditor: Auditor::new(),
            prophet: Prophet::new(config.prophet_enabled),
            scrapyard: Arc::new(RwLock::new(Scrapyard::new())),
            config,
            evaluation_history: VecDeque::with_capacity(1000),
            warden_error_count: 0,
            warden_suspended: false,
            last_good_state: None,
            baseline_allocations: Self::default_baseline(),
            started_at: Instant::now(),
        }
    }

    /// Default baseline allocations
    fn default_baseline() -> HashMap<ComputeLane, f64> {
        let mut baseline = HashMap::new();
        baseline.insert(ComputeLane::Security, 0.20);
        baseline.insert(ComputeLane::ChainOps, 0.15);
        baseline.insert(ComputeLane::Research, 0.20);
        baseline.insert(ComputeLane::Strategy, 0.20);
        baseline.insert(ComputeLane::AiAgents, 0.10);
        baseline.insert(ComputeLane::Ecosystem, 0.08);
        baseline.insert(ComputeLane::Storage, 0.04);
        baseline.insert(ComputeLane::Overflow, 0.03);
        baseline.insert(ComputeLane::Evolution, 0.00); // Disabled by default
        baseline
    }

    /// Execute a Crown evaluation cycle
    pub async fn evaluate(
        &mut self,
        warden_state: &SwarmState,
        warden_decision: Option<&WardenDecision>,
    ) -> CrownVerdict {
        let mut issues = Vec::new();

        // 1. Run full audit
        let audit_report = self.auditor.full_audit(warden_state).await;

        // 2. Check for security threats
        self.check_security_threats(&audit_report, &mut issues);

        // 3. Check chain health
        self.check_chain_health(&audit_report, &mut issues);

        // 4. Check profit flows
        self.check_profit_flows(&audit_report, &mut issues);

        // 5. Audit Warden for drift and bias
        if let Some(decision) = warden_decision {
            self.audit_warden_decision(decision, &mut issues);
        }

        // 6. Check for evolution gaming
        self.check_evolution_gaming(warden_state, &mut issues);

        // 7. Check pillar health
        self.check_pillar_health(warden_state, &mut issues);

        // 8. Get Prophet forecast (if enabled)
        let forecast = if self.config.prophet_enabled {
            Some(self.prophet.forecast(&audit_report).await)
        } else {
            None
        };

        // 9. Integrate Prophet warnings
        if let Some(ref f) = forecast {
            self.integrate_prophet_warnings(f, &mut issues);
        }

        // 10. Determine verdict
        let verdict = self.determine_verdict(&issues, &audit_report);

        // 11. Record evaluation
        self.record_evaluation(warden_state, &audit_report, forecast, &verdict);

        // 12. Update state based on verdict
        self.update_state_from_verdict(&verdict, warden_state);

        verdict
    }

    /// Check for security threats
    fn check_security_threats(&self, audit: &AuditReport, issues: &mut Vec<CrownIssue>) {
        for threat in &audit.security_threats {
            issues.push(CrownIssue {
                category: IssueCategory::SecurityThreat,
                description: threat.description.clone(),
                severity: match threat.severity {
                    AuditSeverity::Low => IssueSeverity::Low,
                    AuditSeverity::Medium => IssueSeverity::Medium,
                    AuditSeverity::High => IssueSeverity::High,
                    AuditSeverity::Critical => IssueSeverity::Critical,
                },
                detected_at: audit.timestamp,
                evidence: threat.indicators.clone(),
                suggested_fix: Some(threat.recommended_action.clone()),
            });
        }
    }

    /// Check chain health metrics
    fn check_chain_health(&self, audit: &AuditReport, issues: &mut Vec<CrownIssue>) {
        let health = &audit.chain_health;

        // Block time anomaly
        if health.avg_block_time_ms > 12000.0 {
            // > 12 seconds
            issues.push(CrownIssue {
                category: IssueCategory::ChainHealth,
                description: format!(
                    "Block time degraded: {:.1}ms (expected <12000ms)",
                    health.avg_block_time_ms
                ),
                severity: IssueSeverity::High,
                detected_at: audit.timestamp,
                evidence: vec![format!("Avg block time: {:.1}ms", health.avg_block_time_ms)],
                suggested_fix: Some("Reduce chain operations load".to_string()),
            });
        }

        // High error rate
        if health.error_rate > 0.05 {
            issues.push(CrownIssue {
                category: IssueCategory::ChainHealth,
                description: format!("High error rate: {:.1}%", health.error_rate * 100.0),
                severity: if health.error_rate > 0.15 {
                    IssueSeverity::Critical
                } else {
                    IssueSeverity::High
                },
                detected_at: audit.timestamp,
                evidence: vec![format!("Error rate: {:.2}%", health.error_rate * 100.0)],
                suggested_fix: Some("Investigate error sources, increase ChainOps allocation".to_string()),
            });
        }

        // Storage pressure
        if health.storage_usage > 0.90 {
            issues.push(CrownIssue {
                category: IssueCategory::ResourceExhaustion,
                description: format!("Storage critical: {:.1}%", health.storage_usage * 100.0),
                severity: IssueSeverity::Critical,
                detected_at: audit.timestamp,
                evidence: vec![format!("Storage usage: {:.1}%", health.storage_usage * 100.0)],
                suggested_fix: Some("Prune old data, increase storage allocation".to_string()),
            });
        }

        // Consensus issues
        if !health.consensus_healthy {
            issues.push(CrownIssue {
                category: IssueCategory::ChainHealth,
                description: "Consensus degraded or unhealthy".to_string(),
                severity: IssueSeverity::Critical,
                detected_at: audit.timestamp,
                evidence: health.consensus_warnings.clone(),
                suggested_fix: Some("Prioritize Security and ChainOps lanes immediately".to_string()),
            });
        }
    }

    /// Check profit flow metrics
    fn check_profit_flows(&self, audit: &AuditReport, issues: &mut Vec<CrownIssue>) {
        let profit = &audit.profit_flows;

        // Net loss
        if profit.net_profit < 0.0 {
            let loss_severity = if profit.net_profit < -1000.0 {
                IssueSeverity::Critical
            } else if profit.net_profit < -100.0 {
                IssueSeverity::High
            } else {
                IssueSeverity::Medium
            };

            issues.push(CrownIssue {
                category: IssueCategory::ProfitDecline,
                description: format!("Net loss: {} tokens", profit.net_profit),
                severity: loss_severity,
                detected_at: audit.timestamp,
                evidence: vec![
                    format!("Revenue: {:.2}", profit.total_revenue),
                    format!("Costs: {:.2}", profit.total_costs),
                    format!("Net: {:.2}", profit.net_profit),
                ],
                suggested_fix: Some(
                    "Increase Strategy allocation, reduce Research if needed".to_string(),
                ),
            });
        }

        // Declining trend
        if profit.profit_trend < -self.config.profit_loss_threshold {
            issues.push(CrownIssue {
                category: IssueCategory::ProfitDecline,
                description: format!(
                    "Profit declining rapidly: {:.1}%",
                    profit.profit_trend * 100.0
                ),
                severity: IssueSeverity::High,
                detected_at: audit.timestamp,
                evidence: vec![format!("Trend: {:.2}%", profit.profit_trend * 100.0)],
                suggested_fix: Some("Shift resources from Research to Strategy lanes".to_string()),
            });
        }
    }

    /// Audit Warden decision for drift and bias
    fn audit_warden_decision(&mut self, decision: &WardenDecision, issues: &mut Vec<CrownIssue>) {
        let allocations = &decision.allocation_plan.allocations;

        // Check for drift from baseline
        for (lane, alloc) in allocations {
            if let Some(&baseline) = self.baseline_allocations.get(&alloc.lane) {
                let drift = (alloc.compute_percent - baseline).abs();
                if drift > self.config.drift_threshold {
                    issues.push(CrownIssue {
                        category: IssueCategory::WardenDrift,
                        description: format!(
                            "{:?} drifted {:.1}% from baseline",
                            lane,
                            drift * 100.0
                        ),
                        severity: if drift > 0.25 {
                            IssueSeverity::High
                        } else {
                            IssueSeverity::Medium
                        },
                        detected_at: decision.decided_at,
                        evidence: vec![
                            format!("Current: {:.1}%", alloc.compute_percent * 100.0),
                            format!("Baseline: {:.1}%", baseline * 100.0),
                        ],
                        suggested_fix: Some("Consider resetting to baseline if unjustified".to_string()),
                    });
                }
            }
        }

        // Check for over-concentration
        for (lane, alloc) in allocations {
            if alloc.compute_percent > self.config.max_lane_concentration {
                issues.push(CrownIssue {
                    category: IssueCategory::AllocationBias,
                    description: format!(
                        "{:?} over-concentrated at {:.1}%",
                        lane,
                        alloc.compute_percent * 100.0
                    ),
                    severity: IssueSeverity::Medium,
                    detected_at: decision.decided_at,
                    evidence: vec![format!(
                        "Max allowed: {:.1}%",
                        self.config.max_lane_concentration * 100.0
                    )],
                    suggested_fix: Some("Redistribute to other lanes".to_string()),
                });
            }
        }

        // Check plan confidence
        if decision.allocation_plan.confidence < 0.5 {
            issues.push(CrownIssue {
                category: IssueCategory::WardenDrift,
                description: format!(
                    "Low Warden confidence: {:.1}%",
                    decision.allocation_plan.confidence * 100.0
                ),
                severity: IssueSeverity::Medium,
                detected_at: decision.decided_at,
                evidence: vec![format!(
                    "Confidence: {:.2}",
                    decision.allocation_plan.confidence
                )],
                suggested_fix: Some("Warden may need more signal data".to_string()),
            });
        }
    }

    /// Check for Evo Babies gaming the system
    fn check_evolution_gaming(&self, state: &SwarmState, issues: &mut Vec<CrownIssue>) {
        if let Some(evo_alloc) = state.allocations.get(&ComputeLane::Evolution) {
            // Evolution taking too much
            if *evo_alloc > self.config.max_evolution_allocation {
                issues.push(CrownIssue {
                    category: IssueCategory::EvolutionGaming,
                    description: format!(
                        "Evolution lane at {:.1}% - possible gaming",
                        evo_alloc * 100.0
                    ),
                    severity: IssueSeverity::High,
                    detected_at: state.timestamp,
                    evidence: vec![
                        format!("Current: {:.1}%", evo_alloc * 100.0),
                        format!("Max allowed: {:.1}%", self.config.max_evolution_allocation * 100.0),
                    ],
                    suggested_fix: Some(
                        "Cap Evolution allocation, audit Evo Baby signals".to_string(),
                    ),
                });
            }
        }
    }

    /// Check pillar health
    fn check_pillar_health(&self, state: &SwarmState, issues: &mut Vec<CrownIssue>) {
        let scores = state.pillars.pillar_scores();

        let check_pillar = |name: &str, score: f64, issues: &mut Vec<CrownIssue>| {
            if score < self.config.min_pillar_score {
                issues.push(CrownIssue {
                    category: IssueCategory::ChainHealth,
                    description: format!("{} pillar critical: {:.1}%", name, score * 100.0),
                    severity: if score < 0.15 {
                        IssueSeverity::Critical
                    } else {
                        IssueSeverity::High
                    },
                    detected_at: state.timestamp,
                    evidence: vec![format!("Score: {:.2}", score)],
                    suggested_fix: Some(format!("Boost {} pillar contributing lanes", name)),
                });
            }
        };

        check_pillar("Profit", scores.profit, issues);
        check_pillar("Intelligence", scores.intelligence, issues);
        check_pillar("Infrastructure", scores.infrastructure, issues);
        check_pillar("Ecosystem", scores.ecosystem, issues);
    }

    /// Integrate Prophet warnings
    fn integrate_prophet_warnings(&self, forecast: &MarketForecast, issues: &mut Vec<CrownIssue>) {
        // High volatility warning
        if matches!(forecast.volatility, VolatilityRegime::Extreme) {
            issues.push(CrownIssue {
                category: IssueCategory::SecurityThreat,
                description: "Prophet predicts extreme volatility incoming".to_string(),
                severity: IssueSeverity::High,
                detected_at: forecast.generated_at,
                evidence: vec![format!("Regime: {:?}", forecast.volatility)],
                suggested_fix: Some("Increase Security, reduce Strategy risk exposure".to_string()),
            });
        }

        // Threat forecast
        for threat in &forecast.threat_forecasts {
            if threat.probability > 0.7 {
                issues.push(CrownIssue {
                    category: IssueCategory::SecurityThreat,
                    description: format!(
                        "Prophet predicts: {} ({:.0}% probability)",
                        threat.threat_type, threat.probability * 100.0
                    ),
                    severity: if threat.probability > 0.9 {
                        IssueSeverity::Critical
                    } else {
                        IssueSeverity::High
                    },
                    detected_at: forecast.generated_at,
                    evidence: threat.indicators.clone(),
                    suggested_fix: threat.mitigation.clone(),
                });
            }
        }
    }

    /// Determine final verdict based on issues
    fn determine_verdict(&mut self, issues: &[CrownIssue], audit: &AuditReport) -> CrownVerdict {
        let critical_count = issues.iter().filter(|i| i.severity == IssueSeverity::Critical).count();
        let high_count = issues.iter().filter(|i| i.severity == IssueSeverity::High).count();

        // Critical threshold - Crown takes over
        if critical_count >= 2 || (critical_count >= 1 && high_count >= 3) {
            self.warden_error_count += 1;

            if self.warden_error_count >= self.config.max_warden_errors {
                return CrownVerdict::Override {
                    reason: format!(
                        "{} critical and {} high severity issues detected. Warden suspended after {} consecutive errors.",
                        critical_count, high_count, self.warden_error_count
                    ),
                    emergency_plan: self.create_emergency_plan(issues, audit),
                    warden_suspended: true,
                };
            }
        }

        // Warning threshold
        if critical_count >= 1 || high_count >= 2 {
            let required_actions = self.generate_required_actions(issues);
            return CrownVerdict::Warning {
                issues: issues.to_vec(),
                required_actions,
            };
        }

        // Caution threshold
        if !issues.is_empty() {
            let recommendations = self.generate_recommendations(issues);
            return CrownVerdict::Caution {
                issues: issues.to_vec(),
                recommendations,
            };
        }

        // All clear - reset error count
        self.warden_error_count = 0;

        CrownVerdict::Healthy {
            confidence: audit.overall_health_score,
            commendations: self.generate_commendations(audit),
        }
    }

    /// Create emergency plan for Crown takeover
    fn create_emergency_plan(&self, issues: &[CrownIssue], _audit: &AuditReport) -> EmergencyPlan {
        let mut forced_allocations = HashMap::new();
        let mut halt_lanes = Vec::new();
        let mut quarantine_modules = Vec::new();
        let mut actions = Vec::new();

        // Emergency allocation - prioritize survival
        forced_allocations.insert(ComputeLane::Security, 0.35);
        forced_allocations.insert(ComputeLane::ChainOps, 0.25);
        forced_allocations.insert(ComputeLane::Research, 0.15);
        forced_allocations.insert(ComputeLane::Strategy, 0.15);
        forced_allocations.insert(ComputeLane::AiAgents, 0.05);
        forced_allocations.insert(ComputeLane::Ecosystem, 0.03);
        forced_allocations.insert(ComputeLane::Storage, 0.02);
        forced_allocations.insert(ComputeLane::Overflow, 0.00);
        forced_allocations.insert(ComputeLane::Evolution, 0.00); // Always halt in emergency

        // Halt risky lanes
        halt_lanes.push(ComputeLane::Evolution);
        halt_lanes.push(ComputeLane::Overflow);

        // Check for evolution gaming - quarantine evo babies
        if issues.iter().any(|i| i.category == IssueCategory::EvolutionGaming) {
            quarantine_modules.push("evolution_engine".to_string());
            quarantine_modules.push("evo_baby_*".to_string());
        }

        // Add emergency governance actions
        actions.push(GovernanceAction::UpdateThreatLevel {
            level: ThreatLevel::Critical,
            source: "Crown Emergency".to_string(),
        });

        let justification = issues
            .iter()
            .map(|i| format!("{:?}: {}", i.category, i.description))
            .collect::<Vec<_>>()
            .join("; ");

        EmergencyPlan {
            forced_allocations,
            halt_lanes,
            quarantine_modules,
            actions,
            duration: Duration::from_secs(1800), // 30 minutes
            justification,
        }
    }

    /// Generate required actions for warning state
    fn generate_required_actions(&self, issues: &[CrownIssue]) -> Vec<GovernanceAction> {
        let mut actions = Vec::new();

        for issue in issues {
            match issue.category {
                IssueCategory::SecurityThreat if issue.severity >= IssueSeverity::High => {
                    actions.push(GovernanceAction::UpdateThreatLevel {
                        level: ThreatLevel::High,
                        source: "Crown detection".to_string(),
                    });
                }
                IssueCategory::EvolutionGaming => {
                    actions.push(GovernanceAction::HaltLane {
                        lane: ComputeLane::Evolution,
                        reason: "Evolution gaming detected".to_string(),
                    });
                }
                _ => {}
            }
        }

        actions
    }

    /// Generate recommendations for caution state
    fn generate_recommendations(&self, issues: &[CrownIssue]) -> Vec<String> {
        issues
            .iter()
            .filter_map(|i| i.suggested_fix.clone())
            .collect()
    }

    /// Generate commendations for healthy state
    fn generate_commendations(&self, audit: &AuditReport) -> Vec<String> {
        let mut commendations = Vec::new();

        if audit.profit_flows.net_profit > 0.0 {
            commendations.push(format!(
                "Profitable: +{:.2} tokens",
                audit.profit_flows.net_profit
            ));
        }

        if audit.chain_health.error_rate < 0.01 {
            commendations.push("Excellent error rate (<1%)".to_string());
        }

        if audit.overall_health_score > 0.8 {
            commendations.push(format!(
                "Outstanding health score: {:.1}%",
                audit.overall_health_score * 100.0
            ));
        }

        commendations
    }

    /// Record evaluation to history
    fn record_evaluation(
        &mut self,
        state: &SwarmState,
        audit: &AuditReport,
        forecast: Option<MarketForecast>,
        verdict: &CrownVerdict,
    ) {
        let evaluation = CrownEvaluation {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            verdict: verdict.clone(),
            warden_state: state.clone(),
            audit_report: audit.clone(),
            prophet_forecast: forecast,
            pillar_scores: state.pillars.clone(),
            actions_taken: match verdict {
                CrownVerdict::Warning { required_actions, .. } => required_actions.clone(),
                CrownVerdict::Override { emergency_plan, .. } => emergency_plan.actions.clone(),
                _ => vec![],
            },
        };

        if self.evaluation_history.len() >= 1000 {
            self.evaluation_history.pop_front();
        }
        self.evaluation_history.push_back(evaluation);
    }

    /// Update internal state based on verdict
    fn update_state_from_verdict(&mut self, verdict: &CrownVerdict, state: &SwarmState) {
        match verdict {
            CrownVerdict::Healthy { .. } => {
                self.last_good_state = Some(state.clone());
                self.warden_suspended = false;
            }
            CrownVerdict::Override { warden_suspended, .. } => {
                self.warden_suspended = *warden_suspended;
            }
            _ => {}
        }
    }

    /// Quarantine a module via the Scrapyard
    pub async fn quarantine_module(&self, module_id: &str, reason: QuarantineReason) {
        if self.config.scrapyard_enabled {
            let mut scrapyard = self.scrapyard.write().await;
            scrapyard.quarantine(module_id.to_string(), reason);
        }
    }

    /// Process scrapyard and recycle knowledge
    pub async fn process_scrapyard(&self) -> Vec<RecycledKnowledge> {
        if self.config.scrapyard_enabled {
            let mut scrapyard = self.scrapyard.write().await;
            scrapyard.process_all().await
        } else {
            vec![]
        }
    }

    /// Is Warden currently suspended?
    pub fn is_warden_suspended(&self) -> bool {
        self.warden_suspended
    }

    /// Resume Warden after suspension
    pub fn resume_warden(&mut self) {
        self.warden_suspended = false;
        self.warden_error_count = 0;
    }

    /// Get evaluation history
    pub fn history(&self) -> &VecDeque<CrownEvaluation> {
        &self.evaluation_history
    }

    /// Get uptime
    pub fn uptime(&self) -> Duration {
        self.started_at.elapsed()
    }

    /// Get configuration
    pub fn config(&self) -> &CrownConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crown_config_default() {
        let config = CrownConfig::default();
        assert!(config.max_evolution_allocation < 0.15);
        assert!(config.drift_threshold > 0.0);
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
            emergency_plan: EmergencyPlan {
                forced_allocations: HashMap::new(),
                halt_lanes: vec![],
                quarantine_modules: vec![],
                actions: vec![],
                duration: Duration::from_secs(60),
                justification: "test".to_string(),
            },
            warden_suspended: true,
        };

        assert!(healthy.is_healthy());
        assert!(!override_v.is_healthy());
        assert!(override_v.severity() > healthy.severity());
    }

    #[tokio::test]
    async fn test_crown_creation() {
        let crown = Crown::default();
        assert!(!crown.is_warden_suspended());
        assert!(crown.history().is_empty());
    }
}
