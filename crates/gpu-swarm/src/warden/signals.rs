//! Lane Signals and Signal Aggregation
//!
//! Collects telemetry from all compute lanes to inform allocation decisions.

use crate::warden::policy::ComputeLane;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

/// Types of signals from compute lanes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SignalType {
    /// Current load percentage (0.0 - 1.0)
    Load(f64),
    /// Queue depth (number of waiting jobs)
    QueueDepth(u32),
    /// Job completion rate (jobs/second)
    Throughput(f64),
    /// Average latency (ms)
    Latency(f64),
    /// Error rate (0.0 - 1.0)
    ErrorRate(f64),
    /// Revenue generated (tokens/hour)
    Revenue(f64),
    /// GPU memory usage (MB)
    VramUsage(u32),
    /// Custom metric
    Custom(String, f64),
}

/// A signal from a compute lane
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaneSignal {
    /// Which lane generated this signal
    pub lane: ComputeLane,
    /// Type of signal
    pub signal_type: SignalType,
    /// Timestamp (Unix epoch ms)
    pub timestamp_ms: u64,
    /// Source node ID
    pub source_node: Option<String>,
}

impl LaneSignal {
    /// Create a new lane signal
    pub fn new(lane: ComputeLane, signal_type: SignalType) -> Self {
        Self {
            lane,
            signal_type,
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            source_node: None,
        }
    }

    /// Create with source node
    pub fn with_source(lane: ComputeLane, signal_type: SignalType, source: String) -> Self {
        let mut signal = Self::new(lane, signal_type);
        signal.source_node = Some(source);
        signal
    }
}

/// Aggregated metrics for a lane
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LaneMetrics {
    /// Average load over window
    pub avg_load: f64,
    /// Max load in window
    pub max_load: f64,
    /// Current queue depth
    pub queue_depth: u32,
    /// Average throughput
    pub avg_throughput: f64,
    /// Average latency
    pub avg_latency: f64,
    /// Current error rate
    pub error_rate: f64,
    /// Revenue generated
    pub revenue: f64,
    /// VRAM usage
    pub vram_usage: u32,
    /// Number of signals in window
    pub sample_count: u32,
    /// Last update timestamp
    pub last_update_ms: u64,
}

impl LaneMetrics {
    /// Calculate urgency score (higher = needs more resources)
    pub fn urgency_score(&self) -> f64 {
        let load_factor = self.avg_load * 2.0;
        let queue_factor = (self.queue_depth as f64 / 100.0).min(2.0);
        let latency_factor = (self.avg_latency / 1000.0).min(1.0);
        let error_factor = self.error_rate * 3.0;

        (load_factor + queue_factor + latency_factor + error_factor) / 4.0
    }

    /// Check if lane is healthy
    pub fn is_healthy(&self) -> bool {
        self.avg_load < 0.9 && self.error_rate < 0.1 && self.avg_latency < 5000.0
    }

    /// Check if lane is starved (needs more resources)
    pub fn is_starved(&self) -> bool {
        self.avg_load > 0.85 || self.queue_depth > 50
    }

    /// Check if lane has excess capacity
    pub fn has_excess_capacity(&self) -> bool {
        self.avg_load < 0.3 && self.queue_depth < 5
    }
}

/// Sliding window for signal history
struct SignalWindow {
    signals: VecDeque<(Instant, SignalType)>,
    window_duration: Duration,
    max_signals: usize,
}

impl SignalWindow {
    fn new(window_duration: Duration, max_signals: usize) -> Self {
        Self {
            signals: VecDeque::with_capacity(max_signals),
            window_duration,
            max_signals,
        }
    }

    fn push(&mut self, signal: SignalType) {
        let now = Instant::now();

        // Remove old signals
        self.prune(now);

        // Add new signal
        if self.signals.len() >= self.max_signals {
            self.signals.pop_front();
        }
        self.signals.push_back((now, signal));
    }

    fn prune(&mut self, now: Instant) {
        while let Some((time, _)) = self.signals.front() {
            if now.duration_since(*time) > self.window_duration {
                self.signals.pop_front();
            } else {
                break;
            }
        }
    }

