/**
 * Repurposing Intent Converter
 *
 * Transforms high-level repurposing requests into execution intents
 * for specific target platforms with validation and optimization
 */

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Asset type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum AssetType {
    Clip,
    FullEpisode,
    DubLocalization,
    Subtitle,
    Transcript,
    Highlight,
    Teaser,
    BTS,
}

impl AssetType {
    pub fn as_str(&self) -> &str {
        match self {
            AssetType::Clip => "Clip",
            AssetType::FullEpisode => "FullEpisode",
            AssetType::DubLocalization => "DubLocalization",
            AssetType::Subtitle => "Subtitle",
            AssetType::Transcript => "Transcript",
            AssetType::Highlight => "Highlight",
            AssetType::Teaser => "Teaser",
            AssetType::BTS => "BTS",
        }
    }
}

/// Target platform enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum DerivationTarget {
    YouTube,
    TikTok,
    Instagram,
    Twitter,
    LinkedIn,
    Facebook,
    Snapchat,
    Twitch,
    Discord,
    Custom,
}

impl DerivationTarget {
    pub fn as_str(&self) -> &str {
        match self {
            DerivationTarget::YouTube => "YouTube",
            DerivationTarget::TikTok => "TikTok",
            DerivationTarget::Instagram => "Instagram",
            DerivationTarget::Twitter => "Twitter",
            DerivationTarget::LinkedIn => "LinkedIn",
            DerivationTarget::Facebook => "Facebook",
            DerivationTarget::Snapchat => "Snapchat",
            DerivationTarget::Twitch => "Twitch",
            DerivationTarget::Discord => "Discord",
            DerivationTarget::Custom => "Custom",
        }
    }

    /// Get recommended aspect ratio for target platform
    pub fn recommended_aspect_ratio(&self) -> (u32, u32) {
        match self {
            // Vertical formats
            DerivationTarget::TikTok => (9, 16),    // 9:16
            DerivationTarget::Instagram => (9, 16), // 9:16 reels
            DerivationTarget::Snapchat => (9, 16),  // 9:16
            
            // Horizontal formats
            DerivationTarget::YouTube => (16, 9),   // 16:9
            DerivationTarget::Twitch => (16, 9),    // 16:9
            
            // Square formats
            DerivationTarget::Facebook => (1, 1),   // 1:1
            DerivationTarget::Instagram => (1, 1),  // 1:1 square
            
            // Professional
            DerivationTarget::LinkedIn => (16, 9),  // 16:9
            DerivationTarget::Twitter => (16, 9),   // 16:9
            
            // Custom
            DerivationTarget::Discord => (16, 9),
            DerivationTarget::Custom => (16, 9),
        }
    }

    /// Get maximum duration in seconds for target platform
    pub fn max_duration_seconds(&self) -> Option<u32> {
        match self {
            DerivationTarget::TikTok => Some(600),        // 10 minutes
            DerivationTarget::Instagram => Some(3600),    // 60 minutes for reels
            DerivationTarget::Snapchat => Some(1000),     // ~16 minutes
            DerivationTarget::Twitter => Some(140),       // ~2.3 minutes
            DerivationTarget::YouTube => None,            // No limit
            DerivationTarget::Twitch => None,             // No limit
            DerivationTarget::LinkedIn => Some(600),      // 10 minutes
            DerivationTarget::Facebook => None,           // No limit
            DerivationTarget::Discord => Some(25),        // ~25 seconds for embeds
            DerivationTarget::Custom => None,             // No limit
        }
    }

    /// Get minimum duration in seconds for target platform
    pub fn min_duration_seconds(&self) -> Option<u32> {
        match self {
            DerivationTarget::TikTok => Some(3),      // 3 seconds minimum
            DerivationTarget::Instagram => Some(3),   // 3 seconds minimum
            DerivationTarget::Snapchat => Some(1),    // 1 second minimum
            DerivationTarget::Twitter => Some(1),     // 1 second minimum
            _ => None,
        }
    }

    /// Get recommended frame rate in FPS
    pub fn recommended_fps(&self) -> u32 {
        match self {
            DerivationTarget::TikTok => 24,
            DerivationTarget::Instagram => 24,
            DerivationTarget::YouTube => 30,
            DerivationTarget::Twitch => 60,
            _ => 30,
        }
    }

