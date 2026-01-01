//! Block Announcer - On-Chain Decision Visibility
//!
//! The Announcer broadcasts all GPU Swarm governance decisions to the blockchain,
//! making them visible to anyone paying attention. Every Warden allocation,
//! Crown verdict, Prophet forecast, and Scrapyard action gets recorded.
//!
//! # Design Philosophy
//!
//! "Decisions made in darkness breed corruption. Decisions on-chain breed trust."
//!
//! All major swarm decisions are emitted as events that:
//! - Get included in blocks for permanent visibility
//! - Can be indexed by explorers and analytics
//! - Create accountability trails for AI governance
//! - Enable external validators to audit decision quality
//!
//! # Event Categories
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────┐
//! │  Block N                                                         │
//! │  ├─ WardenDecisionAnnounced { allocation_changes, justification }│
//! │  ├─ CrownVerdictAnnounced { verdict, issues, emergency_plan }    │
//! │  ├─ ProphetForecastAnnounced { cycle, volatility, confidence }   │
//! │  ├─ ScrapyardActionAnnounced { module_id, stage, verdict }       │
//! │  └─ FundingCampaignAnnounced { campaign_type, targets, status }  │
//! └──────────────────────────────────────────────────────────────────┘
//! ```

use crate::crown::{
    AuditReport, CrownEvaluation, CrownVerdict, MarketCycle, MarketForecast,
    QuarantineReason, ScrapyardVerdict, VolatilityRegime,
};
use crate::warden::{ComputeLane, PillarScores, SwarmState, ThreatLevel, WardenDecision};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{mpsc, RwLock};

/// Announcement types for different governance layers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnnouncementType {
    /// Warden GPU allocation decision
    WardenDecision,
    /// Crown meta-governance verdict
    CrownVerdict,
    /// Prophet market forecast
    ProphetForecast,
    /// Scrapyard module action
    ScrapyardAction,
    /// Funding campaign status
    FundingCampaign,
    /// Auditor security alert
    AuditorAlert,
    /// System emergency
    Emergency,
}

/// Severity level for announcements
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnnouncementSeverity {
    /// Routine information
    Info,
    /// Notable change
    Notice,
    /// Important decision
    Important,
    /// Critical action required
    Critical,
    /// Emergency broadcast
    Emergency,
}

/// Core announcement structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmAnnouncement {
    /// Unique announcement ID
    pub id: u64,
    /// Announcement type
    pub announcement_type: AnnouncementType,
    /// Severity level
    pub severity: AnnouncementSeverity,
    /// Block number when announced
    pub block_number: u64,
    /// Timestamp (Unix seconds)
    pub timestamp: u64,
    /// Announcement payload
    pub payload: AnnouncementPayload,
    /// Hash of previous announcement (chain integrity)
    pub previous_hash: [u8; 32],
    /// Announcement hash
    pub hash: [u8; 32],
}

/// Typed payloads for different announcement types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnnouncementPayload {
    /// Warden allocation decision
    Warden(WardenAnnouncementPayload),
    /// Crown governance verdict
    Crown(CrownAnnouncementPayload),
    /// Prophet market forecast
    Prophet(ProphetAnnouncementPayload),
    /// Scrapyard module action
    Scrapyard(ScrapyardAnnouncementPayload),
    /// Funding campaign update
    Funding(FundingAnnouncementPayload),
    /// Auditor security alert
    Auditor(AuditorAnnouncementPayload),
    /// Emergency broadcast
    Emergency(EmergencyAnnouncementPayload),
}

// ============================================================================
// Warden Announcements
// ============================================================================

/// Payload for Warden allocation decisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WardenAnnouncementPayload {
    /// Previous allocations by lane
    pub previous_allocations: HashMap<String, f64>,
    /// New allocations by lane
    pub new_allocations: HashMap<String, f64>,
    /// Changes from previous
    pub changes: Vec<AllocationChange>,
    /// Pillar scores that drove the decision
    pub pillar_scores: PillarScoresSummary,
    /// Current threat level
    pub threat_level: String,
    /// Strategic justification
    pub justification: String,
    /// Prediction that influenced this
    pub influenced_by_prediction: bool,
}

/// Single allocation change for announcement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocationChange {
    pub lane: String,
    pub from_percent: f64,
    pub to_percent: f64,
    pub delta: f64,
    pub reason: String,
}

