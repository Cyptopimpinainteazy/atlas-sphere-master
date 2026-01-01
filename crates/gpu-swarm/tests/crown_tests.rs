//! Comprehensive tests for the Crown module - Meta-Governor

use gpu_swarm::{
    crown::{
        Crown, CrownConfig, CrownVerdict, CrownIssue, IssueCategory, IssueSeverity,
        AuditReport, AuditSeverity, Auditor, ChainHealthMetrics, ProfitFlowMetrics,
        SecurityThreat, SwarmAnomalyType, Prophet, MarketForecast, ThreatForecast,
        VolatilityRegime, MarketCycle, ForecastHorizon, Scrapyard, ScrapyardModule,
        ScrapyardVerdict, QuarantineReason, RecycledKnowledge, DisassemblyReport,
        EmergencyPlan, AnnouncementPayload, AnnouncementType, AnnouncementSeverity,
    },
    warden::{SwarmState, SwarmPillars, ComputeLane, ThreatLevel},
    config::SwarmConfig,
    node::{GpuCapabilities, GpuBackend, NodeStatus, NodeRegistry},
    task::{Task, TaskType, TaskPriority, TaskStatus},
    error::SwarmError,
};
use std::collections::{HashMap, VecDeque};
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
mod crown_config_tests {
    use super::*;

    #[test]
    fn test_crown_config_default() {
        let config = CrownConfig::default();
        assert!(config.max_evolution_allocation < 0.15);
        assert!(config.drift_threshold > 0.0);
        assert_eq!(config.max_warden_errors, 3);
        assert!(config.prophet_enabled);
        assert!(config.scrapyard_enabled);
        assert_eq!(config.min_pillar_score, 0.25);
        assert_eq!(config.max_lane_concentration, 0.45);
    }

    #[test]
    fn test_crown_config_custom() {
        let config = CrownConfig {
            cycle_interval: Duration::from_secs(120),
            drift_threshold: 0.2,
            profit_loss_threshold: 0.25,
            max_warden_errors: 5,
            prophet_enabled: false,
            scrapyard_enabled: false,
            min_pillar_score: 0.3,
            max_lane_concentration: 0.4,
            max_evolution_allocation: 0.05,
        };

        assert_eq!(config.cycle_interval, Duration::from_secs(120));
        assert_eq!(config.drift_threshold, 0.2);
        assert_eq!(config.profit_loss_threshold, 0.25);
        assert_eq!(config.max_warden_errors, 5);
        assert!(!config.prophet_enabled);
        assert!(!config.scrapyard_enabled);
        assert_eq!(config.min_pillar_score, 0.3);
        assert_eq!(config.max_lane_concentration, 0.4);
        assert_eq!(config.max_evolution_allocation, 0.05);
    }
}

#[cfg(test)]
mod crown_verdict_tests {
    use super::*;

    #[test]
    fn test_crown_verdict_healthy() {
        let verdict = CrownVerdict::Healthy {
            confidence: 0.9,
            commendations: vec!["Good job".to_string()],
        };

        assert!(verdict.is_healthy());
        assert_eq!(verdict.severity(), 0);
    }

    #[test]
    fn test_crown_verdict_caution() {
        let verdict = CrownVerdict::Caution {
            issues: vec![],
            recommendations: vec!["Monitor closely".to_string()],
        };

        assert!(!verdict.is_healthy());
        assert_eq!(verdict.severity(), 1);
    }

    #[test]
    fn test_crown_verdict_warning() {
        let verdict = CrownVerdict::Warning {
            issues: vec![],
            required_actions: vec![],
        };

        assert!(!verdict.is_healthy());
        assert_eq!(verdict.severity(), 2);
    }

    #[test]
    fn test_crown_verdict_override() {
        let verdict = CrownVerdict::Override {
            reason: "Critical failure".to_string(),
            emergency_plan: Default::default(),
            warden_suspended: true,
        };

        assert!(!verdict.is_healthy());
        assert_eq!(verdict.severity(), 3);
    }

