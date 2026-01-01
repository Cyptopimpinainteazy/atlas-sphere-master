//! Swarm Metrics Collection
//!
//! Tracks the four pillars: Profit (P↑), Intelligence (I↑), Infrastructure (S↑), Ecosystem (E↑)

use crate::warden::policy::ComputeLane;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Profit metrics (P↑)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProfitMetrics {
    /// Total revenue generated (tokens)
    pub total_revenue: f64,
    /// Revenue this cycle
    pub cycle_revenue: f64,
    /// MEV captured (tokens)
    pub mev_captured: f64,
    /// Arbitrage profits
    pub arbitrage_profits: f64,
    /// Trading bot profits
    pub trading_profits: f64,
    /// Service fees collected
    pub service_fees: f64,
    /// GPU rental revenue
    pub rental_revenue: f64,
    /// Operating costs this cycle
    pub operating_costs: f64,
    /// Net profit (revenue - costs)
    pub net_profit: f64,
    /// Profit trend (positive = growing)
    pub profit_trend: f64,
}

impl ProfitMetrics {
    /// Calculate profit score (0.0 - 1.0)
    pub fn score(&self) -> f64 {
        let profit_factor = if self.cycle_revenue > 0.0 {
            (self.net_profit / self.cycle_revenue).max(0.0).min(1.0)
        } else {
            0.0
        };

        let trend_factor = (self.profit_trend + 1.0) / 2.0; // Normalize -1..1 to 0..1
        let efficiency = if self.operating_costs > 0.0 {
            (self.net_profit / self.operating_costs).max(0.0).min(2.0) / 2.0
        } else {
            0.5
        };

        (profit_factor * 0.4 + trend_factor * 0.3 + efficiency * 0.3).min(1.0)
    }

    /// Is profit healthy?
    pub fn is_healthy(&self) -> bool {
        self.net_profit >= 0.0 && self.profit_trend >= -0.1
    }

    /// Update net profit
    pub fn update(&mut self) {
        self.cycle_revenue = self.mev_captured
            + self.arbitrage_profits
            + self.trading_profits
            + self.service_fees
            + self.rental_revenue;
        self.net_profit = self.cycle_revenue - self.operating_costs;
    }
}

/// Intelligence metrics (I↑)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IntelligenceMetrics {
    /// Number of models trained
    pub models_trained: u32,
    /// Model accuracy improvements
    pub accuracy_improvements: f64,
    /// Research tasks completed
    pub research_completed: u32,
    /// AI agent success rate
    pub agent_success_rate: f64,
    /// Novel strategies discovered
    pub strategies_discovered: u32,
    /// Knowledge base growth (entries)
    pub knowledge_growth: u32,
    /// Training throughput (samples/sec)
    pub training_throughput: f64,
    /// Inference latency (ms)
    pub inference_latency: f64,
    /// Intelligence trend
    pub intelligence_trend: f64,
}

impl IntelligenceMetrics {
    /// Calculate intelligence score (0.0 - 1.0)
    pub fn score(&self) -> f64 {
        let success_factor = self.agent_success_rate;
        let accuracy_factor = (self.accuracy_improvements * 10.0).min(1.0);
        let throughput_factor = (self.training_throughput / 10000.0).min(1.0);
        let latency_factor = (1.0 - (self.inference_latency / 1000.0)).max(0.0);

        (success_factor * 0.3
            + accuracy_factor * 0.25
            + throughput_factor * 0.25
            + latency_factor * 0.2)
            .min(1.0)
    }

    /// Is intelligence growing?
    pub fn is_growing(&self) -> bool {
        self.intelligence_trend >= 0.0
    }
}

/// Infrastructure metrics (S↑)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InfrastructureMetrics {
    /// Total GPU nodes online
    pub nodes_online: u32,
    /// Total GPU nodes registered
    pub nodes_total: u32,
    /// Average node uptime (%)
    pub avg_uptime: f64,
    /// Network latency (ms)
    pub network_latency: f64,
    /// Storage capacity used (%)
    pub storage_utilization: f64,
    /// Memory utilization (%)
    pub memory_utilization: f64,
    /// Compute utilization (%)
    pub compute_utilization: f64,
    /// Blocks produced this cycle
    pub blocks_produced: u32,
    /// Chain finality time (ms)
    pub finality_time: f64,
    /// Failed tasks this cycle
    pub failed_tasks: u32,
    /// Total tasks this cycle
    pub total_tasks: u32,
    /// Infrastructure trend
    pub stability_trend: f64,
}

impl InfrastructureMetrics {
    /// Calculate infrastructure score (0.0 - 1.0)
    pub fn score(&self) -> f64 {
        let uptime_factor = self.avg_uptime / 100.0;
        let utilization_factor = 1.0 - (self.compute_utilization.max(0.9) - 0.6).abs() * 2.5;
        let success_rate = if self.total_tasks > 0 {
            1.0 - (self.failed_tasks as f64 / self.total_tasks as f64)
        } else {
            0.5
        };
        let latency_factor = (1.0 - (self.network_latency / 500.0)).max(0.0);

        (uptime_factor * 0.3
            + utilization_factor * 0.25
            + success_rate * 0.25
            + latency_factor * 0.2)
            .min(1.0)
    }

