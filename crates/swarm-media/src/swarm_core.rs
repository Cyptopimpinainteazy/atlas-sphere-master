// ============================================================================
// X3 ATLAS SPHERE - GLOBAL MARKETING & GROWTH SWARM
// Core Orchestration Engine
// ============================================================================
//
// This module implements the complete swarm orchestration system including:
// - Multi-region, multi-language content generation
// - Platform-specific publishing strategies
// - Real-time analytics and optimization
// - Ethical constraints and safety governance
// - Full audit trail and compliance
//
// CRITICAL RULES:
// - NO impersonation: All accounts are official brand or clearly labeled AI
// - NO fake humans: Generated content uses abstract art, not fake faces
// - NO undisclosed automation: All AI-assisted content is disclosed
// - NO spam: Rate limits enforced across all platforms
// - FULL AUDIT: Every action logged with cryptographic signatures

use crate::marketing_agents::{
    AgentMetrics, AgentTask, AgentTaskResult, AgentType, HealthStatus, MarketingAgent,
    MarketingPlatform, ResourceUsage,
};
use crate::marketing_governance::{
    CircuitBreaker, CircuitBreakerState, ComplianceCheck, GovernanceState, KillSwitchEvent,
    KillSwitchTarget, KillSwitchType, RateLimit, SystemStatus,
};
use async_trait::async_trait;
use chrono::{DateTime, Duration, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

// ============================================================================
// LANGUAGE & REGION DEFINITIONS
// ============================================================================

/// Supported languages for content localization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    // Tier 1: Primary languages
    English,
    Spanish,
    Portuguese,
    French,
    German,
    Japanese,
    ChineseSimplified,
    ChineseTraditional,
    Korean,
    Russian,
    Arabic,

    // Tier 2: Secondary languages
    Italian,
    Dutch,
    Polish,
    Turkish,
    Vietnamese,
    Thai,
    Indonesian,
    Hindi,

    // Tier 3: Emerging markets
    Swahili,
    Filipino,
    Malay,
    Bengali,
    Urdu,
    Persian,
    Hebrew,
    Greek,
    Czech,
    Swedish,
    Norwegian,
    Danish,
    Finnish,
    Romanian,
    Hungarian,
    Ukrainian,
}

impl Language {
    pub fn iso_code(&self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Spanish => "es",
            Self::Portuguese => "pt",
            Self::French => "fr",
            Self::German => "de",
            Self::Japanese => "ja",
            Self::ChineseSimplified => "zh-CN",
            Self::ChineseTraditional => "zh-TW",
            Self::Korean => "ko",
            Self::Russian => "ru",
            Self::Arabic => "ar",
            Self::Italian => "it",
            Self::Dutch => "nl",
            Self::Polish => "pl",
            Self::Turkish => "tr",
            Self::Vietnamese => "vi",
            Self::Thai => "th",
            Self::Indonesian => "id",
            Self::Hindi => "hi",
            Self::Swahili => "sw",
            Self::Filipino => "fil",
            Self::Malay => "ms",
            Self::Bengali => "bn",
            Self::Urdu => "ur",
            Self::Persian => "fa",
            Self::Hebrew => "he",
            Self::Greek => "el",
            Self::Czech => "cs",
            Self::Swedish => "sv",
            Self::Norwegian => "no",
            Self::Danish => "da",
            Self::Finnish => "fi",
            Self::Romanian => "ro",
            Self::Hungarian => "hu",
            Self::Ukrainian => "uk",
        }
    }

    pub fn tier(&self) -> u8 {
        match self {
            Self::English
            | Self::Spanish
            | Self::Portuguese
            | Self::French
            | Self::German
            | Self::Japanese
            | Self::ChineseSimplified
            | Self::ChineseTraditional
            | Self::Korean
            | Self::Russian
            | Self::Arabic => 1,

            Self::Italian
            | Self::Dutch
            | Self::Polish
            | Self::Turkish
            | Self::Vietnamese
            | Self::Thai
            | Self::Indonesian
            | Self::Hindi => 2,

            _ => 3,
        }
    }

    pub fn is_rtl(&self) -> bool {
        matches!(self, Self::Arabic | Self::Hebrew | Self::Persian | Self::Urdu)
    }
}

