//! Comprehensive tests for the Warden module - GPU allocation intelligence

use gpu_swarm::{
    warden::{
        Warden, WardenConfig, WardenDecision, ComputeLane, ThreatLevel, SwarmState, 
        SwarmPillars, AllocationPlan, LaneSignal, SignalAggregator, MetricsCollector,
        LoadPredictor, GovernanceEngine, GuardBot, GuardType, LaneAllocation,
        AllocationPolicy, SignalType, GovernanceAction, GuardAction, GuardState,
        GuardConfig, GuardMetrics, GuardEvent, GuardEventKind, GuardEventSeverity,
    },
    config::SwarmConfig,
    node::{GpuCapabilities, GpuBackend, NodeStatus, NodeRegistry},
    task::{Task, TaskType, TaskPriority, TaskStatus},
    error::SwarmError,
};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Test helper: Create test GPU capabilities
fn test_gpu_capabilities(vram_gb: u64) -> GpuCapabilities {
    GpuCapabilities {
        backends: vec![GpuBackend::Vulkan],
        device_name: format!("Test GPU {}GB", vram_gb),
        vendor: "Test Vendor".to_string(),
        total_vram: vram_gb * 1024 * 1024 * 1024,
        available_vram: (vram_gb * 1024 * 1024 * 1024) * 3 / 4,
        compute_units: 32,
        max_workgroup_size: 1024,
        max_threads: 32768,
        compute_capability: None,
        supports_fp64: false,
        supports_fp16: true,
        supports_tensor_cores: false,
    }
}

/// Test helper: Create test submitter ID
fn test_submitter() -> [u8; 32] {
    let mut id = [0u8; 32];
    id[0] = 0xFF;
    id
}

/// Test helper: Create test task
fn create_test_task(task_type: TaskType, reward: u64) -> Task {
    Task::new(task_type, test_submitter(), reward)
}

#[cfg(test)]
mod warden_config_tests {
    use super::*;

    #[test]
    fn test_warden_config_default() {
        let config = WardenConfig::default();
        assert!(config.enabled);
        assert_eq!(config.update_interval, Duration::from_secs(60));
        assert_eq!(config.max_allocations, 8);
        assert_eq!(config.min_pillar_score, 0.25);
        assert_eq!(config.max_lane_concentration, 0.45);
    }

    #[test]
    fn test_warden_config_custom() {
        let config = WardenConfig {
            enabled: false,
            update_interval: Duration::from_secs(30),
            max_allocations: 4,
            min_pillar_score: 0.3,
            max_lane_concentration: 0.4,
        };

        assert!(!config.enabled);
        assert_eq!(config.update_interval, Duration::from_secs(30));
        assert_eq!(config.max_allocations, 4);
        assert_eq!(config.min_pillar_score, 0.3);
        assert_eq!(config.max_lane_concentration, 0.4);
    }
}

#[cfg(test)]
mod compute_lane_tests {
    use super::*;

    #[test]
    fn test_compute_lane_values() {
        let lanes = vec![
            ComputeLane::Security,
            ComputeLane::ChainOps,
            ComputeLane::Research,
            ComputeLane::Strategy,
            ComputeLane::AiAgents,
            ComputeLane::Ecosystem,
            ComputeLane::Storage,
            ComputeLane::Overflow,
            ComputeLane::Evolution,
        ];

        for lane in lanes {
            assert!(lane.is_valid());
            assert!(!lane.to_string().is_empty());
        }
    }

    #[test]
    fn test_compute_lane_priorities() {
        // Security should have highest priority
        assert!(ComputeLane::Security > ComputeLane::ChainOps);
        assert!(ComputeLane::ChainOps > ComputeLane::Research);
        assert!(ComputeLane::Research > ComputeLane::Strategy);
        assert!(ComputeLane::Strategy > ComputeLane::AiAgents);
        assert!(ComputeLane::AiAgents > ComputeLane::Ecosystem);
        assert!(ComputeLane::Ecosystem > ComputeLane::Storage);
        assert!(ComputeLane::Storage > ComputeLane::Overflow);
        assert!(ComputeLane::Overflow > ComputeLane::Evolution);
    }