    fn iter_values(&self) -> impl Iterator<Item = &SignalType> {
        self.signals.iter().map(|(_, s)| s)
    }

    fn len(&self) -> usize {
        self.signals.len()
    }
}

/// Aggregates signals from all compute lanes
pub struct SignalAggregator {
    /// Signal windows per lane
    windows: HashMap<ComputeLane, SignalWindow>,
    /// Aggregated metrics per lane
    metrics: HashMap<ComputeLane, LaneMetrics>,
    /// Aggregation window duration
    window_duration: Duration,
    /// Max signals per window
    max_signals: usize,
    /// Callback for urgent signals
    urgent_threshold: f64,
}

impl Default for SignalAggregator {
    fn default() -> Self {
        Self::new(Duration::from_secs(60), 1000)
    }
}

impl SignalAggregator {
    /// Create new signal aggregator
    pub fn new(window_duration: Duration, max_signals: usize) -> Self {
        let mut windows = HashMap::new();
        let mut metrics = HashMap::new();

        for lane in ComputeLane::all() {
            windows.insert(lane, SignalWindow::new(window_duration, max_signals));
            metrics.insert(lane, LaneMetrics::default());
        }

        Self {
            windows,
            metrics,
            window_duration,
            max_signals,
            urgent_threshold: 0.8,
        }
    }

    /// Ingest a signal
    pub fn ingest(&mut self, signal: LaneSignal) -> Option<LaneAlert> {
        // Add to window
        if let Some(window) = self.windows.get_mut(&signal.lane) {
            window.push(signal.signal_type.clone());
        }

        // Update metrics
        self.update_metrics(signal.lane);

        // Check for urgent conditions
        self.check_alerts(signal.lane)
    }

    /// Ingest multiple signals
    pub fn ingest_batch(&mut self, signals: Vec<LaneSignal>) -> Vec<LaneAlert> {
        let mut alerts = Vec::new();
        for signal in signals {
            if let Some(alert) = self.ingest(signal) {
                alerts.push(alert);
            }
        }
        alerts
    }

    /// Update aggregated metrics for a lane
    fn update_metrics(&mut self, lane: ComputeLane) {
        let window = match self.windows.get(&lane) {
            Some(w) => w,
            None => return,
        };

        let mut load_sum = 0.0;
        let mut load_max = 0.0_f64;
        let mut load_count = 0;

        let mut throughput_sum = 0.0;
        let mut throughput_count = 0;

        let mut latency_sum = 0.0;
        let mut latency_count = 0;

        let mut error_sum = 0.0;
        let mut error_count = 0;

        let mut revenue_sum = 0.0;
        let mut queue_depth = 0u32;
        let mut vram_usage = 0u32;

        for signal in window.iter_values() {
            match signal {
                SignalType::Load(v) => {
                    load_sum += v;
                    load_max = load_max.max(*v);
                    load_count += 1;
                }
                SignalType::QueueDepth(v) => {
                    queue_depth = queue_depth.max(*v);
                }
                SignalType::Throughput(v) => {
                    throughput_sum += v;
                    throughput_count += 1;
                }
                SignalType::Latency(v) => {
                    latency_sum += v;
                    latency_count += 1;
                }
                SignalType::ErrorRate(v) => {
                    error_sum += v;
                    error_count += 1;
                }
                SignalType::Revenue(v) => {
                    revenue_sum += v;
                }
                SignalType::VramUsage(v) => {
                    vram_usage = vram_usage.max(*v);
                }
                SignalType::Custom(_, _) => {}
            }
        }

        let metrics = LaneMetrics {
            avg_load: if load_count > 0 {
                load_sum / load_count as f64
            } else {
                0.0
            },
            max_load: load_max,
            queue_depth,
            avg_throughput: if throughput_count > 0 {
                throughput_sum / throughput_count as f64
            } else {
                0.0
            },
            avg_latency: if latency_count > 0 {
                latency_sum / latency_count as f64
            } else {
                0.0
            },
            error_rate: if error_count > 0 {
                error_sum / error_count as f64
            } else {
                0.0
            },
            revenue: revenue_sum,
            vram_usage,
            sample_count: window.len() as u32,
            last_update_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        };

        self.metrics.insert(lane, metrics);
    }

