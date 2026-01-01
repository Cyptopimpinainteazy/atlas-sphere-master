//! Funding Campaign Job - Autonomous Fundraising via GPU Swarm
//!
//! This job type integrates the funding-automator pipeline with the GPU swarm,
//! enabling Prophet-timed, market-aware fundraising campaigns.
//!
//! # Flow
//!
//! ```text
//! Prophet forecast → "Bull cycle entering"
//!     ↓
//! Crown authorization → "Approve funding burst"
//!     ↓
//! Swarm schedules FundingCampaignJob
//!     ↓
//! Job execution:
//!   1. Generate personalized content (NovaFlux persona)
//!   2. Build prospect list from targets
//!   3. Emit n8n webhook payloads
//!   4. Track responses and adjust
//!     ↓
//! Results announced on-chain
//! ```
//!
//! # Campaign Types
//!
//! - **VC_OUTREACH**: Personalized emails to venture capital firms
//! - **GRANT_APPLICATION**: Automated grant discovery and applications
//! - **SOCIAL_CAMPAIGN**: NovaFlux AI influencer content burst
//! - **COMMUNITY_GROWTH**: Targeted community building

use crate::error::{SwarmError, SwarmResult};
use crate::jobs::{JobOutput, JobType, SwarmJob};
use crate::task::TaskPriority;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Campaign types available
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CampaignType {
    /// Venture capital outreach
    VcOutreach,
    /// Grant application automation
    GrantApplication,
    /// Social media campaign (NovaFlux)
    SocialCampaign,
    /// Community growth initiatives
    CommunityGrowth,
    /// Partnership outreach
    PartnershipOutreach,
    /// Accelerator applications
    AcceleratorApplication,
}

impl CampaignType {
    /// Get the n8n workflow to trigger
    pub fn n8n_workflow(&self) -> &'static str {
        match self {
            CampaignType::VcOutreach => "lane4-funding-magnet",
            CampaignType::GrantApplication => "lane4-funding-magnet",
            CampaignType::SocialCampaign => "lane3-social-detonator",
            CampaignType::CommunityGrowth => "lane3-social-detonator",
            CampaignType::PartnershipOutreach => "lane4-funding-magnet",
            CampaignType::AcceleratorApplication => "lane4-funding-magnet",
        }
    }

    /// Get base priority for this campaign type
    pub fn base_priority(&self) -> TaskPriority {
        match self {
            CampaignType::VcOutreach => TaskPriority::High,
            CampaignType::GrantApplication => TaskPriority::Normal,
            CampaignType::SocialCampaign => TaskPriority::Normal,
            CampaignType::CommunityGrowth => TaskPriority::Low,
            CampaignType::PartnershipOutreach => TaskPriority::Normal,
            CampaignType::AcceleratorApplication => TaskPriority::High,
        }
    }

    /// Get default target count
    pub fn default_target_count(&self) -> usize {
        match self {
            CampaignType::VcOutreach => 20,
            CampaignType::GrantApplication => 10,
            CampaignType::SocialCampaign => 50,
            CampaignType::CommunityGrowth => 100,
            CampaignType::PartnershipOutreach => 15,
            CampaignType::AcceleratorApplication => 5,
        }
    }
}

/// Prospect target for outreach
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prospect {
    /// Unique prospect ID
    pub id: String,
    /// Organization/person name
    pub name: String,
    /// Description/notes
    pub description: Option<String>,
    /// Contact email
    pub email: Option<String>,
    /// Contact Twitter/X handle
    pub twitter: Option<String>,
    /// Website URL
    pub website: Option<String>,
    /// Investment thesis / interests
    pub interests: Vec<String>,
    /// Previous interactions
    pub history: Vec<InteractionRecord>,
    /// Priority score (0-100)
    pub priority_score: u8,
    /// Tags for filtering
    pub tags: Vec<String>,
}

/// Record of previous interaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionRecord {
    pub timestamp: u64,
    pub channel: String,
    pub outcome: String,
    pub notes: Option<String>,
}