    #[test]
    fn test_compute_lane_pillar_mapping() {
        let security_pillars = ComputeLane::Security.pillar_contributions();
        assert!(security_pillars.infrastructure > 0.0);
        assert!(security_pillars.ecosystem > 0.0);

        let research_pillars = ComputeLane::Research.pillar_contributions();
        assert!(research_pillars.intelligence > 0.0);
        assert!(research_pillars.profit > 0.0);

        let strategy_pillars = ComputeLane::Strategy.pillar_contributions();
        assert!(strategy_pillars.profit > 0.0);
        assert!(strategy_pillars.intelligence > 0.0);
    }
}

#[cfg(test)]
mod threat_level_tests {
    use super::*;

    #[test]
    fn test_threat_level_values() {
        let levels = vec![
            ThreatLevel::Low,
            ThreatLevel::Medium,
            ThreatLevel::High,
            ThreatLevel::Critical,
        ];

        for level in levels {
            assert!(level.is_valid());
            assert!(!level.to_string().is_empty());
        }
    }

    #[test]
    fn test_threat_level_ordering() {
        assert!(ThreatLevel::Critical > ThreatLevel::High);
        assert!(ThreatLevel::High > ThreatLevel::Medium);
        assert!(ThreatLevel::Medium > ThreatLevel::Low);
    }

    #[test]
    fn test_threat_level_severity() {
        assert_eq!(ThreatLevel::Low.severity(), 0);
        assert_eq!(ThreatLevel::Medium.severity(), 1);
        assert_eq!(ThreatLevel::High.severity(), 2);
        assert_eq!(ThreatLevel::Critical.severity(), 3);
    }
}

#[cfg(test)]
mod swarm_state_tests {
    use super::*;

    #[test]
    fn test_swarm_state_creation() {
        let state = SwarmState {
            timestamp: 1234567890,
            allocations: HashMap::new(),
            pillars: SwarmPillars::default(),
            threat_level: ThreatLevel::Low,
        };

        assert_eq!(state.timestamp, 1234567890);
        assert_eq!(state.threat_level, ThreatLevel::Low);
        assert!(state.allocations.is_empty());
    }

    #[test]
    fn test_swarm_state_with_allocations() {
        let mut allocations = HashMap::new();
        allocations.insert(ComputeLane::Security, 0.25);
        allocations.insert(ComputeLane::Strategy, 0.35);

        let state = SwarmState {
            timestamp: 1234567890,
            allocations,
            pillars: SwarmPillars::default(),
            threat_level: ThreatLevel::Medium,
        };

        assert_eq!(state.allocations.len(), 2);
        assert_eq!(state.allocations[&ComputeLane::Security], 0.25);
        assert_eq!(state.allocations[&ComputeLane::Strategy], 0.35);
    }
}

#[cfg(test)]
mod swarm_pillars_tests {
    use super::*;

    #[test]
    fn test_swarm_pillars_default() {
        let pillars = SwarmPillars::default();
        let scores = pillars.pillar_scores();
        
        assert_eq!(scores.profit, 0.25);
        assert_eq!(scores.intelligence, 0.25);
        assert_eq!(scores.infrastructure, 0.25);
        assert_eq!(scores.ecosystem, 0.25);
    }

    #[test]
    fn test_swarm_pillars_update() {
        let mut pillars = SwarmPillars::default();
        
        // Update profit pillar
        pillars.update_profit(0.1, 0.05);
        let scores = pillars.pillar_scores();
        assert!(scores.profit > 0.25);
        
        // Update intelligence pillar
        pillars.update_intelligence(0.2, 0.1);
        let scores = pillars.pillar_scores();
        assert!(scores.intelligence > 0.25);
        
        // Update infrastructure pillar
        pillars.update_infrastructure(0.15, 0.05);
        let scores = pillars.pillar_scores();
        assert!(scores.infrastructure > 0.25);
        
        // Update ecosystem pillar
        pillars.update_ecosystem(0.1, 0.08);
        let scores = pillars.pillar_scores();
        assert!(scores.ecosystem > 0.25);
    }

    #[test]
    fn test_swarm_pillars_normalization() {
        let mut pillars = SwarmPillars::default();
        
        // Push one pillar very high
        for _ in 0..10 {
            pillars.update_profit(0.5, 0.1);
        }
        
        let scores = pillars.pillar_scores();
        assert!(scores.profit > 0.5);
        
        // Normalize should bring it back to reasonable range
        pillars.normalize();
        let normalized_scores = pillars.pillar_scores();
        assert!(normalized_scores.profit <= 1.0);
        assert!(normalized_scores.profit >= 0.0);
    }

