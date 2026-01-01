//! Auditor Daemon - The Eyes of the Crown
//!
//! The Auditor monitors EVERYTHING and sends signals to the Crown and Warden:
//! - Chain logs, block times, errors
//! - Storage loads, MEV spikes
//! - Profit flows, security threats
//! - Swarm anomalies, gaming attempts
//!
//! The Auditor is a hybrid on-chain + off-chain observer.

use crate::warden::{ComputeLane, SwarmState};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

/// Audit interval for continuous monitoring
pub const AUDIT_INTERVAL_SECS: u64 = 30;

/// Maximum anomalies before escalation
pub const ANOMALY_ESCALATION_THRESHOLD: usize = 5;

/// Audit severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AuditSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Types of swarm anomalies detected
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SwarmAnomalyType {
    /// Module requesting excessive resources
    ResourceHogging { module_id: String, requested: f64 },
    /// Signals don't match actual performance
    SignalMismatch { lane: ComputeLane, expected: f64, actual: f64 },
    /// Module producing suspicious results
    SuspiciousOutput { module_id: String, reason: String },
    /// Unusual communication patterns
    AbnormalNetwork { source: String, target: String, frequency: u64 },
    /// Module trying to escalate privileges
    PrivilegeEscalation { module_id: String, attempted_action: String },
    /// Results that don't verify
    UnverifiableWork { module_id: String, task_id: String },
    /// Gaming attempt detected
    GamingAttempt { module_id: String, strategy: String },
    /// Runaway resource consumption
    RunawayConsumption { module_id: String, resource: String },
}

/// Security threat detected by auditor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityThreat {
    pub threat_type: String,
    pub severity: AuditSeverity,
    pub description: String,
    pub indicators: Vec<String>,
    pub source: Option<String>,
    pub recommended_action: String,
    pub detected_at: u64,
}

/// Chain health metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainHealthMetrics {
    /// Average block production time in milliseconds
    pub avg_block_time_ms: f64,
    /// Current block height
    pub block_height: u64,
    /// Finality lag (blocks behind head)
    pub finality_lag: u64,
    /// Pending transaction pool size
    pub mempool_size: u64,
    /// Transaction error rate (0.0 - 1.0)
    pub error_rate: f64,
    /// Total errors in observation window
    pub total_errors: u64,
    /// Storage usage percentage (0.0 - 1.0)
    pub storage_usage: f64,
    /// Database size in bytes
    pub db_size_bytes: u64,
    /// Is consensus healthy?
    pub consensus_healthy: bool,
    /// Consensus warnings
    pub consensus_warnings: Vec<String>,
    /// Active validator count
    pub active_validators: u32,
    /// Expected validator count
    pub expected_validators: u32,
    /// CPU load percentage
    pub cpu_load: f64,
    /// Memory usage percentage
    pub memory_usage: f64,
    /// Network latency in ms
    pub network_latency_ms: f64,
    /// Gas price (if applicable)
    pub gas_price: Option<f64>,
    /// MEV observed
    pub mev_detected: bool,
    /// MEV amount if detected
    pub mev_amount: Option<f64>,
}

impl Default for ChainHealthMetrics {
    fn default() -> Self {
        Self {
            avg_block_time_ms: 6000.0,
            block_height: 0,
            finality_lag: 0,
            mempool_size: 0,
            error_rate: 0.0,
            total_errors: 0,
            storage_usage: 0.0,
            db_size_bytes: 0,
            consensus_healthy: true,
            consensus_warnings: vec![],
            active_validators: 1,
            expected_validators: 1,
            cpu_load: 0.0,
            memory_usage: 0.0,
            network_latency_ms: 0.0,
            gas_price: None,
            mev_detected: false,
            mev_amount: None,
        }
    }
}