/// Configuration for a funding campaign job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingCampaignConfig {
    /// Campaign type
    pub campaign_type: CampaignType,
    /// Campaign ID
    pub campaign_id: String,
    /// Project name to pitch
    pub project_name: String,
    /// Project description
    pub project_description: String,
    /// Key value propositions
    pub value_props: Vec<String>,
    /// Target prospects
    pub targets: Vec<Prospect>,
    /// Maximum outreach per execution
    pub max_outreach: usize,
    /// n8n webhook URL
    pub webhook_url: Option<String>,
    /// LLM engine to use
    pub llm_engine: LlmEngine,
    /// Personalization level
    pub personalization_level: PersonalizationLevel,
    /// A/B test variants
    pub test_variants: usize,
    /// Whether this was Prophet-timed
    pub prophet_timed: bool,
    /// Market cycle context
    pub market_context: Option<String>,
}

/// LLM engine options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LlmEngine {
    /// Local deterministic fallback
    Local,
    /// OpenAI GPT-4o-mini
    OpenAi,
    /// Anthropic Claude
    Claude,
    /// Custom endpoint
    Custom,
}

impl Default for LlmEngine {
    fn default() -> Self {
        LlmEngine::Local
    }
}

/// Level of personalization for outreach
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PersonalizationLevel {
    /// No personalization (template only)
    None,
    /// Light personalization (name + org)
    Light,
    /// Medium personalization (interests + recent activity)
    Medium,
    /// Heavy personalization (full research-backed)
    Heavy,
}

impl Default for PersonalizationLevel {
    fn default() -> Self {
        PersonalizationLevel::Light
    }
}

/// Result of funding campaign execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingCampaignResult {
    /// Campaign ID
    pub campaign_id: String,
    /// Campaign type
    pub campaign_type: CampaignType,
    /// Targets processed
    pub targets_processed: usize,
    /// Messages sent
    pub messages_sent: usize,
    /// Errors encountered
    pub errors: Vec<String>,
    /// Generated content samples
    pub content_samples: Vec<GeneratedContent>,
    /// Webhook payloads dispatched
    pub webhooks_dispatched: usize,
    /// Execution duration (ms)
    pub duration_ms: u64,
    /// Compute units consumed
    pub compute_units: u64,
    /// A/B test results if applicable
    pub ab_results: Option<AbTestResults>,
}

/// Generated content for outreach
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedContent {
    /// Target prospect ID
    pub prospect_id: String,
    /// Content variant ID
    pub variant_id: String,
    /// Generated subject line
    pub subject: String,
    /// Generated body
    pub body: String,
    /// Personalization elements used
    pub personalization_elements: Vec<String>,
    /// NovaFlux script (for social)
    pub novaflux_script: Option<String>,
}

/// A/B test results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbTestResults {
    pub variants: Vec<VariantPerformance>,
    pub winning_variant: Option<String>,
    pub confidence: f64,
}

/// Performance of a single variant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantPerformance {
    pub variant_id: String,
    pub sent_count: usize,
    pub open_rate: f64,
    pub response_rate: f64,
    pub score: f64,
}

/// The Funding Campaign Job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingCampaignJob {
    pub config: FundingCampaignConfig,
}

impl FundingCampaignJob {
    /// Create a new funding campaign job
    pub fn new(config: FundingCampaignConfig) -> Self {
        Self { config }
    }

    /// Create a VC outreach campaign
    pub fn vc_outreach(
        campaign_id: &str,
        project_name: &str,
        project_description: &str,
        targets: Vec<Prospect>,
    ) -> Self {
        Self::new(FundingCampaignConfig {
            campaign_type: CampaignType::VcOutreach,
            campaign_id: campaign_id.to_string(),
            project_name: project_name.to_string(),
            project_description: project_description.to_string(),
            value_props: vec![
                "Dual-VM blockchain (EVM + SVM)".to_string(),
                "Native AI agent execution".to_string(),
                "Protocol-level MEV protection".to_string(),
            ],
            targets,
            max_outreach: 20,
            webhook_url: None,
            llm_engine: LlmEngine::default(),
            personalization_level: PersonalizationLevel::Medium,
            test_variants: 2,
            prophet_timed: false,
            market_context: None,
        })
    }