    #[test]
    fn test_swarm_pillars_health_check() {
        let mut pillars = SwarmPillars::default();
        
        // Healthy state
        assert!(pillars.is_healthy());
        
        // Unhealthy state - one pillar too low
        pillars.update_profit(-0.5, -0.3);
        assert!(!pillars.is_healthy());
        
        // Recover
        pillars.update_profit(0.4, 0.2);
        assert!(pillars.is_healthy());
    }
}

#[cfg(test)]
mod allocation_plan_tests {
    use super::*;

    #[test]
    fn test_allocation_plan_creation() {
        let mut allocations = HashMap::new();
        allocations.insert(ComputeLane::Security, 0.25);
        allocations.insert(ComputeLane::Strategy, 0.35);
        allocations.insert(ComputeLane::Research, 0.20);
        allocations.insert(ComputeLane::Ecosystem, 0.20);

        let plan = AllocationPlan {
            allocations,
            confidence: 0.85,
            reasoning: "Test allocation".to_string(),
            timestamp: 1234567890,
        };

        assert_eq!(plan.confidence, 0.85);
        assert_eq!(plan.reasoning, "Test allocation");
        assert_eq!(plan.timestamp, 1234567890);
        assert_eq!(plan.allocations.len(), 4);
    }

    #[test]
    fn test_allocation_plan_validation() {
        let mut allocations = HashMap::new();
        allocations.insert(ComputeLane::Security, 0.5);
        allocations.insert(ComputeLane::Strategy, 0.6); // Sum > 1.0

        let plan = AllocationPlan {
            allocations,
            confidence: 0.85,
            reasoning: "Invalid allocation".to_string(),
            timestamp: 1234567890,
        };

        // Should detect invalid allocation
        let total: f64 = plan.allocations.values().sum();
        assert!(total > 1.0);
    }

    #[test]
    fn test_allocation_plan_balance() {
        let mut allocations = HashMap::new();
        allocations.insert(ComputeLane::Security, 0.25);
        allocations.insert(ComputeLane::Strategy, 0.25);
        allocations.insert(ComputeLane::Research, 0.25);
        allocations.insert(ComputeLane::Ecosystem, 0.25);

        let plan = AllocationPlan {
            allocations,
            confidence: 0.9,
            reasoning: "Balanced allocation".to_string(),
            timestamp: 1234567890,
        };

        let total: f64 = plan.allocations.values().sum();
        assert!((total - 1.0).abs() < 0.001);
    }
}

#[cfg(test)]
mod lane_signal_tests {
    use super::*;

    #[test]
    fn test_lane_signal_creation() {
        let signal = LaneSignal {
            lane: ComputeLane::Strategy,
            signal_type: SignalType::Profit,
            value: 0.8,
            confidence: 0.9,
            timestamp: 1234567890,
            source: "test".to_string(),
        };

        assert_eq!(signal.lane, ComputeLane::Strategy);
        assert_eq!(signal.signal_type, SignalType::Profit);
        assert_eq!(signal.value, 0.8);
        assert_eq!(signal.confidence, 0.9);
        assert_eq!(signal.timestamp, 1234567890);
        assert_eq!(signal.source, "test");
    }

    #[test]
    fn test_lane_signal_types() {
        let signal_types = vec![
            SignalType::Profit,
            SignalType::Intelligence,
            SignalType::Infrastructure,
            SignalType::Ecosystem,
            SignalType::Threat,
            SignalType::Opportunity,
            SignalType::Resource,
            SignalType::Performance,
        ];

        for signal_type in signal_types {
            assert!(signal_type.is_valid());
            assert!(!signal_type.to_string().is_empty());
        }
    }

    #[test]
    fn test_lane_signal_validation() {
        // Valid signal
        let valid_signal = LaneSignal {
            lane: ComputeLane::Strategy,
            signal_type: SignalType::Profit,
            value: 0.8,
            confidence: 0.9,
            timestamp: 1234567890,
            source: "test".to_string(),
        };
        assert!(valid_signal.is_valid());

        // Invalid signal - value out of range
        let invalid_signal = LaneSignal {
            lane: ComputeLane::Strategy,
            signal_type: SignalType::Profit,
            value: 1.5, // > 1.0
            confidence: 0.9,
            timestamp: 1234567890,
            source: "test".to_string(),
        };
        assert!(!invalid_signal.is_valid());

        // Invalid signal - confidence too low
        let invalid_signal2 = LaneSignal {
            lane: ComputeLane::Strategy,
            signal_type: SignalType::Profit,
            value: 0.8,
            confidence: 0.1, // < 0.2
            timestamp: 1234567890,
            source: "test".to_string(),
        };
        assert!(!invalid_signal2.is_valid());
    }
}