/// Profit flow metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfitFlowMetrics {
    /// Total revenue in observation window
    pub total_revenue: f64,
    /// Total costs in observation window
    pub total_costs: f64,
    /// Net profit
    pub net_profit: f64,
    /// Revenue by lane
    pub revenue_by_lane: HashMap<ComputeLane, f64>,
    /// Costs by lane
    pub costs_by_lane: HashMap<ComputeLane, f64>,
    /// Profit trend (-1.0 to 1.0, negative = declining)
    pub profit_trend: f64,
    /// Revenue per GPU hour
    pub revenue_per_gpu_hour: f64,
    /// Cost per GPU hour
    pub cost_per_gpu_hour: f64,
    /// ROI percentage
    pub roi_percent: f64,
    /// Most profitable lane
    pub top_lane: Option<ComputeLane>,
    /// Least profitable lane
    pub bottom_lane: Option<ComputeLane>,
    /// Fee revenue
    pub fee_revenue: f64,
    /// Staking rewards
    pub staking_rewards: f64,
    /// Compute sales revenue
    pub compute_sales: f64,
    /// Research sales revenue
    pub research_sales: f64,
    /// Operating expenses
    pub operating_expenses: f64,
    /// GPU power costs
    pub power_costs: f64,
}

impl Default for ProfitFlowMetrics {
    fn default() -> Self {
        Self {
            total_revenue: 0.0,
            total_costs: 0.0,
            net_profit: 0.0,
            revenue_by_lane: HashMap::new(),
            costs_by_lane: HashMap::new(),
            profit_trend: 0.0,
            revenue_per_gpu_hour: 0.0,
            cost_per_gpu_hour: 0.0,
            roi_percent: 0.0,
            top_lane: None,
            bottom_lane: None,
            fee_revenue: 0.0,
            staking_rewards: 0.0,
            compute_sales: 0.0,
            research_sales: 0.0,
            operating_expenses: 0.0,
            power_costs: 0.0,
        }
    }
}

/// Complete audit report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    /// Report timestamp
    pub timestamp: u64,
    /// Chain health metrics
    pub chain_health: ChainHealthMetrics,
    /// Profit flow metrics
    pub profit_flows: ProfitFlowMetrics,
    /// Detected security threats
    pub security_threats: Vec<SecurityThreat>,
    /// Detected swarm anomalies
    pub anomalies: Vec<SwarmAnomalyType>,
    /// Lane-by-lane audit
    pub lane_audits: HashMap<ComputeLane, LaneAudit>,
    /// Overall health score (0.0 - 1.0)
    pub overall_health_score: f64,
    /// Summary of findings
    pub summary: String,
    /// Modules flagged for review
    pub flagged_modules: Vec<String>,
    /// Recommended actions
    pub recommendations: Vec<String>,
}

/// Per-lane audit data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaneAudit {
    pub lane: ComputeLane,
    /// Allocated compute percent
    pub allocation: f64,
    /// Actual utilization
    pub utilization: f64,
    /// Tasks completed
    pub tasks_completed: u64,
    /// Tasks failed
    pub tasks_failed: u64,
    /// Success rate
    pub success_rate: f64,
    /// Revenue generated
    pub revenue: f64,
    /// Costs incurred
    pub costs: f64,
    /// Net contribution
    pub net_contribution: f64,
    /// Anomalies detected
    pub anomaly_count: u32,
    /// Health status
    pub healthy: bool,
    /// Notes
    pub notes: Vec<String>,
}

/// The Auditor Daemon
pub struct Auditor {
    /// Recent audit reports
    reports: VecDeque<AuditReport>,
    /// Anomaly tracking
    anomaly_history: VecDeque<(Instant, SwarmAnomalyType)>,
    /// Module reputation scores
    module_reputation: HashMap<String, f64>,
    /// Running profit tracking
    profit_history: VecDeque<(u64, f64)>,
    /// Block time samples
    block_time_samples: VecDeque<f64>,
    /// Started at
    started_at: Instant,
}

impl Default for Auditor {
    fn default() -> Self {
        Self::new()
    }
}

impl Auditor {
    /// Create new Auditor
    pub fn new() -> Self {
        Self {
            reports: VecDeque::with_capacity(100),
            anomaly_history: VecDeque::with_capacity(1000),
            module_reputation: HashMap::new(),
            profit_history: VecDeque::with_capacity(1000),
            block_time_samples: VecDeque::with_capacity(100),
            started_at: Instant::now(),
        }
    }

