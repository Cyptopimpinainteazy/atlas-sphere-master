//! Campaign Orchestrator - Prophet-Aware Funding Scheduling
//!
//! Orchestrates funding campaigns based on market conditions detected by Prophet.
//! Ensures campaigns launch at optimal times and tracks results.

use super::novaflux::NovaFlux;
use super::webhook::{WebhookBridge, WebhookPayload};
use crate::crown::{MarketCycle, MarketForecast, VolatilityRegime};
use crate::jobs::funding_campaign::{CampaignType, FundingCampaignConfig, Prospect};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

/// Orchestrator configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorConfig {
    /// Enable automatic scheduling
    pub auto_schedule: bool,
    /// Maximum concurrent campaigns
    pub max_concurrent: usize,
    /// Cooldown between campaigns of same type (seconds)
    pub type_cooldown_secs: u64,
    /// Enable Prophet-based timing
    pub prophet_enabled: bool,
    /// Market conditions that trigger campaigns
    pub trigger_conditions: TriggerConditions,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            auto_schedule: true,
            max_concurrent: 3,
            type_cooldown_secs: 86400, // 24 hours
            prophet_enabled: true,
            trigger_conditions: TriggerConditions::default(),
        }
    }
}

/// Conditions that trigger automatic campaigns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerConditions {
    /// Market cycles that trigger VC outreach
    pub vc_trigger_cycles: Vec<MarketCycle>,
    /// Market cycles that trigger social campaigns
    pub social_trigger_cycles: Vec<MarketCycle>,
    /// Minimum confidence for Prophet triggers
    pub min_prophet_confidence: u8,
    /// Volatility regimes that pause campaigns
    pub pause_volatility: Vec<VolatilityRegime>,
}

impl Default for TriggerConditions {
    fn default() -> Self {
        Self {
            vc_trigger_cycles: vec![
                MarketCycle::Accumulation,
                MarketCycle::Bull,
            ],
            social_trigger_cycles: vec![
                MarketCycle::Bull,
                MarketCycle::Accumulation,
            ],
            min_prophet_confidence: 65,
            pause_volatility: vec![VolatilityRegime::Extreme],
        }
    }
}

/// Scheduled campaign
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignSchedule {
    /// Campaign ID
    pub campaign_id: String,
    /// Campaign type
    pub campaign_type: CampaignType,
    /// Scheduled start (Unix timestamp)
    pub scheduled_start: u64,
    /// Status
    pub status: CampaignStatus,
    /// Prophet forecast that triggered (if any)
    pub trigger_forecast: Option<ForecastTrigger>,
    /// Configuration
    pub config: FundingCampaignConfig,
    /// Results (when completed)
    pub results: Option<CampaignResults>,
}

/// Campaign status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CampaignStatus {
    Scheduled,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

/// Forecast that triggered a campaign
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastTrigger {
    pub cycle: MarketCycle,
    pub confidence: u8,
    pub volatility: VolatilityRegime,
    pub sentiment: i8,
}

/// Campaign results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignResults {
    pub targets_reached: usize,
    pub messages_sent: usize,
    pub responses_received: usize,
    pub positive_responses: usize,
    pub conversion_rate: f64,
    pub duration_secs: u64,
}

/// The Campaign Orchestrator
pub struct CampaignOrchestrator {
    config: OrchestratorConfig,
    /// NovaFlux instance for content
    novaflux: Arc<RwLock<NovaFlux>>,
    /// Webhook bridge for dispatch
    webhook_bridge: Arc<WebhookBridge>,
    /// Scheduled campaigns
    schedules: Arc<RwLock<Vec<CampaignSchedule>>>,
    /// Campaign counter
    campaign_counter: Arc<RwLock<u64>>,
    /// Last campaign time by type
    last_campaign: Arc<RwLock<HashMap<CampaignType, u64>>>,
    /// Current Prophet forecast
    current_forecast: Arc<RwLock<Option<MarketForecast>>>,
    /// Started at
    started_at: Instant,
}

impl CampaignOrchestrator {
    /// Create a new orchestrator
    pub fn new(
        config: OrchestratorConfig,
        novaflux: NovaFlux,
        webhook_bridge: WebhookBridge,
    ) -> Self {
        Self {
            config,
            novaflux: Arc::new(RwLock::new(novaflux)),
            webhook_bridge: Arc::new(webhook_bridge),
            schedules: Arc::new(RwLock::new(Vec::new())),
            campaign_counter: Arc::new(RwLock::new(0)),
            last_campaign: Arc::new(RwLock::new(HashMap::new())),
            current_forecast: Arc::new(RwLock::new(None)),
            started_at: Instant::now(),
        }
    }