#[cfg(test)]
mod signal_aggregator_tests {
    use super::*;

    #[test]
    fn test_signal_aggregator_creation() {
        let aggregator = SignalAggregator::new();
        assert!(aggregator.signals().is_empty());
        assert_eq!(aggregator.get_signal_count(), 0);
    }

    #[test]
    fn test_signal_aggregation() {
        let mut aggregator = SignalAggregator::new();
        
        let signal1 = LaneSignal {
            lane: ComputeLane::Strategy,
            signal_type: SignalType::Profit,
            value: 0.8,
            confidence: 0.9,
            timestamp: 1234567890,
            source: "source1".to_string(),
        };

        let signal2 = LaneSignal {
            lane: ComputeLane::Strategy,
            signal_type: SignalType::Profit,
            value: 0.6,
            confidence: 0.7,
            timestamp: 1234567891,
            source: "source2".to_string(),
        };

        aggregator.add_signal(signal1);
        aggregator.add_signal(signal2);
        
        assert_eq!(aggregator.get_signal_count(), 2);
        
        let aggregated = aggregator.aggregate_signals();
        assert!(aggregated.contains_key(&ComputeLane::Strategy));
    }

    #[test]
    fn test_signal_expiry() {
        let mut aggregator = SignalAggregator::new();
        
        let old_signal = LaneSignal {
            lane: ComputeLane::Strategy,
            signal_type: SignalType::Profit,
            value: 0.8,
            confidence: 0.9,
            timestamp: 1234567890, // Very old
            source: "old".to_string(),
        };

        let new_signal = LaneSignal {
            lane: ComputeLane::Strategy,
            signal_type: SignalType::Profit,
            value: 0.6,
            confidence: 0.7,
            timestamp: 1234567890 + 3600, // Recent
            source: "new".to_string(),
        };

        aggregator.add_signal(old_signal);
        aggregator.add_signal(new_signal);
        
        // Should prioritize newer signal
        let aggregated = aggregator.aggregate_signals();
        let strategy_signals = aggregated.get(&ComputeLane::Strategy).unwrap();
        assert_eq!(strategy_signals.len(), 1);
    }
}

#[cfg(test)]
mod metrics_collector_tests {
    use super::*;

    #[test]
    fn test_metrics_collector_creation() {
        let collector = MetricsCollector::new();
        assert_eq!(collector.get_metric_count(), 0);
        assert!(collector.get_latest_metrics().is_empty());
    }

    #[test]
    fn test_metrics_collection() {
        let mut collector = MetricsCollector::new();
        
        collector.record_metric("profit", 100.0);
        collector.record_metric("cost", 50.0);
        collector.record_metric("revenue", 150.0);
        
        assert_eq!(collector.get_metric_count(), 3);
        
        let latest = collector.get_latest_metrics();
        assert_eq!(latest.get("profit"), Some(&100.0));
        assert_eq!(latest.get("cost"), Some(&50.0));
        assert_eq!(latest.get("revenue"), Some(&150.0));
    }

    #[test]
    fn test_metrics_trend() {
        let mut collector = MetricsCollector::new();
        
        // Record increasing trend
        for i in 1..=5 {
            collector.record_metric("profit", i as f64 * 10.0);
        }
        
        let trend = collector.get_metric_trend("profit");
        assert!(trend > 0.0); // Should be positive trend
        
        // Record decreasing trend
        for i in 1..=5 {
            collector.record_metric("cost", 100.0 - (i as f64 * 5.0));
        }
        
        let cost_trend = collector.get_metric_trend("cost");
        assert!(cost_trend < 0.0); // Should be negative trend
    }

    #[test]
    fn test_metrics_health() {
        let mut collector = MetricsCollector::new();
        
        // Healthy metrics
        collector.record_metric("error_rate", 0.01);
        collector.record_metric("availability", 0.99);
        collector.record_metric("latency_p95", 100.0);
        
        let health = collector.get_overall_health();
        assert!(health > 0.8); // Should be healthy
        
        // Unhealthy metrics
        collector.record_metric("error_rate", 0.15);
        collector.record_metric("availability", 0.85);
        collector.record_metric("latency_p95", 1000.0);
        
        let health2 = collector.get_overall_health();
        assert!(health2 < 0.5); // Should be unhealthy
    }
}