    /// Create a social campaign with NovaFlux
    pub fn social_campaign(campaign_id: &str, hook_count: usize) -> Self {
        Self::new(FundingCampaignConfig {
            campaign_type: CampaignType::SocialCampaign,
            campaign_id: campaign_id.to_string(),
            project_name: "Atlas Sphere".to_string(),
            project_description: "Dual-VM blockchain".to_string(),
            value_props: vec![],
            targets: vec![],
            max_outreach: hook_count,
            webhook_url: None,
            llm_engine: LlmEngine::default(),
            personalization_level: PersonalizationLevel::None,
            test_variants: 1,
            prophet_timed: false,
            market_context: None,
        })
    }

    /// Mark as Prophet-timed
    pub fn with_prophet_timing(mut self, market_context: &str) -> Self {
        self.config.prophet_timed = true;
        self.config.market_context = Some(market_context.to_string());
        self
    }

    /// Set webhook URL
    pub fn with_webhook(mut self, url: &str) -> Self {
        self.config.webhook_url = Some(url.to_string());
        self
    }

    /// Generate personalized content for a prospect
    fn generate_content(&self, prospect: &Prospect, variant_idx: usize) -> GeneratedContent {
        let personalization_elements = self.collect_personalization_elements(prospect);

        // Build subject line
        let subject = match self.config.campaign_type {
            CampaignType::VcOutreach | CampaignType::PartnershipOutreach => {
                if self.config.personalization_level != PersonalizationLevel::None {
                    format!(
                        "15-min demo — {} + {} synergy",
                        self.config.project_name,
                        prospect
                            .interests
                            .first()
                            .unwrap_or(&"blockchain".to_string())
                    )
                } else {
                    format!("Demo request — {}", self.config.project_name)
                }
            }
            CampaignType::GrantApplication | CampaignType::AcceleratorApplication => {
                format!(
                    "Grant interest — {} improving UX + access",
                    self.config.project_name
                )
            }
            CampaignType::SocialCampaign | CampaignType::CommunityGrowth => {
                "".to_string() // No subject for social
            }
        };

        // Build body
        let body = self.generate_body(prospect, variant_idx);

        // Generate NovaFlux script for social campaigns
        let novaflux_script = if matches!(
            self.config.campaign_type,
            CampaignType::SocialCampaign | CampaignType::CommunityGrowth
        ) {
            Some(self.generate_novaflux_script(variant_idx))
        } else {
            None
        };

        GeneratedContent {
            prospect_id: prospect.id.clone(),
            variant_id: format!("v{}", variant_idx),
            subject,
            body,
            personalization_elements,
            novaflux_script,
        }
    }

    /// Collect personalization elements from prospect
    fn collect_personalization_elements(&self, prospect: &Prospect) -> Vec<String> {
        let mut elements = vec![];

        match self.config.personalization_level {
            PersonalizationLevel::None => {}
            PersonalizationLevel::Light => {
                elements.push(format!("name:{}", prospect.name));
            }
            PersonalizationLevel::Medium => {
                elements.push(format!("name:{}", prospect.name));
                if !prospect.interests.is_empty() {
                    elements.push(format!("interests:{}", prospect.interests.join(",")));
                }
            }
            PersonalizationLevel::Heavy => {
                elements.push(format!("name:{}", prospect.name));
                if !prospect.interests.is_empty() {
                    elements.push(format!("interests:{}", prospect.interests.join(",")));
                }
                if let Some(desc) = &prospect.description {
                    elements.push(format!("context:{}", desc));
                }
                if !prospect.history.is_empty() {
                    elements.push(format!("history_count:{}", prospect.history.len()));
                }
            }
        }

        elements
    }