    /// Get maximum file size in MB
    pub fn max_file_size_mb(&self) -> Option<u32> {
        match self {
            DerivationTarget::TikTok => Some(500),      // 500 MB
            DerivationTarget::Instagram => Some(4000),  // 4 GB
            DerivationTarget::YouTube => Some(256000),  // 256 GB
            DerivationTarget::Twitch => Some(100000),   // 100 GB
            DerivationTarget::Twitter => Some(512),     // 512 MB
            DerivationTarget::Facebook => Some(4000),   // 4 GB
            DerivationTarget::LinkedIn => Some(5000),   // 5 GB
            _ => Some(1000),
        }
    }
}

/// Repurposing intent parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepurposingIntent {
    /// Unique intent ID
    pub intent_id: String,
    
    /// Source asset information
    pub source_asset_id: String,
    pub source_asset_type: AssetType,
    pub source_duration_seconds: Option<u32>,
    pub source_resolution: Option<(u32, u32)>, // (width, height)
    
    /// Target platform
    pub target_platform: DerivationTarget,
    
    /// Output specifications
    pub output_aspect_ratio: (u32, u32),
    pub output_resolution: (u32, u32),
    pub output_fps: u32,
    pub output_codec: String,
    pub max_file_size_mb: Option<u32>,
    
    /// Content metadata
    pub title: String,
    pub description: String,
    pub tags: Vec<String>,
    pub hashtags: Vec<String>,
    
    /// Transformation settings
    pub add_watermark: bool,
    pub add_captions: bool,
    pub auto_cut_silence: bool,
    pub brightness_adjustment: Option<f32>, // -1.0 to 1.0
    pub contrast_adjustment: Option<f32>,    // -1.0 to 1.0
    pub speed_adjustment: Option<f32>,       // 0.5 to 2.0
    pub add_intro: bool,
    pub add_outro: bool,
    pub add_music_bed: bool,
    
    /// Validation flags
    pub validated: bool,
    pub validation_errors: Vec<String>,
    
    /// Optimization settings
    pub auto_optimize: bool,
    pub compress_audio: bool,
    pub compress_video: bool,
}

impl RepurposingIntent {
    /// Create new repurposing intent from parameters
    pub fn new(
        source_asset_id: String,
        source_asset_type: AssetType,
        target_platform: DerivationTarget,
        title: String,
        description: String,
    ) -> Self {
        use uuid::Uuid;
        
        let aspect_ratio = target_platform.recommended_aspect_ratio();
        let fps = target_platform.recommended_fps();
        let max_file_size = target_platform.max_file_size_mb();
        
        // Calculate output resolution based on aspect ratio
        let output_resolution = match aspect_ratio {
            (9, 16) => (1080, 1920),  // Vertical HD
            (16, 9) => (1920, 1080),  // Horizontal HD
            (1, 1) => (1080, 1080),   // Square HD
            _ => (1920, 1080),        // Default
        };

        Self {
            intent_id: Uuid::new_v4().to_string(),
            source_asset_id,
            source_asset_type,
            source_duration_seconds: None,
            source_resolution: None,
            target_platform,
            output_aspect_ratio: aspect_ratio,
            output_resolution,
            output_fps: fps,
            output_codec: "h264".to_string(),
            max_file_size_mb: max_file_size,
            title,
            description,
            tags: Vec::new(),
            hashtags: Vec::new(),
            add_watermark: true,
            add_captions: true,
            auto_cut_silence: true,
            brightness_adjustment: None,
            contrast_adjustment: None,
            speed_adjustment: None,
            add_intro: false,
            add_outro: false,
            add_music_bed: false,
            validated: false,
            validation_errors: Vec::new(),
            auto_optimize: true,
            compress_audio: true,
            compress_video: true,
        }
    }