#[cfg(test)]
mod load_predictor_tests {
    use super::*;

    #[test]
    fn test_load_predictor_creation() {
        let predictor = LoadPredictor::new();
        assert!(predictor.predict_load().is_ok());
    }

    #[test]
    fn test_load_prediction() {
        let mut predictor = LoadPredictor::new();
        
        // Train with some data
        for i in 1..=100 {
            predictor.record_load(i as f64);
        }
        
        let prediction = predictor.predict_load().unwrap();
        assert!(prediction > 0.0);
        assert!(prediction < 200.0); // Should be reasonable prediction
    }

    #[test]
    fn test_load_trend_prediction() {
        let mut predictor = LoadPredictor::new();
        
        // Train with increasing trend
        for i in 1..=100 {
            predictor.record_load(i as f64 * 1.1);
        }
        
        let trend = predictor.predict_trend().unwrap();
        assert!(trend > 0.0); // Should predict increasing trend
    }
}

#[cfg(test)]
mod governance_engine_tests {
    use super::*;

    #[test]
    fn test_governance_engine_creation() {
        let engine = GovernanceEngine::new();
        assert!(engine.get_governance_actions().is_empty());
    }

    #[test]
    fn test_governance_action_creation() {
        let action = GovernanceAction::UpdateThreatLevel {
            level: ThreatLevel::High,
            source: "test".to_string(),
        };
        
        assert!(action.is_valid());
        assert!(!action.to_string().is_empty());
    }

    #[test]
    fn test_governance_action_execution() {
        let mut engine = GovernanceEngine::new();
        
        let action = GovernanceAction::UpdateThreatLevel {
            level: ThreatLevel::High,
            source: "test".to_string(),
        };
        
        let result = engine.execute_action(&action);
        assert!(result.is_ok());
        
        let actions = engine.get_governance_actions();
        assert_eq!(actions.len(), 1);
    }
}

#[cfg(test)]
mod guard_bot_tests {
    use super::*;

    #[test]
    fn test_guard_bot_creation() {
        let config = GuardConfig {
            guard_type: GuardType::Security,
            enabled: true,
            check_interval: Duration::from_secs(60),
            alert_threshold: 0.5,
        };

        let guard = GuardBot::new(config);
        assert_eq!(guard.guard_type(), GuardType::Security);
        assert!(guard.is_enabled());
    }

    #[test]
    fn test_guard_bot_monitoring() {
        let config = GuardConfig {
            guard_type: GuardType::Security,
            enabled: true,
            check_interval: Duration::from_secs(60),
            alert_threshold: 0.5,
        };

        let guard = GuardBot::new(config);
        
        // Simulate monitoring
        let state = SwarmState {
            timestamp: 1234567890,
            allocations: HashMap::new(),
            pillars: SwarmPillars::default(),
            threat_level: ThreatLevel::Low,
        };

        let events = guard.monitor(&state);
        assert!(events.is_ok());
    }

    #[test]
    fn test_guard_bot_alerts() {
        let config = GuardConfig {
            guard_type: GuardType::Security,
            enabled: true,
            check_interval: Duration::from_secs(60),
            alert_threshold: 0.1, // Very sensitive
        };

        let guard = GuardBot::new(config);
        
        // Create unhealthy state
        let mut pillars = SwarmPillars::default();
        pillars.update_profit(-0.8, -0.5); // Very bad profit
        
        let state = SwarmState {
            timestamp: 1234567890,
            allocations: HashMap::new(),
            pillars,
            threat_level: ThreatLevel::Critical,
        };

        let events = guard.monitor(&state);
        assert!(events.is_ok());
        
        let events = events.unwrap();
        assert!(!events.is_empty());
    }
}

#[cfg(test)]
mod guard_type_tests {
    use super::*;

    #[test]
    fn test_guard_type_values() {
        let guard_types = vec![
            GuardType::Security,
            GuardType::Profit,
            GuardType::Intelligence,
            GuardType::Infrastructure,
            GuardType::Ecosystem,
        ];

        for guard_type in guard_types {
            assert!(guard_type.is_valid());
            assert!(!guard_type.to_string().is_empty());
        }
    }