/// Summary of pillar scores for announcement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PillarScoresSummary {
    pub profit: f64,
    pub intelligence: f64,
    pub security: f64,
    pub ecosystem: f64,
}

// ============================================================================
// Crown Announcements
// ============================================================================

/// Payload for Crown governance verdicts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrownAnnouncementPayload {
    /// Verdict level
    pub verdict: String,
    /// Issues detected
    pub issues: Vec<IssueAnnouncement>,
    /// Actions taken
    pub actions_taken: Vec<String>,
    /// Emergency plan if activated
    pub emergency_plan: Option<EmergencyPlanSummary>,
    /// Warden adjustment requested
    pub warden_adjustment: Option<String>,
    /// Evaluation cycle number
    pub evaluation_cycle: u64,
}

/// Issue announcement structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueAnnouncement {
    pub category: String,
    pub severity: String,
    pub description: String,
    pub detected_at: u64,
}

/// Emergency plan summary for announcement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergencyPlanSummary {
    pub plan_type: String,
    pub target_allocation: HashMap<String, f64>,
    pub duration_blocks: u64,
    pub auto_expire: bool,
}

// ============================================================================
// Prophet Announcements
// ============================================================================

/// Payload for Prophet market forecasts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProphetAnnouncementPayload {
    /// Current market cycle
    pub market_cycle: String,
    /// Cycle confidence (0-100)
    pub cycle_confidence: u8,
    /// Volatility regime
    pub volatility_regime: String,
    /// Predicted threats
    pub threats: Vec<ThreatForecastSummary>,
    /// Recommended lane adjustments
    pub lane_hints: HashMap<String, i8>,
    /// Forecast horizon
    pub horizon: String,
    /// Overall market sentiment (-100 to +100)
    pub sentiment: i8,
}

/// Threat forecast summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatForecastSummary {
    pub threat_type: String,
    pub probability: f64,
    pub estimated_impact: String,
    pub window_blocks: u64,
}

// ============================================================================
// Scrapyard Announcements
// ============================================================================

/// Payload for Scrapyard module actions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapyardAnnouncementPayload {
    /// Module ID affected
    pub module_id: String,
    /// Module type
    pub module_type: String,
    /// Current stage
    pub stage: String,
    /// Action taken
    pub action: String,
    /// Quarantine reason if applicable
    pub quarantine_reason: Option<String>,
    /// Verdict if rendered
    pub verdict: Option<String>,
    /// Knowledge extracted if recycled
    pub knowledge_extracted: Option<Vec<String>>,
    /// Blacklisted if executed
    pub blacklisted: bool,
}

// ============================================================================
// Funding Announcements
// ============================================================================

/// Payload for Funding campaign updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingAnnouncementPayload {
    /// Campaign ID
    pub campaign_id: String,
    /// Campaign type
    pub campaign_type: String,
    /// Status
    pub status: String,
    /// Target count
    pub target_count: usize,
    /// Sent count
    pub sent_count: usize,
    /// Response count
    pub response_count: usize,
    /// Success rate
    pub success_rate: f64,
    /// Prophet-timed (was this triggered by market forecast?)
    pub prophet_timed: bool,
}

// ============================================================================
// Auditor Announcements
// ============================================================================

/// Payload for Auditor security alerts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditorAnnouncementPayload {
    /// Alert type
    pub alert_type: String,
    /// Severity
    pub severity: String,
    /// Entity involved
    pub entity: String,
    /// Anomaly type if applicable
    pub anomaly_type: Option<String>,
    /// Metrics snapshot
    pub metrics: AuditMetricsSummary,
    /// Recommended action
    pub recommended_action: String,
}

/// Audit metrics summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditMetricsSummary {
    pub chain_health_score: f64,
    pub profit_flow_roi: f64,
    pub security_threat_level: String,
    pub active_anomalies: usize,
}

// ============================================================================
// Emergency Announcements
// ============================================================================

/// Payload for system emergencies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmergencyAnnouncementPayload {
    /// Emergency type
    pub emergency_type: String,
    /// Trigger source
    pub triggered_by: String,
    /// Affected systems
    pub affected_systems: Vec<String>,
    /// Automatic actions taken
    pub auto_actions: Vec<String>,
    /// Manual intervention required
    pub requires_manual: bool,
    /// Estimated recovery blocks
    pub estimated_recovery_blocks: Option<u64>,
}

// ============================================================================
// Announcer Implementation
// ============================================================================