    /// Validate the repurposing intent
    pub fn validate(&mut self) -> bool {
        self.validation_errors.clear();

        // Check source duration
        if let Some(duration) = self.source_duration_seconds {
            if let Some(max) = self.target_platform.max_duration_seconds() {
                if duration > max {
                    self.validation_errors.push(format!(
                        "Source duration {} seconds exceeds maximum {} seconds for {}",
                        duration, max, self.target_platform.as_str()
                    ));
                }
            }

            if let Some(min) = self.target_platform.min_duration_seconds() {
                if duration < min {
                    self.validation_errors.push(format!(
                        "Source duration {} seconds is below minimum {} seconds for {}",
                        duration, min, self.target_platform.as_str()
                    ));
                }
            }
        }

        // Check resolution
        if let Some((width, height)) = self.source_resolution {
            if width < self.output_resolution.0 || height < self.output_resolution.1 {
                self.validation_errors.push(format!(
                    "Source resolution {}x{} is lower than target {}x{}",
                    width, height, self.output_resolution.0, self.output_resolution.1
                ));
            }
        }

        // Check brightness/contrast adjustments
        if let Some(brightness) = self.brightness_adjustment {
            if brightness < -1.0 || brightness > 1.0 {
                self.validation_errors.push(
                    "Brightness adjustment must be between -1.0 and 1.0".to_string()
                );
            }
        }

        if let Some(contrast) = self.contrast_adjustment {
            if contrast < -1.0 || contrast > 1.0 {
                self.validation_errors.push(
                    "Contrast adjustment must be between -1.0 and 1.0".to_string()
                );
            }
        }

        // Check speed adjustment
        if let Some(speed) = self.speed_adjustment {
            if speed < 0.5 || speed > 2.0 {
                self.validation_errors.push(
                    "Speed adjustment must be between 0.5 and 2.0".to_string()
                );
            }
        }

        // Check required metadata
        if self.title.is_empty() {
            self.validation_errors.push("Title is required".to_string());
        }

        self.validated = self.validation_errors.is_empty();
        self.validated
    }

    /// Get optimization recommendations based on target platform
    pub fn get_optimization_recommendations(&self) -> Vec<String> {
        let mut recommendations = Vec::new();

        match self.target_platform {
            DerivationTarget::TikTok => {
                recommendations.push(
                    "Recommend adding captions for TikTok (high engagement)".to_string()
                );
                recommendations.push("Recommend vertical aspect ratio (9:16)".to_string());
                recommendations.push("Recommend music bed (TikTok users love music)".to_string());
            }
            DerivationTarget::YouTube => {
                recommendations.push("Recommend horizontal aspect ratio (16:9)".to_string());
                recommendations.push("Recommend descriptive title and tags for SEO".to_string());
            }
            DerivationTarget::Instagram => {
                recommendations.push("Consider both vertical and square formats".to_string());
                recommendations.push("Recommend hashtags for discoverability".to_string());
            }
            DerivationTarget::LinkedIn => {
                recommendations.push("Consider professional tone and captions".to_string());
                recommendations.push("Recommend adding company branding".to_string());
            }
            _ => {}
        }

        recommendations
    }

    /// Get estimated processing time in seconds
    pub fn estimate_processing_time(&self) -> u32 {
        let mut base_time = 30; // Base processing time

        if let Some(duration) = self.source_duration_seconds {
            // ~1 second of processing per 2 seconds of video
            base_time += duration / 2;
        }

        // Add time for transformations
        if self.add_captions {
            base_time += 30;
        }
        if self.add_watermark {
            base_time += 10;
        }
        if self.add_intro || self.add_outro {
            base_time += 20;
        }
        if self.compress_video {
            base_time += 20;
        }

        base_time
    }

    /// Convert to execution parameters for processing engine
    pub fn to_execution_params(&self) -> HashMap<String, String> {
        let mut params = HashMap::new();

        params.insert("intent_id".to_string(), self.intent_id.clone());
        params.insert("source_asset_id".to_string(), self.source_asset_id.clone());
        params.insert("target_platform".to_string(), self.target_platform.as_str().to_string());
        params.insert("output_width".to_string(), self.output_resolution.0.to_string());
        params.insert("output_height".to_string(), self.output_resolution.1.to_string());
        params.insert("output_fps".to_string(), self.output_fps.to_string());
        params.insert("output_codec".to_string(), self.output_codec.clone());
        params.insert("title".to_string(), self.title.clone());
        params.insert("description".to_string(), self.description.clone());
        params.insert("add_captions".to_string(), self.add_captions.to_string());
        params.insert("add_watermark".to_string(), self.add_watermark.to_string());
        params.insert("auto_cut_silence".to_string(), self.auto_cut_silence.to_string());

        if let Some(brightness) = self.brightness_adjustment {
            params.insert("brightness_adjustment".to_string(), brightness.to_string());
        }

        if let Some(contrast) = self.contrast_adjustment {
            params.insert("contrast_adjustment".to_string(), contrast.to_string());
        }

        if let Some(speed) = self.speed_adjustment {
            params.insert("speed_adjustment".to_string(), speed.to_string());
        }

        params
    }
}