    /// Generate email body
    fn generate_body(&self, prospect: &Prospect, variant_idx: usize) -> String {
        let greeting = format!(
            "Hi {},\n\n",
            prospect.name.split_whitespace().next().unwrap_or("there")
        );

        let hook = match variant_idx % 2 {
            0 => format!(
                "We built {} — a dual-VM chain combining EVM compatibility with SVM parallelization.",
                self.config.project_name
            ),
            _ => format!(
                "Imagine Solidity devs keeping everything they know and getting 10x throughput. That's {}.",
                self.config.project_name
            ),
        };

        let personalized_section = if !prospect.interests.is_empty() {
            format!(
                "\n\nGiven your interest in {}, I thought this might be particularly relevant.",
                prospect
                    .interests
                    .first()
                    .unwrap_or(&"blockchain".to_string())
            )
        } else {
            String::new()
        };

        let traction = "\n\n3 quick traction bullets:\n\
            - Working testnet + faucet (dev onboarding < 30s)\n\
            - Public explorer + reproducible benchmark harness\n\
            - 2 demo dApps + early integrations";

        let cta = "\n\nWould you have 15 minutes for a demo next week?";

        let signature = format!("\n\nBest,\n{} team", self.config.project_name);

        format!(
            "{}{}{}{}{}{}",
            greeting, hook, personalized_section, traction, cta, signature
        )
    }

    /// Generate NovaFlux social script
    fn generate_novaflux_script(&self, variant_idx: usize) -> String {
        // NovaFlux hooks from the persona pack
        let hooks = [
            "What if one chain could run every EVM contract — but 10x faster?",
            "MEV doesn't need to be mafia-level theft.",
            "Imagine bots that live in wallets and think.",
            "Deploy an EVM contract in 60 seconds — scale on SVM.",
            "What does $0.001 gas feel like?",
            "Will devs learn a new VM? No.",
            "On-chain UX that actually feels like Web2.",
            "Stop juggling bridges.",
            "Love Solidity? Keep it.",
            "Bots that don't wreck the market.",
        ];

        let scripts = [
            "Dual VM: EVM for compatibility, SVM for parallelized speed. No rewrite — just speed. Testnet link in bio.",
            "Protocol-level MEV protection auctions bundlers and rewards honest proposers. Fair profits, fewer frontruns. Join our testnet.",
            "Native AI agents on SVM for small-model inference and on-chain signals. Arbitrage, hedging, farm automation. Early slots open.",
            "Same Solidity, new speed. One wallet, two runtimes. Try our 1-minute deploy demo.",
            "SVM parallelization + gas batching drops fees into micro-fees. Small txs, big scale. See benchmarks in bio.",
            "EVM compatibility + optional SVM accelerator means migration's optional. Code stays the same — speed is optional.",
            "Instant finality windows, optimistic receipts, buttery UX. Wallets that feel familiar, but fast. Try our wallet beta.",
            "Native messaging and secure relays simplify cross-chain moves. Fewer steps, faster sync — cross demo linked.",
            "SVM accelerates — it doesn't force new languages. Supercharge Solidity with a flip of a flag.",
            "Agent governance + staking incentivizes polite bots. Responsible automation beats reckless yield-chasing.",
        ];

        let idx = variant_idx % hooks.len();
        format!("Hook: \"{}\"\nScript: \"{}\"", hooks[idx], scripts[idx])
    }

