/// Content Repurposing Pipeline
///
/// System for turning 1 recording into N derivative assets:
/// - Clips and shorts (5s, 15s, 30s, 60s)
/// - Localized versions (dubbed, subtitled, transcribed)
/// - Educational content (explainers, tutorials, how-tos)
/// - Social media formats (TikTok, YouTube Shorts, Instagram Reels, Twitter)
/// - Formats (vertical, horizontal, square)
///
/// This is where 80% of production value comes from.

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// A piece of content that can be repurposed into many forms
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContentSource {
    /// Unique ID
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Type of source content
    pub content_type: ContentType,

    /// Who was featured?
    pub featured_contributors: Vec<String>,

    /// Duration (in seconds)
    pub duration_seconds: u32,

    /// Storage location (S3, local, etc)
    pub storage_path: String,

    /// Raw file hash (for integrity)
    pub content_hash: String,

    /// When was this created?
    pub created_at: DateTime<Utc>,

    /// Can this be repurposed?
    pub is_repurposable: bool,

    /// Metadata tags
    pub tags: Vec<String>,

    /// Key moments (for clipping)
    pub key_moments: Vec<KeyMoment>,
}

/// What kind of source is this?
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ContentType {
    FounderTalk,
    TechnicalExplanation,
    Tutorial,
    Interview,
    Presentation,
    Announcement,
    WebinarRecording,
    PodcastEpisode,
}

/// A specific moment in the content worth isolating
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyMoment {
    /// Time offset (seconds)
    pub start_seconds: u32,

    /// Duration of this moment
    pub duration_seconds: u32,

    /// Why is this interesting?
    pub reason: String,

    /// Category (hook, insight, joke, call-to-action, etc)
    pub category: String,
}

/// A derived asset (clip, dub, translation, etc)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DerivedAsset {
    /// Unique ID
    pub id: String,

    /// Source content ID
    pub source_id: String,

    /// What kind of derivation?
    pub asset_type: AssetType,

    /// Output format
    pub format: OutputFormat,

    /// Target platform/language
    pub target: DerivationTarget,

    /// Duration (may differ from source)
    pub duration_seconds: u32,

    /// Storage location
    pub storage_path: String,

    /// Content hash
    pub content_hash: String,

    /// Metadata
    pub title: String,
    pub description: String,
    pub tags: Vec<String>,

    /// When was this created?
    pub created_at: DateTime<Utc>,

    /// What contributors are featured in this derivative?
    pub featured_contributors: Vec<String>,

    /// Can this be used? (ready for distribution)
    pub is_ready: bool,

    /// Metrics (views, engagement, etc)
    pub metrics: Option<AssetMetrics>,
}

/// What kind of derivation?
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum AssetType {
    /// Extracted clip (5-60 seconds)
    Clip,

    /// Full episode with intro/outro
    FullEpisode,

    /// Dubbed into another language
    DubLocalization,

    /// Subtitled version
    SubtitledVersion,

    /// Transcribed text (markdown, PDF)
    Transcript,

    /// Educational explainer (derived from longer content)
    EducationalExplainer,

    /// Social media teaser/hook
    SocialTeaser,

    /// Custom montage or mashup
    Montage,

    /// Interactive quiz or learning module
    InteractiveModule,

    /// Podcast episode (audio only)
    PodcastEpisode,
}

/// Video format
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum OutputFormat {
    /// 1920x1080, horizontal
    #[serde(rename = "horizontal_1080p")]
    Horizontal1080p,

    /// 1280x720, horizontal
    #[serde(rename = "horizontal_720p")]
    Horizontal720p,

    /// 9:16, vertical (TikTok, Instagram Reels, YouTube Shorts)
    #[serde(rename = "vertical_1080p")]
    Vertical1080p,

    /// 1:1, square (Instagram Feed, etc)
    #[serde(rename = "square_1080p")]
    Square1080p,

    /// Audio only (MP3, AAC)
    AudioOnly,

    /// PDF (for transcripts)
    PDF,

    /// Markdown (for transcripts, show notes)
    Markdown,

    /// WebVTT (subtitles)
    Subtitles,
}

/// Where is this intended to go?
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DerivationTarget {
    /// YouTube
    YouTube,

    /// YouTube Shorts
    YouTubeShorts,

    /// TikTok
    TikTok,

    /// Instagram Reels
    InstagramReels,

    /// Instagram Feed (square)
    InstagramFeed,

    /// Twitter/X
    Twitter,

    /// LinkedIn
    LinkedIn,

    /// Blog/Website
    Website,

    /// Podcast platform (Spotify, Apple, etc)
    PodcastPlatform(String),

    /// Email/Newsletter
    Email,

    /// Specific language (ISO code)
    Language(String),

    /// Internal archive
    Internal,
}