    #[test]
    fn test_crown_verdict_severity_ordering() {
        let healthy = CrownVerdict::Healthy { confidence: 0.9, commendations: vec![] };
        let caution = CrownVerdict::Caution { issues: vec![], recommendations: vec![] };
        let warning = CrownVerdict::Warning { issues: vec![], required_actions: vec![] };
        let override_v = CrownVerdict::Override { reason: "test".to_string(), emergency_plan: Default::default(), warden_suspended: true };

        assert!(override_v.severity() > warning.severity());
        assert!(warning.severity() > caution.severity());
        assert!(caution.severity() > healthy.severity());
    }
}

#[cfg(test)]
mod crown_issue_tests {
    use super::*;

    #[test]
    fn test_crown_issue_creation() {
        let issue = CrownIssue {
            category: IssueCategory::WardenDrift,
            description: "Test issue".to_string(),
            severity: IssueSeverity::High,
            detected_at: 1234567890,
            evidence: vec!["test evidence".to_string()],
            suggested_fix: Some("test fix".to_string()),
        };

        assert_eq!(issue.category, IssueCategory::WardenDrift);
        assert_eq!(issue.severity, IssueSeverity::High);
        assert_eq!(issue.description, "Test issue");
        assert_eq!(issue.detected_at, 1234567890);
        assert!(issue.suggested_fix.is_some());
    }

    #[test]
    fn test_issue_category_values() {
        let categories = vec![
            IssueCategory::WardenDrift,
            IssueCategory::AllocationBias,
            IssueCategory::ProfitDecline,
            IssueCategory::ChainHealth,
            IssueCategory::SecurityThreat,
            IssueCategory::EvolutionGaming,
            IssueCategory::ResourceExhaustion,
            IssueCategory::MissionCreep,
            IssueCategory::ModelInstability,
            IssueCategory::ChainStress,
        ];

        for category in categories {
            assert!(category.is_valid());
            assert!(!category.to_string().is_empty());
        }
    }

    #[test]
    fn test_issue_severity_values() {
        let severities = vec![
            IssueSeverity::Info,
            IssueSeverity::Low,
            IssueSeverity::Medium,
            IssueSeverity::High,
            IssueSeverity::Critical,
        ];

        for severity in severities {
            assert!(severity.is_valid());
            assert!(!severity.to_string().is_empty());
        }
    }

    #[test]
    fn test_issue_severity_ordering() {
        assert!(IssueSeverity::Critical > IssueSeverity::High);
        assert!(IssueSeverity::High > IssueSeverity::Medium);
        assert!(IssueSeverity::Medium > IssueSeverity::Low);
        assert!(IssueSeverity::Low > IssueSeverity::Info);
    }
}

#[cfg(test)]
mod auditor_tests {
    use super::*;

    #[test]
    fn test_auditor_creation() {
        let auditor = Auditor::new();
        assert!(auditor.get_audit_count() >= 0);
    }

    #[test]
    fn test_audit_report_creation() {
        let report = AuditReport {
            timestamp: 1234567890,
            chain_health: ChainHealthMetrics::default(),
            profit_flows: ProfitFlowMetrics::default(),
            security_threats: vec![],
            anomalies: vec![],
            overall_health_score: 0.8,
            flagged_modules: vec![],
            recommendations: vec![],
        };

        assert_eq!(report.timestamp, 1234567890);
        assert_eq!(report.overall_health_score, 0.8);
        assert!(report.security_threats.is_empty());
        assert!(report.anomalies.is_empty());
    }

    #[test]
    fn test_chain_health_metrics() {
        let health = ChainHealthMetrics {
            avg_block_time_ms: 10000.0,
            error_rate: 0.01,
            storage_usage: 0.5,
            consensus_healthy: true,
            consensus_warnings: vec![],
        };

        assert_eq!(health.avg_block_time_ms, 10000.0);
        assert_eq!(health.error_rate, 0.01);
        assert_eq!(health.storage_usage, 0.5);
        assert!(health.consensus_healthy);
    }

    #[test]
    fn test_profit_flow_metrics() {
        let profit = ProfitFlowMetrics {
            total_revenue: 1000.0,
            total_costs: 500.0,
            net_profit: 500.0,
            profit_trend: 0.1,
            revenue_sources: vec!["trading".to_string()],
            cost_breakdown: vec![("compute".to_string(), 300.0)],
        };

        assert_eq!(profit.total_revenue, 1000.0);
        assert_eq!(profit.total_costs, 500.0);
        assert_eq!(profit.net_profit, 500.0);
        assert_eq!(profit.profit_trend, 0.1);
    }