/// Repurposing intent converter
pub struct IntentConverter;

impl IntentConverter {
    /// Convert a repurposing request to an execution intent
    pub fn convert_to_intent(
        source_asset_id: String,
        source_asset_type: String,
        target_platform: String,
        title: String,
        description: String,
        tags: Vec<String>,
    ) -> Result<RepurposingIntent, String> {
        // Parse asset type
        let asset_type = match source_asset_type.as_str() {
            "Clip" => AssetType::Clip,
            "FullEpisode" => AssetType::FullEpisode,
            "DubLocalization" => AssetType::DubLocalization,
            "Subtitle" => AssetType::Subtitle,
            "Transcript" => AssetType::Transcript,
            "Highlight" => AssetType::Highlight,
            "Teaser" => AssetType::Teaser,
            "BTS" => AssetType::BTS,
            _ => return Err(format!("Unknown asset type: {}", source_asset_type)),
        };

        // Parse target platform
        let target = match target_platform.as_str() {
            "YouTube" => DerivationTarget::YouTube,
            "TikTok" => DerivationTarget::TikTok,
            "Instagram" => DerivationTarget::Instagram,
            "Twitter" => DerivationTarget::Twitter,
            "LinkedIn" => DerivationTarget::LinkedIn,
            "Facebook" => DerivationTarget::Facebook,
            "Snapchat" => DerivationTarget::Snapchat,
            "Twitch" => DerivationTarget::Twitch,
            "Discord" => DerivationTarget::Discord,
            "Custom" => DerivationTarget::Custom,
            _ => return Err(format!("Unknown target platform: {}", target_platform)),
        };

        let mut intent = RepurposingIntent::new(source_asset_id, asset_type, target, title, description);
        intent.tags = tags;

        Ok(intent)
    }

    /// Batch convert multiple requests
    pub fn batch_convert(
        requests: Vec<(String, String, String, String, String, Vec<String>)>,
    ) -> (Vec<RepurposingIntent>, Vec<String>) {
        let mut intents = Vec::new();
        let mut errors = Vec::new();

        for (source_id, asset_type, target, title, desc, tags) in requests {
            match Self::convert_to_intent(source_id, asset_type, target, title, desc, tags) {
                Ok(intent) => intents.push(intent),
                Err(e) => errors.push(e),
            }
        }

        (intents, errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intent_creation() {
        let intent = RepurposingIntent::new(
            "asset-123".to_string(),
            AssetType::Clip,
            DerivationTarget::YouTube,
            "Test Title".to_string(),
            "Test Description".to_string(),
        );

        assert_eq!(intent.source_asset_id, "asset-123");
        assert_eq!(intent.target_platform, DerivationTarget::YouTube);
        assert_eq!(intent.output_aspect_ratio, (16, 9));
    }

    #[test]
    fn test_validation() {
        let mut intent = RepurposingIntent::new(
            "asset-123".to_string(),
            AssetType::Clip,
            DerivationTarget::TikTok,
            "Test".to_string(),
            "Test".to_string(),
        );

        intent.source_duration_seconds = Some(700); // Exceeds TikTok max
        let valid = intent.validate();

        assert!(!valid);
        assert!(!intent.validation_errors.is_empty());
    }

    #[test]
    fn test_aspect_ratios() {
        assert_eq!(DerivationTarget::TikTok.recommended_aspect_ratio(), (9, 16));
        assert_eq!(DerivationTarget::YouTube.recommended_aspect_ratio(), (16, 9));
        assert_eq!(DerivationTarget::Instagram.recommended_aspect_ratio(), (9, 16));
    }

    #[test]
    fn test_intent_converter() {
        let result = IntentConverter::convert_to_intent(
            "asset-123".to_string(),
            "Clip".to_string(),
            "YouTube".to_string(),
            "Title".to_string(),
            "Description".to_string(),
            vec!["tag1".to_string()],
        );

        assert!(result.is_ok());
        let intent = result.unwrap();
        assert_eq!(intent.target_platform, DerivationTarget::YouTube);
    }

    #[test]
    fn test_processing_time_estimation() {
        let intent = RepurposingIntent::new(
            "asset-123".to_string(),
            AssetType::Clip,
            DerivationTarget::YouTube,
            "Title".to_string(),
            "Description".to_string(),
        );

        let time = intent.estimate_processing_time();
        assert!(time > 0);
    }
}