    /// Run a full audit of the swarm state
    pub async fn full_audit(&mut self, state: &SwarmState) -> AuditReport {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or_default();

        // Collect chain health metrics
        let chain_health = self.audit_chain_health(state).await;

        // Collect profit flow metrics
        let profit_flows = self.audit_profit_flows(state).await;

        // Detect security threats
        let security_threats = self.detect_security_threats(state).await;

        // Detect swarm anomalies
        let anomalies = self.detect_anomalies(state).await;

        // Audit each lane
        let lane_audits = self.audit_lanes(state).await;

        // Calculate overall health score
        let overall_health_score = self.calculate_health_score(
            &chain_health,
            &profit_flows,
            &security_threats,
            &anomalies,
        );

        // Generate summary
        let summary = self.generate_summary(
            &chain_health,
            &profit_flows,
            &security_threats,
            &anomalies,
            overall_health_score,
        );

        // Flag suspicious modules
        let flagged_modules = self.flag_modules(state, &anomalies);

        // Generate recommendations
        let recommendations = self.generate_recommendations(
            &chain_health,
            &profit_flows,
            &security_threats,
            &anomalies,
        );

        let report = AuditReport {
            timestamp,
            chain_health,
            profit_flows,
            security_threats,
            anomalies,
            lane_audits,
            overall_health_score,
            summary,
            flagged_modules,
            recommendations,
        };

        // Store report
        if self.reports.len() >= 100 {
            self.reports.pop_front();
        }
        self.reports.push_back(report.clone());

        report
    }

    /// Audit chain health
    async fn audit_chain_health(&mut self, _state: &SwarmState) -> ChainHealthMetrics {
        // In production, this would query actual chain metrics
        // For now, we provide simulated data with realistic logic

        let mut metrics = ChainHealthMetrics::default();

        // Simulate block time from samples
        if !self.block_time_samples.is_empty() {
            metrics.avg_block_time_ms =
                self.block_time_samples.iter().sum::<f64>() / self.block_time_samples.len() as f64;
        }

        // Simulate consensus health check
        metrics.consensus_healthy = metrics.avg_block_time_ms < 15000.0;
        if !metrics.consensus_healthy {
            metrics
                .consensus_warnings
                .push("Block time exceeds threshold".to_string());
        }

        // Simulate error rate
        metrics.error_rate = 0.02; // 2% baseline

        // Simulate resource usage
        metrics.cpu_load = 0.45; // 45%
        metrics.memory_usage = 0.60; // 60%
        metrics.storage_usage = 0.55; // 55%

        metrics
    }

    /// Audit profit flows
    async fn audit_profit_flows(&mut self, state: &SwarmState) -> ProfitFlowMetrics {
        let mut metrics = ProfitFlowMetrics::default();

        // Calculate revenue by lane (simulated based on allocation)
        for (lane, alloc) in &state.allocations {
            let lane_revenue = match lane {
                ComputeLane::Strategy => alloc * 1000.0, // Trading most profitable
                ComputeLane::Research => alloc * 300.0,
                ComputeLane::AiAgents => alloc * 200.0,
                ComputeLane::Ecosystem => alloc * 150.0,
                _ => alloc * 50.0,
            };
            metrics.revenue_by_lane.insert(*lane, lane_revenue);
            metrics.total_revenue += lane_revenue;

            let lane_cost = alloc * 100.0; // Base GPU cost
            metrics.costs_by_lane.insert(*lane, lane_cost);
            metrics.total_costs += lane_cost;
        }

        metrics.net_profit = metrics.total_revenue - metrics.total_costs;

        // Calculate trend from history
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if self.profit_history.len() >= 10 {
            let recent: Vec<f64> = self.profit_history.iter().rev().take(10).map(|p| p.1).collect();
            let older: Vec<f64> = self.profit_history.iter().rev().skip(10).take(10).map(|p| p.1).collect();

            if !older.is_empty() {
                let recent_avg = recent.iter().sum::<f64>() / recent.len() as f64;
                let older_avg = older.iter().sum::<f64>() / older.len() as f64;

                if older_avg.abs() > 0.001 {
                    metrics.profit_trend = (recent_avg - older_avg) / older_avg;
                }
            }
        }

        // Record profit
        if self.profit_history.len() >= 1000 {
            self.profit_history.pop_front();
        }
        self.profit_history.push_back((timestamp, metrics.net_profit));

        // Calculate ROI
        if metrics.total_costs > 0.0 {
            metrics.roi_percent = (metrics.net_profit / metrics.total_costs) * 100.0;
        }

        // Find top/bottom lanes
        if let Some((lane, _)) = metrics.revenue_by_lane.iter().max_by(|a, b| {
            let net_a = a.1 - metrics.costs_by_lane.get(a.0).unwrap_or(&0.0);
            let net_b = b.1 - metrics.costs_by_lane.get(b.0).unwrap_or(&0.0);
            net_a.partial_cmp(&net_b).unwrap_or(std::cmp::Ordering::Equal)
        }) {
            metrics.top_lane = Some(*lane);
        }

        metrics
    }