/// Configuration for the Announcer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnouncerConfig {
    /// Enable announcements
    pub enabled: bool,
    /// Minimum severity to announce
    pub min_severity: AnnouncementSeverity,
    /// Maximum announcements per block
    pub max_per_block: usize,
    /// Enable announcement chaining (hash chain)
    pub enable_chaining: bool,
    /// Webhook URL for external notifications
    pub webhook_url: Option<String>,
    /// Enable on-chain event emission
    pub emit_on_chain: bool,
}

impl Default for AnnouncerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_severity: AnnouncementSeverity::Info,
            max_per_block: 100,
            enable_chaining: true,
            webhook_url: None,
            emit_on_chain: true,
        }
    }
}

/// Callback for on-chain event emission
pub type OnChainEmitter = Box<dyn Fn(SwarmAnnouncement) + Send + Sync>;

/// The Block Announcer
pub struct Announcer {
    config: AnnouncerConfig,
    /// Current block number
    current_block: Arc<RwLock<u64>>,
    /// Announcement counter
    announcement_counter: Arc<RwLock<u64>>,
    /// Last announcement hash (for chaining)
    last_hash: Arc<RwLock<[u8; 32]>>,
    /// Pending announcements for current block
    pending: Arc<RwLock<Vec<SwarmAnnouncement>>>,
    /// Announcement history (last N blocks)
    history: Arc<RwLock<Vec<SwarmAnnouncement>>>,
    /// Channel for announcement dispatch
    dispatch_tx: mpsc::Sender<SwarmAnnouncement>,
    /// On-chain emitter callback
    on_chain_emitter: Option<Arc<OnChainEmitter>>,
    /// Started timestamp
    started_at: Instant,
}

impl Announcer {
    /// Create a new Announcer
    pub fn new(config: AnnouncerConfig) -> Self {
        let (dispatch_tx, _dispatch_rx) = mpsc::channel(1000);

        Self {
            config,
            current_block: Arc::new(RwLock::new(0)),
            announcement_counter: Arc::new(RwLock::new(0)),
            last_hash: Arc::new(RwLock::new([0u8; 32])),
            pending: Arc::new(RwLock::new(Vec::new())),
            history: Arc::new(RwLock::new(Vec::new())),
            dispatch_tx,
            on_chain_emitter: None,
            started_at: Instant::now(),
        }
    }

    /// Set the on-chain emitter callback
    pub fn set_on_chain_emitter(&mut self, emitter: OnChainEmitter) {
        self.on_chain_emitter = Some(Arc::new(emitter));
    }

    /// Update current block number
    pub async fn set_block(&self, block: u64) {
        let mut current = self.current_block.write().await;
        *current = block;
    }

    /// Compute announcement hash
    fn compute_hash(announcement: &SwarmAnnouncement) -> [u8; 32] {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        announcement.id.hash(&mut hasher);
        announcement.block_number.hash(&mut hasher);
        announcement.timestamp.hash(&mut hasher);

        let hash_value = hasher.finish();
        let mut result = [0u8; 32];
        result[..8].copy_from_slice(&hash_value.to_le_bytes());
        result[8..16].copy_from_slice(&announcement.previous_hash[..8]);
        result
    }

    /// Create and queue an announcement
    async fn create_announcement(
        &self,
        announcement_type: AnnouncementType,
        severity: AnnouncementSeverity,
        payload: AnnouncementPayload,
    ) -> SwarmAnnouncement {
        let mut counter = self.announcement_counter.write().await;
        *counter += 1;
        let id = *counter;

        let block_number = *self.current_block.read().await;
        let previous_hash = *self.last_hash.read().await;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut announcement = SwarmAnnouncement {
            id,
            announcement_type,
            severity,
            block_number,
            timestamp,
            payload,
            previous_hash,
            hash: [0u8; 32],
        };

        announcement.hash = Self::compute_hash(&announcement);

        // Update last hash
        if self.config.enable_chaining {
            let mut last = self.last_hash.write().await;
            *last = announcement.hash;
        }

        announcement
    }