    /// Update Prophet forecast
    pub async fn update_forecast(&self, forecast: MarketForecast) {
        let mut current = self.current_forecast.write().await;
        *current = Some(forecast.clone());

        // Check if we should auto-schedule campaigns
        if self.config.auto_schedule && self.config.prophet_enabled {
            self.evaluate_triggers(&forecast).await;
        }
    }

    /// Evaluate if forecast should trigger campaigns
    async fn evaluate_triggers(&self, forecast: &MarketForecast) {
        // Check if volatility is too high
        if self.config.trigger_conditions.pause_volatility.contains(&forecast.volatility) {
            return;
        }

        // Check confidence threshold (convert f64 to u8 for comparison)
        let confidence_pct = (forecast.cycle_confidence * 100.0) as u8;
        if confidence_pct < self.config.trigger_conditions.min_prophet_confidence {
            return;
        }

        // Check VC trigger conditions
        if self.config.trigger_conditions.vc_trigger_cycles.contains(&forecast.cycle) {
            if self.can_schedule(CampaignType::VcOutreach).await {
                self.auto_schedule_vc_campaign(forecast).await;
            }
        }

        // Check social trigger conditions
        if self.config.trigger_conditions.social_trigger_cycles.contains(&forecast.cycle) {
            if self.can_schedule(CampaignType::SocialCampaign).await {
                self.auto_schedule_social_campaign(forecast).await;
            }
        }
    }

    /// Check if we can schedule a campaign type
    async fn can_schedule(&self, campaign_type: CampaignType) -> bool {
        // Check concurrent limit
        let schedules = self.schedules.read().await;
        let running = schedules
            .iter()
            .filter(|s| s.status == CampaignStatus::Running)
            .count();

        if running >= self.config.max_concurrent {
            return false;
        }

        // Check cooldown
        let last = self.last_campaign.read().await;
        if let Some(last_time) = last.get(&campaign_type) {
            let now = timestamp_now();
            if now - last_time < self.config.type_cooldown_secs {
                return false;
            }
        }

        true
    }

    /// Auto-schedule a VC outreach campaign
    async fn auto_schedule_vc_campaign(&self, forecast: &MarketForecast) {
        let campaign_id = self.generate_campaign_id().await;

        let config = FundingCampaignConfig {
            campaign_type: CampaignType::VcOutreach,
            campaign_id: campaign_id.clone(),
            project_name: "Atlas Sphere".to_string(),
            project_description: "Dual-VM blockchain (EVM + SVM)".to_string(),
            value_props: vec![
                "EVM compatibility for existing contracts".to_string(),
                "SVM parallelization for 10x throughput".to_string(),
                "Native AI agent execution".to_string(),
            ],
            targets: vec![], // Would be populated from prospect database
            max_outreach: 20,
            webhook_url: None,
            llm_engine: crate::jobs::funding_campaign::LlmEngine::Local,
            personalization_level: crate::jobs::funding_campaign::PersonalizationLevel::Medium,
            test_variants: 2,
            prophet_timed: true,
            market_context: Some(format!(
                "Market cycle: {:?}, Sentiment: {}, Confidence: {}%",
                forecast.cycle, forecast.sentiment, forecast.cycle_confidence
            )),
        };

        let schedule = CampaignSchedule {
            campaign_id,
            campaign_type: CampaignType::VcOutreach,
            scheduled_start: timestamp_now(),
            status: CampaignStatus::Scheduled,
            trigger_forecast: Some(ForecastTrigger {
                cycle: forecast.cycle.clone(),
                confidence: (forecast.cycle_confidence * 100.0) as u8,
                volatility: forecast.volatility.clone(),
                sentiment: (forecast.sentiment * 100.0) as i8,
            }),
            config,
            results: None,
        };

        let mut schedules = self.schedules.write().await;
        schedules.push(schedule);

        // Update last campaign time
        let mut last = self.last_campaign.write().await;
        last.insert(CampaignType::VcOutreach, timestamp_now());
    }

