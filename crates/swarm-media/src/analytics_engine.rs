// ============================================================================
// X3 ATLAS SPHERE - ANALYTICS & OPTIMIZATION ENGINE
// Real-time metrics, A/B testing, and performance optimization
// ============================================================================

use crate::marketing_agents::MarketingPlatform;
use crate::swarm_core::{ContentType, Language, Region};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use uuid::Uuid;

// ============================================================================
// METRICS TYPES
// ============================================================================

/// Core engagement metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EngagementMetrics {
    pub impressions: u64,
    pub reach: u64,
    pub engagement: u64,
    pub likes: u64,
    pub comments: u64,
    pub shares: u64,
    pub saves: Option<u64>,
    pub clicks: Option<u64>,
    pub video_views: Option<u64>,
    pub video_watch_time_seconds: Option<u64>,
    pub profile_visits: Option<u64>,
    pub follows: Option<u64>,
}

impl EngagementMetrics {
    pub fn engagement_rate(&self) -> f32 {
        if self.impressions == 0 {
            return 0.0;
        }
        self.engagement as f32 / self.impressions as f32
    }

    pub fn click_through_rate(&self) -> Option<f32> {
        self.clicks.map(|c| {
            if self.impressions == 0 {
                0.0
            } else {
                c as f32 / self.impressions as f32
            }
        })
    }

    pub fn video_completion_rate(&self) -> Option<f32> {
        match (self.video_views, self.video_watch_time_seconds) {
            (Some(views), Some(watch_time)) if views > 0 => {
                // Assuming average video is 60 seconds
                Some((watch_time as f32 / views as f32) / 60.0)
            }
            _ => None,
        }
    }
}

/// Sentiment breakdown
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SentimentMetrics {
    pub positive: f32,
    pub neutral: f32,
    pub negative: f32,
    pub sample_size: u32,
    pub top_positive_keywords: Vec<String>,
    pub top_negative_keywords: Vec<String>,
}

impl SentimentMetrics {
    pub fn sentiment_score(&self) -> f32 {
        // -1.0 (all negative) to 1.0 (all positive)
        self.positive - self.negative
    }

    pub fn is_concerning(&self) -> bool {
        self.negative > 0.2 || self.sentiment_score() < -0.1
    }
}

/// Conversion metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConversionMetrics {
    pub conversions: u64,
    pub conversion_type: String,
    pub conversion_value: f64,
    pub cost_per_conversion: f64,
    pub return_on_ad_spend: f64,
}

/// Time-series data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsDataPoint {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
    pub label: Option<String>,
}

/// Content performance record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentPerformance {
    pub content_id: Uuid,
    pub platform: MarketingPlatform,
    pub content_type: ContentType,
    pub language: Language,
    pub region: Option<Region>,
    pub published_at: DateTime<Utc>,
    pub metrics: EngagementMetrics,
    pub sentiment: SentimentMetrics,
    pub conversions: ConversionMetrics,
    pub time_series: Vec<MetricsDataPoint>,
    pub performance_score: f32,
    pub vs_platform_average: f32,
    pub vs_similar_content: f32,
    pub decay_detected: bool,
    pub decay_started_at: Option<DateTime<Utc>>,
}

impl ContentPerformance {
    pub fn new(
        content_id: Uuid,
        platform: MarketingPlatform,
        content_type: ContentType,
        language: Language,
    ) -> Self {
        Self {
            content_id,
            platform,
            content_type,
            language,
            region: None,
            published_at: Utc::now(),
            metrics: EngagementMetrics::default(),
            sentiment: SentimentMetrics::default(),
            conversions: ConversionMetrics::default(),
            time_series: Vec::new(),
            performance_score: 0.0,
            vs_platform_average: 1.0,
            vs_similar_content: 1.0,
            decay_detected: false,
            decay_started_at: None,
        }
    }