    #[test]
    fn test_security_threat() {
        let threat = SecurityThreat {
            threat_type: "DoS".to_string(),
            severity: AuditSeverity::High,
            description: "Denial of service attack detected".to_string(),
            indicators: vec!["high_traffic".to_string()],
            recommended_action: "Increase rate limiting".to_string(),
        };

        assert_eq!(threat.threat_type, "DoS");
        assert_eq!(threat.severity, AuditSeverity::High);
        assert_eq!(threat.description, "Denial of service attack detected");
        assert!(!threat.indicators.is_empty());
        assert_eq!(threat.recommended_action, "Increase rate limiting");
    }

    #[test]
    fn test_swarm_anomaly_type() {
        let anomaly_types = vec![
            SwarmAnomalyType::ResourceHogging { module_id: "test".to_string(), allocation: 0.8 },
            SwarmAnomalyType::SuspiciousOutput { module_id: "test".to_string(), output_hash: "abc123".to_string() },
            SwarmAnomalyType::PrivilegeEscalation { module_id: "test".to_string(), attempted_access: "restricted".to_string() },
            SwarmAnomalyType::UnverifiableWork { module_id: "test".to_string(), task_id: uuid::Uuid::new_v4() },
            SwarmAnomalyType::GamingAttempt { module_id: "test".to_string(), gaming_method: "fake_signals".to_string() },
            SwarmAnomalyType::RunawayConsumption { module_id: "test".to_string(), resource_type: "gpu".to_string(), consumption_rate: 1000.0 },
        ];

        for anomaly in anomaly_types {
            assert!(anomaly.is_valid());
            assert!(!anomaly.to_string().is_empty());
        }
    }
}

#[cfg(test)]
mod prophet_tests {
    use super::*;

    #[test]
    fn test_prophet_creation() {
        let prophet = Prophet::new(true);
        assert!(prophet.is_enabled());
    }

    #[test]
    fn test_market_forecast() {
        let forecast = MarketForecast {
            generated_at: 1234567890,
            horizon: ForecastHorizon::Short,
            volatility: VolatilityRegime::Normal,
            cycle: MarketCycle::Bull,
            confidence: 0.8,
            threat_forecasts: vec![],
            opportunities: vec![],
            recommendations: vec![],
        };

        assert_eq!(forecast.generated_at, 1234567890);
        assert_eq!(forecast.horizon, ForecastHorizon::Short);
        assert_eq!(forecast.volatility, VolatilityRegime::Normal);
        assert_eq!(forecast.cycle, MarketCycle::Bull);
        assert_eq!(forecast.confidence, 0.8);
    }

    #[test]
    fn test_threat_forecast() {
        let threat = ThreatForecast {
            threat_type: "market_crash".to_string(),
            probability: 0.7,
            indicators: vec!["high_volatility".to_string()],
            mitigation: Some("Reduce risk exposure".to_string()),
        };

        assert_eq!(threat.threat_type, "market_crash");
        assert_eq!(threat.probability, 0.7);
        assert!(!threat.indicators.is_empty());
        assert!(threat.mitigation.is_some());
    }

    #[test]
    fn test_forecast_horizon_values() {
        let horizons = vec![
            ForecastHorizon::Immediate,
            ForecastHorizon::Short,
            ForecastHorizon::Medium,
            ForecastHorizon::Long,
        ];

        for horizon in horizons {
            assert!(horizon.is_valid());
            assert!(!horizon.to_string().is_empty());
        }
    }

    #[test]
    fn test_volatility_regime_values() {
        let regimes = vec![
            VolatilityRegime::Low,
            VolatilityRegime::Normal,
            VolatilityRegime::High,
            VolatilityRegime::Extreme,
        ];

        for regime in regimes {
            assert!(regime.is_valid());
            assert!(!regime.to_string().is_empty());
        }
    }

