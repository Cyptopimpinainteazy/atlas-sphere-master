//! Load Predictor
//!
//! Predicts future load across compute lanes to enable proactive reallocation.

use crate::warden::policy::ComputeLane;
use crate::warden::signals::{LaneMetrics, SignalAggregator};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::time::Duration;

/// Time horizon for predictions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PredictionHorizon {
    /// Next 10 seconds
    Immediate,
    /// Next 1 minute
    ShortTerm,
    /// Next 5 minutes
    MediumTerm,
    /// Next 15 minutes
    LongTerm,
}

impl PredictionHorizon {
    pub fn duration(&self) -> Duration {
        match self {
            Self::Immediate => Duration::from_secs(10),
            Self::ShortTerm => Duration::from_secs(60),
            Self::MediumTerm => Duration::from_secs(300),
            Self::LongTerm => Duration::from_secs(900),
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            Self::Immediate,
            Self::ShortTerm,
            Self::MediumTerm,
            Self::LongTerm,
        ]
    }
}

/// Predicted load for a lane
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaneForecast {
    /// Lane being forecasted
    pub lane: ComputeLane,
    /// Current load
    pub current_load: f64,
    /// Predicted load
    pub predicted_load: f64,
    /// Prediction horizon
    pub horizon: PredictionHorizon,
    /// Confidence in prediction (0.0 - 1.0)
    pub confidence: f64,
    /// Trend direction
    pub trend: LoadTrend,
    /// Predicted queue depth
    pub predicted_queue: u32,
    /// Recommended action
    pub recommendation: PredictionAction,
}

impl LaneForecast {
    /// Delta between predicted and current
    pub fn load_delta(&self) -> f64 {
        self.predicted_load - self.current_load
    }

    /// Is the prediction concerning?
    pub fn is_concerning(&self) -> bool {
        self.predicted_load > 0.85 || self.predicted_queue > 50
    }
}

/// Load trend direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoadTrend {
    /// Load decreasing rapidly
    FallingFast,
    /// Load decreasing
    Falling,
    /// Load stable
    Stable,
    /// Load increasing
    Rising,
    /// Load increasing rapidly
    RisingFast,
}

impl LoadTrend {
    pub fn from_delta(delta: f64) -> Self {
        if delta < -0.1 {
            Self::FallingFast
        } else if delta < -0.02 {
            Self::Falling
        } else if delta < 0.02 {
            Self::Stable
        } else if delta < 0.1 {
            Self::Rising
        } else {
            Self::RisingFast
        }
    }
}

/// Recommended action based on prediction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PredictionAction {
    /// No action needed
    None,
    /// Monitor closely
    Monitor,
    /// Preemptively scale up
    ScaleUp,
    /// Can safely scale down
    ScaleDown,
    /// Urgent: immediate scaling needed
    UrgentScale,
}

/// Complete forecast for the swarm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmForecast {
    /// Per-lane forecasts
    pub lane_forecasts: HashMap<ComputeLane, LaneForecast>,
    /// Overall predicted utilization
    pub predicted_utilization: f64,
    /// Lanes predicted to need more resources
    pub scaling_up: Vec<ComputeLane>,
    /// Lanes that can release resources
    pub scaling_down: Vec<ComputeLane>,
    /// Critical warnings
    pub warnings: Vec<String>,
    /// Prediction timestamp
    pub generated_at_ms: u64,
}

impl SwarmForecast {
    /// Get forecast for a lane
    pub fn get(&self, lane: ComputeLane) -> Option<&LaneForecast> {
        self.lane_forecasts.get(&lane)
    }

    /// Check if any lane has concerning predictions
    pub fn has_concerns(&self) -> bool {
        self.lane_forecasts.values().any(|f| f.is_concerning())
    }
}

/// Historical data point for a lane
#[derive(Debug, Clone)]
struct HistoricalPoint {
    load: f64,
    queue_depth: u32,
    timestamp_ms: u64,
}

/// Load Predictor - forecasts future load using historical trends
pub struct LoadPredictor {
    /// Historical data per lane
    history: HashMap<ComputeLane, VecDeque<HistoricalPoint>>,
    /// Maximum history entries per lane
    max_history: usize,
    /// Minimum data points needed for prediction
    min_data_points: usize,
    /// Exponential smoothing factor (0.0 - 1.0)
    smoothing_factor: f64,
}

impl Default for LoadPredictor {
    fn default() -> Self {
        Self::new(1000, 10, 0.3)
    }
}

impl LoadPredictor {
    /// Create new predictor
    pub fn new(max_history: usize, min_data_points: usize, smoothing_factor: f64) -> Self {
        let mut history = HashMap::new();
        for lane in ComputeLane::all() {
            history.insert(lane, VecDeque::with_capacity(max_history));
        }

        Self {
            history,
            max_history,
            min_data_points,
            smoothing_factor: smoothing_factor.clamp(0.0, 1.0),
        }
    }