    #[test]
    fn test_guard_type_monitoring() {
        let guard_types = vec![
            GuardType::Security,
            GuardType::Profit,
            GuardType::Intelligence,
            GuardType::Infrastructure,
            GuardType::Ecosystem,
        ];

        for guard_type in guard_types {
            let config = GuardConfig {
                guard_type: guard_type.clone(),
                enabled: true,
                check_interval: Duration::from_secs(60),
                alert_threshold: 0.5,
            };

            let guard = GuardBot::new(config);
            assert_eq!(guard.guard_type(), guard_type);
        }
    }
}

#[cfg(test)]
mod guard_action_tests {
    use super::*;

    #[test]
    fn test_guard_action_values() {
        let actions = vec![
            GuardAction::Alert,
            GuardAction::Quarantine,
            GuardAction::Shutdown,
            GuardAction::Rebalance,
            GuardAction::Escalate,
        ];

        for action in actions {
            assert!(action.is_valid());
            assert!(!action.to_string().is_empty());
        }
    }

    #[test]
    fn test_guard_action_priority() {
        assert!(GuardAction::Shutdown > GuardAction::Quarantine);
        assert!(GuardAction::Quarantine > GuardAction::Alert);
        assert!(GuardAction::Rebalance > GuardAction::Alert);
        assert!(GuardAction::Escalate > GuardAction::Alert);
    }
}

#[cfg(test)]
mod guard_state_tests {
    use super::*;

    #[test]
    fn test_guard_state_values() {
        let states = vec![
            GuardState::Normal,
            GuardState::Warning,
            GuardState::Critical,
            GuardState::Quarantined,
            GuardState::Shutdown,
        ];

        for state in states {
            assert!(state.is_valid());
            assert!(!state.to_string().is_empty());
        }
    }

    #[test]
    fn test_guard_state_transitions() {
        let mut state = GuardState::Normal;
        
        // Normal -> Warning
        state = GuardState::Warning;
        assert!(state.is_valid());
        
        // Warning -> Critical
        state = GuardState::Critical;
        assert!(state.is_valid());
        
        // Critical -> Quarantined
        state = GuardState::Quarantined;
        assert!(state.is_valid());
        
        // Quarantined -> Shutdown
        state = GuardState::Shutdown;
        assert!(state.is_valid());
    }
}

#[cfg(test)]
mod guard_config_tests {
    use super::*;

    #[test]
    fn test_guard_config_default() {
        let config = GuardConfig::default();
        assert!(config.enabled);
        assert_eq!(config.check_interval, Duration::from_secs(60));
        assert_eq!(config.alert_threshold, 0.5);
    }

    #[test]
    fn test_guard_config_custom() {
        let config = GuardConfig {
            guard_type: GuardType::Security,
            enabled: false,
            check_interval: Duration::from_secs(30),
            alert_threshold: 0.8,
        };

        assert!(!config.enabled);
        assert_eq!(config.check_interval, Duration::from_secs(30));
        assert_eq!(config.alert_threshold, 0.8);
    }
}

#[cfg(test)]
mod guard_metrics_tests {
    use super::*;

    #[test]
    fn test_guard_metrics_creation() {
        let metrics = GuardMetrics {
            alerts_sent: 5,
            actions_taken: 3,
            false_positives: 1,
            true_positives: 4,
            last_check: Instant::now(),
            uptime: Duration::from_secs(3600),
        };

        assert_eq!(metrics.alerts_sent, 5);
        assert_eq!(metrics.actions_taken, 3);
        assert_eq!(metrics.false_positives, 1);
        assert_eq!(metrics.true_positives, 4);
        assert!(metrics.uptime.as_secs() > 0);
    }

    #[test]
    fn test_guard_metrics_accuracy() {
        let metrics = GuardMetrics {
            alerts_sent: 5,
            actions_taken: 3,
            false_positives: 1,
            true_positives: 4,
            last_check: Instant::now(),
            uptime: Duration::from_secs(3600),
        };

        let accuracy = metrics.accuracy();
        assert!(accuracy > 0.0);
        assert!(accuracy <= 1.0);
    }
}

#[cfg(test)]
mod guard_event_tests {
    use super::*;