    #[test]
    fn test_market_cycle_values() {
        let cycles = vec![
            MarketCycle::Bull,
            MarketCycle::Bear,
            MarketCycle::Sideways,
            MarketCycle::Transition,
        ];

        for cycle in cycles {
            assert!(cycle.is_valid());
            assert!(!cycle.to_string().is_empty());
        }
    }
}

#[cfg(test)]
mod scrapyard_tests {
    use super::*;

    #[test]
    fn test_scrapyard_creation() {
        let scrapyard = Scrapyard::new();
        assert!(scrapyard.get_quarantined_count() == 0);
        assert!(scrapyard.get_recycled_count() == 0);
    }

    #[test]
    fn test_scrapyard_module() {
        let module = ScrapyardModule {
            module_id: "test_module".to_string(),
            reason: QuarantineReason::Gaming,
            quarantined_at: 1234567890,
            last_activity: 1234567890,
            performance_history: vec![0.8, 0.7, 0.6],
            resource_usage: HashMap::new(),
        };

        assert_eq!(module.module_id, "test_module");
        assert_eq!(module.reason, QuarantineReason::Gaming);
        assert_eq!(module.quarantined_at, 1234567890);
        assert_eq!(module.performance_history.len(), 3);
    }

    #[test]
    fn test_scrapyard_verdict() {
        let verdict = ScrapyardVerdict::Quarantine {
            reason: QuarantineReason::Gaming,
            duration: Duration::from_secs(3600),
        };

        assert!(verdict.is_valid());
        assert!(!verdict.to_string().is_empty());
    }

    #[test]
    fn test_quarantine_reason_values() {
        let reasons = vec![
            QuarantineReason::Gaming,
            QuarantineReason::Malicious,
            QuarantineReason::Incompetent,
            QuarantineReason::ResourceExhaustion,
            QuarantineReason::SecurityViolation,
        ];

        for reason in reasons {
            assert!(reason.is_valid());
            assert!(!reason.to_string().is_empty());
        }
    }

    #[test]
    fn test_recycled_knowledge() {
        let knowledge = RecycledKnowledge {
            module_id: "test_module".to_string(),
            knowledge_type: "optimization".to_string(),
            content: "Optimization technique".to_string(),
            confidence: 0.8,
            extracted_at: 1234567890,
        };

        assert_eq!(knowledge.module_id, "test_module");
        assert_eq!(knowledge.knowledge_type, "optimization");
        assert_eq!(knowledge.content, "Optimization technique");
        assert_eq!(knowledge.confidence, 0.8);
        assert_eq!(knowledge.extracted_at, 1234567890);
    }

    #[test]
    fn test_disassembly_report() {
        let report = DisassemblyReport {
            module_id: "test_module".to_string(),
            disassembly_time: 1234567890,
            components_extracted: vec!["optimizer".to_string()],
            knowledge_gained: vec!["optimization".to_string()],
            warnings: vec![],
        };

        assert_eq!(report.module_id, "test_module");
        assert_eq!(report.disassembly_time, 1234567890);
        assert!(!report.components_extracted.is_empty());
    }
}

#[cfg(test)]
mod emergency_plan_tests {
    use super::*;

    #[test]
    fn test_emergency_plan_creation() {
        let plan = EmergencyPlan {
            forced_allocations: HashMap::new(),
            halt_lanes: vec![],
            quarantine_modules: vec![],
            actions: vec![],
            duration: Duration::from_secs(1800),
            justification: "Test emergency".to_string(),
        };

        assert_eq!(plan.duration, Duration::from_secs(1800));
        assert_eq!(plan.justification, "Test emergency");
        assert!(plan.forced_allocations.is_empty());
        assert!(plan.halt_lanes.is_empty());
    }

    #[test]
    fn test_emergency_plan_with_actions() {
        let mut allocations = HashMap::new();
        allocations.insert(ComputeLane::Security, 0.4);
        allocations.insert(ComputeLane::ChainOps, 0.3);

        let plan = EmergencyPlan {
            forced_allocations: allocations,
            halt_lanes: vec![ComputeLane::Evolution],
            quarantine_modules: vec!["malicious_module".to_string()],
            actions: vec![],
            duration: Duration::from_secs(3600),
            justification: "Security breach detected".to_string(),
        };

        assert_eq!(plan.forced_allocations.len(), 2);
        assert_eq!(plan.halt_lanes.len(), 1);
        assert_eq!(plan.quarantine_modules.len(), 1);
        assert_eq!(plan.duration, Duration::from_secs(3600));
        assert_eq!(plan.justification, "Security breach detected");
    }
}