/// Geographic regions for targeting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Region {
    NorthAmerica,
    Europe,
    LatinAmerica,
    MiddleEast,
    Africa,
    SouthAsia,
    EastAsia,
    SoutheastAsia,
    Oceania,
    Global,
}

impl Region {
    pub fn primary_languages(&self) -> Vec<Language> {
        match self {
            Self::NorthAmerica => vec![Language::English, Language::Spanish],
            Self::Europe => vec![
                Language::English,
                Language::German,
                Language::French,
                Language::Spanish,
                Language::Italian,
            ],
            Self::LatinAmerica => vec![Language::Spanish, Language::Portuguese],
            Self::MiddleEast => vec![Language::Arabic, Language::Persian, Language::Turkish],
            Self::Africa => vec![Language::English, Language::French, Language::Swahili],
            Self::SouthAsia => vec![Language::Hindi, Language::English, Language::Bengali],
            Self::EastAsia => vec![
                Language::ChineseSimplified,
                Language::Japanese,
                Language::Korean,
            ],
            Self::SoutheastAsia => vec![
                Language::Indonesian,
                Language::Vietnamese,
                Language::Thai,
            ],
            Self::Oceania => vec![Language::English],
            Self::Global => vec![Language::English],
        }
    }

    pub fn peak_hours_utc(&self) -> (u32, u32) {
        // Returns (start_hour, end_hour) in UTC for peak engagement
        match self {
            Self::NorthAmerica => (13, 22),     // 9AM-6PM EST
            Self::Europe => (7, 19),            // 8AM-8PM CET
            Self::LatinAmerica => (14, 23),     // 10AM-7PM São Paulo
            Self::MiddleEast => (6, 18),        // 9AM-9PM Dubai
            Self::Africa => (6, 18),            // 9AM-9PM Johannesburg
            Self::SouthAsia => (3, 15),         // 8AM-8PM IST
            Self::EastAsia => (0, 12),          // 9AM-9PM JST/CST
            Self::SoutheastAsia => (1, 13),     // 8AM-8PM SGT
            Self::Oceania => (21, 9),           // 8AM-8PM Sydney (crosses midnight UTC)
            Self::Global => (0, 24),            // Always
        }
    }
}

// ============================================================================
// CONTENT TYPES & FORMATS
// ============================================================================

/// Types of content the swarm can generate
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContentType {
    // Short-form text
    Tweet,
    ThreadStart,
    ThreadContinuation,
    LinkedInPost,
    InstagramCaption,
    RedditPost,
    RedditComment,
    DiscordMessage,
    TelegramMessage,

    // Long-form text
    BlogPost,
    Newsletter,
    SubstackArticle,
    MediumArticle,
    TechnicalDoc,
    Whitepaper,
    CaseStudy,
    PressRelease,

    // Visual content
    StaticImage,
    Infographic,
    Diagram,
    Thumbnail,
    ProfileBanner,
    Meme, // Brand-safe, clearly labeled

    // Video content
    YouTubeVideo,
    YouTubeShort,
    TikTokVideo,
    InstagramReel,
    LinkedInVideo,

    // Audio content
    PodcastEpisode,
    VoiceOverScript,
    TwitterSpaceOutline,

    // Email content
    EmailNewsletter,
    OutreachEmail,
    FollowUpEmail,
    WelcomeSequence,

    // Interactive
    Poll,
    Quiz,
    AMAQuestions,
}

impl ContentType {
    pub fn supported_platforms(&self) -> Vec<MarketingPlatform> {
        match self {
            Self::Tweet | Self::ThreadStart | Self::ThreadContinuation => {
                vec![MarketingPlatform::Twitter]
            }
            Self::LinkedInPost | Self::LinkedInVideo => vec![MarketingPlatform::LinkedIn],
            Self::InstagramCaption | Self::InstagramReel => vec![MarketingPlatform::Instagram],
            Self::RedditPost | Self::RedditComment => vec![MarketingPlatform::Reddit],
            Self::DiscordMessage => vec![MarketingPlatform::Discord],
            Self::TelegramMessage => vec![MarketingPlatform::Telegram],
            Self::YouTubeVideo | Self::YouTubeShort => vec![MarketingPlatform::YouTube],
            Self::TikTokVideo => vec![MarketingPlatform::TikTok],
            Self::BlogPost | Self::TechnicalDoc => vec![MarketingPlatform::Medium],
            Self::Newsletter | Self::SubstackArticle => vec![MarketingPlatform::Substack],
            Self::EmailNewsletter | Self::OutreachEmail | Self::WelcomeSequence => {
                vec![MarketingPlatform::Email]
            }
            _ => vec![], // Cross-platform assets
        }
    }