    #[test]
    fn test_guard_event_creation() {
        let event = GuardEvent {
            event_type: GuardEventKind::Alert,
            severity: GuardEventSeverity::High,
            message: "Test alert".to_string(),
            timestamp: 1234567890,
            source: "test".to_string(),
        };

        assert_eq!(event.event_type, GuardEventKind::Alert);
        assert_eq!(event.severity, GuardEventSeverity::High);
        assert_eq!(event.message, "Test alert");
        assert_eq!(event.timestamp, 1234567890);
        assert_eq!(event.source, "test");
    }

    #[test]
    fn test_guard_event_severity() {
        let severities = vec![
            GuardEventSeverity::Info,
            GuardEventSeverity::Low,
            GuardEventSeverity::Medium,
            GuardEventSeverity::High,
            GuardEventSeverity::Critical,
        ];

        for severity in severities {
            assert!(severity.is_valid());
            assert!(!severity.to_string().is_empty());
        }
    }

    #[test]
    fn test_guard_event_kinds() {
        let kinds = vec![
            GuardEventKind::Alert,
            GuardEventKind::Quarantine,
            GuardEventKind::Shutdown,
            GuardEventKind::Rebalance,
            GuardEventKind::Escalate,
        ];

        for kind in kinds {
            assert!(kind.is_valid());
            assert!(!kind.to_string().is_empty());
        }
    }
}

#[cfg(test)]
mod warden_integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_warden_creation() {
        let config = WardenConfig::default();
        let warden = Warden::new(config);
        
        assert!(warden.is_enabled());
        assert_eq!(warden.get_update_interval(), Duration::from_secs(60));
    }

    #[tokio::test]
    async fn test_warden_decision_making() {
        let config = WardenConfig::default();
        let mut warden = Warden::new(config);
        
        let state = SwarmState {
            timestamp: 1234567890,
            allocations: HashMap::new(),
            pillars: SwarmPillars::default(),
            threat_level: ThreatLevel::Low,
        };

        let decision = warden.make_decision(&state).await;
        assert!(decision.is_ok());
        
        let decision = decision.unwrap();
        assert!(decision.allocation_plan.confidence > 0.0);
        assert!(!decision.allocation_plan.reasoning.is_empty());
    }

    #[tokio::test]
    async fn test_warden_with_signals() {
        let config = WardenConfig::default();
        let mut warden = Warden::new(config);
        
        // Add some signals
        let signal = LaneSignal {
            lane: ComputeLane::Strategy,
            signal_type: SignalType::Profit,
            value: 0.8,
            confidence: 0.9,
            timestamp: 1234567890,
            source: "test".to_string(),
        };

        warden.add_signal(signal);
        
        let state = SwarmState {
            timestamp: 1234567890,
            allocations: HashMap::new(),
            pillars: SwarmPillars::default(),
            threat_level: ThreatLevel::Low,
        };

        let decision = warden.make_decision(&state).await;
        assert!(decision.is_ok());
    }

    #[tokio::test]
    async fn test_warden_with_high_threat() {
        let config = WardenConfig::default();
        let mut warden = Warden::new(config);
        
        let state = SwarmState {
            timestamp: 1234567890,
            allocations: HashMap::new(),
            pillars: SwarmPillars::default(),
            threat_level: ThreatLevel::Critical,
        };

        let decision = warden.make_decision(&state).await;
        assert!(decision.is_ok());
        
        let decision = decision.unwrap();
        
        // Should prioritize Security lane in high threat
        let security_alloc = decision.allocation_plan.allocations
            .get(&ComputeLane::Security)
            .copied()
            .unwrap_or(0.0);
        
        assert!(security_alloc > 0.2); // Should have significant security allocation
    }
}

#[cfg(test)]
mod warden_edge_cases {
    use super::*;

    #[test]
    fn test_warden_with_empty_state() {
        let config = WardenConfig::default();
        let mut warden = Warden::new(config);
        
        let state = SwarmState {
            timestamp: 1234567890,
            allocations: HashMap::new(),
            pillars: SwarmPillars::default(),
            threat_level: ThreatLevel::Low,
        };

        let decision = warden.make_decision(&state);
        assert!(decision.is_ok());
    }