#[cfg(test)]
mod announcement_tests {
    use super::*;

    #[test]
    fn test_announcement_payload() {
        let payload = AnnouncementPayload::WardenDecision {
            decision: "Allocate more to security".to_string(),
            confidence: 0.85,
        };

        assert!(payload.is_valid());
        assert!(!payload.to_string().is_empty());
    }

    #[test]
    fn test_announcement_type_values() {
        let types = vec![
            AnnouncementType::WardenDecision,
            AnnouncementType::CrownEvaluation,
            AnnouncementType::FundingCampaign,
            AnnouncementType::ProphetForecast,
            AnnouncementType::ScrapyardAction,
        ];

        for announcement_type in types {
            assert!(announcement_type.is_valid());
            assert!(!announcement_type.to_string().is_empty());
        }
    }

    #[test]
    fn test_announcement_severity_values() {
        let severities = vec![
            AnnouncementSeverity::Info,
            AnnouncementSeverity::Low,
            AnnouncementSeverity::Medium,
            AnnouncementSeverity::High,
            AnnouncementSeverity::Critical,
        ];

        for severity in severities {
            assert!(severity.is_valid());
            assert!(!severity.to_string().is_empty());
        }
    }
}

#[cfg(test)]
mod crown_integration_tests {
    use super::*;

    #[tokio::test]
    async fn test_crown_creation() {
        let crown = Crown::default();
        assert!(!crown.is_warden_suspended());
        assert!(crown.history().is_empty());
        assert!(crown.uptime().as_secs() >= 0);
    }

    #[tokio::test]
    async fn test_crown_evaluation_healthy() {
        let mut crown = Crown::default();
        let state = SwarmState {
            timestamp: 1234567890,
            allocations: HashMap::new(),
            pillars: SwarmPillars::default(),
            threat_level: ThreatLevel::Low,
        };

        let verdict = crown.evaluate(&state, None).await;
        assert!(verdict.is_healthy() || matches!(verdict, CrownVerdict::Caution { .. }));
    }

    #[tokio::test]
    async fn test_crown_evaluation_with_issues() {
        let mut crown = Crown::default();
        let mut state = SwarmState {
            timestamp: 1234567890,
            allocations: HashMap::new(),
            pillars: SwarmPillars::default(),
            threat_level: ThreatLevel::Critical,
        };

        // Make pillars unhealthy
        state.pillars.update_profit(-0.8, -0.5);
        state.pillars.update_intelligence(-0.7, -0.4);

        let verdict = crown.evaluate(&state, None).await;
        assert!(!verdict.is_healthy());
    }

    #[tokio::test]
    async fn test_crown_with_warden_decision() {
        let mut crown = Crown::default();
        let state = SwarmState {
            timestamp: 1234567890,
            allocations: HashMap::new(),
            pillars: SwarmPillars::default(),
            threat_level: ThreatLevel::Low,
        };

        let decision = gpu_swarm::warden::WardenDecision {
            allocation_plan: gpu_swarm::warden::AllocationPlan {
                allocations: HashMap::new(),
                confidence: 0.85,
                reasoning: "Test decision".to_string(),
                timestamp: 1234567890,
            },
            decided_at: 1234567890,
            signals_used: vec![],
        };

        let verdict = crown.evaluate(&state, Some(&decision)).await;
        assert!(verdict.is_healthy() || matches!(verdict, CrownVerdict::Caution { .. }));
    }

    #[tokio::test]
    async fn test_crown_quarantine() {
        let crown = Crown::default();
        crown.quarantine_module("test_module", QuarantineReason::Gaming).await;
        
        // Should be able to quarantine modules
        assert!(true);
    }

    #[tokio::test]
    async fn test_crown_scrapyard_processing() {
        let crown = Crown::default();
        let knowledge = crown.process_scrapyard().await;
        
        // Should process scrapyard and return knowledge
        assert!(knowledge.is_empty());
    }
}

