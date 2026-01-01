//! The Prophet - AI Forecast Engine
//!
//! Predicts the future so the Warden can allocate BEFORE the world changes:
//! - Crypto cycles (bull/bear/accumulation)
//! - Profit windows
//! - Volatility regimes
//! - Chain usage patterns
//! - Trending niches
//! - Upcoming threats
//! - User adoption spikes
//!
//! The Prophet makes the swarm feel ALIVE and ANTICIPATORY instead of reactive.

use super::auditor::AuditReport;
use crate::warden::ComputeLane;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

/// Forecast horizon
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ForecastHorizon {
    /// Next hour
    Immediate,
    /// Next 24 hours
    Daily,
    /// Next week
    Weekly,
    /// Next month
    Monthly,
    /// Next quarter
    Quarterly,
}

impl ForecastHorizon {
    pub fn duration(&self) -> Duration {
        match self {
            Self::Immediate => Duration::from_secs(3600),
            Self::Daily => Duration::from_secs(86400),
            Self::Weekly => Duration::from_secs(604800),
            Self::Monthly => Duration::from_secs(2592000),
            Self::Quarterly => Duration::from_secs(7776000),
        }
    }
}

/// Market cycle phase
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarketCycle {
    /// Market recovering, accumulation phase
    Accumulation,
    /// Uptrend with increasing momentum
    Bull,
    /// Peak/distribution, high volatility
    Distribution,
    /// Downtrend, risk-off
    Bear,
    /// Sideways, low volatility
    Consolidation,
    /// Unknown/unclear
    Unknown,
}

impl MarketCycle {
    /// Recommended compute bias for this cycle
    pub fn recommended_strategy_bias(&self) -> f64 {
        match self {
            Self::Accumulation => 0.3,  // Moderate strategy
            Self::Bull => 0.45,          // Aggressive strategy
            Self::Distribution => 0.2,  // Reduce exposure
            Self::Bear => 0.1,           // Minimal strategy
            Self::Consolidation => 0.25, // Standard
            Self::Unknown => 0.2,
        }
    }

    /// Recommended security boost for this cycle
    pub fn security_multiplier(&self) -> f64 {
        match self {
            Self::Distribution => 1.5, // High risk period
            Self::Bear => 1.3,          // Elevated risk
            Self::Bull => 1.0,          // Normal
            _ => 1.0,
        }
    }
}

/// Volatility regime
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VolatilityRegime {
    /// < 20% annualized vol
    Low,
    /// 20-40% annualized vol
    Normal,
    /// 40-80% annualized vol
    Elevated,
    /// 80-150% annualized vol
    High,
    /// > 150% annualized vol
    Extreme,
}

impl VolatilityRegime {
    pub fn from_volatility(vol: f64) -> Self {
        if vol < 0.2 {
            Self::Low
        } else if vol < 0.4 {
            Self::Normal
        } else if vol < 0.8 {
            Self::Elevated
        } else if vol < 1.5 {
            Self::High
        } else {
            Self::Extreme
        }
    }

    /// Risk adjustment factor
    pub fn risk_adjustment(&self) -> f64 {
        match self {
            Self::Low => 1.2,      // Can take more risk
            Self::Normal => 1.0,
            Self::Elevated => 0.8,
            Self::High => 0.5,
            Self::Extreme => 0.2, // Minimal risk
        }
    }
}

/// Threat forecast
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatForecast {
    pub threat_type: String,
    pub probability: f64,
    pub expected_timing: ForecastHorizon,
    pub impact_severity: f64,
    pub indicators: Vec<String>,
    pub mitigation: Option<String>,
}

/// Opportunity forecast
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpportunityForecast {
    pub opportunity_type: String,
    pub probability: f64,
    pub expected_timing: ForecastHorizon,
    pub expected_return: f64,
    pub required_lanes: Vec<ComputeLane>,
    pub indicators: Vec<String>,
}

/// Lane demand forecast
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaneDemandForecast {
    pub lane: ComputeLane,
    pub current_demand: f64,
    pub forecasted_demand: HashMap<ForecastHorizon, f64>,
    pub trend: DemandTrend,
    pub confidence: f64,
}

/// Demand trend
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DemandTrend {
    Surging,
    Rising,
    Stable,
    Declining,
    Collapsing,
}