    /// Detect security threats
    async fn detect_security_threats(&self, _state: &SwarmState) -> Vec<SecurityThreat> {
        let mut threats = Vec::new();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Check for anomaly escalation
        let recent_anomalies: Vec<_> = self
            .anomaly_history
            .iter()
            .filter(|(instant, _)| instant.elapsed() < Duration::from_secs(300))
            .collect();

        if recent_anomalies.len() >= ANOMALY_ESCALATION_THRESHOLD {
            threats.push(SecurityThreat {
                threat_type: "AnomalyCluster".to_string(),
                severity: AuditSeverity::High,
                description: format!(
                    "{} anomalies in last 5 minutes - possible coordinated attack",
                    recent_anomalies.len()
                ),
                indicators: recent_anomalies
                    .iter()
                    .take(5)
                    .map(|(_, a)| format!("{:?}", a))
                    .collect(),
                source: None,
                recommended_action: "Investigate anomaly sources, consider lane isolation".to_string(),
                detected_at: timestamp,
            });
        }

        // Check for privilege escalation attempts
        let escalations: Vec<_> = recent_anomalies
            .iter()
            .filter(|(_, a)| matches!(a, SwarmAnomalyType::PrivilegeEscalation { .. }))
            .collect();

        if !escalations.is_empty() {
            threats.push(SecurityThreat {
                threat_type: "PrivilegeEscalation".to_string(),
                severity: AuditSeverity::Critical,
                description: format!("{} privilege escalation attempts detected", escalations.len()),
                indicators: escalations
                    .iter()
                    .map(|(_, a)| format!("{:?}", a))
                    .collect(),
                source: None,
                recommended_action: "Quarantine offending modules immediately".to_string(),
                detected_at: timestamp,
            });
        }

        threats
    }

    /// Detect swarm anomalies
    async fn detect_anomalies(&mut self, state: &SwarmState) -> Vec<SwarmAnomalyType> {
        let mut anomalies = Vec::new();

        // Check for resource hogging
        for (lane, alloc) in &state.allocations {
            if *alloc > 0.5 && *lane != ComputeLane::Security {
                anomalies.push(SwarmAnomalyType::ResourceHogging {
                    module_id: format!("{:?}_lane", lane),
                    requested: *alloc,
                });
            }
        }

        // Check for Evolution gaming (if Evolution enabled and taking too much)
        if let Some(evo_alloc) = state.allocations.get(&ComputeLane::Evolution) {
            if *evo_alloc > 0.15 {
                anomalies.push(SwarmAnomalyType::GamingAttempt {
                    module_id: "evolution_engine".to_string(),
                    strategy: format!(
                        "Evolution requesting {:.1}% - above safety threshold",
                        evo_alloc * 100.0
                    ),
                });
            }
        }

        // Record anomalies
        for anomaly in &anomalies {
            if self.anomaly_history.len() >= 1000 {
                self.anomaly_history.pop_front();
            }
            self.anomaly_history
                .push_back((Instant::now(), anomaly.clone()));
        }

        anomalies
    }

    /// Audit each compute lane
    async fn audit_lanes(&self, state: &SwarmState) -> HashMap<ComputeLane, LaneAudit> {
        let mut audits = HashMap::new();

        for (lane, allocation) in &state.allocations {
            let audit = LaneAudit {
                lane: *lane,
                allocation: *allocation,
                utilization: allocation * 0.85, // Simulated 85% utilization
                tasks_completed: (allocation * 1000.0) as u64,
                tasks_failed: (allocation * 20.0) as u64,
                success_rate: 0.98,
                revenue: allocation * 200.0,
                costs: allocation * 100.0,
                net_contribution: allocation * 100.0,
                anomaly_count: 0,
                healthy: true,
                notes: vec![],
            };
            audits.insert(*lane, audit);
        }

        audits
    }