    /// Node availability percentage
    pub fn availability(&self) -> f64 {
        if self.nodes_total > 0 {
            self.nodes_online as f64 / self.nodes_total as f64
        } else {
            0.0
        }
    }

    /// Is infrastructure stable?
    pub fn is_stable(&self) -> bool {
        self.availability() >= 0.8 && self.avg_uptime >= 95.0
    }
}

/// Ecosystem metrics (E↑)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EcosystemMetrics {
    /// Active developers
    pub active_developers: u32,
    /// dApps deployed
    pub dapps_deployed: u32,
    /// New dApps this cycle
    pub new_dapps: u32,
    /// Total transactions
    pub total_transactions: u64,
    /// Unique users
    pub unique_users: u32,
    /// New users this cycle
    pub new_users: u32,
    /// Community growth rate
    pub growth_rate: f64,
    /// Token price trend
    pub price_trend: f64,
    /// Ecosystem TVL
    pub total_tvl: f64,
    /// Partner integrations
    pub integrations: u32,
    /// Ecosystem trend
    pub ecosystem_trend: f64,
}

impl EcosystemMetrics {
    /// Calculate ecosystem score (0.0 - 1.0)
    pub fn score(&self) -> f64 {
        let growth_factor = (self.growth_rate + 1.0) / 2.0; // Normalize -1..1 to 0..1
        let user_factor = (self.unique_users as f64 / 10000.0).min(1.0);
        let dapp_factor = (self.dapps_deployed as f64 / 100.0).min(1.0);
        let tvl_factor = (self.total_tvl / 10_000_000.0).min(1.0);

        (growth_factor * 0.3 + user_factor * 0.25 + dapp_factor * 0.25 + tvl_factor * 0.2).min(1.0)
    }

    /// Is ecosystem growing?
    pub fn is_growing(&self) -> bool {
        self.growth_rate > 0.0 && self.ecosystem_trend >= 0.0
    }
}

/// Combined swarm pillars
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SwarmPillars {
    /// Profit metrics
    pub profit: ProfitMetrics,
    /// Intelligence metrics
    pub intelligence: IntelligenceMetrics,
    /// Infrastructure metrics
    pub infrastructure: InfrastructureMetrics,
    /// Ecosystem metrics
    pub ecosystem: EcosystemMetrics,
}

impl SwarmPillars {
    /// Calculate composite score (0.0 - 1.0)
    pub fn composite_score(&self) -> f64 {
        let p = self.profit.score();
        let i = self.intelligence.score();
        let s = self.infrastructure.score();
        let e = self.ecosystem.score();

        // Infrastructure is weighted higher (it's foundational)
        (p * 0.25 + i * 0.25 + s * 0.30 + e * 0.20)
    }

    /// Get individual pillar scores
    pub fn pillar_scores(&self) -> PillarScores {
        PillarScores {
            profit: self.profit.score(),
            intelligence: self.intelligence.score(),
            infrastructure: self.infrastructure.score(),
            ecosystem: self.ecosystem.score(),
        }
    }

    /// Identify weakest pillar
    pub fn weakest_pillar(&self) -> Pillar {
        let scores = self.pillar_scores();
        let min_score = scores
            .profit
            .min(scores.intelligence)
            .min(scores.infrastructure)
            .min(scores.ecosystem);

        if scores.profit == min_score {
            Pillar::Profit
        } else if scores.intelligence == min_score {
            Pillar::Intelligence
        } else if scores.infrastructure == min_score {
            Pillar::Infrastructure
        } else {
            Pillar::Ecosystem
        }
    }

    /// Get critical pillars (score < 0.3)
    pub fn critical_pillars(&self) -> Vec<Pillar> {
        let mut critical = Vec::new();
        let scores = self.pillar_scores();

        if scores.profit < 0.3 {
            critical.push(Pillar::Profit);
        }
        if scores.intelligence < 0.3 {
            critical.push(Pillar::Intelligence);
        }
        if scores.infrastructure < 0.3 {
            critical.push(Pillar::Infrastructure);
        }
        if scores.ecosystem < 0.3 {
            critical.push(Pillar::Ecosystem);
        }

        critical
    }

    /// Overall health status
    pub fn health_status(&self) -> HealthStatus {
        let score = self.composite_score();
        let critical = self.critical_pillars();

        if !critical.is_empty() {
            HealthStatus::Critical(critical)
        } else if score >= 0.8 {
            HealthStatus::Excellent
        } else if score >= 0.6 {
            HealthStatus::Good
        } else if score >= 0.4 {
            HealthStatus::Fair
        } else {
            HealthStatus::Poor
        }
    }
}

/// Individual pillar scores
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PillarScores {
    pub profit: f64,
    pub intelligence: f64,
    pub infrastructure: f64,
    pub ecosystem: f64,
}

/// Pillar identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Pillar {
    Profit,
    Intelligence,
    Infrastructure,
    Ecosystem,
}