    #[test]
    fn test_warden_with_invalid_allocations() {
        let config = WardenConfig::default();
        let mut warden = Warden::new(config);
        
        let mut allocations = HashMap::new();
        allocations.insert(ComputeLane::Security, 1.5); // > 1.0
        allocations.insert(ComputeLane::Strategy, 0.5);

        let state = SwarmState {
            timestamp: 1234567890,
            allocations,
            pillars: SwarmPillars::default(),
            threat_level: ThreatLevel::Low,
        };

        let decision = warden.make_decision(&state);
        assert!(decision.is_ok());
        
        // Should normalize allocations
        let decision = decision.unwrap();
        let total: f64 = decision.allocation_plan.allocations.values().sum();
        assert!((total - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_warden_with_zero_pillars() {
        let config = WardenConfig::default();
        let mut warden = Warden::new(config);
        
        let pillars = SwarmPillars {
            profit_score: 0.0,
            intelligence_score: 0.0,
            infrastructure_score: 0.0,
            ecosystem_score: 0.0,
        };

        let state = SwarmState {
            timestamp: 1234567890,
            allocations: HashMap::new(),
            pillars,
            threat_level: ThreatLevel::Low,
        };

        let decision = warden.make_decision(&state);
        assert!(decision.is_ok());
    }
}

#[cfg(test)]
mod warden_performance_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_warden_decision_performance() {
        let config = WardenConfig::default();
        let mut warden = Warden::new(config);
        
        let state = SwarmState {
            timestamp: 1234567890,
            allocations: HashMap::new(),
            pillars: SwarmPillars::default(),
            threat_level: ThreatLevel::Low,
        };

        let start = Instant::now();
        
        for _ in 0..100 {
            let _decision = warden.make_decision(&state);
        }
        
        let duration = start.elapsed();
        println!("Made 100 decisions in {:?}", duration);
        assert!(duration.as_millis() < 1000); // Should be fast
    }

    #[test]
    fn test_signal_aggregation_performance() {
        let mut aggregator = SignalAggregator::new();
        
        let start = Instant::now();
        
        // Add many signals
        for i in 0..1000 {
            let signal = LaneSignal {
                lane: ComputeLane::Strategy,
                signal_type: SignalType::Profit,
                value: (i % 100) as f64 / 100.0,
                confidence: 0.8,
                timestamp: 1234567890 + i,
                source: format!("source{}", i),
            };
            aggregator.add_signal(signal);
        }
        
        // Aggregate
        let _aggregated = aggregator.aggregate_signals();
        
        let duration = start.elapsed();
        println!("Aggregated 1000 signals in {:?}", duration);
        assert!(duration.as_millis() < 100); // Should be fast
    }
}

#[cfg(test)]
mod warden_serialization_tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_swarm_state_serialization() {
        let state = SwarmState {
            timestamp: 1234567890,
            allocations: HashMap::new(),
            pillars: SwarmPillars::default(),
            threat_level: ThreatLevel::Low,
        };

        let serialized = serde_json::to_string(&state).expect("Failed to serialize");
        let deserialized: SwarmState = serde_json::from_str(&serialized).expect("Failed to deserialize");
        
        assert_eq!(state.timestamp, deserialized.timestamp);
        assert_eq!(state.threat_level, deserialized.threat_level);
    }

    #[test]
    fn test_allocation_plan_serialization() {
        let mut allocations = HashMap::new();
        allocations.insert(ComputeLane::Security, 0.25);
        allocations.insert(ComputeLane::Strategy, 0.35);

        let plan = AllocationPlan {
            allocations,
            confidence: 0.85,
            reasoning: "Test allocation".to_string(),
            timestamp: 1234567890,
        };

        let serialized = serde_json::to_string(&plan).expect("Failed to serialize");
        let deserialized: AllocationPlan = serde_json::from_str(&serialized).expect("Failed to deserialize");
        
        assert_eq!(plan.confidence, deserialized.confidence);
        assert_eq!(plan.reasoning, deserialized.reasoning);
        assert_eq!(plan.allocations.len(), deserialized.allocations.len());
    }

    #[test]
    fn test_lane_signal_serialization() {
        let signal = LaneSignal {
            lane: ComputeLane::Strategy,
            signal_type: SignalType::Profit,
            value: 0.8,
            confidence: 0.9,
            timestamp: 1234567890,
            source: "test".to_string(),
        };

        let serialized = serde_json::to_string(&signal).expect("Failed to serialize");
        let deserialized: LaneSignal = serde_json::from_str(&serialized).expect("Failed to deserialize");
        
        assert_eq!(signal.lane, deserialized.lane);
        assert_eq!(signal.signal_type, deserialized.signal_type);
        assert_eq!(signal.value, deserialized.value);
        assert_eq!(signal.confidence, deserialized.confidence);
    }
}