    /// Check for alert conditions
    fn check_alerts(&self, lane: ComputeLane) -> Option<LaneAlert> {
        let metrics = self.metrics.get(&lane)?;

        // Critical: High error rate
        if metrics.error_rate > 0.2 {
            return Some(LaneAlert {
                lane,
                severity: AlertSeverity::Critical,
                message: format!("High error rate: {:.1}%", metrics.error_rate * 100.0),
            });
        }

        // High: Overloaded
        if metrics.avg_load > 0.95 {
            return Some(LaneAlert {
                lane,
                severity: AlertSeverity::High,
                message: format!("Lane overloaded: {:.1}% load", metrics.avg_load * 100.0),
            });
        }

        // Medium: Large queue
        if metrics.queue_depth > 100 {
            return Some(LaneAlert {
                lane,
                severity: AlertSeverity::Medium,
                message: format!("Large queue: {} jobs waiting", metrics.queue_depth),
            });
        }

        // Low: High latency
        if metrics.avg_latency > 3000.0 {
            return Some(LaneAlert {
                lane,
                severity: AlertSeverity::Low,
                message: format!("High latency: {:.0}ms", metrics.avg_latency),
            });
        }

        None
    }

    /// Get current metrics for a lane
    pub fn get_metrics(&self, lane: ComputeLane) -> Option<&LaneMetrics> {
        self.metrics.get(&lane)
    }

    /// Get all lane metrics
    pub fn all_metrics(&self) -> &HashMap<ComputeLane, LaneMetrics> {
        &self.metrics
    }

    /// Get lanes that need more resources
    pub fn starved_lanes(&self) -> Vec<(ComputeLane, &LaneMetrics)> {
        self.metrics
            .iter()
            .filter(|(_, m)| m.is_starved())
            .map(|(l, m)| (*l, m))
            .collect()
    }

    /// Get lanes with excess capacity
    pub fn excess_lanes(&self) -> Vec<(ComputeLane, &LaneMetrics)> {
        self.metrics
            .iter()
            .filter(|(_, m)| m.has_excess_capacity())
            .map(|(l, m)| (*l, m))
            .collect()
    }

    /// Get overall swarm health score
    pub fn swarm_health(&self) -> f64 {
        let healthy_count = self.metrics.values().filter(|m| m.is_healthy()).count();
        healthy_count as f64 / self.metrics.len() as f64
    }
}

/// Alert severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// An alert from a lane
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaneAlert {
    pub lane: ComputeLane,
    pub severity: AlertSeverity,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_aggregation() {
        let mut aggregator = SignalAggregator::default();

        // Send some load signals
        for i in 0..10 {
            aggregator.ingest(LaneSignal::new(
                ComputeLane::Research,
                SignalType::Load(0.5 + (i as f64 * 0.02)),
            ));
        }

        let metrics = aggregator.get_metrics(ComputeLane::Research).unwrap();
        assert!(metrics.avg_load > 0.5);
        assert!(metrics.avg_load < 0.7);
    }

    #[test]
    fn test_urgency_score() {
        let metrics = LaneMetrics {
            avg_load: 0.9,
            queue_depth: 100,
            error_rate: 0.05,
            ..Default::default()
        };

        let score = metrics.urgency_score();
        assert!(score > 0.5, "High load should increase urgency");
    }

    #[test]
    fn test_health_check() {
        let healthy = LaneMetrics {
            avg_load: 0.5,
            error_rate: 0.01,
            avg_latency: 100.0,
            ..Default::default()
        };
        assert!(healthy.is_healthy());

        let unhealthy = LaneMetrics {
            avg_load: 0.95,
            error_rate: 0.2,
            avg_latency: 6000.0,
            ..Default::default()
        };
        assert!(!unhealthy.is_healthy());
    }

    #[test]
    fn test_alert_generation() {
        let mut aggregator = SignalAggregator::default();

        // Send high error rate signal
        let alert = aggregator.ingest(LaneSignal::new(
            ComputeLane::Security,
            SignalType::ErrorRate(0.25),
        ));

        assert!(alert.is_some());
        let alert = alert.unwrap();
        assert_eq!(alert.severity, AlertSeverity::Critical);
    }
}