impl Pillar {
    /// Get lanes that contribute to this pillar
    pub fn contributing_lanes(&self) -> Vec<ComputeLane> {
        match self {
            Self::Profit => vec![
                ComputeLane::Strategy,
                ComputeLane::AiAgents,
                ComputeLane::Overflow,
            ],
            Self::Intelligence => vec![
                ComputeLane::Research,
                ComputeLane::AiAgents,
                ComputeLane::Evolution,
            ],
            Self::Infrastructure => vec![
                ComputeLane::ChainOps,
                ComputeLane::Security,
                ComputeLane::Storage,
            ],
            Self::Ecosystem => vec![ComputeLane::Ecosystem, ComputeLane::Overflow],
        }
    }

    /// Display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Profit => "Profit (P↑)",
            Self::Intelligence => "Intelligence (I↑)",
            Self::Infrastructure => "Infrastructure (S↑)",
            Self::Ecosystem => "Ecosystem (E↑)",
        }
    }
}

/// Overall health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    Excellent,
    Good,
    Fair,
    Poor,
    Critical(Vec<Pillar>),
}

impl HealthStatus {
    pub fn is_healthy(&self) -> bool {
        matches!(self, Self::Excellent | Self::Good)
    }
}

/// Metrics collector
pub struct MetricsCollector {
    /// Current pillar metrics
    pillars: SwarmPillars,
    /// Historical scores (for trend calculation)
    score_history: Vec<(u64, f64)>,
    /// Max history entries
    max_history: usize,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsCollector {
    /// Create new collector
    pub fn new() -> Self {
        Self {
            pillars: SwarmPillars::default(),
            score_history: Vec::new(),
            max_history: 1000,
        }
    }

    /// Update profit metrics
    pub fn update_profit(&mut self, update: impl FnOnce(&mut ProfitMetrics)) {
        update(&mut self.pillars.profit);
        self.pillars.profit.update();
        self.record_score();
    }

    /// Update intelligence metrics
    pub fn update_intelligence(&mut self, update: impl FnOnce(&mut IntelligenceMetrics)) {
        update(&mut self.pillars.intelligence);
        self.record_score();
    }

    /// Update infrastructure metrics
    pub fn update_infrastructure(&mut self, update: impl FnOnce(&mut InfrastructureMetrics)) {
        update(&mut self.pillars.infrastructure);
        self.record_score();
    }

    /// Update ecosystem metrics
    pub fn update_ecosystem(&mut self, update: impl FnOnce(&mut EcosystemMetrics)) {
        update(&mut self.pillars.ecosystem);
        self.record_score();
    }

    /// Record current score to history
    fn record_score(&mut self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        self.score_history
            .push((now, self.pillars.composite_score()));

        if self.score_history.len() > self.max_history {
            self.score_history.remove(0);
        }
    }

    /// Get current pillars
    pub fn pillars(&self) -> &SwarmPillars {
        &self.pillars
    }

    /// Get current composite score
    pub fn composite_score(&self) -> f64 {
        self.pillars.composite_score()
    }

    /// Calculate score trend
    pub fn score_trend(&self) -> f64 {
        if self.score_history.len() < 10 {
            return 0.0;
        }

        let recent: Vec<_> = self.score_history.iter().rev().take(10).collect();
        let older: Vec<_> = self.score_history.iter().rev().skip(10).take(10).collect();

        if older.is_empty() {
            return 0.0;
        }

        let recent_avg: f64 = recent.iter().map(|(_, s)| s).sum::<f64>() / recent.len() as f64;
        let older_avg: f64 = older.iter().map(|(_, s)| s).sum::<f64>() / older.len() as f64;

        recent_avg - older_avg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profit_score() {
        let mut metrics = ProfitMetrics::default();
        metrics.mev_captured = 100.0;
        metrics.operating_costs = 50.0;
        metrics.update();

        assert!(metrics.score() > 0.0);
        assert!(metrics.is_healthy());
    }

    #[test]
    fn test_pillar_composite() {
        let pillars = SwarmPillars::default();
        let score = pillars.composite_score();

        // Default should be low
        assert!(score >= 0.0 && score <= 1.0);
    }

    #[test]
    fn test_health_status() {
        let mut pillars = SwarmPillars::default();

        // Default is poor
        match pillars.health_status() {
            HealthStatus::Poor | HealthStatus::Critical(_) => {}
            _ => panic!("Expected poor health with default metrics"),
        }

        // Improve infrastructure
        pillars.infrastructure.avg_uptime = 99.0;
        pillars.infrastructure.compute_utilization = 70.0;
        pillars.infrastructure.nodes_online = 100;
        pillars.infrastructure.nodes_total = 100;

        assert!(pillars.infrastructure.is_stable());
    }

    #[test]
    fn test_pillar_lanes() {
        let profit_lanes = Pillar::Profit.contributing_lanes();
        assert!(profit_lanes.contains(&ComputeLane::Strategy));

        let infra_lanes = Pillar::Infrastructure.contributing_lanes();
        assert!(infra_lanes.contains(&ComputeLane::Security));
    }
}