/// Complete market forecast
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketForecast {
    /// When this forecast was generated
    pub generated_at: u64,
    /// Forecast validity duration
    pub valid_for: Duration,
    /// Current market cycle
    pub cycle: MarketCycle,
    /// Cycle confidence
    pub cycle_confidence: f64,
    /// Next expected cycle
    pub next_cycle: Option<MarketCycle>,
    /// Current volatility regime
    pub volatility: VolatilityRegime,
    /// Volatility trend
    pub volatility_trend: DemandTrend,
    /// Threat forecasts
    pub threat_forecasts: Vec<ThreatForecast>,
    /// Opportunity forecasts
    pub opportunities: Vec<OpportunityForecast>,
    /// Lane demand forecasts
    pub lane_demands: HashMap<ComputeLane, LaneDemandForecast>,
    /// Recommended allocation adjustments
    pub allocation_hints: HashMap<ComputeLane, f64>,
    /// Overall market sentiment (-1.0 to 1.0)
    pub sentiment: f64,
    /// Key insights
    pub insights: Vec<String>,
}

/// Historical data point for forecasting
#[derive(Debug, Clone)]
struct HistoricalDataPoint {
    timestamp: u64,
    profit: f64,
    chain_load: f64,
    error_rate: f64,
    lane_utilizations: HashMap<ComputeLane, f64>,
}

/// The Prophet - AI Forecast Engine
pub struct Prophet {
    /// Is Prophet enabled?
    enabled: bool,
    /// Historical data for pattern analysis
    history: VecDeque<HistoricalDataPoint>,
    /// Previous forecasts for accuracy tracking
    previous_forecasts: VecDeque<MarketForecast>,
    /// Forecast accuracy score
    accuracy_score: f64,
    /// Current detected cycle
    detected_cycle: MarketCycle,
    /// Cycle detection confidence
    cycle_confidence: f64,
    /// Running volatility estimate
    volatility_estimate: f64,
    /// Started at
    started_at: Instant,
}

impl Default for Prophet {
    fn default() -> Self {
        Self::new(true)
    }
}