    /// Auto-schedule a social campaign
    async fn auto_schedule_social_campaign(&self, forecast: &MarketForecast) {
        let campaign_id = self.generate_campaign_id().await;

        let config = FundingCampaignConfig {
            campaign_type: CampaignType::SocialCampaign,
            campaign_id: campaign_id.clone(),
            project_name: "Atlas Sphere".to_string(),
            project_description: "Dual-VM blockchain".to_string(),
            value_props: vec![],
            targets: vec![],
            max_outreach: 10, // 10 social posts
            webhook_url: None,
            llm_engine: crate::jobs::funding_campaign::LlmEngine::Local,
            personalization_level: crate::jobs::funding_campaign::PersonalizationLevel::None,
            test_variants: 1,
            prophet_timed: true,
            market_context: Some(format!("Market sentiment: {}", forecast.sentiment)),
        };

        let schedule = CampaignSchedule {
            campaign_id,
            campaign_type: CampaignType::SocialCampaign,
            scheduled_start: timestamp_now(),
            status: CampaignStatus::Scheduled,
            trigger_forecast: Some(ForecastTrigger {
                cycle: forecast.cycle.clone(),
                confidence: (forecast.cycle_confidence * 100.0) as u8,
                volatility: forecast.volatility.clone(),
                sentiment: (forecast.sentiment * 100.0) as i8,
            }),
            config,
            results: None,
        };

        let mut schedules = self.schedules.write().await;
        schedules.push(schedule);

        let mut last = self.last_campaign.write().await;
        last.insert(CampaignType::SocialCampaign, timestamp_now());
    }

    /// Generate a unique campaign ID
    async fn generate_campaign_id(&self) -> String {
        let mut counter = self.campaign_counter.write().await;
        *counter += 1;
        format!("camp-{:06}", *counter)
    }

    /// Manually schedule a campaign
    pub async fn schedule_campaign(&self, config: FundingCampaignConfig) -> String {
        let campaign_id = config.campaign_id.clone();

        let schedule = CampaignSchedule {
            campaign_id: campaign_id.clone(),
            campaign_type: config.campaign_type,
            scheduled_start: timestamp_now(),
            status: CampaignStatus::Scheduled,
            trigger_forecast: None,
            config,
            results: None,
        };

        let mut schedules = self.schedules.write().await;
        schedules.push(schedule);

        campaign_id
    }

    /// Start a scheduled campaign
    pub async fn start_campaign(&self, campaign_id: &str) -> bool {
        let mut schedules = self.schedules.write().await;

        if let Some(schedule) = schedules.iter_mut().find(|s| s.campaign_id == campaign_id) {
            if schedule.status == CampaignStatus::Scheduled {
                schedule.status = CampaignStatus::Running;
                return true;
            }
        }

        false
    }

    /// Execute a running campaign
    pub async fn execute_campaign(&self, campaign_id: &str) -> Option<CampaignResults> {
        // Get campaign config
        let config = {
            let schedules = self.schedules.read().await;
            schedules
                .iter()
                .find(|s| s.campaign_id == campaign_id && s.status == CampaignStatus::Running)
                .map(|s| s.config.clone())
        }?;

        let start = Instant::now();
        let mut messages_sent = 0;

        // Execute based on campaign type
        match config.campaign_type {
            CampaignType::SocialCampaign | CampaignType::CommunityGrowth => {
                // Generate and dispatch social content
                let mut novaflux = self.novaflux.write().await;
                
                for i in 0..config.max_outreach {
                    let script = novaflux.generate_short(None, super::novaflux::ContentTone::Confident);
                    
                    let payload = self.webhook_bridge.build_social_payload(
                        &config.campaign_id,
                        &script.id,
                        &script.hook,
                        &script.script,
                        &script.cta,
                        script.captions.iter().map(|c| (c.start_ms, c.end_ms, c.text.clone())).collect(),
                    );
                    
                    let _ = self.webhook_bridge.dispatch(payload).await;
                    messages_sent += 1;
                }
            }
            _ => {
                // VC/Grant/Partnership outreach
                for (i, target) in config.targets.iter().take(config.max_outreach).enumerate() {
                    let subject = format!("Demo request - {}", config.project_name);
                    let body = format!(
                        "Hi {},\n\n{}\n\nBest,\n{} team",
                        target.name.split_whitespace().next().unwrap_or("there"),
                        config.project_description,
                        config.project_name
                    );
                    
                    let payload = self.webhook_bridge.build_funding_payload(
                        &config.campaign_id,
                        &target.id,
                        &target.name,
                        target.email.as_deref(),
                        &subject,
                        &body,
                        &format!("v{}", i % 2),
                    );
                    
                    let _ = self.webhook_bridge.dispatch(payload).await;
                    messages_sent += 1;
                }
            }
        }

        let results = CampaignResults {
            targets_reached: config.targets.len().min(config.max_outreach),
            messages_sent,
            responses_received: 0, // Would track async
            positive_responses: 0,
            conversion_rate: 0.0,
            duration_secs: start.elapsed().as_secs(),
        };

        // Update schedule with results
        {
            let mut schedules = self.schedules.write().await;
            if let Some(schedule) = schedules.iter_mut().find(|s| s.campaign_id == campaign_id) {
                schedule.status = CampaignStatus::Completed;
                schedule.results = Some(results.clone());
            }
        }

        Some(results)
    }