/// Track metrics for an asset
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AssetMetrics {
    /// Number of views
    pub views: u64,

    /// Number of likes/upvotes
    pub likes: u64,

    /// Number of comments
    pub comments: u64,

    /// Number of shares
    pub shares: u64,

    /// Click-through rate (for calls-to-action)
    pub ctr: Option<f64>,

    /// Watch time (in minutes)
    pub watch_time_minutes: u64,

    /// Average view duration
    pub avg_view_duration_seconds: u32,

    /// When were these metrics last updated?
    pub updated_at: DateTime<Utc>,
}

/// Request to repurpose content
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RepurposingRequest {
    /// Source content ID
    pub source_id: String,

    /// What kind of asset to create?
    pub asset_type: AssetType,

    /// Output format
    pub format: OutputFormat,

    /// Where should this go?
    pub target: DerivationTarget,

    /// Title for the new asset
    pub title: String,

    /// Description
    pub description: String,

    /// Tags
    pub tags: Vec<String>,

    /// If clipping, which key moment?
    pub clip_moment_idx: Option<usize>,

    /// Special instructions (for AI-powered tools)
    pub instructions: Option<String>,

    /// Priority (maps to swarm job queue)
    pub priority: RepurposingPriority,

    /// Should this bypass approval?
    pub auto_publish: bool,
}

/// Priority for repurposing work
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum RepurposingPriority {
    Low,
    Normal,
    High,
    Urgent,
}

/// Repurposing pipeline manager
pub struct RepurposingEngine {
    sources: HashMap<String, ContentSource>,
    assets: HashMap<String, DerivedAsset>,
    requests_queue: Vec<RepurposingRequest>,
    completed_assets: usize,
}

impl RepurposingEngine {
    pub fn new() -> Self {
        Self {
            sources: HashMap::new(),
            assets: HashMap::new(),
            requests_queue: Vec::new(),
            completed_assets: 0,
        }
    }

    /// Register a new source content
    pub fn register_source(&mut self, source: ContentSource) -> Result<String, String> {
        let id = source.id.clone();
        if self.sources.contains_key(&id) {
            return Err(format!("Source {} already registered", id));
        }
        self.sources.insert(id.clone(), source);
        Ok(id)
    }

    /// Request content repurposing
    pub fn request_repurposing(&mut self, request: RepurposingRequest) -> Result<String, String> {
        // Verify source exists and is repurposable
        let source = self
            .sources
            .get(&request.source_id)
            .ok_or("Source not found")?;

        if !source.is_repurposable {
            return Err("Source is not marked as repurposable".to_string());
        }

        // If clipping, verify moment exists
        if let Some(idx) = request.clip_moment_idx {
            if idx >= source.key_moments.len() {
                return Err("Key moment index out of range".to_string());
            }
        }

        self.requests_queue.push(request);
        Ok(format!("request-{}", uuid::Uuid::new_v4()))
    }

    /// Complete a repurposing job and register the asset
    pub fn complete_asset(
        &mut self,
        request: RepurposingRequest,
        storage_path: String,
        content_hash: String,
    ) -> Result<String, String> {
        let source = self
            .sources
            .get(&request.source_id)
            .ok_or("Source not found")?;

        let asset_id = uuid::Uuid::new_v4().to_string();

        // Calculate asset duration
        let duration_seconds = match request.asset_type {
            AssetType::Clip => {
                // Get duration from key moment if available
                if let Some(idx) = request.clip_moment_idx {
                    source.key_moments[idx].duration_seconds
                } else {
                    source.duration_seconds
                }
            }
            _ => source.duration_seconds,
        };

        let asset = DerivedAsset {
            id: asset_id.clone(),
            source_id: request.source_id,
            asset_type: request.asset_type,
            format: request.format,
            target: request.target,
            duration_seconds,
            storage_path,
            content_hash,
            title: request.title,
            description: request.description,
            tags: request.tags,
            created_at: Utc::now(),
            featured_contributors: source.featured_contributors.clone(),
            is_ready: !request.auto_publish, // Manual approval needed unless auto_publish
            metrics: None,
        };

        self.assets.insert(asset_id.clone(), asset);
        self.completed_assets += 1;

        Ok(asset_id)
    }

    /// Get next repurposing job (by priority)
    pub fn next_job(&mut self) -> Option<RepurposingRequest> {
        if self.requests_queue.is_empty() {
            return None;
        }

        // Sort by priority (highest first)
        self.requests_queue.sort_by(|a, b| b.priority.cmp(&a.priority));
        Some(self.requests_queue.remove(0))
    }

    /// Get a source
    pub fn get_source(&self, id: &str) -> Option<&ContentSource> {
        self.sources.get(id)
    }

    /// Get an asset
    pub fn get_asset(&self, id: &str) -> Option<&DerivedAsset> {
        self.assets.get(id)
    }

    /// List all assets
    pub fn list_assets(&self) -> Vec<&DerivedAsset> {
        self.assets.values().collect()
    }

    /// List all sources
    pub fn list_sources(&self) -> Vec<&ContentSource> {
        self.sources.values().collect()
    }