    pub fn requires_approval(&self) -> bool {
        matches!(
            self,
            Self::PressRelease
                | Self::Whitepaper
                | Self::OutreachEmail
                | Self::Meme
                | Self::Poll
        )
    }

    pub fn character_limit(&self) -> Option<usize> {
        match self {
            Self::Tweet => Some(280),
            Self::ThreadStart | Self::ThreadContinuation => Some(280),
            Self::LinkedInPost => Some(3000),
            Self::InstagramCaption => Some(2200),
            Self::TikTokVideo => Some(150), // Caption
            Self::RedditPost => Some(40000),
            _ => None,
        }
    }
}

/// Tone variants for content generation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContentTone {
    Technical,        // For developers, detailed
    Professional,     // For investors, business-focused
    Casual,           // For community, friendly
    Educational,      // For learners, explaining concepts
    Visionary,        // Big picture, future-focused
    Urgent,           // Time-sensitive announcements
    Celebratory,      // Milestones, achievements
    QuestionAsking,   // Engagement-focused
}

// ============================================================================
// CONTENT IDEA & CAMPAIGN STRUCTURES
// ============================================================================

/// A content idea generated by strategy agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentIdea {
    pub idea_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub signal_source: SignalSource,
    pub content_type: ContentType,
    pub priority_score: f32,      // 0.0 - 1.0
    pub novelty_score: f32,       // 0.0 - 1.0
    pub alignment_score: f32,     // 0.0 - 1.0 (brand alignment)
    pub urgency: ContentUrgency,
    pub target_audience: TargetAudience,
    pub suggested_tones: Vec<ContentTone>,
    pub suggested_angles: Vec<ContentAngle>,
    pub regional_fit: HashMap<Region, f32>,
    pub estimated_effort_hours: f32,
    pub status: IdeaStatus,
    pub rejection_reason: Option<String>,
    pub approved_by: Option<String>,
    pub approved_at: Option<DateTime<Utc>>,
}

/// Source of a content idea signal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalSource {
    pub source_type: SignalType,
    pub reference_url: Option<String>,
    pub detected_at: DateTime<Utc>,
    pub trend_momentum: f32, // 0.0 - 1.0, how fast is this growing
    pub confidence: f32,     // 0.0 - 1.0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalType {
    TwitterTrend,
    RedditTrending,
    CommunityQuestion,
    CompetitorActivity,
    RoadmapEvent,
    PartnerAnnouncement,
    MarketNews,
    TechnicalMilestone,
    UserFeedback,
    ScheduledCampaign,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContentUrgency {
    Immediate,    // Post within 1 hour
    High,         // Post within 24 hours
    Medium,       // Post within 1 week
    Low,          // Evergreen, can schedule anytime
    Scheduled,    // Specific date/time
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetAudience {
    Developers,
    Investors,
    Community,
    Enterprises,
    Newcomers,
    TechnicalExperts,
    General,
}

/// A specific angle for approaching content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentAngle {
    pub angle_id: Uuid,
    pub angle_type: AngleType,
    pub hook: String,
    pub body_outline: String,
    pub cta_suggestion: Option<String>,
    pub engagement_prediction: f32, // 0.0 - 1.0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AngleType {
    Emotional,      // Fear, excitement, curiosity
    Rational,       // Data-driven, logical
    Educational,    // Teaching, explaining
    Controversial,  // Safe debate angle
    DeveloperFirst, // Technical deep-dive
    InvestorGrade,  // Business opportunity
    StoryTelling,   // Narrative arc
    Comparison,     // Us vs. alternatives
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IdeaStatus {
    New,
    UnderReview,
    Approved,
    Rejected,
    InProduction,
    Published,
    Archived,
}

// ============================================================================
// CONTENT ASSET STRUCTURES
// ============================================================================

/// A generated content asset ready for publishing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentAsset {
    pub asset_id: Uuid,
    pub idea_id: Uuid,
    pub campaign_id: Option<Uuid>,
    pub content_type: ContentType,
    pub platform: MarketingPlatform,
    pub language: Language,
    pub region: Option<Region>,
    pub tone: ContentTone,

    // Content
    pub title: Option<String>,
    pub body: String,
    pub media_urls: Vec<MediaAsset>,
    pub hashtags: Vec<String>,
    pub mentions: Vec<String>,
    pub cta: Option<CallToAction>,

    // Metadata
    pub word_count: usize,
    pub estimated_read_time_seconds: u32,
    pub quality_score: f32,
    pub brand_alignment_score: f32,

    // Compliance
    pub disclosure: DisclosureInfo,
    pub compliance_status: ComplianceStatus,

    // Lineage
    pub generated_by: String,
    pub generated_at: DateTime<Utc>,
    pub model_used: String,
    pub generation_params: serde_json::Value,

    // Status
    pub status: AssetStatus,
    pub scheduled_for: Option<DateTime<Utc>>,
    pub published_at: Option<DateTime<Utc>>,
    pub platform_post_id: Option<String>,
}