    /// Cancel a campaign
    pub async fn cancel_campaign(&self, campaign_id: &str) -> bool {
        let mut schedules = self.schedules.write().await;

        if let Some(schedule) = schedules.iter_mut().find(|s| s.campaign_id == campaign_id) {
            if matches!(schedule.status, CampaignStatus::Scheduled | CampaignStatus::Running) {
                schedule.status = CampaignStatus::Cancelled;
                return true;
            }
        }

        false
    }

    /// Get campaign by ID
    pub async fn get_campaign(&self, campaign_id: &str) -> Option<CampaignSchedule> {
        let schedules = self.schedules.read().await;
        schedules.iter().find(|s| s.campaign_id == campaign_id).cloned()
    }

    /// Get all campaigns
    pub async fn get_all_campaigns(&self) -> Vec<CampaignSchedule> {
        self.schedules.read().await.clone()
    }

    /// Get campaigns by status
    pub async fn get_campaigns_by_status(&self, status: CampaignStatus) -> Vec<CampaignSchedule> {
        let schedules = self.schedules.read().await;
        schedules.iter().filter(|s| s.status == status).cloned().collect()
    }

    /// Get statistics
    pub async fn get_stats(&self) -> OrchestratorStats {
        let schedules = self.schedules.read().await;
        let forecast = self.current_forecast.read().await;

        let mut by_status: HashMap<String, usize> = HashMap::new();
        let mut by_type: HashMap<String, usize> = HashMap::new();

        for s in schedules.iter() {
            *by_status.entry(format!("{:?}", s.status)).or_default() += 1;
            *by_type.entry(format!("{:?}", s.campaign_type)).or_default() += 1;
        }

        let total_messages_sent: usize = schedules
            .iter()
            .filter_map(|s| s.results.as_ref())
            .map(|r| r.messages_sent)
            .sum();

        OrchestratorStats {
            total_campaigns: schedules.len(),
            by_status,
            by_type,
            total_messages_sent,
            prophet_enabled: self.config.prophet_enabled,
            current_market_cycle: forecast.as_ref().map(|f| format!("{:?}", f.cycle)),
            uptime_secs: self.started_at.elapsed().as_secs(),
        }
    }
}

/// Orchestrator statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorStats {
    pub total_campaigns: usize,
    pub by_status: HashMap<String, usize>,
    pub by_type: HashMap<String, usize>,
    pub total_messages_sent: usize,
    pub prophet_enabled: bool,
    pub current_market_cycle: Option<String>,
    pub uptime_secs: u64,
}