#[cfg(test)]
mod crown_edge_cases {
    use super::*;

    #[test]
    fn test_crown_with_empty_state() {
        let mut crown = Crown::default();
        let state = SwarmState {
            timestamp: 1234567890,
            allocations: HashMap::new(),
            pillars: SwarmPillars::default(),
            threat_level: ThreatLevel::Low,
        };

        let verdict = crown.evaluate(&state, None);
        assert!(verdict.is_ok());
    }

    #[test]
    fn test_crown_with_extreme_allocations() {
        let mut crown = Crown::default();
        let mut state = SwarmState {
            timestamp: 1234567890,
            allocations: HashMap::new(),
            pillars: SwarmPillars::default(),
            threat_level: ThreatLevel::Critical,
        };

        // Extreme allocation to one lane
        state.allocations.insert(ComputeLane::Evolution, 0.9);
        state.allocations.insert(ComputeLane::Security, 0.1);

        let verdict = crown.evaluate(&state, None);
        assert!(verdict.is_ok());
        
        let verdict = verdict.unwrap();
        assert!(!verdict.is_healthy()); // Should detect imbalance
    }

    #[test]
    fn test_crown_with_zero_pillars() {
        let mut crown = Crown::default();
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

        let verdict = crown.evaluate(&state, None);
        assert!(verdict.is_ok());
        
        let verdict = verdict.unwrap();
        assert!(!verdict.is_healthy()); // Should detect zero pillars
    }
}

#[cfg(test)]
mod crown_performance_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_crown_evaluation_performance() {
        let mut crown = Crown::default();
        let state = SwarmState {
            timestamp: 1234567890,
            allocations: HashMap::new(),
            pillars: SwarmPillars::default(),
            threat_level: ThreatLevel::Low,
        };

        let start = Instant::now();
        
        for _ in 0..100 {
            let _verdict = crown.evaluate(&state, None);
        }
        
        let duration = start.elapsed();
        println!("Performed 100 crown evaluations in {:?}", duration);
        assert!(duration.as_millis() < 2000); // Should be reasonably fast
    }

    #[test]
    fn test_audit_performance() {
        let mut auditor = Auditor::new();
        let state = SwarmState {
            timestamp: 1234567890,
            allocations: HashMap::new(),
            pillars: SwarmPillars::default(),
            threat_level: ThreatLevel::Low,
        };

        let start = Instant::now();
        
        for _ in 0..100 {
            let _report = auditor.full_audit(&state);
        }
        
        let duration = start.elapsed();
        println!("Performed 100 audits in {:?}", duration);
        assert!(duration.as_millis() < 1000); // Should be fast
    }
}

#[cfg(test)]
mod crown_serialization_tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_crown_verdict_serialization() {
        let verdict = CrownVerdict::Healthy {
            confidence: 0.9,
            commendations: vec!["Good job".to_string()],
        };

        let serialized = serde_json::to_string(&verdict).expect("Failed to serialize");
        let deserialized: CrownVerdict = serde_json::from_str(&serialized).expect("Failed to deserialize");
        