    /// Announce a Warden decision
    pub async fn announce_warden_decision(&self, decision: &WardenDecision, previous_state: &SwarmState) {
        if !self.config.enabled {
            return;
        }

        let mut previous_allocations = HashMap::new();
        let mut new_allocations = HashMap::new();
        let mut changes = Vec::new();

        // Convert allocations to string keys
        for (lane, alloc_pct) in &previous_state.allocations {
            previous_allocations.insert(format!("{:?}", lane), *alloc_pct);
        }

        for (lane, alloc) in &decision.allocation_plan.allocations {
            let lane_str = format!("{:?}", lane);
            let prev = previous_allocations.get(&lane_str).copied().unwrap_or(0.0);
            new_allocations.insert(lane_str.clone(), alloc.compute_percent);

            if (alloc.compute_percent - prev).abs() > 0.01 {
                changes.push(AllocationChange {
                    lane: lane_str,
                    from_percent: prev * 100.0,
                    to_percent: alloc.compute_percent * 100.0,
                    delta: (alloc.compute_percent - prev) * 100.0,
                    reason: alloc.reason.clone(),
                });
            }
        }

        // Get pillar scores by calling methods
        let pillar_scores = decision.conditions.pillars.pillar_scores();

        let payload = WardenAnnouncementPayload {
            previous_allocations,
            new_allocations,
            changes,
            pillar_scores: PillarScoresSummary {
                profit: pillar_scores.profit,
                intelligence: pillar_scores.intelligence,
                security: pillar_scores.infrastructure, // infrastructure acts as security
                ecosystem: pillar_scores.ecosystem,
            },
            threat_level: format!("{:?}", decision.conditions.threat_level),
            justification: decision.justification.clone(),
            influenced_by_prediction: !decision.predictions.lane_forecasts.is_empty(),
        };

        let severity = match decision.conditions.threat_level {
            ThreatLevel::None => AnnouncementSeverity::Info,
            ThreatLevel::Low => AnnouncementSeverity::Notice,
            ThreatLevel::Elevated => AnnouncementSeverity::Important,
            ThreatLevel::High => AnnouncementSeverity::Critical,
            ThreatLevel::Critical => AnnouncementSeverity::Emergency,
        };

        let announcement = self
            .create_announcement(AnnouncementType::WardenDecision, severity, AnnouncementPayload::Warden(payload))
            .await;

        self.dispatch(announcement).await;
    }

    /// Announce a Crown verdict
    pub async fn announce_crown_verdict(&self, evaluation: &CrownEvaluation) {
        if !self.config.enabled {
            return;
        }

        // Extract issues from verdict if present
        let issues: Vec<IssueAnnouncement> = match &evaluation.verdict {
            CrownVerdict::Caution { issues, .. } |
            CrownVerdict::Warning { issues, .. } => {
                issues.iter().map(|i| IssueAnnouncement {
                    category: format!("{:?}", i.category),
                    severity: format!("{:?}", i.severity),
                    description: i.description.clone(),
                    detected_at: i.detected_at,
                }).collect()
            },
            _ => Vec::new(),
        };

        // Extract emergency plan from verdict if present
        let emergency_plan = match &evaluation.verdict {
            CrownVerdict::Override { emergency_plan, .. } => {
                Some(EmergencyPlanSummary {
                    plan_type: "Override".to_string(),
                    target_allocation: emergency_plan
                        .forced_allocations
                        .iter()
                        .map(|(k, v)| (format!("{:?}", k), *v))
                        .collect(),
                    duration_blocks: (emergency_plan.duration.as_secs() / 6) as u64, // Convert duration to blocks (6s per block)
                    auto_expire: true, // EmergencyPlan doesn't have auto_expire, assume true
                })
            },
            _ => None,
        };

        let actions_taken: Vec<String> = evaluation
            .actions_taken
            .iter()
            .map(|a| format!("{:?}", a))
            .collect();

        let payload = CrownAnnouncementPayload {
            verdict: format!("{:?}", evaluation.verdict),
            issues,
            actions_taken,
            emergency_plan,
            warden_adjustment: None, // CrownEvaluation doesn't have this field
            evaluation_cycle: evaluation.timestamp,
        };

        let severity = match &evaluation.verdict {
            CrownVerdict::Healthy { .. } => AnnouncementSeverity::Info,
            CrownVerdict::Caution { .. } => AnnouncementSeverity::Notice,
            CrownVerdict::Warning { .. } => AnnouncementSeverity::Important,
            CrownVerdict::Override { .. } => AnnouncementSeverity::Critical,
        };

        let announcement = self
            .create_announcement(AnnouncementType::CrownVerdict, severity, AnnouncementPayload::Crown(payload))
            .await;

        self.dispatch(announcement).await;
    }