/// Get current timestamp
fn timestamp_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::funding::novaflux::NovaFluxConfig;
    use crate::funding::webhook::WebhookConfig;

    fn test_orchestrator() -> CampaignOrchestrator {
        let novaflux = NovaFlux::new(NovaFluxConfig::default());
        let webhook = WebhookBridge::new(WebhookConfig::default());
        CampaignOrchestrator::new(OrchestratorConfig::default(), novaflux, webhook)
    }

    #[tokio::test]
    async fn test_orchestrator_creation() {
        let orch = test_orchestrator();
        let stats = orch.get_stats().await;

        assert_eq!(stats.total_campaigns, 0);
        assert!(stats.prophet_enabled);
    }

    #[tokio::test]
    async fn test_manual_schedule() {
        let orch = test_orchestrator();

        let config = FundingCampaignConfig {
            campaign_type: CampaignType::VcOutreach,
            campaign_id: "test-001".to_string(),
            project_name: "Test Project".to_string(),
            project_description: "Test".to_string(),
            value_props: vec![],
            targets: vec![],
            max_outreach: 10,
            webhook_url: None,
            llm_engine: crate::jobs::funding_campaign::LlmEngine::Local,
            personalization_level: crate::jobs::funding_campaign::PersonalizationLevel::None,
            test_variants: 1,
            prophet_timed: false,
            market_context: None,
        };

        let id = orch.schedule_campaign(config).await;
        assert_eq!(id, "test-001");

        let campaign = orch.get_campaign(&id).await;
        assert!(campaign.is_some());
        assert_eq!(campaign.unwrap().status, CampaignStatus::Scheduled);
    }

    #[tokio::test]
    async fn test_campaign_lifecycle() {
        let orch = test_orchestrator();

        let config = FundingCampaignConfig {
            campaign_type: CampaignType::SocialCampaign,
            campaign_id: "social-001".to_string(),
            project_name: "Atlas Sphere".to_string(),
            project_description: "Dual-VM blockchain".to_string(),
            value_props: vec![],
            targets: vec![],
            max_outreach: 3,
            webhook_url: None,
            llm_engine: crate::jobs::funding_campaign::LlmEngine::Local,
            personalization_level: crate::jobs::funding_campaign::PersonalizationLevel::None,
            test_variants: 1,
            prophet_timed: false,
            market_context: None,
        };

        let id = orch.schedule_campaign(config).await;

        // Start campaign
        assert!(orch.start_campaign(&id).await);

        let campaign = orch.get_campaign(&id).await.unwrap();
        assert_eq!(campaign.status, CampaignStatus::Running);

        // Execute campaign
        let results = orch.execute_campaign(&id).await;
        assert!(results.is_some());
        assert_eq!(results.unwrap().messages_sent, 3);

        // Verify completed
        let campaign = orch.get_campaign(&id).await.unwrap();
        assert_eq!(campaign.status, CampaignStatus::Completed);
    }

    #[tokio::test]
    async fn test_cancel_campaign() {
        let orch = test_orchestrator();

        let config = FundingCampaignConfig {
            campaign_type: CampaignType::VcOutreach,
            campaign_id: "cancel-001".to_string(),
            project_name: "Test".to_string(),
            project_description: "Test".to_string(),
            value_props: vec![],
            targets: vec![],
            max_outreach: 10,
            webhook_url: None,
            llm_engine: crate::jobs::funding_campaign::LlmEngine::Local,
            personalization_level: crate::jobs::funding_campaign::PersonalizationLevel::None,
            test_variants: 1,
            prophet_timed: false,
            market_context: None,
        };

        let id = orch.schedule_campaign(config).await;
        assert!(orch.cancel_campaign(&id).await);

        let campaign = orch.get_campaign(&id).await.unwrap();
        assert_eq!(campaign.status, CampaignStatus::Cancelled);
    }

    #[tokio::test]
    async fn test_prophet_trigger() {
        let mut config = OrchestratorConfig::default();
        config.type_cooldown_secs = 0; // Disable cooldown for test

        let novaflux = NovaFlux::new(NovaFluxConfig::default());
        let webhook = WebhookBridge::new(WebhookConfig::default());
        let orch = CampaignOrchestrator::new(config, novaflux, webhook);

        // Create a forecast that should trigger campaigns
        let forecast = MarketForecast {
            generated_at: timestamp_now(),
            valid_for: std::time::Duration::from_secs(3600),
            cycle: MarketCycle::Bull,
            cycle_confidence: 0.80, // 80%
            next_cycle: None,
            volatility: VolatilityRegime::Normal,
            volatility_trend: crate::crown::prophet::DemandTrend::Stable,
            threat_forecasts: vec![],
            opportunities: vec![],
            lane_demands: HashMap::new(),
            allocation_hints: HashMap::new(),
            sentiment: 0.50, // 50%
            insights: vec![],
        };

        orch.update_forecast(forecast).await;

        // Should have auto-scheduled campaigns
        let stats = orch.get_stats().await;
        assert!(stats.total_campaigns >= 1);
    }

    #[tokio::test]
    async fn test_volatility_pause() {
        let config = OrchestratorConfig::default();
        let novaflux = NovaFlux::new(NovaFluxConfig::default());
        let webhook = WebhookBridge::new(WebhookConfig::default());
        let orch = CampaignOrchestrator::new(config, novaflux, webhook);

        // Create a high volatility forecast
        let forecast = MarketForecast {
            generated_at: timestamp_now(),
            valid_for: std::time::Duration::from_secs(3600),
            cycle: MarketCycle::Bull,
            cycle_confidence: 0.90, // 90%
            next_cycle: None,
            volatility: VolatilityRegime::Extreme, // Should pause
            volatility_trend: crate::crown::prophet::DemandTrend::Stable,
            threat_forecasts: vec![],
            opportunities: vec![],
            lane_demands: HashMap::new(),
            allocation_hints: HashMap::new(),
            sentiment: 0.50,
            insights: vec![],
        };

        orch.update_forecast(forecast).await;

        // Should NOT have auto-scheduled due to extreme volatility
        let stats = orch.get_stats().await;
        assert_eq!(stats.total_campaigns, 0);
    }
}