/// Media asset (image, video, audio)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaAsset {
    pub media_id: Uuid,
    pub media_type: MediaType,
    pub url: String,
    pub cdn_url: Option<String>,
    pub file_hash: String, // SHA-256
    pub dimensions: Option<Dimensions>,
    pub duration_seconds: Option<u32>,
    pub file_size_bytes: u64,
    pub alt_text: String, // Accessibility
    pub generated_by: String,
    pub prompt_used: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MediaType {
    Image,
    Video,
    Audio,
    Gif,
    Document,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
    pub aspect_ratio: String, // e.g., "16:9", "1:1", "9:16"
}

/// Call to action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallToAction {
    pub cta_type: CtaType,
    pub text: String,
    pub url: Option<String>,
    pub tracking_params: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CtaType {
    LearnMore,
    TryNow,
    JoinCommunity,
    ReadMore,
    WatchVideo,
    Subscribe,
    Download,
    ContactUs,
    None,
}

/// Disclosure information (REQUIRED for all AI-generated content)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisclosureInfo {
    pub ai_generated: bool,
    pub ai_assisted: bool,
    pub disclosure_text: String,
    pub models_used: Vec<String>,
    pub human_reviewed: bool,
    pub reviewer: Option<String>,
    pub reviewed_at: Option<DateTime<Utc>>,
}