    /// Announce a Prophet forecast
    pub async fn announce_prophet_forecast(&self, forecast: &MarketForecast) {
        if !self.config.enabled {
            return;
        }

        let threats: Vec<ThreatForecastSummary> = forecast
            .threat_forecasts
            .iter()
            .map(|t| ThreatForecastSummary {
                threat_type: t.threat_type.clone(),
                probability: t.probability,
                estimated_impact: format!("{:.2}", t.impact_severity),
                window_blocks: match t.expected_timing {
                    crate::crown::ForecastHorizon::Immediate => 100,
                    crate::crown::ForecastHorizon::Daily => 14400, // ~24h at 6s blocks
                    crate::crown::ForecastHorizon::Weekly => 100800,
                    crate::crown::ForecastHorizon::Monthly => 432000,
                    crate::crown::ForecastHorizon::Quarterly => 1296000,
                },
            })
            .collect();

        // Convert allocation_hints (f64) to lane_hints (i8 percentage)
        let lane_hints: HashMap<String, i8> = forecast
            .allocation_hints
            .iter()
            .map(|(k, v)| (format!("{:?}", k), (*v * 100.0) as i8))
            .collect();

        let payload = ProphetAnnouncementPayload {
            market_cycle: format!("{:?}", forecast.cycle),
            cycle_confidence: (forecast.cycle_confidence * 100.0) as u8,
            volatility_regime: format!("{:?}", forecast.volatility),
            threats,
            lane_hints,
            horizon: format!("{:?}s", forecast.valid_for.as_secs()),
            sentiment: (forecast.sentiment * 100.0) as i8,
        };

        // Higher severity for extreme market conditions
        let severity = match forecast.volatility {
            VolatilityRegime::Low | VolatilityRegime::Normal => AnnouncementSeverity::Info,
            VolatilityRegime::Elevated => AnnouncementSeverity::Notice,
            VolatilityRegime::High => AnnouncementSeverity::Important,
            VolatilityRegime::Extreme => AnnouncementSeverity::Critical,
        };

        let announcement = self
            .create_announcement(
                AnnouncementType::ProphetForecast,
                severity,
                AnnouncementPayload::Prophet(payload),
            )
            .await;

        self.dispatch(announcement).await;
    }

    /// Announce a Scrapyard action
    pub async fn announce_scrapyard_action(
        &self,
        module_id: &str,
        module_type: &str,
        stage: &str,
        action: &str,
        quarantine_reason: Option<QuarantineReason>,
        verdict: Option<ScrapyardVerdict>,
        knowledge: Option<Vec<String>>,
        blacklisted: bool,
    ) {
        if !self.config.enabled {
            return;
        }

        let verdict_str = verdict.as_ref().map(|v| format!("{:?}", v));
        let has_verdict = verdict.is_some();

        let payload = ScrapyardAnnouncementPayload {
            module_id: module_id.to_string(),
            module_type: module_type.to_string(),
            stage: stage.to_string(),
            action: action.to_string(),
            quarantine_reason: quarantine_reason.map(|r| format!("{:?}", r)),
            verdict: verdict_str,
            knowledge_extracted: knowledge,
            blacklisted,
        };

        let severity = if blacklisted {
            AnnouncementSeverity::Critical
        } else if has_verdict {
            AnnouncementSeverity::Important
        } else {
            AnnouncementSeverity::Notice
        };

        let announcement = self
            .create_announcement(
                AnnouncementType::ScrapyardAction,
                severity,
                AnnouncementPayload::Scrapyard(payload),
            )
            .await;

        self.dispatch(announcement).await;
    }

    /// Announce a Funding campaign update
    pub async fn announce_funding_campaign(
        &self,
        campaign_id: &str,
        campaign_type: &str,
        status: &str,
        target_count: usize,
        sent_count: usize,
        response_count: usize,
        prophet_timed: bool,
    ) {
        if !self.config.enabled {
            return;
        }

        let success_rate = if sent_count > 0 {
            (response_count as f64 / sent_count as f64) * 100.0
        } else {
            0.0
        };

        let payload = FundingAnnouncementPayload {
            campaign_id: campaign_id.to_string(),
            campaign_type: campaign_type.to_string(),
            status: status.to_string(),
            target_count,
            sent_count,
            response_count,
            success_rate,
            prophet_timed,
        };

        let announcement = self
            .create_announcement(
                AnnouncementType::FundingCampaign,
                AnnouncementSeverity::Notice,
                AnnouncementPayload::Funding(payload),
            )
            .await;

        self.dispatch(announcement).await;
    }