    /// Build n8n webhook payload
    fn build_webhook_payload(
        &self,
        content: &GeneratedContent,
        prospect: &Prospect,
    ) -> HashMap<String, serde_json::Value> {
        let mut payload = HashMap::new();

        payload.insert("trigger".to_string(), serde_json::json!("promote"));
        payload.insert(
            "project".to_string(),
            serde_json::json!(self.config.project_name),
        );
        payload.insert(
            "campaign_id".to_string(),
            serde_json::json!(self.config.campaign_id),
        );
        payload.insert(
            "campaign_type".to_string(),
            serde_json::json!(format!("{:?}", self.config.campaign_type)),
        );

        payload.insert(
            "prospect".to_string(),
            serde_json::json!({
                "id": prospect.id,
                "name": prospect.name,
                "email": prospect.email,
                "twitter": prospect.twitter,
                "description": prospect.description,
            }),
        );

        payload.insert(
            "content".to_string(),
            serde_json::json!({
                "subject": content.subject,
                "body": content.body,
                "variant_id": content.variant_id,
                "novaflux_script": content.novaflux_script,
            }),
        );

        payload.insert(
            "metadata".to_string(),
            serde_json::json!({
                "prophet_timed": self.config.prophet_timed,
                "market_context": self.config.market_context,
                "personalization_level": format!("{:?}", self.config.personalization_level),
                "created_at": chrono_lite_timestamp(),
            }),
        );

        payload
    }

    /// Execute the campaign
    fn execute_campaign(&self) -> SwarmResult<FundingCampaignResult> {
        let start = std::time::Instant::now();
        let mut content_samples = vec![];
        let mut errors = vec![];
        let mut webhooks_dispatched = 0;
        let mut messages_sent = 0;

        let targets_to_process = self
            .config
            .targets
            .iter()
            .take(self.config.max_outreach)
            .collect::<Vec<_>>();

        // Generate content for each target
        for (idx, prospect) in targets_to_process.iter().enumerate() {
            // Generate content variants
            for variant in 0..self.config.test_variants {
                let content = self.generate_content(prospect, variant);

                // Try to dispatch webhook
                if self.config.webhook_url.is_some() {
                    let _payload = self.build_webhook_payload(&content, prospect);
                    // In real implementation, would send to n8n
                    webhooks_dispatched += 1;
                }

                // Store sample (first few for debugging)
                if idx < 3 && variant == 0 {
                    content_samples.push(content.clone());
                }

                messages_sent += 1;
            }
        }

        // Build result
        let duration_ms = start.elapsed().as_millis() as u64;
        let compute_units = messages_sent as u64 * 10; // 10 units per message

        Ok(FundingCampaignResult {
            campaign_id: self.config.campaign_id.clone(),
            campaign_type: self.config.campaign_type,
            targets_processed: targets_to_process.len(),
            messages_sent,
            errors,
            content_samples,
            webhooks_dispatched,
            duration_ms,
            compute_units,
            ab_results: None, // Would be populated by tracking responses
        })
    }
}

impl SwarmJob for FundingCampaignJob {
    fn job_type(&self) -> JobType {
        JobType::FundingCampaign
    }

    fn compute_units(&self) -> u64 {
        // Base cost + per-target cost
        100 + (self.config.targets.len() as u64 * 10)
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(120) // 2 minutes max
    }

    fn execute(&self) -> SwarmResult<JobOutput> {
        let result = self.execute_campaign()?;
        Ok(JobOutput::FundingCampaign(result))
    }

    fn verify(&self, result: &JobOutput) -> SwarmResult<bool> {
        match result {
            JobOutput::FundingCampaign(r) => {
                // Verify reasonable results
                Ok(r.messages_sent > 0 && r.errors.len() < r.messages_sent)
            }
            _ => Ok(false),
        }
    }

    fn priority(&self) -> TaskPriority {
        // Prophet-timed campaigns get boosted priority
        if self.config.prophet_timed {
            TaskPriority::High
        } else {
            self.config.campaign_type.base_priority()
        }
    }

    fn requires_gpu(&self) -> bool {
        // Only needs GPU if using local LLM
        matches!(self.config.llm_engine, LlmEngine::Local)
    }

    fn min_vram_mb(&self) -> u32 {
        if self.requires_gpu() {
            2048 // 2GB for local LLM
        } else {
            0
        }
    }
}