impl Prophet {
    /// Create new Prophet
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            history: VecDeque::with_capacity(10000),
            previous_forecasts: VecDeque::with_capacity(100),
            accuracy_score: 0.7, // Start with moderate confidence
            detected_cycle: MarketCycle::Unknown,
            cycle_confidence: 0.0,
            volatility_estimate: 0.3,
            started_at: Instant::now(),
        }
    }

    /// Generate comprehensive market forecast
    pub async fn forecast(&mut self, audit: &AuditReport) -> MarketForecast {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or_default();

        // Record data point
        self.record_data_point(audit);

        // Detect market cycle
        self.detect_cycle();

        // Update volatility estimate
        self.update_volatility();

        // Generate forecasts
        let threat_forecasts = self.forecast_threats(audit);
        let opportunities = self.forecast_opportunities(audit);
        let lane_demands = self.forecast_lane_demands(audit);
        let allocation_hints = self.generate_allocation_hints(&lane_demands);

        // Calculate sentiment
        let sentiment = self.calculate_sentiment(audit, &threat_forecasts, &opportunities);

        // Generate insights
        let insights = self.generate_insights(
            &threat_forecasts,
            &opportunities,
            &lane_demands,
            sentiment,
        );

        let forecast = MarketForecast {
            generated_at: timestamp,
            valid_for: Duration::from_secs(3600), // 1 hour validity
            cycle: self.detected_cycle,
            cycle_confidence: self.cycle_confidence,
            next_cycle: self.predict_next_cycle(),
            volatility: VolatilityRegime::from_volatility(self.volatility_estimate),
            volatility_trend: self.volatility_trend(),
            threat_forecasts,
            opportunities,
            lane_demands,
            allocation_hints,
            sentiment,
            insights,
        };

        // Store forecast
        if self.previous_forecasts.len() >= 100 {
            self.previous_forecasts.pop_front();
        }
        self.previous_forecasts.push_back(forecast.clone());

        forecast
    }

    /// Record audit data point for pattern analysis
    fn record_data_point(&mut self, audit: &AuditReport) {
        let mut lane_utils = HashMap::new();
        for (lane, audit_data) in &audit.lane_audits {
            lane_utils.insert(*lane, audit_data.utilization);
        }

        let point = HistoricalDataPoint {
            timestamp: audit.timestamp,
            profit: audit.profit_flows.net_profit,
            chain_load: audit.chain_health.cpu_load,
            error_rate: audit.chain_health.error_rate,
            lane_utilizations: lane_utils,
        };

        if self.history.len() >= 10000 {
            self.history.pop_front();
        }
        self.history.push_back(point);
    }

    /// Detect current market cycle from patterns
    fn detect_cycle(&mut self) {
        if self.history.len() < 100 {
            self.detected_cycle = MarketCycle::Unknown;
            self.cycle_confidence = 0.1;
            return;
        }

        // Analyze profit trend
        let profits: Vec<f64> = self.history.iter().rev().take(100).map(|p| p.profit).collect();

        let early_avg = profits[50..].iter().sum::<f64>() / 50.0;
        let late_avg = profits[..50].iter().sum::<f64>() / 50.0;

        let trend = if early_avg.abs() > 0.001 {
            (late_avg - early_avg) / early_avg.abs()
        } else {
            0.0
        };

        // Detect cycle based on trend and volatility
        let (cycle, confidence) = if trend > 0.3 && self.volatility_estimate < 0.5 {
            (MarketCycle::Bull, 0.7)
        } else if trend > 0.3 && self.volatility_estimate > 0.7 {
            (MarketCycle::Distribution, 0.6)
        } else if trend < -0.3 && self.volatility_estimate > 0.5 {
            (MarketCycle::Bear, 0.7)
        } else if trend < -0.1 && self.volatility_estimate < 0.4 {
            (MarketCycle::Accumulation, 0.5)
        } else if self.volatility_estimate < 0.25 {
            (MarketCycle::Consolidation, 0.6)
        } else {
            (MarketCycle::Unknown, 0.3)
        };

        self.detected_cycle = cycle;
        self.cycle_confidence = confidence;
    }

    /// Update volatility estimate
    fn update_volatility(&mut self) {
        if self.history.len() < 20 {
            return;
        }

        let profits: Vec<f64> = self.history.iter().rev().take(20).map(|p| p.profit).collect();
        let mean = profits.iter().sum::<f64>() / profits.len() as f64;
        let variance =
            profits.iter().map(|p| (p - mean).powi(2)).sum::<f64>() / profits.len() as f64;
        let std_dev = variance.sqrt();

        // Normalize to annualized volatility estimate
        self.volatility_estimate = (std_dev / mean.abs().max(1.0)).min(2.0);
    }

    /// Get volatility trend
    fn volatility_trend(&self) -> DemandTrend {
        // Compare recent vs older volatility
        if self.history.len() < 40 {
            return DemandTrend::Stable;
        }

        let recent: Vec<f64> = self.history.iter().rev().take(20).map(|p| p.profit).collect();
        let older: Vec<f64> = self.history.iter().rev().skip(20).take(20).map(|p| p.profit).collect();

        let recent_var = Self::variance(&recent);
        let older_var = Self::variance(&older);

        let change = if older_var > 0.001 {
            (recent_var - older_var) / older_var
        } else {
            0.0
        };

        if change > 0.5 {
            DemandTrend::Surging
        } else if change > 0.2 {
            DemandTrend::Rising
        } else if change < -0.5 {
            DemandTrend::Collapsing
        } else if change < -0.2 {
            DemandTrend::Declining
        } else {
            DemandTrend::Stable
        }
    }

    fn variance(data: &[f64]) -> f64 {
        if data.is_empty() {
            return 0.0;
        }
        let mean = data.iter().sum::<f64>() / data.len() as f64;
        data.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / data.len() as f64
    }

    /// Predict next market cycle
    fn predict_next_cycle(&self) -> Option<MarketCycle> {
        Some(match self.detected_cycle {
            MarketCycle::Accumulation => MarketCycle::Bull,
            MarketCycle::Bull => MarketCycle::Distribution,
            MarketCycle::Distribution => MarketCycle::Bear,
            MarketCycle::Bear => MarketCycle::Accumulation,
            MarketCycle::Consolidation => MarketCycle::Bull, // Assume breakout
            MarketCycle::Unknown => return None,
        })
    }

    /// Forecast potential threats
    fn forecast_threats(&self, audit: &AuditReport) -> Vec<ThreatForecast> {
        let mut threats = Vec::new();

        // Chain stress threat
        if audit.chain_health.cpu_load > 0.7 || audit.chain_health.memory_usage > 0.8 {
            threats.push(ThreatForecast {
                threat_type: "ChainOverload".to_string(),
                probability: (audit.chain_health.cpu_load + audit.chain_health.memory_usage) / 2.0,
                expected_timing: ForecastHorizon::Immediate,
                impact_severity: 0.7,
                indicators: vec![
                    format!("CPU: {:.0}%", audit.chain_health.cpu_load * 100.0),
                    format!("Memory: {:.0}%", audit.chain_health.memory_usage * 100.0),
                ],
                mitigation: Some("Reduce compute-heavy lanes, increase ChainOps".to_string()),
            });
        }

        // Profit crisis threat
        if audit.profit_flows.profit_trend < -0.2 {
            threats.push(ThreatForecast {
                threat_type: "ProfitCrisis".to_string(),
                probability: audit.profit_flows.profit_trend.abs().min(1.0),
                expected_timing: ForecastHorizon::Daily,
                impact_severity: 0.8,
                indicators: vec![format!(
                    "Profit trend: {:.1}%",
                    audit.profit_flows.profit_trend * 100.0
                )],
                mitigation: Some("Shift to Strategy lane, reduce Research spend".to_string()),
            });
        }

        // High volatility threat (based on cycle)
        if matches!(self.detected_cycle, MarketCycle::Distribution) {
            threats.push(ThreatForecast {
                threat_type: "MarketCorrection".to_string(),
                probability: 0.65,
                expected_timing: ForecastHorizon::Weekly,
                impact_severity: 0.6,
                indicators: vec![
                    "Distribution phase detected".to_string(),
                    format!("Volatility: {:?}", VolatilityRegime::from_volatility(self.volatility_estimate)),
                ],
                mitigation: Some("Reduce Strategy exposure, increase Security allocation".to_string()),
            });
        }

        threats
    }

    /// Forecast opportunities
    fn forecast_opportunities(&self, audit: &AuditReport) -> Vec<OpportunityForecast> {
        let mut opportunities = Vec::new();

        // Bull market opportunity
        if matches!(self.detected_cycle, MarketCycle::Bull) {
            opportunities.push(OpportunityForecast {
                opportunity_type: "BullMarketProfit".to_string(),
                probability: self.cycle_confidence,
                expected_timing: ForecastHorizon::Daily,
                expected_return: 0.15, // 15% return
                required_lanes: vec![ComputeLane::Strategy, ComputeLane::AiAgents],
                indicators: vec!["Bull cycle detected".to_string()],
            });
        }

        // Low volatility opportunity
        if self.volatility_estimate < 0.25 {
            opportunities.push(OpportunityForecast {
                opportunity_type: "StableAccumulation".to_string(),
                probability: 0.7,
                expected_timing: ForecastHorizon::Weekly,
                expected_return: 0.05,
                required_lanes: vec![ComputeLane::Research, ComputeLane::Evolution],
                indicators: vec!["Low volatility regime".to_string()],
            });
        }

        // Research opportunity (when profits are good)
        if audit.profit_flows.net_profit > 100.0 {
            opportunities.push(OpportunityForecast {
                opportunity_type: "ResearchExpansion".to_string(),
                probability: 0.6,
                expected_timing: ForecastHorizon::Monthly,
                expected_return: 0.25, // Long-term value
                required_lanes: vec![ComputeLane::Research, ComputeLane::AiAgents],
                indicators: vec![format!("Net profit: +{:.0}", audit.profit_flows.net_profit)],
            });
        }

        opportunities
    }

    /// Forecast lane demands
    fn forecast_lane_demands(&self, audit: &AuditReport) -> HashMap<ComputeLane, LaneDemandForecast> {
        let mut forecasts = HashMap::new();

        for (lane, lane_audit) in &audit.lane_audits {
            let current_demand = lane_audit.utilization;

            // Forecast based on cycle and lane type
            let cycle_modifier = match (lane, self.detected_cycle) {
                (ComputeLane::Strategy, MarketCycle::Bull) => 1.3,
                (ComputeLane::Strategy, MarketCycle::Bear) => 0.6,
                (ComputeLane::Security, MarketCycle::Distribution) => 1.4,
                (ComputeLane::Research, MarketCycle::Consolidation) => 1.2,
                _ => 1.0,
            };

            let immediate = current_demand * cycle_modifier;
            let daily = immediate * 1.05;
            let weekly = daily * 1.02;

            let mut forecasted = HashMap::new();
            forecasted.insert(ForecastHorizon::Immediate, immediate.min(1.0));
            forecasted.insert(ForecastHorizon::Daily, daily.min(1.0));
            forecasted.insert(ForecastHorizon::Weekly, weekly.min(1.0));

            let trend = if cycle_modifier > 1.1 {
                DemandTrend::Rising
            } else if cycle_modifier < 0.9 {
                DemandTrend::Declining
            } else {
                DemandTrend::Stable
            };

            forecasts.insert(
                *lane,
                LaneDemandForecast {
                    lane: *lane,
                    current_demand,
                    forecasted_demand: forecasted,
                    trend,
                    confidence: self.cycle_confidence * 0.8,
                },
            );
        }

        forecasts
    }

    /// Generate allocation hints based on forecasts
    fn generate_allocation_hints(
        &self,
        lane_demands: &HashMap<ComputeLane, LaneDemandForecast>,
    ) -> HashMap<ComputeLane, f64> {
        let mut hints = HashMap::new();

        // Base allocation on cycle
        let strategy_bias = self.detected_cycle.recommended_strategy_bias();
        let security_mult = self.detected_cycle.security_multiplier();

        // Apply volatility adjustment
        let vol_regime = VolatilityRegime::from_volatility(self.volatility_estimate);
        let risk_adj = vol_regime.risk_adjustment();

        hints.insert(ComputeLane::Strategy, strategy_bias * risk_adj);
        hints.insert(ComputeLane::Security, 0.20 * security_mult);

        // Adjust based on demand forecasts
        for (lane, forecast) in lane_demands {
            let current = hints.get(lane).copied().unwrap_or(0.15);
            let adjustment = match forecast.trend {
                DemandTrend::Surging => 0.1,
                DemandTrend::Rising => 0.05,
                DemandTrend::Stable => 0.0,
                DemandTrend::Declining => -0.05,
                DemandTrend::Collapsing => -0.1,
            };
            hints.insert(*lane, (current + adjustment).max(0.05));
        }

        // Normalize to sum to ~1.0
        let total: f64 = hints.values().sum();
        if total > 0.0 {
            for value in hints.values_mut() {
                *value /= total;
            }
        }

        hints
    }

    /// Calculate overall market sentiment
    fn calculate_sentiment(
        &self,
        audit: &AuditReport,
        threats: &[ThreatForecast],
        opportunities: &[OpportunityForecast],
    ) -> f64 {
        let mut sentiment = 0.0;

        // Cycle sentiment
        sentiment += match self.detected_cycle {
            MarketCycle::Bull => 0.4,
            MarketCycle::Accumulation => 0.2,
            MarketCycle::Consolidation => 0.0,
            MarketCycle::Distribution => -0.2,
            MarketCycle::Bear => -0.4,
            MarketCycle::Unknown => 0.0,
        };

        // Profit sentiment
        if audit.profit_flows.net_profit > 0.0 {
            sentiment += 0.2;
        } else {
            sentiment -= 0.2;
        }

        // Threat/opportunity balance
        let threat_impact: f64 = threats.iter().map(|t| t.probability * t.impact_severity).sum();
        let opportunity_value: f64 = opportunities.iter().map(|o| o.probability * o.expected_return).sum();

        sentiment += (opportunity_value - threat_impact) * 0.5;

        sentiment.clamp(-1.0, 1.0)
    }

    /// Generate human-readable insights
    fn generate_insights(
        &self,
        threats: &[ThreatForecast],
        opportunities: &[OpportunityForecast],
        lane_demands: &HashMap<ComputeLane, LaneDemandForecast>,
        sentiment: f64,
    ) -> Vec<String> {
        let mut insights = Vec::new();

        // Cycle insight
        insights.push(format!(
            "Market cycle: {:?} ({:.0}% confidence)",
            self.detected_cycle,
            self.cycle_confidence * 100.0
        ));

        // Volatility insight
        let vol_regime = VolatilityRegime::from_volatility(self.volatility_estimate);
        insights.push(format!("Volatility regime: {:?}", vol_regime));

        // Sentiment insight
        let sentiment_desc = if sentiment > 0.3 {
            "Bullish"
        } else if sentiment > 0.0 {
            "Cautiously Optimistic"
        } else if sentiment > -0.3 {
            "Cautious"
        } else {
            "Bearish"
        };
        insights.push(format!("Sentiment: {} ({:.2})", sentiment_desc, sentiment));

        // Top threat
        if let Some(threat) = threats.iter().max_by(|a, b| {
            (a.probability * a.impact_severity)
                .partial_cmp(&(b.probability * b.impact_severity))
                .unwrap_or(std::cmp::Ordering::Equal)
        }) {
            insights.push(format!(
                "Top threat: {} ({:.0}% probability)",
                threat.threat_type,
                threat.probability * 100.0
            ));
        }

        // Top opportunity
        if let Some(opp) = opportunities.iter().max_by(|a, b| {
            (a.probability * a.expected_return)
                .partial_cmp(&(b.probability * b.expected_return))
                .unwrap_or(std::cmp::Ordering::Equal)
        }) {
            insights.push(format!(
                "Best opportunity: {} (+{:.0}% expected return)",
                opp.opportunity_type,
                opp.expected_return * 100.0
            ));
        }

        // Surging lanes
        for (lane, forecast) in lane_demands {
            if forecast.trend == DemandTrend::Surging {
                insights.push(format!("{:?} demand surging", lane));
            }
        }

        insights
    }

    /// Update accuracy score based on forecast vs actual
    pub fn update_accuracy(&mut self, forecast: &MarketForecast, actual_profit: f64) {
        // Simple accuracy update based on profit direction
        let predicted_positive = forecast.sentiment > 0.0;
        let actual_positive = actual_profit > 0.0;

        if predicted_positive == actual_positive {
            self.accuracy_score = (self.accuracy_score * 0.9 + 0.1).min(1.0);
        } else {
            self.accuracy_score = (self.accuracy_score * 0.9).max(0.3);
        }
    }

    /// Get accuracy score
    pub fn accuracy(&self) -> f64 {
        self.accuracy_score
    }

    /// Is Prophet enabled?
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get uptime
    pub fn uptime(&self) -> Duration {
        self.started_at.elapsed()
    }

    /// Get current detected cycle
    pub fn current_cycle(&self) -> MarketCycle {
        self.detected_cycle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prophet_creation() {
        let prophet = Prophet::new(true);
        assert!(prophet.is_enabled());
        assert_eq!(prophet.current_cycle(), MarketCycle::Unknown);
    }

    #[test]
    fn test_market_cycle_recommendations() {
        assert!(MarketCycle::Bull.recommended_strategy_bias() > MarketCycle::Bear.recommended_strategy_bias());
        assert!(MarketCycle::Distribution.security_multiplier() > MarketCycle::Bull.security_multiplier());
    }

    #[test]
    fn test_volatility_regime() {
        assert_eq!(VolatilityRegime::from_volatility(0.1), VolatilityRegime::Low);
        assert_eq!(VolatilityRegime::from_volatility(0.3), VolatilityRegime::Normal);
        assert_eq!(VolatilityRegime::from_volatility(0.6), VolatilityRegime::Elevated);
        assert_eq!(VolatilityRegime::from_volatility(1.0), VolatilityRegime::High);
        assert_eq!(VolatilityRegime::from_volatility(2.0), VolatilityRegime::Extreme);
    }

    #[test]
    fn test_forecast_horizon_duration() {
        assert!(ForecastHorizon::Immediate.duration() < ForecastHorizon::Daily.duration());
        assert!(ForecastHorizon::Daily.duration() < ForecastHorizon::Weekly.duration());
    }

    #[test]
    fn test_accuracy_update() {
        let mut prophet = Prophet::new(true);

        let forecast = MarketForecast {
            generated_at: 0,
            valid_for: Duration::from_secs(3600),
            cycle: MarketCycle::Bull,
            cycle_confidence: 0.7,
            next_cycle: None,
            volatility: VolatilityRegime::Normal,
            volatility_trend: DemandTrend::Stable,
            threat_forecasts: vec![],
            opportunities: vec![],
            lane_demands: HashMap::new(),
            allocation_hints: HashMap::new(),
            sentiment: 0.5, // Positive
            insights: vec![],
        };

        let initial_accuracy = prophet.accuracy();

        // Correct prediction
        prophet.update_accuracy(&forecast, 100.0);
        assert!(prophet.accuracy() >= initial_accuracy);

        // Incorrect prediction
        prophet.update_accuracy(&forecast, -100.0);
        assert!(prophet.accuracy() < initial_accuracy + 0.1);
    }
}