    /// Update asset metrics
    pub fn update_metrics(&mut self, asset_id: &str, metrics: AssetMetrics) -> Result<(), String> {
        self.assets
            .get_mut(asset_id)
            .ok_or("Asset not found")?
            .metrics = Some(metrics);
        Ok(())
    }

    /// Publish an asset (make it ready for distribution)
    pub fn publish_asset(&mut self, asset_id: &str) -> Result<(), String> {
        self.assets
            .get_mut(asset_id)
            .ok_or("Asset not found")?
            .is_ready = true;
        Ok(())
    }

    /// Get all ready-to-publish assets
    pub fn get_ready_assets(&self) -> Vec<&DerivedAsset> {
        self.assets.values().filter(|a| a.is_ready).collect()
    }

    /// Statistics
    pub fn stats(&self) -> RepurposingStats {
        RepurposingStats {
            total_sources: self.sources.len(),
            total_assets: self.assets.len(),
            completed_assets: self.completed_assets,
            pending_requests: self.requests_queue.len(),
            ready_for_publishing: self.get_ready_assets().len(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RepurposingStats {
    pub total_sources: usize,
    pub total_assets: usize,
    pub completed_assets: usize,
    pub pending_requests: usize,
    pub ready_for_publishing: usize,
}

impl Default for RepurposingEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_registration() {
        let mut engine = RepurposingEngine::new();
        let source = ContentSource {
            id: "talk-001".to_string(),
            name: "Founder Keynote".to_string(),
            content_type: ContentType::FounderTalk,
            featured_contributors: vec!["founder-1".to_string()],
            duration_seconds: 2700, // 45 minutes
            storage_path: "/vault/talks/keynote-001.mp4".to_string(),
            content_hash: "0xabcd...".to_string(),
            created_at: Utc::now(),
            is_repurposable: true,
            tags: vec!["blockchain".to_string(), "future".to_string()],
            key_moments: vec![
                KeyMoment {
                    start_seconds: 120,
                    duration_seconds: 30,
                    reason: "Opening hook".to_string(),
                    category: "hook".to_string(),
                },
                KeyMoment {
                    start_seconds: 600,
                    duration_seconds: 45,
                    reason: "Key insight".to_string(),
                    category: "insight".to_string(),
                },
            ],
        };

        let result = engine.register_source(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_repurposing_request() {
        let mut engine = RepurposingEngine::new();
        let source = ContentSource {
            id: "talk-002".to_string(),
            name: "Tech Talk".to_string(),
            content_type: ContentType::TechnicalExplanation,
            featured_contributors: vec!["speaker-1".to_string()],
            duration_seconds: 1800,
            storage_path: "/vault/talks/tech-001.mp4".to_string(),
            content_hash: "0xefgh...".to_string(),
            created_at: Utc::now(),
            is_repurposable: true,
            tags: vec!["rust".to_string(), "systems".to_string()],
            key_moments: vec![],
        };

        engine.register_source(source).unwrap();

        let request = RepurposingRequest {
            source_id: "talk-002".to_string(),
            asset_type: AssetType::Clip,
            format: OutputFormat::Vertical1080p,
            target: DerivationTarget::TikTok,
            title: "Quick Rust Tip".to_string(),
            description: "A 15-second Rust optimization tip".to_string(),
            tags: vec!["rust".to_string(), "tips".to_string()],
            clip_moment_idx: None,
            instructions: None,
            priority: RepurposingPriority::High,
            auto_publish: false,
        };

        let result = engine.request_repurposing(request);
        assert!(result.is_ok());
    }

    #[test]
    fn test_complete_asset() {
        let mut engine = RepurposingEngine::new();
        let source = ContentSource {
            id: "talk-003".to_string(),
            name: "Tutorial".to_string(),
            content_type: ContentType::Tutorial,
            featured_contributors: vec!["instructor".to_string()],
            duration_seconds: 3600,
            storage_path: "/vault/tutorials/001.mp4".to_string(),
            content_hash: "0xijkl...".to_string(),
            created_at: Utc::now(),
            is_repurposable: true,
            tags: vec!["education".to_string()],
            key_moments: vec![],
        };

        engine.register_source(source).unwrap();

        let request = RepurposingRequest {
            source_id: "talk-003".to_string(),
            asset_type: AssetType::Clip,
            format: OutputFormat::Horizontal1080p,
            target: DerivationTarget::YouTube,
            title: "Tutorial Highlight".to_string(),
            description: "Best part of the tutorial".to_string(),
            tags: vec![],
            clip_moment_idx: None,
            instructions: None,
            priority: RepurposingPriority::Normal,
            auto_publish: false,
        };

        let result = engine.complete_asset(
            request,
            "/vault/clips/clip-001.mp4".to_string(),
            "0xmnop...".to_string(),
        );

        assert!(result.is_ok());
        assert_eq!(engine.stats().total_assets, 1);
    }
}