    /// Calculate overall health score
    fn calculate_health_score(
        &self,
        chain: &ChainHealthMetrics,
        profit: &ProfitFlowMetrics,
        threats: &[SecurityThreat],
        anomalies: &[SwarmAnomalyType],
    ) -> f64 {
        let mut score = 1.0;

        // Deduct for chain issues
        if !chain.consensus_healthy {
            score -= 0.3;
        }
        if chain.error_rate > 0.05 {
            score -= 0.2;
        }
        if chain.storage_usage > 0.9 {
            score -= 0.15;
        }

        // Deduct for profit issues
        if profit.net_profit < 0.0 {
            score -= 0.2;
        }
        if profit.profit_trend < -0.1 {
            score -= 0.1;
        }

        // Deduct for security threats
        for threat in threats {
            match threat.severity {
                AuditSeverity::Critical => score -= 0.25,
                AuditSeverity::High => score -= 0.15,
                AuditSeverity::Medium => score -= 0.08,
                AuditSeverity::Low => score -= 0.03,
            }
        }

        // Deduct for anomalies
        score -= anomalies.len() as f64 * 0.05;

        score.max(0.0)
    }

    /// Generate audit summary
    fn generate_summary(
        &self,
        chain: &ChainHealthMetrics,
        profit: &ProfitFlowMetrics,
        threats: &[SecurityThreat],
        anomalies: &[SwarmAnomalyType],
        score: f64,
    ) -> String {
        let mut parts = Vec::new();

        // Health score
        let health_status = if score > 0.8 {
            "HEALTHY"
        } else if score > 0.5 {
            "CAUTION"
        } else {
            "CRITICAL"
        };
        parts.push(format!("System Status: {} ({:.0}%)", health_status, score * 100.0));

        // Chain status
        if chain.consensus_healthy {
            parts.push("Chain: Healthy".to_string());
        } else {
            parts.push("Chain: DEGRADED".to_string());
        }

        // Profit status
        if profit.net_profit > 0.0 {
            parts.push(format!("Profit: +{:.2}", profit.net_profit));
        } else {
            parts.push(format!("Profit: {:.2} (LOSS)", profit.net_profit));
        }

        // Threats
        if !threats.is_empty() {
            parts.push(format!("Threats: {}", threats.len()));
        }

        // Anomalies
        if !anomalies.is_empty() {
            parts.push(format!("Anomalies: {}", anomalies.len()));
        }

        parts.join(" | ")
    }

    /// Flag suspicious modules
    fn flag_modules(&self, _state: &SwarmState, anomalies: &[SwarmAnomalyType]) -> Vec<String> {
        let mut flagged = Vec::new();

        for anomaly in anomalies {
            match anomaly {
                SwarmAnomalyType::ResourceHogging { module_id, .. }
                | SwarmAnomalyType::SuspiciousOutput { module_id, .. }
                | SwarmAnomalyType::PrivilegeEscalation { module_id, .. }
                | SwarmAnomalyType::UnverifiableWork { module_id, .. }
                | SwarmAnomalyType::GamingAttempt { module_id, .. }
                | SwarmAnomalyType::RunawayConsumption { module_id, .. } => {
                    if !flagged.contains(module_id) {
                        flagged.push(module_id.clone());
                    }
                }
                _ => {}
            }
        }

        flagged
    }