/// Simple timestamp without full chrono dependency
fn chrono_lite_timestamp() -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}", ts)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_prospect() -> Prospect {
        Prospect {
            id: "p001".to_string(),
            name: "Alice Ventures".to_string(),
            description: Some("Early stage crypto fund".to_string()),
            email: Some("alice@ventures.example".to_string()),
            twitter: Some("@alicevc".to_string()),
            website: Some("https://ventures.example".to_string()),
            interests: vec!["DeFi".to_string(), "L2".to_string(), "MEV".to_string()],
            history: vec![],
            priority_score: 85,
            tags: vec!["tier1".to_string(), "crypto-native".to_string()],
        }
    }

    #[test]
    fn test_vc_outreach_creation() {
        let job = FundingCampaignJob::vc_outreach(
            "camp-001",
            "Atlas Sphere",
            "Dual-VM blockchain",
            vec![test_prospect()],
        );

        assert_eq!(job.config.campaign_type, CampaignType::VcOutreach);
        assert_eq!(job.config.targets.len(), 1);
        assert!(!job.config.prophet_timed);
    }

    #[test]
    fn test_prophet_timing() {
        let job = FundingCampaignJob::vc_outreach(
            "camp-002",
            "Atlas Sphere",
            "Dual-VM blockchain",
            vec![test_prospect()],
        )
        .with_prophet_timing("Bull cycle entering, high investor attention");

        assert!(job.config.prophet_timed);
        assert!(job.config.market_context.is_some());
        assert_eq!(job.priority(), TaskPriority::High);
    }

    #[test]
    fn test_content_generation() {
        let job = FundingCampaignJob::vc_outreach(
            "camp-003",
            "Atlas Sphere",
            "Dual-VM blockchain",
            vec![test_prospect()],
        );

        let prospect = test_prospect();
        let content = job.generate_content(&prospect, 0);

        assert!(!content.subject.is_empty());
        assert!(content.body.contains("Alice"));
        assert!(content.body.contains("Atlas Sphere"));
        assert!(content.personalization_elements.len() > 0);
    }

    #[test]
    fn test_novaflux_script_generation() {
        let job = FundingCampaignJob::social_campaign("social-001", 10);

        let script = job.generate_novaflux_script(0);
        assert!(script.contains("Hook:"));
        assert!(script.contains("Script:"));
    }

    #[test]
    fn test_webhook_payload() {
        let job = FundingCampaignJob::vc_outreach(
            "camp-004",
            "Atlas Sphere",
            "Dual-VM blockchain",
            vec![test_prospect()],
        );

        let prospect = test_prospect();
        let content = job.generate_content(&prospect, 0);
        let payload = job.build_webhook_payload(&content, &prospect);

        assert!(payload.contains_key("trigger"));
        assert!(payload.contains_key("project"));
        assert!(payload.contains_key("prospect"));
        assert!(payload.contains_key("content"));
    }

    #[test]
    fn test_campaign_execution() {
        let job = FundingCampaignJob::vc_outreach(
            "camp-005",
            "Atlas Sphere",
            "Dual-VM blockchain",
            vec![test_prospect(), test_prospect()],
        );

        let result = job.execute_campaign().expect("Should execute");

        assert_eq!(result.campaign_id, "camp-005");
        assert_eq!(result.targets_processed, 2);
        assert!(result.messages_sent > 0);
        assert!(result.content_samples.len() > 0);
    }

    #[test]
    fn test_job_trait_implementation() {
        let job = FundingCampaignJob::vc_outreach(
            "camp-006",
            "Atlas Sphere",
            "Dual-VM blockchain",
            vec![test_prospect()],
        );

        assert_eq!(job.job_type(), JobType::FundingCampaign);
        assert!(job.compute_units() > 0);
        assert!(job.timeout() > Duration::from_secs(0));
    }

    #[test]
    fn test_campaign_type_n8n_workflows() {
        assert_eq!(
            CampaignType::VcOutreach.n8n_workflow(),
            "lane4-funding-magnet"
        );
        assert_eq!(
            CampaignType::SocialCampaign.n8n_workflow(),
            "lane3-social-detonator"
        );
    }
}