impl Default for DisclosureInfo {
    fn default() -> Self {
        Self {
            ai_generated: true,
            ai_assisted: true,
            disclosure_text: "Created with AI assistance by Atlas Sphere".to_string(),
            models_used: vec![],
            human_reviewed: false,
            reviewer: None,
            reviewed_at: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComplianceStatus {
    Pending,
    Passed,
    Failed,
    RequiresHumanReview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetStatus {
    Draft,
    PendingReview,
    Approved,
    Rejected,
    Scheduled,
    Published,
    Paused,
    Archived,
    Failed,
}

// ============================================================================
// CAMPAIGN STRUCTURES
// ============================================================================

/// A marketing campaign containing multiple content pieces
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Campaign {
    pub campaign_id: Uuid,
    pub name: String,
    pub description: String,
    pub objective: CampaignObjective,
    pub status: CampaignStatus,

    // Timeline
    pub created_at: DateTime<Utc>,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,

    // Targeting
    pub target_regions: Vec<Region>,
    pub target_languages: Vec<Language>,
    pub target_platforms: Vec<MarketingPlatform>,
    pub target_audience: TargetAudience,

    // Content
    pub narrative_arc: NarrativeArc,
    pub content_ideas: Vec<Uuid>,
    pub content_assets: Vec<Uuid>,

    // Budget
    pub budget_usd: f64,
    pub spent_usd: f64,

    // Metrics
    pub metrics: CampaignMetrics,

    // Team
    pub created_by: String,
    pub approved_by: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CampaignObjective {
    Awareness,
    Trust,
    Authority,
    Adoption,
    DeveloperInterest,
    InvestorInterest,
    EcosystemGrowth,
    ProductLaunch,
    Partnership,
    CrisisResponse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CampaignStatus {
    Planning,
    Active,
    Paused,
    Completed,
    Archived,
    Cancelled,
}

/// Narrative arc for a campaign
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeArc {
    pub opening_hook: String,
    pub key_messages: Vec<String>,
    pub story_beats: Vec<StoryBeat>,
    pub closing_cta: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryBeat {
    pub day_offset: i32,
    pub content_type: ContentType,
    pub platform: MarketingPlatform,
    pub narrative_role: NarrativeRole,
    pub content_id: Option<Uuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NarrativeRole {
    Teaser,
    Reveal,
    Proof,
    CallToAction,
    FollowUp,
    Reminder,
}

/// Campaign performance metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CampaignMetrics {
    pub total_reach: u64,
    pub total_impressions: u64,
    pub total_engagement: u64,
    pub engagement_rate: f32,
    pub clicks: u64,
    pub conversions: u64,
    pub conversion_rate: f32,
    pub sentiment_positive: f32,
    pub sentiment_neutral: f32,
    pub sentiment_negative: f32,
    pub cost_per_engagement: f64,
    pub cost_per_conversion: f64,
    pub roi_estimate: f64,
}

// ============================================================================
// SWARM ORCHESTRATOR
// ============================================================================

/// Central orchestrator for the marketing swarm
pub struct SwarmOrchestrator {
    pub orchestrator_id: Uuid,
    pub governance: Arc<RwLock<GovernanceState>>,
    pub task_queue: Arc<RwLock<VecDeque<AgentTask>>>,
    pub active_campaigns: Arc<RwLock<HashMap<Uuid, Campaign>>>,
    pub content_library: Arc<RwLock<HashMap<Uuid, ContentAsset>>>,
    pub idea_backlog: Arc<RwLock<HashMap<Uuid, ContentIdea>>>,
    pub platform_metrics: Arc<RwLock<HashMap<MarketingPlatform, PlatformHealthMetrics>>>,
    pub agents: HashMap<String, Arc<dyn MarketingAgent>>,
}

/// Health metrics per platform
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlatformHealthMetrics {
    pub platform: String,
    pub is_healthy: bool,
    pub last_post_at: Option<DateTime<Utc>>,
    pub posts_24h: u32,
    pub engagement_rate_24h: f32,
    pub rate_limit_usage: f32,
    pub circuit_breaker_status: String,
    pub compliance_violations_30d: u32,
}

impl SwarmOrchestrator {
    pub fn new(budget_limit_24h: f64) -> Self {
        Self {
            orchestrator_id: Uuid::new_v4(),
            governance: Arc::new(RwLock::new(GovernanceState::new(budget_limit_24h))),
            task_queue: Arc::new(RwLock::new(VecDeque::new())),
            active_campaigns: Arc::new(RwLock::new(HashMap::new())),
            content_library: Arc::new(RwLock::new(HashMap::new())),
            idea_backlog: Arc::new(RwLock::new(HashMap::new())),
            platform_metrics: Arc::new(RwLock::new(HashMap::new())),
            agents: HashMap::new(),
        }
    }

    /// Register an agent with the orchestrator
    pub fn register_agent(&mut self, name: String, agent: Arc<dyn MarketingAgent>) {
        self.agents.insert(name, agent);
    }

    /// Queue a task for processing
    pub async fn queue_task(&self, task: AgentTask) -> Result<Uuid, String> {
        // Check governance allows this
        let governance = self.governance.read().await;
        governance.check_operation_allowed("queue_task")?;
        drop(governance);

        let task_id = task.task_id;
        let mut queue = self.task_queue.write().await;
        queue.push_back(task);

        Ok(task_id)
    }

    /// Get next task from queue (priority-ordered)
    pub async fn get_next_task(&self) -> Option<AgentTask> {
        let mut queue = self.task_queue.write().await;

        // Sort by priority (higher first)
        let mut tasks: Vec<_> = queue.drain(..).collect();
        tasks.sort_by(|a, b| b.priority.cmp(&a.priority));

        let next = tasks.first().cloned();
        queue.extend(tasks.into_iter().skip(1));

        next
    }

    /// Create a new campaign
    pub async fn create_campaign(&self, mut campaign: Campaign) -> Result<Uuid, String> {
        let governance = self.governance.read().await;
        governance.check_operation_allowed("create_campaign")?;
        drop(governance);

        let campaign_id = campaign.campaign_id;
        campaign.status = CampaignStatus::Planning;

        let mut campaigns = self.active_campaigns.write().await;
        campaigns.insert(campaign_id, campaign);

        // Log to audit trail
        let mut governance = self.governance.write().await;
        governance.log_event(
            "campaign_created".to_string(),
            "orchestrator".to_string(),
            campaign_id.to_string(),
            "create".to_string(),
            serde_json::json!({"campaign_id": campaign_id}),
            "success".to_string(),
        );

        Ok(campaign_id)
    }

    /// Submit a content idea
    pub async fn submit_idea(&self, mut idea: ContentIdea) -> Result<Uuid, String> {
        let governance = self.governance.read().await;
        governance.check_operation_allowed("submit_idea")?;
        drop(governance);

        let idea_id = idea.idea_id;
        idea.status = IdeaStatus::New;

        let mut backlog = self.idea_backlog.write().await;
        backlog.insert(idea_id, idea);

        Ok(idea_id)
    }

    /// Store a generated content asset
    pub async fn store_asset(&self, mut asset: ContentAsset) -> Result<Uuid, String> {
        let governance = self.governance.read().await;
        governance.check_operation_allowed("store_asset")?;
        drop(governance);

        // Ensure disclosure is present
        if asset.disclosure.disclosure_text.is_empty() {
            asset.disclosure = DisclosureInfo::default();
        }

        let asset_id = asset.asset_id;
        let mut library = self.content_library.write().await;
        library.insert(asset_id, asset);

        Ok(asset_id)
    }

    /// Get system status
    pub async fn get_system_status(&self) -> serde_json::Value {
        let governance = self.governance.read().await;
        let queue = self.task_queue.read().await;
        let campaigns = self.active_campaigns.read().await;
        let library = self.content_library.read().await;
        let ideas = self.idea_backlog.read().await;

        serde_json::json!({
            "orchestrator_id": self.orchestrator_id,
            "governance": governance.get_health(),
            "task_queue_depth": queue.len(),
            "active_campaigns": campaigns.len(),
            "content_assets": library.len(),
            "idea_backlog": ideas.len(),
            "registered_agents": self.agents.len(),
            "timestamp": Utc::now(),
        })
    }

    /// Trigger emergency stop
    pub async fn emergency_stop(&self, reason: String, operator: String) {
        let mut governance = self.governance.write().await;
        governance.trigger_emergency_stop(reason, operator);
    }

    /// Get platform health for dashboard
    pub async fn get_platform_health(&self) -> HashMap<String, PlatformHealthMetrics> {
        let metrics = self.platform_metrics.read().await;
        metrics
            .iter()
            .map(|(k, v)| (format!("{:?}", k), v.clone()))
            .collect()
    }
}

// ============================================================================
// SCHEDULING ENGINE
// ============================================================================

/// Scheduling engine for optimal content timing
pub struct SchedulingEngine {
    pub scheduler_id: Uuid,
    pub scheduled_posts: Arc<RwLock<Vec<ScheduledPost>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledPost {
    pub schedule_id: Uuid,
    pub asset_id: Uuid,
    pub platform: MarketingPlatform,
    pub scheduled_for: DateTime<Utc>,
    pub target_region: Region,
    pub status: ScheduleStatus,
    pub retry_count: u32,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScheduleStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
    Rescheduled,
}

impl SchedulingEngine {
    pub fn new() -> Self {
        Self {
            scheduler_id: Uuid::new_v4(),
            scheduled_posts: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Calculate optimal posting time for a region
    pub fn calculate_optimal_time(
        &self,
        region: Region,
        platform: MarketingPlatform,
        content_type: &ContentType,
    ) -> DateTime<Utc> {
        let (start_hour, _end_hour) = region.peak_hours_utc();

        // Add platform-specific offsets
        let platform_offset_hours = match platform {
            MarketingPlatform::Twitter => 0,      // Morning is good
            MarketingPlatform::LinkedIn => 2,     // Late morning
            MarketingPlatform::Instagram => 4,    // Afternoon
            MarketingPlatform::TikTok => 6,       // Evening
            MarketingPlatform::YouTube => 3,      // Mid-day
            _ => 1,
        };

        // Add content-type offsets
        let content_offset = match content_type {
            ContentType::Tweet | ContentType::ThreadStart => 0,
            ContentType::BlogPost | ContentType::TechnicalDoc => 2, // Business hours
            ContentType::YouTubeShort | ContentType::TikTokVideo => 5, // Evening
            _ => 1,
        };

        let target_hour = (start_hour + platform_offset_hours + content_offset) % 24;

        // Calculate next occurrence of this hour
        let now = Utc::now();
        let today = now.date_naive();
        let target_time = today.and_hms_opt(target_hour, 0, 0).unwrap();
        let target_datetime = Utc.from_utc_datetime(&target_time);

        if target_datetime > now {
            target_datetime
        } else {
            target_datetime + Duration::days(1)
        }
    }

    /// Schedule a post
    pub async fn schedule_post(
        &self,
        asset_id: Uuid,
        platform: MarketingPlatform,
        target_region: Region,
        content_type: &ContentType,
    ) -> Result<Uuid, String> {
        let scheduled_for = self.calculate_optimal_time(target_region, platform, content_type);

        let post = ScheduledPost {
            schedule_id: Uuid::new_v4(),
            asset_id,
            platform,
            scheduled_for,
            target_region,
            status: ScheduleStatus::Pending,
            retry_count: 0,
            last_error: None,
        };

        let schedule_id = post.schedule_id;
        let mut posts = self.scheduled_posts.write().await;
        posts.push(post);

        Ok(schedule_id)
    }

    /// Get posts due for publishing
    pub async fn get_due_posts(&self) -> Vec<ScheduledPost> {
        let now = Utc::now();
        let posts = self.scheduled_posts.read().await;

        posts
            .iter()
            .filter(|p| p.status == ScheduleStatus::Pending && p.scheduled_for <= now)
            .cloned()
            .collect()
    }
}

// ============================================================================
// LOCALIZATION ENGINE
// ============================================================================

/// Engine for content localization
pub struct LocalizationEngine {
    pub engine_id: Uuid,
    pub cultural_adaptations: HashMap<Language, CulturalAdaptation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CulturalAdaptation {
    pub language: Language,
    pub formal_address: bool,          // Use formal "you" (e.g., German Sie vs. du)
    pub emoji_usage: EmojiUsage,
    pub humor_style: HumorStyle,
    pub sensitive_topics: Vec<String>, // Topics to avoid
    pub preferred_formats: Vec<ContentType>,
    pub cta_style: CtaStyle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmojiUsage {
    Heavy,    // Japanese, Korean
    Moderate, // US, Latin America
    Light,    // German, Nordic
    Minimal,  // Professional/B2B
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HumorStyle {
    Universal,   // Safe across cultures
    Wordplay,    // For languages that support it
    SelfDeprecating,
    None,        // Formal content
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CtaStyle {
    Direct,      // "Buy now", "Sign up"
    Soft,        // "Learn more", "Discover"
    Question,    // "Ready to start?"
    Community,   // "Join us"
}

impl LocalizationEngine {
    pub fn new() -> Self {
        let mut adaptations = HashMap::new();

        // English (US)
        adaptations.insert(
            Language::English,
            CulturalAdaptation {
                language: Language::English,
                formal_address: false,
                emoji_usage: EmojiUsage::Moderate,
                humor_style: HumorStyle::Universal,
                sensitive_topics: vec!["politics".to_string()],
                preferred_formats: vec![ContentType::Tweet, ContentType::YouTubeShort],
                cta_style: CtaStyle::Direct,
            },
        );

        // Japanese
        adaptations.insert(
            Language::Japanese,
            CulturalAdaptation {
                language: Language::Japanese,
                formal_address: true,
                emoji_usage: EmojiUsage::Heavy,
                humor_style: HumorStyle::Universal,
                sensitive_topics: vec!["politics".to_string(), "religion".to_string()],
                preferred_formats: vec![ContentType::Tweet, ContentType::YouTubeVideo],
                cta_style: CtaStyle::Soft,
            },
        );

        // German
        adaptations.insert(
            Language::German,
            CulturalAdaptation {
                language: Language::German,
                formal_address: true,
                emoji_usage: EmojiUsage::Light,
                humor_style: HumorStyle::None,
                sensitive_topics: vec!["WWII".to_string()],
                preferred_formats: vec![ContentType::BlogPost, ContentType::LinkedInPost],
                cta_style: CtaStyle::Soft,
            },
        );

        // Arabic
        adaptations.insert(
            Language::Arabic,
            CulturalAdaptation {
                language: Language::Arabic,
                formal_address: true,
                emoji_usage: EmojiUsage::Moderate,
                humor_style: HumorStyle::Universal,
                sensitive_topics: vec!["religion".to_string(), "politics".to_string()],
                preferred_formats: vec![ContentType::Tweet, ContentType::InstagramReel],
                cta_style: CtaStyle::Community,
            },
        );

        Self {
            engine_id: Uuid::new_v4(),
            cultural_adaptations: adaptations,
        }
    }

    /// Get cultural adaptation rules for a language
    pub fn get_adaptation(&self, language: &Language) -> Option<&CulturalAdaptation> {
        self.cultural_adaptations.get(language)
    }

    /// Check if content contains sensitive topics for a language
    pub fn check_sensitivity(&self, content: &str, language: &Language) -> Vec<String> {
        if let Some(adaptation) = self.get_adaptation(language) {
            adaptation
                .sensitive_topics
                .iter()
                .filter(|topic| content.to_lowercase().contains(&topic.to_lowercase()))
                .cloned()
                .collect()
        } else {
            vec![]
        }
    }
}

// ============================================================================
// UNIT TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_iso_codes() {
        assert_eq!(Language::English.iso_code(), "en");
        assert_eq!(Language::Japanese.iso_code(), "ja");
        assert_eq!(Language::ChineseSimplified.iso_code(), "zh-CN");
    }

    #[test]
    fn test_language_tiers() {
        assert_eq!(Language::English.tier(), 1);
        assert_eq!(Language::Italian.tier(), 2);
        assert_eq!(Language::Swahili.tier(), 3);
    }

    #[test]
    fn test_rtl_languages() {
        assert!(Language::Arabic.is_rtl());
        assert!(Language::Hebrew.is_rtl());
        assert!(!Language::English.is_rtl());
    }

    #[test]
    fn test_region_primary_languages() {
        let na_langs = Region::NorthAmerica.primary_languages();
        assert!(na_langs.contains(&Language::English));
        assert!(na_langs.contains(&Language::Spanish));
    }

    #[test]
    fn test_content_type_character_limits() {
        assert_eq!(ContentType::Tweet.character_limit(), Some(280));
        assert_eq!(ContentType::LinkedInPost.character_limit(), Some(3000));
        assert_eq!(ContentType::BlogPost.character_limit(), None);
    }

    #[test]
    fn test_disclosure_default() {
        let disclosure = DisclosureInfo::default();
        assert!(disclosure.ai_generated);
        assert!(disclosure.disclosure_text.contains("AI assistance"));
    }

    #[tokio::test]
    async fn test_swarm_orchestrator_creation() {
        let orchestrator = SwarmOrchestrator::new(1000.0);
        let status = orchestrator.get_system_status().await;

        assert!(status["task_queue_depth"].as_u64().unwrap() == 0);
        assert!(status["active_campaigns"].as_u64().unwrap() == 0);
    }

    #[tokio::test]
    async fn test_scheduling_engine() {
        let engine = SchedulingEngine::new();
        let optimal_time = engine.calculate_optimal_time(
            Region::NorthAmerica,
            MarketingPlatform::Twitter,
            &ContentType::Tweet,
        );

        assert!(optimal_time > Utc::now());
    }

    #[test]
    fn test_localization_engine() {
        let engine = LocalizationEngine::new();

        let jp_adaptation = engine.get_adaptation(&Language::Japanese).unwrap();
        assert!(jp_adaptation.formal_address);
        assert_eq!(jp_adaptation.emoji_usage, EmojiUsage::Heavy);
    }

    #[test]
    fn test_sensitivity_check() {
        let engine = LocalizationEngine::new();

        let sensitive = engine.check_sensitivity("Let's talk about politics today", &Language::English);
        assert!(sensitive.contains(&"politics".to_string()));

        let clean = engine.check_sensitivity("Great blockchain technology", &Language::English);
        assert!(clean.is_empty());
    }
}