    /// Generate recommendations
    fn generate_recommendations(
        &self,
        chain: &ChainHealthMetrics,
        profit: &ProfitFlowMetrics,
        threats: &[SecurityThreat],
        anomalies: &[SwarmAnomalyType],
    ) -> Vec<String> {
        let mut recs = Vec::new();

        // Chain recommendations
        if !chain.consensus_healthy {
            recs.push("Investigate consensus issues immediately".to_string());
        }
        if chain.storage_usage > 0.85 {
            recs.push("Consider pruning old data or expanding storage".to_string());
        }
        if chain.error_rate > 0.05 {
            recs.push("High error rate - review recent deployments".to_string());
        }

        // Profit recommendations
        if profit.net_profit < 0.0 {
            recs.push("Operating at loss - increase Strategy allocation".to_string());
        }
        if profit.profit_trend < -0.15 {
            recs.push("Profit declining rapidly - review all lane efficiency".to_string());
        }

        // Threat recommendations
        for threat in threats {
            recs.push(threat.recommended_action.clone());
        }

        // Anomaly recommendations
        if anomalies.iter().any(|a| matches!(a, SwarmAnomalyType::GamingAttempt { .. })) {
            recs.push("Gaming detected - audit Evolution lane signals".to_string());
        }

        recs
    }

    /// Record a block time sample
    pub fn record_block_time(&mut self, time_ms: f64) {
        if self.block_time_samples.len() >= 100 {
            self.block_time_samples.pop_front();
        }
        self.block_time_samples.push_back(time_ms);
    }

    /// Report an anomaly
    pub fn report_anomaly(&mut self, anomaly: SwarmAnomalyType) {
        if self.anomaly_history.len() >= 1000 {
            self.anomaly_history.pop_front();
        }
        self.anomaly_history.push_back((Instant::now(), anomaly));
    }

    /// Update module reputation
    pub fn update_reputation(&mut self, module_id: &str, delta: f64) {
        let rep = self.module_reputation.entry(module_id.to_string()).or_insert(1.0);
        *rep = (*rep + delta).clamp(0.0, 2.0);
    }

    /// Get module reputation
    pub fn reputation(&self, module_id: &str) -> f64 {
        *self.module_reputation.get(module_id).unwrap_or(&1.0)
    }

    /// Get recent reports
    pub fn recent_reports(&self) -> &VecDeque<AuditReport> {
        &self.reports
    }

    /// Get uptime
    pub fn uptime(&self) -> Duration {
        self.started_at.elapsed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auditor_creation() {
        let auditor = Auditor::new();
        assert!(auditor.recent_reports().is_empty());
    }

    #[test]
    fn test_reputation_tracking() {
        let mut auditor = Auditor::new();

        auditor.update_reputation("module_a", 0.5);
        assert_eq!(auditor.reputation("module_a"), 1.5);

        auditor.update_reputation("module_a", -1.0);
        assert_eq!(auditor.reputation("module_a"), 0.5);

        // Test clamping
        auditor.update_reputation("module_b", 5.0);
        assert_eq!(auditor.reputation("module_b"), 2.0);

        auditor.update_reputation("module_c", -5.0);
        assert_eq!(auditor.reputation("module_c"), 0.0);
    }

    #[test]
    fn test_block_time_recording() {
        let mut auditor = Auditor::new();

        for i in 0..150 {
            auditor.record_block_time(6000.0 + i as f64);
        }

        // Should be capped at 100
        assert_eq!(auditor.block_time_samples.len(), 100);
    }

    #[test]
    fn test_anomaly_reporting() {
        let mut auditor = Auditor::new();

        auditor.report_anomaly(SwarmAnomalyType::ResourceHogging {
            module_id: "test".to_string(),
            requested: 0.9,
        });

        assert_eq!(auditor.anomaly_history.len(), 1);
    }

    #[tokio::test]
    async fn test_full_audit() {
        use crate::warden::SwarmPillars;

        let mut auditor = Auditor::new();

        let state = SwarmState {
            pillars: SwarmPillars::default(),
            allocations: HashMap::from([
                (ComputeLane::Security, 0.2),
                (ComputeLane::Strategy, 0.3),
                (ComputeLane::Research, 0.2),
            ]),
            active_jobs: HashMap::new(),
            total_compute_units: 1000,
            utilized_compute_units: 500,
            online_nodes: 10,
            total_vram_mb: 80000,
            timestamp: 0,
            threat_level: crate::warden::ThreatLevel::None,
        };

        let report = auditor.full_audit(&state).await;

        assert!(report.overall_health_score > 0.0);
        assert!(report.overall_health_score <= 1.0);
        assert!(!report.summary.is_empty());
    }
}