    /// Record current metrics
    pub fn record(&mut self, aggregator: &SignalAggregator) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        for lane in ComputeLane::all() {
            if let Some(metrics) = aggregator.get_metrics(lane) {
                let point = HistoricalPoint {
                    load: metrics.avg_load,
                    queue_depth: metrics.queue_depth,
                    timestamp_ms: now,
                };

                if let Some(history) = self.history.get_mut(&lane) {
                    if history.len() >= self.max_history {
                        history.pop_front();
                    }
                    history.push_back(point);
                }
            }
        }
    }

    /// Generate forecasts for all lanes
    pub fn forecast(
        &self,
        aggregator: &SignalAggregator,
        horizon: PredictionHorizon,
    ) -> SwarmForecast {
        let mut lane_forecasts = HashMap::new();
        let mut scaling_up = Vec::new();
        let mut scaling_down = Vec::new();
        let mut warnings = Vec::new();

        for lane in ComputeLane::all() {
            let forecast = self.forecast_lane(lane, aggregator.get_metrics(lane), horizon);

            match forecast.recommendation {
                PredictionAction::ScaleUp | PredictionAction::UrgentScale => {
                    scaling_up.push(lane);
                }
                PredictionAction::ScaleDown => {
                    scaling_down.push(lane);
                }
                _ => {}
            }

            if forecast.is_concerning() {
                warnings.push(format!(
                    "{} predicted to hit {:.0}% load in {:?}",
                    lane.display_name(),
                    forecast.predicted_load * 100.0,
                    horizon
                ));
            }

            lane_forecasts.insert(lane, forecast);
        }

        let predicted_utilization = lane_forecasts
            .values()
            .map(|f| f.predicted_load)
            .sum::<f64>()
            / lane_forecasts.len() as f64;

        SwarmForecast {
            lane_forecasts,
            predicted_utilization,
            scaling_up,
            scaling_down,
            warnings,
            generated_at_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        }
    }

    /// Generate forecast for a single lane
    fn forecast_lane(
        &self,
        lane: ComputeLane,
        metrics: Option<&LaneMetrics>,
        horizon: PredictionHorizon,
    ) -> LaneForecast {
        let current_load = metrics.map(|m| m.avg_load).unwrap_or(0.0);
        let current_queue = metrics.map(|m| m.queue_depth).unwrap_or(0);

        let history = self.history.get(&lane);
        let has_enough_data = history
            .map(|h| h.len() >= self.min_data_points)
            .unwrap_or(false);

        // Calculate trend
        let trend = if has_enough_data {
            if let Some(h) = history {
                self.calculate_trend(h)
            } else {
                // Fallback in the unlikely event history is missing despite has_enough_data
                LoadTrend::Stable
            }
        } else {
            LoadTrend::Stable
        };

        // Predict future load
        let (predicted_load, confidence) = if has_enough_data {
            if let Some(h) = history {
                self.predict_load(h, horizon, current_load)
            } else {
                (current_load, 0.3)
            }
        } else {
            (current_load, 0.3) // Low confidence without history
        };

        // Predict queue depth
        let predicted_queue = self.predict_queue(current_queue, trend, horizon);

        // Determine recommendation
        let recommendation = self.determine_action(predicted_load, predicted_queue, trend, lane);

        LaneForecast {
            lane,
            current_load,
            predicted_load,
            horizon,
            confidence,
            trend,
            predicted_queue,
            recommendation,
        }
    }

    /// Calculate load trend from history
    fn calculate_trend(&self, history: &VecDeque<HistoricalPoint>) -> LoadTrend {
        if history.len() < 2 {
            return LoadTrend::Stable;
        }

        // Use last N points for trend
        let n = history.len().min(10);
        let recent: Vec<_> = history.iter().rev().take(n).collect();

        if recent.len() < 2 {
            return LoadTrend::Stable;
        }

        // Linear regression to find slope
        let n_f = recent.len() as f64;
        let sum_x: f64 = (0..recent.len()).map(|i| i as f64).sum();
        let sum_y: f64 = recent.iter().map(|p| p.load).sum();
        let sum_xy: f64 = recent
            .iter()
            .enumerate()
            .map(|(i, p)| i as f64 * p.load)
            .sum();
        let sum_x2: f64 = (0..recent.len()).map(|i| (i as f64).powi(2)).sum();

        let slope = (n_f * sum_xy - sum_x * sum_y) / (n_f * sum_x2 - sum_x.powi(2));

        LoadTrend::from_delta(slope)
    }

    /// Predict future load using exponential smoothing
    fn predict_load(
        &self,
        history: &VecDeque<HistoricalPoint>,
        horizon: PredictionHorizon,
        current: f64,
    ) -> (f64, f64) {
        if history.is_empty() {
            return (current, 0.3);
        }

        // Exponential weighted average
        let mut predicted = current;
        let mut weight = 1.0;
        let mut weight_sum = 0.0;

        for point in history.iter().rev().take(20) {
            predicted += weight * (point.load - current);
            weight_sum += weight;
            weight *= self.smoothing_factor;
        }

        if weight_sum > 0.0 {
            predicted = current + (predicted - current) / weight_sum;
        }

        // Extrapolate based on horizon
        let trend = self.calculate_trend(history);
        let horizon_factor = match horizon {
            PredictionHorizon::Immediate => 1.0,
            PredictionHorizon::ShortTerm => 2.0,
            PredictionHorizon::MediumTerm => 4.0,
            PredictionHorizon::LongTerm => 8.0,
        };

        let trend_delta = match trend {
            LoadTrend::FallingFast => -0.1,
            LoadTrend::Falling => -0.03,
            LoadTrend::Stable => 0.0,
            LoadTrend::Rising => 0.03,
            LoadTrend::RisingFast => 0.1,
        };

        let predicted = (predicted + trend_delta * horizon_factor).clamp(0.0, 1.0);

        // Confidence decreases with horizon and data scarcity
        let data_confidence = (history.len() as f64 / self.max_history as f64).min(1.0);
        let horizon_confidence = match horizon {
            PredictionHorizon::Immediate => 0.9,
            PredictionHorizon::ShortTerm => 0.7,
            PredictionHorizon::MediumTerm => 0.5,
            PredictionHorizon::LongTerm => 0.3,
        };
        let confidence = data_confidence * horizon_confidence;

        (predicted, confidence)
    }

    /// Predict queue depth
    fn predict_queue(&self, current: u32, trend: LoadTrend, horizon: PredictionHorizon) -> u32 {
        let growth_factor = match trend {
            LoadTrend::FallingFast => 0.5,
            LoadTrend::Falling => 0.8,
            LoadTrend::Stable => 1.0,
            LoadTrend::Rising => 1.3,
            LoadTrend::RisingFast => 2.0,
        };

        let horizon_factor = match horizon {
            PredictionHorizon::Immediate => 1.0,
            PredictionHorizon::ShortTerm => 1.5,
            PredictionHorizon::MediumTerm => 2.0,
            PredictionHorizon::LongTerm => 3.0,
        };

        ((current as f64) * growth_factor * horizon_factor) as u32
    }

    /// Determine recommended action
    fn determine_action(
        &self,
        predicted_load: f64,
        predicted_queue: u32,
        trend: LoadTrend,
        lane: ComputeLane,
    ) -> PredictionAction {
        // Critical lanes are more sensitive
        let load_threshold = if lane.is_critical() { 0.75 } else { 0.85 };
        let queue_threshold = if lane.is_critical() { 30 } else { 50 };

        if predicted_load > 0.95 || predicted_queue > 100 {
            return PredictionAction::UrgentScale;
        }

        if predicted_load > load_threshold || predicted_queue > queue_threshold {
            return PredictionAction::ScaleUp;
        }

        if matches!(trend, LoadTrend::Rising | LoadTrend::RisingFast) && predicted_load > 0.6 {
            return PredictionAction::Monitor;
        }

        if predicted_load < 0.2 && matches!(trend, LoadTrend::Falling | LoadTrend::FallingFast) {
            return PredictionAction::ScaleDown;
        }

        PredictionAction::None
    }

    /// Get prediction for multiple horizons
    pub fn multi_horizon_forecast(&self, aggregator: &SignalAggregator) -> Vec<SwarmForecast> {
        PredictionHorizon::all()
            .into_iter()
            .map(|h| self.forecast(aggregator, h))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::warden::signals::{LaneSignal, SignalType};

    #[test]
    fn test_prediction_horizon() {
        assert!(PredictionHorizon::Immediate.duration() < PredictionHorizon::LongTerm.duration());
    }

    #[test]
    fn test_load_trend() {
        assert_eq!(LoadTrend::from_delta(0.0), LoadTrend::Stable);
        assert_eq!(LoadTrend::from_delta(0.15), LoadTrend::RisingFast);
        assert_eq!(LoadTrend::from_delta(-0.15), LoadTrend::FallingFast);
    }

    #[test]
    fn test_basic_forecast() {
        let mut aggregator = SignalAggregator::new(Duration::from_secs(60), 100);
        let predictor = LoadPredictor::default();

        // Add some load signals
        for _ in 0..5 {
            aggregator.ingest(LaneSignal::new(
                ComputeLane::Research,
                SignalType::Load(0.5),
            ));
        }

        let forecast = predictor.forecast(&aggregator, PredictionHorizon::ShortTerm);

        // Should have forecast for research
        let research = forecast.get(ComputeLane::Research).unwrap();
        assert!(research.current_load >= 0.0);
        assert!(research.predicted_load >= 0.0);
    }

    #[test]
    fn test_multi_horizon() {
        let aggregator = SignalAggregator::new(Duration::from_secs(60), 100);
        let predictor = LoadPredictor::default();

        let forecasts = predictor.multi_horizon_forecast(&aggregator);

        assert_eq!(forecasts.len(), 4);
    }
}