    /// Announce an Auditor alert
    pub async fn announce_auditor_alert(&self, report: &AuditReport, alert_type: &str, entity: &str) {
        if !self.config.enabled {
            return;
        }

        // Get first anomaly type if any
        let anomaly_type = report.anomalies.first().map(|a| format!("{:?}", a));

        let payload = AuditorAnnouncementPayload {
            alert_type: alert_type.to_string(),
            severity: format!("{:.2}", report.overall_health_score), // Use health score as severity indicator
            entity: entity.to_string(),
            anomaly_type,
            metrics: AuditMetricsSummary {
                chain_health_score: report.overall_health_score,
                profit_flow_roi: report.profit_flows.roi_percent,
                security_threat_level: if report.security_threats.is_empty() {
                    "None".to_string()
                } else {
                    format!("{:?}", report.security_threats[0].severity)
                },
                active_anomalies: report.anomalies.len(),
            },
            recommended_action: report.recommendations.first().cloned().unwrap_or_default(),
        };

        // Determine severity based on health score
        let severity = if report.overall_health_score > 0.8 {
            AnnouncementSeverity::Info
        } else if report.overall_health_score > 0.6 {
            AnnouncementSeverity::Notice
        } else if report.overall_health_score > 0.4 {
            AnnouncementSeverity::Important
        } else {
            AnnouncementSeverity::Critical
        };

        let announcement = self
            .create_announcement(
                AnnouncementType::AuditorAlert,
                severity,
                AnnouncementPayload::Auditor(payload),
            )
            .await;

        self.dispatch(announcement).await;
    }

    /// Announce an emergency
    pub async fn announce_emergency(
        &self,
        emergency_type: &str,
        triggered_by: &str,
        affected_systems: Vec<String>,
        auto_actions: Vec<String>,
        requires_manual: bool,
        estimated_recovery: Option<u64>,
    ) {
        // Emergencies are always announced regardless of config
        let payload = EmergencyAnnouncementPayload {
            emergency_type: emergency_type.to_string(),
            triggered_by: triggered_by.to_string(),
            affected_systems,
            auto_actions,
            requires_manual,
            estimated_recovery_blocks: estimated_recovery,
        };

        let announcement = self
            .create_announcement(
                AnnouncementType::Emergency,
                AnnouncementSeverity::Emergency,
                AnnouncementPayload::Emergency(payload),
            )
            .await;

        self.dispatch(announcement).await;
    }

    /// Dispatch announcement to all outputs
    async fn dispatch(&self, announcement: SwarmAnnouncement) {
        // Check severity threshold
        if (announcement.severity as u8) < (self.config.min_severity as u8) {
            return;
        }

        // Add to pending for current block
        {
            let mut pending = self.pending.write().await;
            if pending.len() < self.config.max_per_block {
                pending.push(announcement.clone());
            }
        }

        // Add to history
        {
            let mut history = self.history.write().await;
            history.push(announcement.clone());
            // Keep last 10000 announcements
            if history.len() > 10000 {
                history.drain(0..1000);
            }
        }

        // Send to dispatch channel
        let _ = self.dispatch_tx.try_send(announcement.clone());

        // Call on-chain emitter if configured
        if self.config.emit_on_chain {
            if let Some(emitter) = &self.on_chain_emitter {
                emitter(announcement);
            }
        }
    }

    /// Finalize block and get all announcements
    pub async fn finalize_block(&self) -> Vec<SwarmAnnouncement> {
        let mut pending = self.pending.write().await;
        let announcements = std::mem::take(&mut *pending);
        announcements
    }

    /// Get announcement history
    pub async fn get_history(&self, limit: usize) -> Vec<SwarmAnnouncement> {
        let history = self.history.read().await;
        history.iter().rev().take(limit).cloned().collect()
    }

    /// Get announcements for a specific block
    pub async fn get_block_announcements(&self, block: u64) -> Vec<SwarmAnnouncement> {
        let history = self.history.read().await;
        history
            .iter()
            .filter(|a| a.block_number == block)
            .cloned()
            .collect()
    }