    pub fn is_viral(&self) -> bool {
        self.vs_platform_average > 5.0 || self.metrics.engagement_rate() > 0.1
    }

    pub fn needs_optimization(&self) -> bool {
        self.performance_score < 0.5 || self.vs_platform_average < 0.5
    }
}

// ============================================================================
// PLATFORM ANALYTICS
// ============================================================================

/// Platform-level analytics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformAnalytics {
    pub platform: MarketingPlatform,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    
    // Aggregate metrics
    pub total_impressions: u64,
    pub total_engagement: u64,
    pub avg_engagement_rate: f32,
    pub follower_count: u64,
    pub follower_growth: i64,
    pub follower_growth_rate: f32,
    
    // Content metrics
    pub posts_count: u32,
    pub avg_post_engagement: f32,
    pub best_performing_content_type: ContentType,
    pub best_posting_times: Vec<u32>, // Hours in UTC
    pub top_hashtags: Vec<(String, u32)>,
    
    // Audience insights
    pub audience_demographics: AudienceDemographics,
    pub audience_active_hours: Vec<u32>,
    
    // Health metrics
    pub rate_limit_hits: u32,
    pub api_errors: u32,
    pub compliance_warnings: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AudienceDemographics {
    pub age_groups: HashMap<String, f32>, // "18-24" -> 0.25
    pub gender: HashMap<String, f32>,
    pub top_countries: Vec<(String, f32)>,
    pub top_cities: Vec<(String, f32)>,
    pub interests: Vec<(String, f32)>,
}

// ============================================================================
// A/B TESTING
// ============================================================================