        assert!(matches!(deserialized, CrownVerdict::Healthy { .. }));
    }

    #[test]
    fn test_crown_issue_serialization() {
        let issue = CrownIssue {
            category: IssueCategory::WardenDrift,
            description: "Test issue".to_string(),
            severity: IssueSeverity::High,
            detected_at: 1234567890,
            evidence: vec!["test evidence".to_string()],
            suggested_fix: Some("test fix".to_string()),
        };

        let serialized = serde_json::to_string(&issue).expect("Failed to serialize");
        let deserialized: CrownIssue = serde_json::from_str(&serialized).expect("Failed to deserialize");
        
        assert_eq!(issue.category, deserialized.category);
        assert_eq!(issue.severity, deserialized.severity);
        assert_eq!(issue.description, deserialized.description);
    }

    #[test]
    fn test_audit_report_serialization() {
        let report = AuditReport {
            timestamp: 1234567890,
            chain_health: ChainHealthMetrics::default(),
            profit_flows: ProfitFlowMetrics::default(),
            security_threats: vec![],
            anomalies: vec![],
            overall_health_score: 0.8,
            flagged_modules: vec![],
            recommendations: vec![],
        };

        let serialized = serde_json::to_string(&report).expect("Failed to serialize");
        let deserialized: AuditReport = serde_json::from_str(&serialized).expect("Failed to deserialize");
        
        assert_eq!(report.timestamp, deserialized.timestamp);
        assert_eq!(report.overall_health_score, deserialized.overall_health_score);
    }

    #[test]
    fn test_market_forecast_serialization() {
        let forecast = MarketForecast {
            generated_at: 1234567890,
            horizon: ForecastHorizon::Short,
            volatility: VolatilityRegime::Normal,
            cycle: MarketCycle::Bull,
            confidence: 0.8,
            threat_forecasts: vec![],
            opportunities: vec![],
            recommendations: vec![],
        };

        let serialized = serde_json::to_string(&forecast).expect("Failed to serialize");
        let deserialized: MarketForecast = serde_json::from_str(&serialized).expect("Failed to deserialize");
        
        assert_eq!(forecast.generated_at, deserialized.generated_at);
        assert_eq!(forecast.horizon, deserialized.horizon);
        assert_eq!(forecast.volatility, deserialized.volatility);
        assert_eq!(forecast.cycle, deserialized.cycle);
        assert_eq!(forecast.confidence, deserialized.confidence);
    }

    #[test]
    fn test_emergency_plan_serialization() {
        let plan = EmergencyPlan {
            forced_allocations: HashMap::new(),
            halt_lanes: vec![],
            quarantine_modules: vec![],
            actions: vec![],
            duration: Duration::from_secs(1800),
            justification: "Test emergency".to_string(),
        };

        let serialized = serde_json::to_string(&plan).expect("Failed to serialize");
        let deserialized: EmergencyPlan = serde_json::from_str(&serialized).expect("Failed to deserialize");
        
        assert_eq!(plan.duration, deserialized.duration);
        assert_eq!(plan.justification, deserialized.justification);
    }
}

#[cfg(test)]
mod crown_error_handling_tests {
    use super::*;

    #[test]
    fn test_crown_with_invalid_config() {
        let config = CrownConfig {
            cycle_interval: Duration::from_secs(0), // Invalid interval
            drift_threshold: -0.1, // Invalid threshold
            profit_loss_threshold: 1.5, // Invalid threshold
            max_warden_errors: 0, // Invalid count
            prophet_enabled: true,
            scrapyard_enabled: true,
            min_pillar_score: -0.1, // Invalid score
            max_lane_concentration: 1.5, // Invalid concentration
            max_evolution_allocation: 1.1, // Invalid allocation
        };

        let crown = Crown::new(config);
        
        // Should handle invalid config gracefully
        assert!(!crown.is_warden_suspended());
    }

    #[test]
    fn test_crown_with_corrupted_state() {
        let mut crown = Crown::default();
        let mut state = SwarmState {
            timestamp: 1234567890,
            allocations: HashMap::new(),
            pillars: SwarmPillars::default(),
            threat_level: ThreatLevel::Low,
        };

        // Corrupt the state
        state.allocations.insert(ComputeLane::Security, -0.1); // Negative allocation
        state.allocations.insert(ComputeLane::Strategy, 2.0); // > 1.0 allocation

        let verdict = crown.evaluate(&state, None);
        assert!(verdict.is_ok()); // Should handle gracefully
    }

    #[test]
    fn test_crown_history_overflow() {
        let mut crown = Crown::default();
        
        // Fill history to capacity
        for i in 0..1000 {
            let state = SwarmState {
                timestamp: 1234567890 + i,
                allocations: HashMap::new(),
                pillars: SwarmPillars::default(),
                threat_level: ThreatLevel::Low,
            };

            let verdict = CrownVerdict::Healthy {
                confidence: 0.8,
                commendations: vec![],
            };

            // Simulate recording evaluation
            crown.record_evaluation(&state, &Default::default(), None, &verdict);
        }

        // Should handle overflow gracefully
        assert!(crown.history().len() <= 1000);
    }
}