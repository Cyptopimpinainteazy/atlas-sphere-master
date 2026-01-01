// ============================================================================
// X3 ATLAS SPHERE - CONTENT GENERATION PIPELINE
// End-to-end content production with AI models
// ============================================================================
//
// Pipeline stages:
// 1. Signal ingestion → Idea generation
// 2. Idea prioritization → Approval workflow
// 3. Script/copy generation → Tone variants
// 4. Media generation → Images, video, audio
// 5. Localization → 20+ languages
// 6. QA & compliance → Safety checks
// 7. Scheduling → Optimal timing
// 8. Publishing → Distribution
// 9. Monitoring → Performance tracking
// 10. Optimization → A/B testing & iteration

use crate::marketing_agents::{MarketingPlatform, ResourceUsage};
use crate::marketing_governance::{ComplianceCheck, ComplianceViolation};
use crate::swarm_core::{
    AngleType, ContentAngle, ContentIdea, ContentTone, ContentType, ContentUrgency,
    DisclosureInfo, IdeaStatus, Language, MediaAsset, MediaType, Region, SignalSource,
    SignalType, TargetAudience,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ============================================================================
// PIPELINE STAGES
// ============================================================================

/// Stage in the content pipeline
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PipelineStage {
    SignalIngestion,
    IdeaGeneration,
    Prioritization,
    Approval,
    ScriptGeneration,
    MediaGeneration,
    Localization,
    QualityAssurance,
    ComplianceCheck,
    Scheduling,
    Publishing,
    Monitoring,
    Optimization,
}

/// Status of a pipeline job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineJob {
    pub job_id: Uuid,
    pub content_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub current_stage: PipelineStage,
    pub completed_stages: Vec<PipelineStageResult>,
    pub status: PipelineJobStatus,
    pub error: Option<String>,
    pub estimated_completion: Option<DateTime<Utc>>,
    pub total_cost_usd: f64,
    pub metrics: PipelineMetrics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PipelineJobStatus {
    Queued,
    InProgress,
    PendingApproval,
    Completed,
    Failed,
    Cancelled,
    Paused,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStageResult {
    pub stage: PipelineStage,
    pub started_at: DateTime<Utc>,
    pub completed_at: DateTime<Utc>,
    pub success: bool,
    pub output: serde_json::Value,
    pub resources_used: ResourceUsage,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PipelineMetrics {
    pub total_time_seconds: u64,
    pub api_calls: u32,
    pub tokens_used: u32,
    pub images_generated: u32,
    pub videos_generated: u32,
    pub languages_translated: u32,
    pub variants_created: u32,
}

// ============================================================================
// SIGNAL INGESTION
// ============================================================================

/// Signal from external sources triggering content ideas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    pub signal_id: Uuid,
    pub signal_type: SignalType,
    pub source: String,
    pub title: String,
    pub description: String,
    pub url: Option<String>,
    pub detected_at: DateTime<Utc>,
    pub relevance_score: f32,   // 0.0 - 1.0
    pub trend_momentum: f32,    // 0.0 - 1.0
    pub confidence: f32,        // 0.0 - 1.0
    pub keywords: Vec<String>,
    pub related_topics: Vec<String>,
    pub sentiment: Option<f32>, // -1.0 to 1.0
    pub regional_relevance: HashMap<Region, f32>,
}

/// Signal ingestion engine
pub struct SignalIngestor {
    pub ingestor_id: Uuid,
    pub sources: Vec<SignalSourceConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalSourceConfig {
    pub source_type: SignalType,
    pub api_endpoint: Option<String>,
    pub polling_interval_seconds: u32,
    pub enabled: bool,
    pub filters: Vec<String>,
}

impl SignalIngestor {
    pub fn new() -> Self {
        Self {
            ingestor_id: Uuid::new_v4(),
            sources: vec![
                SignalSourceConfig {
                    source_type: SignalType::TwitterTrend,
                    api_endpoint: Some("https://api.twitter.com/2/trends".to_string()),
                    polling_interval_seconds: 300, // 5 minutes
                    enabled: true,
                    filters: vec!["blockchain".to_string(), "web3".to_string(), "crypto".to_string()],
                },
                SignalSourceConfig {
                    source_type: SignalType::RedditTrending,
                    api_endpoint: Some("https://www.reddit.com/r/cryptocurrency/hot.json".to_string()),
                    polling_interval_seconds: 600,
                    enabled: true,
                    filters: vec!["scaling".to_string(), "defi".to_string()],
                },
                SignalSourceConfig {
                    source_type: SignalType::CommunityQuestion,
                    api_endpoint: None, // Internal Discord/Telegram monitoring
                    polling_interval_seconds: 60,
                    enabled: true,
                    filters: vec![],
                },
                SignalSourceConfig {
                    source_type: SignalType::CompetitorActivity,
                    api_endpoint: None,
                    polling_interval_seconds: 1800, // 30 minutes
                    enabled: true,
                    filters: vec![],
                },
            ],
        }
    }

    /// Process raw signal into structured format
    pub fn process_signal(&self, raw_data: &serde_json::Value, source: SignalType) -> Option<Signal> {
        // In production, would parse platform-specific response format
        Some(Signal {
            signal_id: Uuid::new_v4(),
            signal_type: source,
            source: format!("{:?}", source),
            title: raw_data.get("title").and_then(|t| t.as_str()).unwrap_or("").to_string(),
            description: raw_data.get("description").and_then(|d| d.as_str()).unwrap_or("").to_string(),
            url: raw_data.get("url").and_then(|u| u.as_str()).map(String::from),
            detected_at: Utc::now(),
            relevance_score: 0.75,
            trend_momentum: 0.6,
            confidence: 0.8,
            keywords: vec![],
            related_topics: vec![],
            sentiment: Some(0.2),
            regional_relevance: HashMap::new(),
        })
    }
}

// ============================================================================
// IDEA GENERATION
// ============================================================================

/// Engine for generating content ideas from signals
pub struct IdeaGenerator {
    pub generator_id: Uuid,
    pub model: String,
    pub temperature: f32,
    pub max_ideas_per_signal: usize,
}

impl IdeaGenerator {
    pub fn new() -> Self {
        Self {
            generator_id: Uuid::new_v4(),
            model: "gpt-4-turbo".to_string(),
            temperature: 0.8,
            max_ideas_per_signal: 5,
        }
    }

    /// Generate content ideas from a signal
    pub async fn generate_ideas(&self, signal: &Signal) -> Vec<ContentIdea> {
        let mut ideas = Vec::new();

        // Generate multiple angles for the signal
        let angles = vec![
            (AngleType::Educational, ContentTone::Educational),
            (AngleType::Emotional, ContentTone::Visionary),
            (AngleType::DeveloperFirst, ContentTone::Technical),
            (AngleType::InvestorGrade, ContentTone::Professional),
            (AngleType::StoryTelling, ContentTone::Casual),
        ];

        for (angle_type, tone) in angles.into_iter().take(self.max_ideas_per_signal) {
            let idea = ContentIdea {
                idea_id: Uuid::new_v4(),
                created_at: Utc::now(),
                signal_source: SignalSource {
                    source_type: signal.signal_type,
                    reference_url: signal.url.clone(),
                    detected_at: signal.detected_at,
                    trend_momentum: signal.trend_momentum,
                    confidence: signal.confidence,
                },
                content_type: self.suggest_content_type(&signal.signal_type, &angle_type),
                priority_score: signal.relevance_score * signal.confidence,
                novelty_score: 0.7,
                alignment_score: 0.85,
                urgency: self.determine_urgency(signal.trend_momentum),
                target_audience: self.determine_audience(&angle_type),
                suggested_tones: vec![tone],
                suggested_angles: vec![ContentAngle {
                    angle_id: Uuid::new_v4(),
                    angle_type,
                    hook: self.generate_hook(&signal.title, &angle_type),
                    body_outline: format!("Explore {} through {} lens", signal.title, format!("{:?}", angle_type)),
                    cta_suggestion: Some("Learn more about Atlas Sphere".to_string()),
                    engagement_prediction: 0.065,
                }],
                regional_fit: signal.regional_relevance.clone(),
                estimated_effort_hours: self.estimate_effort(&angle_type),
                status: IdeaStatus::New,
                rejection_reason: None,
                approved_by: None,
                approved_at: None,
            };

            ideas.push(idea);
        }

        ideas
    }

    fn suggest_content_type(&self, signal_type: &SignalType, angle_type: &AngleType) -> ContentType {
        match (signal_type, angle_type) {
            (SignalType::TwitterTrend, _) => ContentType::ThreadStart,
            (SignalType::RedditTrending, AngleType::DeveloperFirst) => ContentType::RedditPost,
            (_, AngleType::Educational) => ContentType::BlogPost,
            (_, AngleType::InvestorGrade) => ContentType::LinkedInPost,
            _ => ContentType::Tweet,
        }
    }

    fn determine_urgency(&self, momentum: f32) -> ContentUrgency {
        if momentum > 0.8 {
            ContentUrgency::Immediate
        } else if momentum > 0.6 {
            ContentUrgency::High
        } else if momentum > 0.3 {
            ContentUrgency::Medium
        } else {
            ContentUrgency::Low
        }
    }

    fn determine_audience(&self, angle_type: &AngleType) -> TargetAudience {
        match angle_type {
            AngleType::DeveloperFirst => TargetAudience::Developers,
            AngleType::InvestorGrade => TargetAudience::Investors,
            AngleType::Educational => TargetAudience::Newcomers,
            AngleType::Rational => TargetAudience::TechnicalExperts,
            _ => TargetAudience::Community,
        }
    }

    fn generate_hook(&self, title: &str, angle_type: &AngleType) -> String {
        match angle_type {
            AngleType::Emotional => format!("🚀 {} is changing everything. Here's why:", title),
            AngleType::Educational => format!("Let me explain {} in simple terms:", title),
            AngleType::DeveloperFirst => format!("Technical deep-dive: {} architecture:", title),
            AngleType::InvestorGrade => format!("Market insight: {} represents a strategic opportunity:", title),
            AngleType::StoryTelling => format!("The story behind {}: a thread 🧵", title),
            _ => format!("Exploring {}", title),
        }
    }

    fn estimate_effort(&self, angle_type: &AngleType) -> f32 {
        match angle_type {
            AngleType::DeveloperFirst => 4.0,
            AngleType::InvestorGrade => 3.0,
            AngleType::Educational => 2.5,
            AngleType::StoryTelling => 2.0,
            _ => 1.5,
        }
    }
}

// ============================================================================
// SCRIPT GENERATION
// ============================================================================

/// Generated script/copy for content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedScript {
    pub script_id: Uuid,
    pub idea_id: Uuid,
    pub tone: ContentTone,
    pub platform: MarketingPlatform,
    pub language: Language,
    pub hook_variants: Vec<String>,
    pub body: String,
    pub cta_variants: Vec<String>,
    pub word_count: usize,
    pub estimated_read_time_seconds: u32,
    pub quality_score: f32,
    pub brand_alignment_score: f32,
    pub generated_at: DateTime<Utc>,
    pub model_used: String,
    pub tokens_used: u32,
}

/// Script generation engine
pub struct ScriptGenerator {
    pub generator_id: Uuid,
    pub model: String,
    pub brand_voice_prompt: String,
    pub founder_voice_samples: Vec<String>,
}

impl ScriptGenerator {
    pub fn new() -> Self {
        Self {
            generator_id: Uuid::new_v4(),
            model: "gpt-4-turbo".to_string(),
            brand_voice_prompt: r#"
You are writing content for Atlas Sphere, a next-generation blockchain platform.

Voice characteristics:
- Technically accurate but accessible
- Confident but not arrogant
- Forward-looking and innovative
- Community-focused
- Transparent about AI assistance

Always include appropriate disclosure for AI-generated content.
"#.to_string(),
            founder_voice_samples: vec![],
        }
    }

    /// Generate script variants for an idea
    pub async fn generate_scripts(
        &self,
        idea: &ContentIdea,
        target_platforms: &[MarketingPlatform],
        target_languages: &[Language],
    ) -> Vec<GeneratedScript> {
        let mut scripts = Vec::new();

        for platform in target_platforms {
            for &tone in &idea.suggested_tones {
                for &language in target_languages {
                    let script = self.generate_single_script(idea, *platform, tone, language).await;
                    scripts.push(script);
                }
            }
        }

        scripts
    }

    async fn generate_single_script(
        &self,
        idea: &ContentIdea,
        platform: MarketingPlatform,
        tone: ContentTone,
        language: Language,
    ) -> GeneratedScript {
        // In production, would call LLM API
        let hook_variants = vec![
            format!("🚀 {}", idea.suggested_angles.first().map(|a| a.hook.as_str()).unwrap_or("Check this out")),
            format!("⚡ {}", idea.suggested_angles.first().map(|a| a.hook.as_str()).unwrap_or("Breaking news")),
            format!("💡 {}", idea.suggested_angles.first().map(|a| a.hook.as_str()).unwrap_or("Did you know")),
        ];

        let body = format!(
            "{}\n\nLearn more about how Atlas Sphere is building the future of blockchain.",
            idea.suggested_angles.first().map(|a| a.body_outline.as_str()).unwrap_or("")
        );

        let cta_variants = vec![
            "Try it now →".to_string(),
            "Join the community →".to_string(),
            "Learn more →".to_string(),
        ];

        GeneratedScript {
            script_id: Uuid::new_v4(),
            idea_id: idea.idea_id,
            tone,
            platform,
            language,
            hook_variants,
            body: body.clone(),
            cta_variants,
            word_count: body.split_whitespace().count(),
            estimated_read_time_seconds: (body.len() / 200) as u32 * 60, // ~200 WPM
            quality_score: 0.87,
            brand_alignment_score: 0.92,
            generated_at: Utc::now(),
            model_used: self.model.clone(),
            tokens_used: 850,
        }
    }
}

// ============================================================================
// MEDIA GENERATION
// ============================================================================

/// Request for image generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageGenerationRequest {
    pub request_id: Uuid,
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub style: ImageStyle,
    pub dimensions: ImageDimensions,
    pub variants: u8,
    pub seed: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageStyle {
    PhotoRealistic,
    Illustration,
    Abstract,
    Diagram,
    Infographic,
    ThreeDRender,
    Minimalist,
    Gradient,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ImageDimensions {
    pub width: u32,
    pub height: u32,
}

impl ImageDimensions {
    pub fn square_1024() -> Self {
        Self { width: 1024, height: 1024 }
    }

    pub fn landscape_16_9() -> Self {
        Self { width: 1920, height: 1080 }
    }

    pub fn portrait_9_16() -> Self {
        Self { width: 1080, height: 1920 }
    }

    pub fn twitter_card() -> Self {
        Self { width: 1200, height: 675 }
    }

    pub fn youtube_thumbnail() -> Self {
        Self { width: 1280, height: 720 }
    }
}

/// Image generation engine
pub struct ImageGenerator {
    pub generator_id: Uuid,
    pub model: String,
    pub style_presets: HashMap<ImageStyle, String>,
}

impl ImageGenerator {
    pub fn new() -> Self {
        let mut style_presets = HashMap::new();
        style_presets.insert(
            ImageStyle::Abstract,
            "abstract geometric shapes, blockchain network visualization, gradient colors, modern tech aesthetic, clean lines".to_string()
        );
        style_presets.insert(
            ImageStyle::Diagram,
            "technical diagram, flowchart style, clean white background, professional, labeled components".to_string()
        );
        style_presets.insert(
            ImageStyle::Minimalist,
            "minimalist design, single focal point, ample white space, brand colors, elegant typography".to_string()
        );

        Self {
            generator_id: Uuid::new_v4(),
            model: "flux-1.1-pro".to_string(), // Or SDXL, DALL-E 3
            style_presets,
        }
    }

    /// Generate images for content
    pub async fn generate_images(&self, request: &ImageGenerationRequest) -> Vec<MediaAsset> {
        let mut images = Vec::new();

        for i in 0..request.variants {
            // In production, would call Replicate/OpenAI API
            let image = MediaAsset {
                media_id: Uuid::new_v4(),
                media_type: MediaType::Image,
                url: format!("s3://atlas-media/generated/{}/{}.png", request.request_id, i),
                cdn_url: Some(format!("https://cdn.atlas-sphere.io/media/{}/{}.png", request.request_id, i)),
                file_hash: format!("sha256:{}", Uuid::new_v4()),
                dimensions: Some(crate::swarm_core::Dimensions {
                    width: request.dimensions.width,
                    height: request.dimensions.height,
                    aspect_ratio: format!("{}:{}", 
                        request.dimensions.width / gcd(request.dimensions.width, request.dimensions.height),
                        request.dimensions.height / gcd(request.dimensions.width, request.dimensions.height)
                    ),
                }),
                duration_seconds: None,
                file_size_bytes: 1024 * 1024 * 2, // ~2MB
                alt_text: request.prompt.clone(),
                generated_by: format!("image_generator:{}", self.generator_id),
                prompt_used: Some(request.prompt.clone()),
            };

            images.push(image);
        }

        images
    }

    /// Build prompt with style preset
    pub fn build_prompt(&self, base_prompt: &str, style: &ImageStyle) -> String {
        let style_suffix = self.style_presets.get(style).map(|s| s.as_str()).unwrap_or("");
        format!("{}, {}", base_prompt, style_suffix)
    }
}

/// Greatest common divisor for aspect ratio calculation
fn gcd(a: u32, b: u32) -> u32 {
    if b == 0 { a } else { gcd(b, a % b) }
}

/// Video generation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoGenerationRequest {
    pub request_id: Uuid,
    pub prompt: String,
    pub duration_seconds: u32,
    pub aspect_ratio: String,
    pub style: VideoStyle,
    pub audio: VideoAudioConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoStyle {
    TextToVideo,        // Full AI generation
    ClipAssembly,       // Assembled from existing clips
    AnimatedGraphics,   // Motion graphics
    ScreenRecording,    // With overlay
    SlideDeck,          // Animated slides
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoAudioConfig {
    pub include_voiceover: bool,
    pub voiceover_text: Option<String>,
    pub background_music: Option<String>,
    pub include_captions: bool,
}

/// Video generation engine
pub struct VideoGenerator {
    pub generator_id: Uuid,
    pub text_to_video_model: String,
    pub tts_model: String,
}

impl VideoGenerator {
    pub fn new() -> Self {
        Self {
            generator_id: Uuid::new_v4(),
            text_to_video_model: "runway-gen3".to_string(),
            tts_model: "elevenlabs-multilingual-v2".to_string(),
        }
    }

    /// Generate video from request
    pub async fn generate_video(&self, request: &VideoGenerationRequest) -> Result<MediaAsset, String> {
        // In production, would call Runway/Pika/other video generation API

        Ok(MediaAsset {
            media_id: Uuid::new_v4(),
            media_type: MediaType::Video,
            url: format!("s3://atlas-media/videos/{}.mp4", request.request_id),
            cdn_url: Some(format!("https://cdn.atlas-sphere.io/videos/{}.mp4", request.request_id)),
            file_hash: format!("sha256:{}", Uuid::new_v4()),
            dimensions: Some(match request.aspect_ratio.as_str() {
                "16:9" => crate::swarm_core::Dimensions {
                    width: 1920,
                    height: 1080,
                    aspect_ratio: "16:9".to_string(),
                },
                "9:16" => crate::swarm_core::Dimensions {
                    width: 1080,
                    height: 1920,
                    aspect_ratio: "9:16".to_string(),
                },
                _ => crate::swarm_core::Dimensions {
                    width: 1080,
                    height: 1080,
                    aspect_ratio: "1:1".to_string(),
                },
            }),
            duration_seconds: Some(request.duration_seconds),
            file_size_bytes: request.duration_seconds as u64 * 5 * 1024 * 1024, // ~5MB/sec
            alt_text: request.prompt.clone(),
            generated_by: format!("video_generator:{}", self.generator_id),
            prompt_used: Some(request.prompt.clone()),
        })
    }
}

// ============================================================================
// LOCALIZATION
// ============================================================================

/// Localization request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalizationRequest {
    pub request_id: Uuid,
    pub source_text: String,
    pub source_language: Language,
    pub target_languages: Vec<Language>,
    pub content_type: ContentType,
    pub context: String,
    pub preserve_formatting: bool,
    pub adapt_culturally: bool,
}

/// Localized content result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalizedContent {
    pub localization_id: Uuid,
    pub source_text: String,
    pub language: Language,
    pub translated_text: String,
    pub cultural_adaptations: Vec<String>,
    pub quality_score: f32,
    pub reviewed: bool,
    pub reviewer_notes: Option<String>,
}

/// Localization engine
pub struct Localizer {
    pub localizer_id: Uuid,
    pub translation_model: String,
    pub supported_languages: Vec<Language>,
}

impl Localizer {
    pub fn new() -> Self {
        Self {
            localizer_id: Uuid::new_v4(),
            translation_model: "gpt-4-turbo".to_string(),
            supported_languages: vec![
                Language::English,
                Language::Spanish,
                Language::Portuguese,
                Language::French,
                Language::German,
                Language::Japanese,
                Language::ChineseSimplified,
                Language::Korean,
                Language::Russian,
                Language::Arabic,
            ],
        }
    }

    /// Localize content to multiple languages
    pub async fn localize(&self, request: &LocalizationRequest) -> Vec<LocalizedContent> {
        let mut results = Vec::new();

        for &target_lang in &request.target_languages {
            if target_lang == request.source_language {
                continue;
            }

            let localized = self.translate_single(
                &request.source_text,
                request.source_language,
                target_lang,
                &request.context,
                request.adapt_culturally,
            ).await;

            results.push(localized);
        }

        results
    }

    async fn translate_single(
        &self,
        text: &str,
        _source: Language,
        target: Language,
        _context: &str,
        _adapt_culturally: bool,
    ) -> LocalizedContent {
        // In production, would call translation API with cultural adaptation prompts

        LocalizedContent {
            localization_id: Uuid::new_v4(),
            source_text: text.to_string(),
            language: target,
            translated_text: format!("[{}] {}", target.iso_code(), text), // Placeholder
            cultural_adaptations: vec![],
            quality_score: 0.92,
            reviewed: false,
            reviewer_notes: None,
        }
    }
}

// ============================================================================
// QA & COMPLIANCE
// ============================================================================

/// QA check request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QACheckRequest {
    pub content_id: Uuid,
    pub text: String,
    pub platform: MarketingPlatform,
    pub language: Language,
    pub content_type: ContentType,
    pub media_assets: Vec<MediaAsset>,
    pub disclosure: DisclosureInfo,
}

/// QA check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QACheckResult {
    pub check_id: Uuid,
    pub content_id: Uuid,
    pub passed: bool,
    pub checks: Vec<QACheck>,
    pub overall_score: f32,
    pub recommendations: Vec<String>,
    pub blocked_reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QACheck {
    pub check_type: QACheckType,
    pub passed: bool,
    pub score: f32,
    pub details: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QACheckType {
    SpellingGrammar,
    BrandAlignment,
    ToneConsistency,
    FactualAccuracy,
    DisclosurePresent,
    PlatformCompliance,
    AccessibilityCheck,
    SensitiveContentCheck,
    CopyrightCheck,
    HateSpeechCheck,
    MisinformationCheck,
}

/// QA engine
pub struct QAEngine {
    pub engine_id: Uuid,
    pub brand_guidelines: BrandGuidelines,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrandGuidelines {
    pub voice_keywords: Vec<String>,
    pub avoid_keywords: Vec<String>,
    pub required_disclosure: String,
    pub color_palette: Vec<String>,
    pub logo_usage_rules: String,
}

impl QAEngine {
    pub fn new() -> Self {
        Self {
            engine_id: Uuid::new_v4(),
            brand_guidelines: BrandGuidelines {
                voice_keywords: vec![
                    "innovative".to_string(),
                    "scalable".to_string(),
                    "secure".to_string(),
                    "decentralized".to_string(),
                ],
                avoid_keywords: vec![
                    "guaranteed returns".to_string(),
                    "get rich".to_string(),
                    "moon".to_string(),
                ],
                required_disclosure: "Created with AI assistance by Atlas Sphere".to_string(),
                color_palette: vec!["#1E40AF".to_string(), "#3B82F6".to_string()],
                logo_usage_rules: "Logo must have minimum clear space of 20px".to_string(),
            },
        }
    }

    /// Run QA checks on content
    pub async fn check_quality(&self, request: &QACheckRequest) -> QACheckResult {
        let mut checks = Vec::new();
        let mut blocked_reasons = Vec::new();

        // Disclosure check (MANDATORY)
        let disclosure_check = self.check_disclosure(&request.disclosure);
        if !disclosure_check.passed {
            blocked_reasons.push("Missing AI disclosure".to_string());
        }
        checks.push(disclosure_check);

        // Brand alignment check
        checks.push(self.check_brand_alignment(&request.text));

        // Sensitive content check
        let sensitive_check = self.check_sensitive_content(&request.text);
        if !sensitive_check.passed {
            blocked_reasons.push("Sensitive content detected".to_string());
        }
        checks.push(sensitive_check);

        // Platform compliance check
        checks.push(self.check_platform_compliance(&request.text, request.platform));

        // Accessibility check
        checks.push(self.check_accessibility(&request.media_assets));

        let passed = blocked_reasons.is_empty() && checks.iter().all(|c| c.passed || c.score > 0.6);
        let overall_score = checks.iter().map(|c| c.score).sum::<f32>() / checks.len() as f32;

        QACheckResult {
            check_id: Uuid::new_v4(),
            content_id: request.content_id,
            passed,
            checks,
            overall_score,
            recommendations: vec![],
            blocked_reasons,
        }
    }

    fn check_disclosure(&self, disclosure: &DisclosureInfo) -> QACheck {
        let has_disclosure = !disclosure.disclosure_text.is_empty();
        QACheck {
            check_type: QACheckType::DisclosurePresent,
            passed: has_disclosure,
            score: if has_disclosure { 1.0 } else { 0.0 },
            details: if has_disclosure {
                "AI disclosure present".to_string()
            } else {
                "MISSING: AI disclosure is required".to_string()
            },
        }
    }

    fn check_brand_alignment(&self, text: &str) -> QACheck {
        let text_lower = text.to_lowercase();
        
        // Check for keywords to avoid
        let has_avoid = self.brand_guidelines.avoid_keywords.iter()
            .any(|kw| text_lower.contains(&kw.to_lowercase()));

        QACheck {
            check_type: QACheckType::BrandAlignment,
            passed: !has_avoid,
            score: if has_avoid { 0.3 } else { 0.9 },
            details: if has_avoid {
                "Contains keywords that should be avoided".to_string()
            } else {
                "Content aligns with brand guidelines".to_string()
            },
        }
    }

    fn check_sensitive_content(&self, text: &str) -> QACheck {
        let text_lower = text.to_lowercase();
        
        // Check for financial advice red flags
        let sensitive_patterns = vec![
            "financial advice",
            "guaranteed profit",
            "risk free",
            "not investment advice", // Actually good
        ];

        let has_sensitive = sensitive_patterns.iter()
            .any(|p| text_lower.contains(p));

        QACheck {
            check_type: QACheckType::SensitiveContentCheck,
            passed: !has_sensitive,
            score: if has_sensitive { 0.5 } else { 1.0 },
            details: if has_sensitive {
                "Contains potentially sensitive financial language".to_string()
            } else {
                "No sensitive content detected".to_string()
            },
        }
    }

    fn check_platform_compliance(&self, text: &str, platform: MarketingPlatform) -> QACheck {
        let char_count = text.chars().count();
        let limit = match platform {
            MarketingPlatform::Twitter => 280,
            MarketingPlatform::LinkedIn => 3000,
            MarketingPlatform::Instagram => 2200,
            _ => 10000,
        };

        let passed = char_count <= limit;

        QACheck {
            check_type: QACheckType::PlatformCompliance,
            passed,
            score: if passed { 1.0 } else { 0.0 },
            details: format!("Character count: {} / {} limit", char_count, limit),
        }
    }

    fn check_accessibility(&self, media: &[MediaAsset]) -> QACheck {
        let all_have_alt = media.iter().all(|m| !m.alt_text.is_empty());

        QACheck {
            check_type: QACheckType::AccessibilityCheck,
            passed: all_have_alt,
            score: if all_have_alt { 1.0 } else { 0.5 },
            details: if all_have_alt {
                "All media has alt text".to_string()
            } else {
                "Some media missing alt text".to_string()
            },
        }
    }
}

// ============================================================================
// CONTENT PIPELINE ORCHESTRATOR
// ============================================================================

/// Main content pipeline orchestrator
pub struct ContentPipeline {
    pub pipeline_id: Uuid,
    pub signal_ingestor: SignalIngestor,
    pub idea_generator: IdeaGenerator,
    pub script_generator: ScriptGenerator,
    pub image_generator: ImageGenerator,
    pub video_generator: VideoGenerator,
    pub localizer: Localizer,
    pub qa_engine: QAEngine,
    pub jobs: HashMap<Uuid, PipelineJob>,
}

impl ContentPipeline {
    pub fn new() -> Self {
        Self {
            pipeline_id: Uuid::new_v4(),
            signal_ingestor: SignalIngestor::new(),
            idea_generator: IdeaGenerator::new(),
            script_generator: ScriptGenerator::new(),
            image_generator: ImageGenerator::new(),
            video_generator: VideoGenerator::new(),
            localizer: Localizer::new(),
            qa_engine: QAEngine::new(),
            jobs: HashMap::new(),
        }
    }

    /// Start a new pipeline job
    pub fn start_job(&mut self, content_id: Uuid) -> Uuid {
        let job = PipelineJob {
            job_id: Uuid::new_v4(),
            content_id,
            created_at: Utc::now(),
            current_stage: PipelineStage::SignalIngestion,
            completed_stages: vec![],
            status: PipelineJobStatus::Queued,
            error: None,
            estimated_completion: Some(Utc::now() + chrono::Duration::hours(2)),
            total_cost_usd: 0.0,
            metrics: PipelineMetrics::default(),
        };

        let job_id = job.job_id;
        self.jobs.insert(job_id, job);
        job_id
    }

    /// Get job status
    pub fn get_job_status(&self, job_id: &Uuid) -> Option<&PipelineJob> {
        self.jobs.get(job_id)
    }

    /// Run full pipeline for a signal
    pub async fn process_signal(&mut self, signal: Signal) -> Result<Vec<PipelineJob>, String> {
        // Generate ideas from signal
        let ideas = self.idea_generator.generate_ideas(&signal).await;

        let mut jobs = Vec::new();

        for idea in ideas {
            // Start job for each idea
            let job_id = self.start_job(idea.idea_id);

            // In production, would run async pipeline stages
            // For now, just mark job as queued
            if let Some(job) = self.jobs.get_mut(&job_id) {
                job.status = PipelineJobStatus::InProgress;
                jobs.push(job.clone());
            }
        }

        Ok(jobs)
    }
}

// ============================================================================
// UNIT TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_ingestor_creation() {
        let ingestor = SignalIngestor::new();
        assert!(!ingestor.sources.is_empty());
        assert!(ingestor.sources.iter().any(|s| s.source_type == SignalType::TwitterTrend));
    }

    #[tokio::test]
    async fn test_idea_generator() {
        let generator = IdeaGenerator::new();
        
        let signal = Signal {
            signal_id: Uuid::new_v4(),
            signal_type: SignalType::TwitterTrend,
            source: "Twitter".to_string(),
            title: "Blockchain scaling breakthrough".to_string(),
            description: "New consensus mechanism achieves 1M TPS".to_string(),
            url: None,
            detected_at: Utc::now(),
            relevance_score: 0.85,
            trend_momentum: 0.72,
            confidence: 0.9,
            keywords: vec![],
            related_topics: vec![],
            sentiment: Some(0.6),
            regional_relevance: HashMap::new(),
        };

        let ideas = generator.generate_ideas(&signal).await;
        assert!(!ideas.is_empty());
        assert!(ideas.len() <= generator.max_ideas_per_signal);
    }

    #[tokio::test]
    async fn test_script_generator() {
        let generator = ScriptGenerator::new();

        let idea = ContentIdea {
            idea_id: Uuid::new_v4(),
            created_at: Utc::now(),
            signal_source: SignalSource {
                source_type: SignalType::TwitterTrend,
                reference_url: None,
                detected_at: Utc::now(),
                trend_momentum: 0.7,
                confidence: 0.8,
            },
            content_type: ContentType::Tweet,
            priority_score: 0.8,
            novelty_score: 0.7,
            alignment_score: 0.9,
            urgency: ContentUrgency::High,
            target_audience: TargetAudience::Developers,
            suggested_tones: vec![ContentTone::Technical],
            suggested_angles: vec![ContentAngle {
                angle_id: Uuid::new_v4(),
                angle_type: AngleType::DeveloperFirst,
                hook: "New scaling solution".to_string(),
                body_outline: "Technical details about scaling".to_string(),
                cta_suggestion: Some("Try it".to_string()),
                engagement_prediction: 0.065,
            }],
            regional_fit: HashMap::new(),
            estimated_effort_hours: 2.0,
            status: IdeaStatus::Approved,
            rejection_reason: None,
            approved_by: Some("admin".to_string()),
            approved_at: Some(Utc::now()),
        };

        let scripts = generator.generate_scripts(
            &idea,
            &[MarketingPlatform::Twitter],
            &[Language::English],
        ).await;

        assert!(!scripts.is_empty());
        assert!(!scripts[0].hook_variants.is_empty());
    }

    #[tokio::test]
    async fn test_image_generator() {
        let generator = ImageGenerator::new();

        let request = ImageGenerationRequest {
            request_id: Uuid::new_v4(),
            prompt: "Abstract blockchain visualization".to_string(),
            negative_prompt: None,
            style: ImageStyle::Abstract,
            dimensions: ImageDimensions::square_1024(),
            variants: 3,
            seed: None,
        };

        let images = generator.generate_images(&request).await;
        assert_eq!(images.len(), 3);
        assert!(images.iter().all(|i| i.media_type == MediaType::Image));
    }

    #[tokio::test]
    async fn test_qa_engine_disclosure_check() {
        let engine = QAEngine::new();

        // Missing disclosure should fail
        let request_no_disclosure = QACheckRequest {
            content_id: Uuid::new_v4(),
            text: "Great content here".to_string(),
            platform: MarketingPlatform::Twitter,
            language: Language::English,
            content_type: ContentType::Tweet,
            media_assets: vec![],
            disclosure: DisclosureInfo {
                ai_generated: true,
                ai_assisted: true,
                disclosure_text: "".to_string(), // EMPTY!
                models_used: vec![],
                human_reviewed: false,
                reviewer: None,
                reviewed_at: None,
            },
        };

        let result = engine.check_quality(&request_no_disclosure).await;
        assert!(!result.passed);
        assert!(!result.blocked_reasons.is_empty());
    }

    #[test]
    fn test_image_dimensions() {
        let square = ImageDimensions::square_1024();
        assert_eq!(square.width, 1024);
        assert_eq!(square.height, 1024);

        let landscape = ImageDimensions::landscape_16_9();
        assert_eq!(landscape.width, 1920);
        assert_eq!(landscape.height, 1080);
    }

    #[test]
    fn test_pipeline_job_creation() {
        let mut pipeline = ContentPipeline::new();
        let job_id = pipeline.start_job(Uuid::new_v4());

        let job = pipeline.get_job_status(&job_id);
        assert!(job.is_some());
        assert_eq!(job.unwrap().current_stage, PipelineStage::SignalIngestion);
    }
}