/// A/B test definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ABTest {
    pub test_id: Uuid,
    pub name: String,
    pub description: String,
    pub test_type: ABTestType,
    pub status: ABTestStatus,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub ended_at: Option<DateTime<Utc>>,
    
    // Variants
    pub control: TestVariant,
    pub variants: Vec<TestVariant>,
    
    // Configuration
    pub sample_size_target: u32,
    pub min_duration_hours: u32,
    pub max_duration_hours: u32,
    pub confidence_threshold: f32, // 0.95 for 95% confidence
    
    // Results
    pub winner: Option<String>,
    pub winner_lift: Option<f32>,
    pub statistical_significance: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ABTestType {
    HookVariant,
    CTAVariant,
    TimingVariant,
    ToneVariant,
    FormatVariant,
    MediaVariant,
    HashtagVariant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ABTestStatus {
    Draft,
    Running,
    Paused,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestVariant {
    pub variant_id: String, // "control", "A", "B", "C"
    pub name: String,
    pub description: String,
    pub content_ids: Vec<Uuid>,
    pub sample_size: u32,
    pub metrics: VariantMetrics,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VariantMetrics {
    pub impressions: u64,
    pub engagements: u64,
    pub clicks: u64,
    pub conversions: u64,
    pub engagement_rate: f32,
    pub click_through_rate: f32,
    pub conversion_rate: f32,
}

impl ABTest {
    pub fn new(name: String, test_type: ABTestType, control: TestVariant) -> Self {
        Self {
            test_id: Uuid::new_v4(),
            name,
            description: String::new(),
            test_type,
            status: ABTestStatus::Draft,
            created_at: Utc::now(),
            started_at: None,
            ended_at: None,
            control,
            variants: Vec::new(),
            sample_size_target: 1000,
            min_duration_hours: 24,
            max_duration_hours: 168, // 1 week
            confidence_threshold: 0.95,
            winner: None,
            winner_lift: None,
            statistical_significance: None,
        }
    }

    pub fn add_variant(&mut self, variant: TestVariant) {
        self.variants.push(variant);
    }

    pub fn start(&mut self) {
        self.status = ABTestStatus::Running;
        self.started_at = Some(Utc::now());
    }

    pub fn should_end(&self) -> bool {
        if self.status != ABTestStatus::Running {
            return false;
        }

        let started = self.started_at.unwrap_or(Utc::now());
        let elapsed_hours = (Utc::now() - started).num_hours() as u32;

        // Check minimum duration
        if elapsed_hours < self.min_duration_hours {
            return false;
        }

        // Check sample size
        let total_samples: u32 = self.control.sample_size
            + self.variants.iter().map(|v| v.sample_size).sum::<u32>();

        if total_samples >= self.sample_size_target {
            return true;
        }

        // Check max duration
        elapsed_hours >= self.max_duration_hours
    }

    /// Calculate statistical significance using chi-squared approximation
    pub fn calculate_significance(&mut self) -> f32 {
        let control_rate = self.control.metrics.engagement_rate;
        let best_variant = self.variants.iter()
            .max_by(|a, b| a.metrics.engagement_rate.partial_cmp(&b.metrics.engagement_rate).unwrap());

        if let Some(variant) = best_variant {
            let variant_rate = variant.metrics.engagement_rate;
            
            // Simplified significance calculation
            // In production, would use proper statistical test
            let lift = (variant_rate - control_rate) / control_rate;
            let n1 = self.control.sample_size as f32;
            let n2 = variant.sample_size as f32;
            
            // Z-score approximation
            let pooled_rate = (control_rate * n1 + variant_rate * n2) / (n1 + n2);
            let se = (pooled_rate * (1.0 - pooled_rate) * (1.0/n1 + 1.0/n2)).sqrt();
            
            let z_score = if se > 0.0 {
                (variant_rate - control_rate).abs() / se
            } else {
                0.0
            };

            // Convert to p-value approximation
            let significance = 1.0 - (-z_score * z_score / 2.0).exp() * (0.5 + 0.5 * (1.0 + (z_score / 1.4142135).tanh()));

            self.statistical_significance = Some(significance);

            if significance >= self.confidence_threshold {
                self.winner = Some(variant.variant_id.clone());
                self.winner_lift = Some(lift);
            }

            significance
        } else {
            0.0
        }
    }
}

// ============================================================================
// CONTENT DECAY DETECTION
// ============================================================================

/// Decay detection for content performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecayDetector {
    pub detector_id: Uuid,
    pub decay_threshold: f32,       // Drop from peak to trigger decay (e.g., 0.7 = 70% drop)
    pub window_hours: u32,          // Time window for comparison
    pub min_data_points: usize,     // Minimum data points needed
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecayAnalysis {
    pub content_id: Uuid,
    pub analyzed_at: DateTime<Utc>,
    pub is_decaying: bool,
    pub decay_rate: f32,            // Percentage drop per hour
    pub peak_engagement: u64,
    pub current_engagement: u64,
    pub peak_timestamp: DateTime<Utc>,
    pub estimated_end_of_life: Option<DateTime<Utc>>,
    pub recommendation: DecayRecommendation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DecayRecommendation {
    KeepActive,
    RefreshHook,
    Repost,
    Archive,
    Boost,
}

impl DecayDetector {
    pub fn new() -> Self {
        Self {
            detector_id: Uuid::new_v4(),
            decay_threshold: 0.7,   // 70% drop from peak
            window_hours: 24,
            min_data_points: 6,
        }
    }

    pub fn analyze(&self, performance: &ContentPerformance) -> DecayAnalysis {
        let data_points = &performance.time_series;

        if data_points.len() < self.min_data_points {
            return DecayAnalysis {
                content_id: performance.content_id,
                analyzed_at: Utc::now(),
                is_decaying: false,
                decay_rate: 0.0,
                peak_engagement: performance.metrics.engagement,
                current_engagement: performance.metrics.engagement,
                peak_timestamp: performance.published_at,
                estimated_end_of_life: None,
                recommendation: DecayRecommendation::KeepActive,
            };
        }

        // Find peak
        let peak = data_points.iter()
            .max_by(|a, b| a.value.partial_cmp(&b.value).unwrap())
            .unwrap();

        let current = data_points.last().unwrap();

        let drop_ratio = if peak.value > 0.0 {
            1.0 - (current.value / peak.value)
        } else {
            0.0
        };

        let is_decaying = drop_ratio > (1.0 - self.decay_threshold) as f64;

        // Calculate decay rate
        let hours_since_peak = (current.timestamp - peak.timestamp).num_hours().max(1) as f32;
        let decay_rate = drop_ratio as f32 / hours_since_peak;

        // Estimate end of life (when engagement drops to 10% of peak)
        let estimated_eol = if decay_rate > 0.0 && is_decaying {
            let hours_to_10_percent = 0.9 / decay_rate;
            Some(Utc::now() + Duration::hours(hours_to_10_percent as i64))
        } else {
            None
        };

        // Determine recommendation
        let recommendation = if !is_decaying {
            DecayRecommendation::KeepActive
        } else if drop_ratio < 0.5 {
            DecayRecommendation::RefreshHook
        } else if performance.vs_platform_average > 1.2 {
            DecayRecommendation::Repost
        } else if performance.sentiment.is_concerning() {
            DecayRecommendation::Archive
        } else {
            DecayRecommendation::Archive
        };

        DecayAnalysis {
            content_id: performance.content_id,
            analyzed_at: Utc::now(),
            is_decaying,
            decay_rate,
            peak_engagement: peak.value as u64,
            current_engagement: current.value as u64,
            peak_timestamp: peak.timestamp,
            estimated_end_of_life: estimated_eol,
            recommendation,
        }
    }
}

// ============================================================================
// OPTIMIZATION ENGINE
// ============================================================================

/// Content optimization suggestions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationSuggestion {
    pub suggestion_id: Uuid,
    pub content_id: Uuid,
    pub suggestion_type: OptimizationType,
    pub priority: OptimizationPriority,
    pub title: String,
    pub description: String,
    pub expected_lift: f32,
    pub effort_level: EffortLevel,
    pub created_at: DateTime<Utc>,
    pub applied_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptimizationType {
    HookImprovement,
    CTAImprovement,
    TimingAdjustment,
    HashtagOptimization,
    MediaEnhancement,
    TargetingRefinement,
    ToneAdjustment,
    LengthOptimization,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptimizationPriority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffortLevel {
    Minimal,    // Automated
    Low,        // Quick manual edit
    Medium,     // Some work required
    High,       // Significant effort
}

/// Optimization engine
pub struct OptimizationEngine {
    pub engine_id: Uuid,
    pub learnings: OptimizationLearnings,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OptimizationLearnings {
    pub best_hooks_by_platform: HashMap<MarketingPlatform, Vec<String>>,
    pub best_ctas_by_audience: HashMap<String, Vec<String>>,
    pub best_posting_times: HashMap<MarketingPlatform, Vec<u32>>,
    pub best_hashtags: HashMap<MarketingPlatform, Vec<String>>,
    pub optimal_content_length: HashMap<MarketingPlatform, (usize, usize)>, // (min, max)
}

impl OptimizationEngine {
    pub fn new() -> Self {
        Self {
            engine_id: Uuid::new_v4(),
            learnings: OptimizationLearnings::default(),
        }
    }

    /// Analyze content and generate optimization suggestions
    pub fn analyze_and_suggest(&self, performance: &ContentPerformance) -> Vec<OptimizationSuggestion> {
        let mut suggestions = Vec::new();

        // Check engagement rate vs platform average
        if performance.vs_platform_average < 0.8 {
            suggestions.push(OptimizationSuggestion {
                suggestion_id: Uuid::new_v4(),
                content_id: performance.content_id,
                suggestion_type: OptimizationType::HookImprovement,
                priority: OptimizationPriority::High,
                title: "Hook underperforming".to_string(),
                description: "Content is getting 20%+ less engagement than platform average. Consider testing a new hook.".to_string(),
                expected_lift: 0.15,
                effort_level: EffortLevel::Low,
                created_at: Utc::now(),
                applied_at: None,
            });
        }

        // Check click-through rate
        if let Some(ctr) = performance.metrics.click_through_rate() {
            if ctr < 0.01 {
                suggestions.push(OptimizationSuggestion {
                    suggestion_id: Uuid::new_v4(),
                    content_id: performance.content_id,
                    suggestion_type: OptimizationType::CTAImprovement,
                    priority: OptimizationPriority::Medium,
                    title: "Low click-through rate".to_string(),
                    description: "CTR is below 1%. Test different CTAs or add clearer value proposition.".to_string(),
                    expected_lift: 0.25,
                    effort_level: EffortLevel::Low,
                    created_at: Utc::now(),
                    applied_at: None,
                });
            }
        }

        // Check sentiment
        if performance.sentiment.is_concerning() {
            suggestions.push(OptimizationSuggestion {
                suggestion_id: Uuid::new_v4(),
                content_id: performance.content_id,
                suggestion_type: OptimizationType::ToneAdjustment,
                priority: OptimizationPriority::High,
                title: "Negative sentiment detected".to_string(),
                description: format!(
                    "{}% negative sentiment. Review comments and consider tone adjustment.",
                    (performance.sentiment.negative * 100.0) as u32
                ),
                expected_lift: 0.10,
                effort_level: EffortLevel::Medium,
                created_at: Utc::now(),
                applied_at: None,
            });
        }

        // Check decay
        if performance.decay_detected {
            suggestions.push(OptimizationSuggestion {
                suggestion_id: Uuid::new_v4(),
                content_id: performance.content_id,
                suggestion_type: OptimizationType::TimingAdjustment,
                priority: OptimizationPriority::Low,
                title: "Content decay detected".to_string(),
                description: "Engagement has dropped significantly. Consider reposting with fresh hook or archiving.".to_string(),
                expected_lift: 0.30,
                effort_level: EffortLevel::Minimal,
                created_at: Utc::now(),
                applied_at: None,
            });
        }

        suggestions
    }

    /// Update learnings from successful content
    pub fn learn_from_success(&mut self, performance: &ContentPerformance) {
        // Only learn from high-performing content
        if performance.vs_platform_average <= 1.5 {
            return;
        }

        // Extract and store patterns
        // In production, would do more sophisticated analysis
        let platform = performance.platform;

        // Update best posting times
        let hour = performance.published_at.format("%H").to_string().parse::<u32>().unwrap_or(12);
        let times = self.learnings.best_posting_times.entry(platform).or_insert_with(Vec::new);
        if !times.contains(&hour) && times.len() < 5 {
            times.push(hour);
        }
    }

    /// Get best practices for a platform
    pub fn get_best_practices(&self, platform: &MarketingPlatform) -> serde_json::Value {
        serde_json::json!({
            "platform": format!("{:?}", platform),
            "best_posting_times": self.learnings.best_posting_times.get(platform).unwrap_or(&vec![9, 12, 17]),
            "optimal_length": self.learnings.optimal_content_length.get(platform),
            "top_hashtags": self.learnings.best_hashtags.get(platform).unwrap_or(&vec![]),
            "recommended_post_frequency": match platform {
                MarketingPlatform::Twitter => "3-5 per day",
                MarketingPlatform::LinkedIn => "1-2 per day",
                MarketingPlatform::Instagram => "1-2 per day",
                MarketingPlatform::TikTok => "1-3 per day",
                MarketingPlatform::YouTube => "2-3 per week",
                _ => "1 per day",
            }
        })
    }
}

// ============================================================================
// ANALYTICS DASHBOARD
// ============================================================================

/// Analytics dashboard aggregation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsDashboard {
    pub generated_at: DateTime<Utc>,
    pub period: AnalyticsPeriod,
    
    // Overall metrics
    pub total_impressions: u64,
    pub total_engagement: u64,
    pub total_reach: u64,
    pub avg_engagement_rate: f32,
    pub total_conversions: u64,
    pub total_spend: f64,
    pub roi: f64,
    
    // Platform breakdown
    pub platform_metrics: HashMap<MarketingPlatform, PlatformSummary>,
    
    // Regional breakdown
    pub regional_metrics: HashMap<Region, RegionalSummary>,
    
    // Top performers
    pub top_content: Vec<ContentPerformance>,
    pub viral_content: Vec<ContentPerformance>,
    pub underperformers: Vec<ContentPerformance>,
    
    // Active tests
    pub active_ab_tests: Vec<ABTest>,
    
    // Optimization suggestions
    pub pending_optimizations: Vec<OptimizationSuggestion>,
    
    // Health
    pub system_health: SystemHealthMetrics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnalyticsPeriod {
    Last24Hours,
    Last7Days,
    Last30Days,
    LastQuarter,
    AllTime,
    Custom,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlatformSummary {
    pub platform: String,
    pub impressions: u64,
    pub engagement: u64,
    pub engagement_rate: f32,
    pub follower_growth: i64,
    pub posts_count: u32,
    pub top_post_id: Option<Uuid>,
    pub api_health: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RegionalSummary {
    pub region: String,
    pub impressions: u64,
    pub engagement: u64,
    pub engagement_rate: f32,
    pub top_language: String,
    pub growth_rate: f32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SystemHealthMetrics {
    pub api_uptime_percent: f32,
    pub rate_limit_usage: f32,
    pub budget_usage: f32,
    pub active_agents: u32,
    pub queued_tasks: u32,
    pub errors_24h: u32,
    pub compliance_score: f32,
}

impl AnalyticsDashboard {
    pub fn new(period: AnalyticsPeriod) -> Self {
        Self {
            generated_at: Utc::now(),
            period,
            total_impressions: 0,
            total_engagement: 0,
            total_reach: 0,
            avg_engagement_rate: 0.0,
            total_conversions: 0,
            total_spend: 0.0,
            roi: 0.0,
            platform_metrics: HashMap::new(),
            regional_metrics: HashMap::new(),
            top_content: Vec::new(),
            viral_content: Vec::new(),
            underperformers: Vec::new(),
            active_ab_tests: Vec::new(),
            pending_optimizations: Vec::new(),
            system_health: SystemHealthMetrics::default(),
        }
    }

    /// Get summary for executive reporting
    pub fn executive_summary(&self) -> serde_json::Value {
        serde_json::json!({
            "period": format!("{:?}", self.period),
            "total_reach": self.total_reach,
            "engagement_rate": format!("{:.2}%", self.avg_engagement_rate * 100.0),
            "conversions": self.total_conversions,
            "roi": format!("{:.1}x", self.roi),
            "top_platform": self.platform_metrics.iter()
                .max_by(|a, b| a.1.engagement.cmp(&b.1.engagement))
                .map(|(p, _)| format!("{:?}", p)),
            "viral_posts": self.viral_content.len(),
            "health_score": self.system_health.compliance_score,
        })
    }
}

// ============================================================================
// ANALYTICS MANAGER
// ============================================================================

/// Central analytics management
pub struct AnalyticsManager {
    pub manager_id: Uuid,
    pub content_performance: HashMap<Uuid, ContentPerformance>,
    pub platform_analytics: HashMap<MarketingPlatform, PlatformAnalytics>,
    pub ab_tests: HashMap<Uuid, ABTest>,
    pub decay_detector: DecayDetector,
    pub optimization_engine: OptimizationEngine,
    pub metrics_history: VecDeque<AnalyticsDashboard>,
}

impl AnalyticsManager {
    pub fn new() -> Self {
        Self {
            manager_id: Uuid::new_v4(),
            content_performance: HashMap::new(),
            platform_analytics: HashMap::new(),
            ab_tests: HashMap::new(),
            decay_detector: DecayDetector::new(),
            optimization_engine: OptimizationEngine::new(),
            metrics_history: VecDeque::with_capacity(30), // Keep 30 days
        }
    }

    /// Track new content
    pub fn track_content(&mut self, content_id: Uuid, platform: MarketingPlatform, content_type: ContentType, language: Language) {
        let performance = ContentPerformance::new(content_id, platform, content_type, language);
        self.content_performance.insert(content_id, performance);
    }

    /// Update metrics for content
    pub fn update_metrics(&mut self, content_id: Uuid, metrics: EngagementMetrics) {
        if let Some(performance) = self.content_performance.get_mut(&content_id) {
            performance.metrics = metrics.clone();

            // Add time series data point
            performance.time_series.push(MetricsDataPoint {
                timestamp: Utc::now(),
                value: metrics.engagement as f64,
                label: None,
            });

            // Check for decay
            let decay_analysis = self.decay_detector.analyze(performance);
            performance.decay_detected = decay_analysis.is_decaying;
            performance.decay_started_at = if decay_analysis.is_decaying {
                Some(Utc::now())
            } else {
                None
            };

            // Update platform averages
            self.update_platform_averages(performance.platform);
        }
    }

    fn update_platform_averages(&mut self, platform: MarketingPlatform) {
        let platform_content: Vec<_> = self.content_performance.values()
            .filter(|p| p.platform == platform)
            .collect();

        if platform_content.is_empty() {
            return;
        }

        let avg_rate: f32 = platform_content.iter()
            .map(|p| p.metrics.engagement_rate())
            .sum::<f32>() / platform_content.len() as f32;

        // Update vs_platform_average for all content
        for content_id in platform_content.iter().map(|p| p.content_id).collect::<Vec<_>>() {
            if let Some(perf) = self.content_performance.get_mut(&content_id) {
                perf.vs_platform_average = if avg_rate > 0.0 {
                    perf.metrics.engagement_rate() / avg_rate
                } else {
                    1.0
                };
            }
        }
    }

    /// Create a new A/B test
    pub fn create_ab_test(&mut self, name: String, test_type: ABTestType, control: TestVariant) -> Uuid {
        let test = ABTest::new(name, test_type, control);
        let test_id = test.test_id;
        self.ab_tests.insert(test_id, test);
        test_id
    }

    /// Get optimization suggestions for all content
    pub fn get_all_suggestions(&self) -> Vec<OptimizationSuggestion> {
        self.content_performance.values()
            .flat_map(|p| self.optimization_engine.analyze_and_suggest(p))
            .collect()
    }

    /// Generate dashboard
    pub fn generate_dashboard(&self, period: AnalyticsPeriod) -> AnalyticsDashboard {
        let mut dashboard = AnalyticsDashboard::new(period);

        // Aggregate metrics
        for performance in self.content_performance.values() {
            dashboard.total_impressions += performance.metrics.impressions;
            dashboard.total_engagement += performance.metrics.engagement;
            dashboard.total_reach += performance.metrics.reach;

            // Check for viral content
            if performance.is_viral() {
                dashboard.viral_content.push(performance.clone());
            }

            // Check for underperformers
            if performance.needs_optimization() {
                dashboard.underperformers.push(performance.clone());
            }
        }

        // Calculate average engagement rate
        if dashboard.total_impressions > 0 {
            dashboard.avg_engagement_rate = dashboard.total_engagement as f32 / dashboard.total_impressions as f32;
        }

        // Get top content
        dashboard.top_content = self.content_performance.values()
            .cloned()
            .collect::<Vec<_>>();
        dashboard.top_content.sort_by(|a, b| b.metrics.engagement.cmp(&a.metrics.engagement));
        dashboard.top_content.truncate(10);

        // Get active A/B tests
        dashboard.active_ab_tests = self.ab_tests.values()
            .filter(|t| t.status == ABTestStatus::Running)
            .cloned()
            .collect();

        // Get pending optimizations
        dashboard.pending_optimizations = self.get_all_suggestions();

        dashboard
    }
}

// ============================================================================
// UNIT TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engagement_metrics() {
        let metrics = EngagementMetrics {
            impressions: 10000,
            engagement: 500,
            likes: 300,
            comments: 100,
            shares: 100,
            ..Default::default()
        };

        assert_eq!(metrics.engagement_rate(), 0.05);
    }

    #[test]
    fn test_sentiment_metrics() {
        let sentiment = SentimentMetrics {
            positive: 0.7,
            neutral: 0.2,
            negative: 0.1,
            sample_size: 100,
            ..Default::default()
        };

        assert_eq!(sentiment.sentiment_score(), 0.6);
        assert!(!sentiment.is_concerning());

        let concerning = SentimentMetrics {
            positive: 0.3,
            neutral: 0.3,
            negative: 0.4,
            ..Default::default()
        };
        assert!(concerning.is_concerning());
    }

    #[test]
    fn test_ab_test_creation() {
        let control = TestVariant {
            variant_id: "control".to_string(),
            name: "Original Hook".to_string(),
            description: "Testing original hook".to_string(),
            content_ids: vec![],
            sample_size: 0,
            metrics: VariantMetrics::default(),
        };

        let mut test = ABTest::new("Hook Test".to_string(), ABTestType::HookVariant, control);
        
        test.add_variant(TestVariant {
            variant_id: "A".to_string(),
            name: "Question Hook".to_string(),
            description: "Testing question-based hook".to_string(),
            content_ids: vec![],
            sample_size: 0,
            metrics: VariantMetrics::default(),
        });

        assert_eq!(test.variants.len(), 1);
        assert_eq!(test.status, ABTestStatus::Draft);
    }

    #[test]
    fn test_decay_detector() {
        let detector = DecayDetector::new();
        
        let mut performance = ContentPerformance::new(
            Uuid::new_v4(),
            MarketingPlatform::Twitter,
            ContentType::Tweet,
            Language::English,
        );

        // Add time series showing decay
        for i in 0..10 {
            performance.time_series.push(MetricsDataPoint {
                timestamp: Utc::now() - Duration::hours(10 - i),
                value: (100.0 - i as f64 * 8.0).max(10.0),
                label: None,
            });
        }

        let analysis = detector.analyze(&performance);
        assert!(analysis.is_decaying);
    }

    #[test]
    fn test_optimization_suggestions() {
        let engine = OptimizationEngine::new();
        
        let mut performance = ContentPerformance::new(
            Uuid::new_v4(),
            MarketingPlatform::Twitter,
            ContentType::Tweet,
            Language::English,
        );

        performance.vs_platform_average = 0.5; // Underperforming
        
        let suggestions = engine.analyze_and_suggest(&performance);
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.suggestion_type == OptimizationType::HookImprovement));
    }

    #[test]
    fn test_analytics_manager() {
        let mut manager = AnalyticsManager::new();
        
        let content_id = Uuid::new_v4();
        manager.track_content(content_id, MarketingPlatform::Twitter, ContentType::Tweet, Language::English);
        
        let metrics = EngagementMetrics {
            impressions: 5000,
            engagement: 250,
            likes: 200,
            comments: 25,
            shares: 25,
            ..Default::default()
        };
        
        manager.update_metrics(content_id, metrics);
        
        let perf = manager.content_performance.get(&content_id);
        assert!(perf.is_some());
        assert_eq!(perf.unwrap().metrics.impressions, 5000);
    }

    #[test]
    fn test_dashboard_generation() {
        let manager = AnalyticsManager::new();
        let dashboard = manager.generate_dashboard(AnalyticsPeriod::Last24Hours);
        
        assert!(dashboard.generated_at <= Utc::now());
        assert_eq!(dashboard.period, AnalyticsPeriod::Last24Hours);
    }
}