    /// Get statistics
    pub async fn get_stats(&self) -> AnnouncerStats {
        let history = self.history.read().await;
        let current_block = *self.current_block.read().await;

        let mut by_type: HashMap<String, usize> = HashMap::new();
        let mut by_severity: HashMap<String, usize> = HashMap::new();

        for ann in history.iter() {
            *by_type.entry(format!("{:?}", ann.announcement_type)).or_default() += 1;
            *by_severity.entry(format!("{:?}", ann.severity)).or_default() += 1;
        }

        AnnouncerStats {
            total_announcements: history.len(),
            current_block,
            by_type,
            by_severity,
            uptime_seconds: self.started_at.elapsed().as_secs(),
        }
    }
}

/// Announcer statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnouncerStats {
    pub total_announcements: usize,
    pub current_block: u64,
    pub by_type: HashMap<String, usize>,
    pub by_severity: HashMap<String, usize>,
    pub uptime_seconds: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_announcer_creation() {
        let config = AnnouncerConfig::default();
        let announcer = Announcer::new(config);
        
        announcer.set_block(100).await;
        
        let stats = announcer.get_stats().await;
        assert_eq!(stats.current_block, 100);
        assert_eq!(stats.total_announcements, 0);
    }

    #[tokio::test]
    async fn test_funding_announcement() {
        let config = AnnouncerConfig::default();
        let announcer = Announcer::new(config);
        
        announcer.set_block(1).await;
        
        announcer
            .announce_funding_campaign(
                "campaign-001",
                "VC_OUTREACH",
                "ACTIVE",
                50,
                25,
                5,
                true,
            )
            .await;
        
        let history = announcer.get_history(10).await;
        assert_eq!(history.len(), 1);
        
        if let AnnouncementPayload::Funding(payload) = &history[0].payload {
            assert_eq!(payload.campaign_id, "campaign-001");
            assert_eq!(payload.success_rate, 20.0);
            assert!(payload.prophet_timed);
        } else {
            panic!("Expected Funding payload");
        }
    }

    #[tokio::test]
    async fn test_emergency_announcement() {
        let config = AnnouncerConfig::default();
        let announcer = Announcer::new(config);
        
        announcer.set_block(999).await;
        
        announcer
            .announce_emergency(
                "CHAIN_HALT",
                "Crown",
                vec!["Warden".to_string(), "Scheduler".to_string()],
                vec!["Pause jobs".to_string(), "Save state".to_string()],
                true,
                Some(100),
            )
            .await;
        
        let history = announcer.get_history(10).await;
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].severity, AnnouncementSeverity::Emergency);
    }

    #[tokio::test]
    async fn test_announcement_chaining() {
        let config = AnnouncerConfig {
            enable_chaining: true,
            ..Default::default()
        };
        let announcer = Announcer::new(config);
        
        announcer.set_block(1).await;
        
        // Create multiple announcements
        announcer
            .announce_funding_campaign("c1", "VC", "ACTIVE", 10, 5, 1, false)
            .await;
        announcer
            .announce_funding_campaign("c2", "GRANT", "PENDING", 20, 0, 0, false)
            .await;
        
        let history = announcer.get_history(10).await;
        assert_eq!(history.len(), 2);
        
        // Verify chaining - second announcement should reference first
        // (history is reversed, so [0] is second, [1] is first)
        assert_ne!(history[0].previous_hash, [0u8; 32]);
    }

    #[tokio::test]
    async fn test_severity_filtering() {
        let config = AnnouncerConfig {
            min_severity: AnnouncementSeverity::Important,
            ..Default::default()
        };
        let announcer = Announcer::new(config);
        
        announcer.set_block(1).await;
        
        // Info level should be filtered
        announcer
            .announce_funding_campaign("c1", "VC", "ACTIVE", 10, 5, 1, false)
            .await;
        
        let history = announcer.get_history(10).await;
        // Funding announcements are Notice severity, should be filtered
        assert_eq!(history.len(), 0);
    }

    #[tokio::test]
    async fn test_block_finalization() {
        let config = AnnouncerConfig::default();
        let announcer = Announcer::new(config);
        
        announcer.set_block(100).await;
        
        announcer
            .announce_funding_campaign("c1", "VC", "ACTIVE", 10, 5, 1, false)
            .await;
        announcer
            .announce_funding_campaign("c2", "GRANT", "ACTIVE", 20, 10, 2, true)
            .await;
        
        let block_announcements = announcer.finalize_block().await;
        assert_eq!(block_announcements.len(), 2);
        
        // Pending should be cleared
        let next_block = announcer.finalize_block().await;
        assert_eq!(next_block.len(), 0);
    }
}
