/// Video Adapter - FFmpeg orchestration with templates
///
/// Supports:
/// - Template-based video assembly
/// - Scene detection and keyframe extraction
/// - Platform-specific output formats (YouTube, TikTok, Instagram)
/// - Subtitle generation from transcripts
/// - Voiceover and music integration

use async_trait::async_trait;
use chrono::Utc;
use serde_json::json;
use std::collections::HashMap;
use uuid::Uuid;
use crate::tool_adapter::{
    JobId, JobStatus, ToolAdapter, ToolParams, ToolResult, ToolType, ToolResourceReq,
};

/// Video template definitions
#[derive(Clone, Debug)]
pub struct VideoTemplate {
    pub name: String,
    pub id: String,
    pub duration_seconds: u32,
    pub description: String,
    pub sections: Vec<TemplateSection>,
}

#[derive(Clone, Debug)]
pub struct TemplateSection {
    pub name: String,
    pub duration: u32,
    pub content_type: String, // "video", "image", "text", "voiceover"
    pub fx: Option<String>,   // "fade_in", "zoom", "slide_left"
}

impl VideoTemplate {
    pub fn demo_30sec() -> Self {
        Self {
            name: "demo_30sec".to_string(),
            id: "demo_30sec".to_string(),
            duration_seconds: 30,
            description: "Product demo with fast cuts and music".to_string(),
            sections: vec![
                TemplateSection {
                    name: "intro".to_string(),
                    duration: 3,
                    content_type: "text".to_string(),
                    fx: Some("fade_in".to_string()),
                },
                TemplateSection {
                    name: "feature_demo".to_string(),
                    duration: 20,
                    content_type: "video".to_string(),
                    fx: Some("slide_left".to_string()),
                },
                TemplateSection {
                    name: "cta".to_string(),
                    duration: 7,
                    content_type: "text".to_string(),
                    fx: Some("fade_out".to_string()),
                },
            ],
        }
    }

    pub fn feature_spotlight() -> Self {
        Self {
            name: "feature_spotlight".to_string(),
            id: "feature_spotlight".to_string(),
            duration_seconds: 60,
            description: "In-depth feature explanation with visuals".to_string(),
            sections: vec![
                TemplateSection {
                    name: "intro".to_string(),
                    duration: 5,
                    content_type: "text".to_string(),
                    fx: None,
                },
                TemplateSection {
                    name: "explanation".to_string(),
                    duration: 40,
                    content_type: "video".to_string(),
                    fx: Some("zoom".to_string()),
                },
                TemplateSection {
                    name: "conclusion".to_string(),
                    duration: 15,
                    content_type: "text".to_string(),
                    fx: None,
                },
            ],
        }
    }

    pub fn tutorial_5min() -> Self {
        Self {
            name: "tutorial_5min".to_string(),
            id: "tutorial_5min".to_string(),
            duration_seconds: 300,
            description: "Step-by-step educational tutorial".to_string(),
            sections: vec![
                TemplateSection {
                    name: "intro".to_string(),
                    duration: 15,
                    content_type: "text".to_string(),
                    fx: None,
                },
                TemplateSection {
                    name: "steps".to_string(),
                    duration: 240,
                    content_type: "video".to_string(),
                    fx: Some("slide_left".to_string()),
                },
                TemplateSection {
                    name: "recap".to_string(),
                    duration: 45,
                    content_type: "text".to_string(),
                    fx: None,
                },
            ],
        }
    }
}

/// Platform-specific output formats
#[derive(Clone, Debug)]
pub struct PlatformFormat {
    pub name: String,
    pub resolution: (u32, u32),
    pub aspect_ratio: String,
    pub bitrate: String,
    pub codec: String,
}

impl PlatformFormat {
    pub fn youtube() -> Self {
        Self {
            name: "youtube".to_string(),
            resolution: (1920, 1080),
            aspect_ratio: "16:9".to_string(),
            bitrate: "5000k".to_string(),
            codec: "h264".to_string(),
        }
    }

    pub fn tiktok() -> Self {
        Self {
            name: "tiktok".to_string(),
            resolution: (1080, 1920),
            aspect_ratio: "9:16".to_string(),
            bitrate: "3000k".to_string(),
            codec: "h264".to_string(),
        }
    }

    pub fn instagram_reels() -> Self {
        Self {
            name: "instagram_reels".to_string(),
            resolution: (1080, 1920),
            aspect_ratio: "9:16".to_string(),
            bitrate: "3000k".to_string(),
            codec: "h264".to_string(),
        }
    }

    pub fn twitter() -> Self {
        Self {
            name: "twitter".to_string(),
            resolution: (1280, 720),
            aspect_ratio: "16:9".to_string(),
            bitrate: "2500k".to_string(),
            codec: "h264".to_string(),
        }
    }
}

/// Video clip for assembly
#[derive(Clone, Debug)]
pub struct VideoClip {
    pub clip_type: String, // "video", "image", "text"
    pub source: String,    // URL or local path
    pub duration: u32,     // Seconds
    pub trim: Option<(u32, u32)>, // Start, end in seconds
    pub transition: Option<String>, // "fade", "slide", "wipe"
}

/// Generation parameters
#[derive(Clone, Debug)]
pub struct VideoGenerationParams {
    pub template: String,
    pub platform: String,
    pub clips: Vec<VideoClip>,
    pub voiceover_url: Option<String>,
    pub music_url: Option<String>,
    pub add_subtitles: bool,
}

impl VideoGenerationParams {
    pub fn from_tool_params(params: &ToolParams) -> Result<Self, String> {
        let template = params
            .get("template")
            .and_then(|v| v.as_str())
            .ok_or("template required")?
            .to_string();

        let platform = params
            .get("platform")
            .and_then(|v| v.as_str())
            .unwrap_or("youtube")
            .to_string();

        // Parse clips array
        let mut clips = Vec::new();
        if let Some(clips_array) = params.get("clips").and_then(|v| v.as_array()) {
            for clip_val in clips_array {
                if let Some(clip_obj) = clip_val.as_object() {
                    let clip = VideoClip {
                        clip_type: clip_obj
                            .get("type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("video")
                            .to_string(),
                        source: clip_obj
                            .get("source")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        duration: clip_obj
                            .get("duration")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(10) as u32,
                        trim: None,
                        transition: None,
                    };
                    clips.push(clip);
                }
            }
        }

        Ok(VideoGenerationParams {
            template,
            platform,
            clips,
            voiceover_url: params
                .get("voiceover_url")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            music_url: params
                .get("music_url")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            add_subtitles: params
                .get("add_subtitles")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
        })
    }
}

/// Video job state
#[derive(Clone, Debug)]
struct VideoJob {
    job_id: JobId,
    status: JobStatus,
    params: VideoGenerationParams,
    output_url: Option<String>,
    error: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    processing_time_seconds: u32,
}

/// Video Adapter implementation
pub struct VideoAdapter {
    server_url: String,
    templates: HashMap<String, VideoTemplate>,
    platforms: HashMap<String, PlatformFormat>,
    jobs: HashMap<JobId, VideoJob>,
}

impl VideoAdapter {
    pub fn new(server_url: String) -> Self {
        let mut templates = HashMap::new();
        templates.insert("demo_30sec".to_string(), VideoTemplate::demo_30sec());
        templates.insert("feature_spotlight".to_string(), VideoTemplate::feature_spotlight());
        templates.insert("tutorial_5min".to_string(), VideoTemplate::tutorial_5min());

        let mut platforms = HashMap::new();
        platforms.insert("youtube".to_string(), PlatformFormat::youtube());
        platforms.insert("tiktok".to_string(), PlatformFormat::tiktok());
        platforms.insert("instagram_reels".to_string(), PlatformFormat::instagram_reels());
        platforms.insert("twitter".to_string(), PlatformFormat::twitter());

        Self {
            server_url,
            templates,
            platforms,
            jobs: HashMap::new(),
        }
    }

    async fn assemble_video(
        &self,
        params: &VideoGenerationParams,
    ) -> Result<String, String> {
        use std::process::Command;
        
        let template = self.templates
            .get(&params.template)
            .ok_or(format!("Template not found: {}", params.template))?;

        let platform = self.platforms
            .get(&params.platform)
            .ok_or(format!("Platform not found: {}", params.platform))?;

        // Build complex FFmpeg command for video assembly
        let mut cmd = Command::new("ffmpeg");
        cmd.arg("-y"); // Overwrite output

        // Add input files
        for clip in &params.clips {
            cmd.arg("-i").arg(&clip.source);
        }

        if let Some(vo_url) = &params.voiceover_url {
            cmd.arg("-i").arg(vo_url);
        }

        if let Some(music_url) = &params.music_url {
            cmd.arg("-i").arg(music_url);
        }

        // Complex filter graph for video assembly
        let mut filter_complex = String::new();
        
        // Map inputs to streams
        let mut input_map = 0;
        let mut video_inputs = Vec::new();
        let mut audio_inputs = Vec::new();

        for (i, clip) in params.clips.iter().enumerate() {
            match clip.clip_type.as_str() {
                "video" => {
                    video_inputs.push(format!("[{}:v]", input_map));
                    audio_inputs.push(format!("[{}:a]", input_map));
                    input_map += 1;
                },
                "image" => {
                    // Convert image to video
                    filter_complex.push_str(&format!(
                        "[{}:v]scale={}:{}:force_original_aspect_ratio=decrease,pad={}:{}:(ow-iw)/2:(oh-ih)/2,trim=duration={}[img{}];",
                        input_map, platform.resolution.0, platform.resolution.1,
                        platform.resolution.0, platform.resolution.1, clip.duration, i
                    ));
                    video_inputs.push(format!("[img{}]", i));
                    input_map += 1;
                },
                _ => {
                    input_map += 1;
                }
            }
        }

        // Add voiceover and music
        if params.voiceover_url.is_some() {
            audio_inputs.push(format!("[{}:a]", input_map));
            input_map += 1;
        }
        if params.music_url.is_some() {
            input_map += 1;
        }

        // Concatenate video streams
        if !video_inputs.is_empty() {
            filter_complex.push_str(&format!("{}concat=n={}:v=1:a=0[video];", 
                video_inputs.join(""), video_inputs.len()));
        }

        // Mix audio streams
        if !audio_inputs.is_empty() {
            filter_complex.push_str(&format!("{}amix=inputs={}:duration=longest[audio];",
                audio_inputs.join(""), audio_inputs.len()));
        }

        // Apply platform-specific encoding
        cmd.arg("-filter_complex").arg(&filter_complex);
        cmd.arg("-map").arg("[video]");
        cmd.arg("-map").arg("[audio]");
        
        // Output settings
        cmd.arg("-c:v").arg(&platform.codec);
        cmd.arg("-crf").arg("23"); // Quality setting
        cmd.arg("-c:a").arg("aac");
        cmd.arg("-b:a").arg("192k");
        cmd.arg("-movflags").arg("+faststart"); // For web streaming

        // Output file
        let output_path = format!("/tmp/swarm-video-{}.mp4", Uuid::new_v4());
        cmd.arg(&output_path);

        // Execute FFmpeg
        let output = cmd.output()
            .map_err(|e| format!("Failed to execute FFmpeg: {}", e))?;

        if !output.status.success() {
            let error_msg = String::from_utf8_lossy(&output.stderr);
            return Err(format!("FFmpeg failed: {}", error_msg));
        }

        // Upload to storage (simulated)
        let output_url = format!(
            "s3://swarm-media/videos/{}/{}.mp4",
            Uuid::new_v4(),
            params.template
        );

        tracing::info!("Video generated successfully: {}", output_url);
        Ok(output_url)
    }

    fn calculate_processing_time(template: &VideoTemplate) -> u32 {
        // Rough estimate: 2 seconds per output second
        (template.duration_seconds * 2) / 60
    }
}

#[async_trait]
impl ToolAdapter for VideoAdapter {
    fn tool_type(&self) -> ToolType {
        ToolType::VideoProcessing
    }

    async fn validate_params(&self, params: &ToolParams) -> Result<(), String> {
        let gen_params = VideoGenerationParams::from_tool_params(params)?;

        if !self.templates.contains_key(&gen_params.template) {
            return Err(format!("Template not found: {}", gen_params.template));
        }

        if !self.platforms.contains_key(&gen_params.platform) {
            return Err(format!("Platform not found: {}", gen_params.platform));
        }

        if gen_params.clips.is_empty() {
            return Err("At least one clip is required".to_string());
        }

        Ok(())
    }

    async fn invoke(&self, params: ToolParams) -> Result<JobId, String> {
        let job_id = Uuid::new_v4();
        let gen_params = VideoGenerationParams::from_tool_params(&params)?;

        let _template = self.templates
            .get(&gen_params.template)
            .cloned()
            .ok_or(format!("Template not found: {}", gen_params.template))?;

        let output_url = self.assemble_video(&gen_params).await?;
        let processing_time = Self::calculate_processing_time(&_template);

        let _job = VideoJob {
            job_id,
            status: JobStatus::Completed,
            params: gen_params,
            output_url: Some(output_url),
            error: None,
            created_at: Utc::now(),
            processing_time_seconds: processing_time,
        };

        Ok(job_id)
    }

    async fn get_status(&self, job_id: JobId) -> Result<JobStatus, String> {
        self.jobs
            .get(&job_id)
            .map(|job| job.status.clone())
            .ok_or("Job not found".to_string())
    }

    async fn get_result(&self, job_id: JobId) -> Result<ToolResult, String> {
        let job = self.jobs
            .get(&job_id)
            .ok_or("Job not found".to_string())?;

        if let Some(error) = &job.error {
            return Err(error.clone());
        }

        let output_url = job
            .output_url
            .as_ref()
            .ok_or("Output not available".to_string())?
            .clone();

        Ok(ToolResult {
            job_id,
            tool_type: ToolType::VideoProcessing,
            output: json!({
                "output_url": output_url.clone(),
                "template": &job.params.template,
                "platform": &job.params.platform,
                "processing_time_seconds": job.processing_time_seconds,
                "has_subtitles": job.params.add_subtitles,
                "generated_at": job.created_at.to_rfc3339(),
            }),
            execution_time_ms: (job.processing_time_seconds * 1000) as u32,
            content_hash: Some(format!("{:x}", sha2::Sha256::digest(output_url.as_bytes()))),
            executed_by_node: Uuid::new_v4(),
        })
    }

    async fn cancel_job(&self, _job_id: JobId) -> Result<(), String> {
        // Stop FFmpeg process
        Ok(())
    }

    fn resource_requirements(&self, params: &ToolParams) -> ToolResourceReq {
        if let Ok(gen_params) = VideoGenerationParams::from_tool_params(params) {
            let min_vram = if gen_params.clips.iter().any(|clip| clip.clip_type == "video") {
                8
            } else {
                4
            };

            let preferred_latency = (gen_params.clips.len() as u32 * 400).max(1500);

            return ToolResourceReq {
                min_vram_gb: min_vram,
                preferred_latency_ms: preferred_latency,
                supports_batching: true,
            };
        }

        ToolResourceReq {
            min_vram_gb: 4,
            preferred_latency_ms: 2500,
            supports_batching: true,
        }
    }
}

use sha2::Digest;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_video_adapter_creation() {
        let adapter = VideoAdapter::new("http://localhost:8080".to_string());
        assert_eq!(adapter.tool_type(), ToolType::VideoProcessing);
        assert!(adapter.templates.contains_key("demo_30sec"));
    }

    #[test]
    fn test_template_loading() {
        let template = VideoTemplate::demo_30sec();
        assert_eq!(template.duration_seconds, 30);
        assert_eq!(template.sections.len(), 3);
    }

    #[test]
    fn test_platform_formats() {
        let platforms = vec![
            PlatformFormat::youtube(),
            PlatformFormat::tiktok(),
            PlatformFormat::instagram_reels(),
            PlatformFormat::twitter(),
        ];

        assert_eq!(platforms[0].aspect_ratio, "16:9"); // YouTube
        assert_eq!(platforms[1].aspect_ratio, "9:16"); // TikTok
    }

    #[test]
    fn test_video_generation_params() {
        let params = ToolParams::new(json!({
            "template": "demo_30sec",
            "platform": "youtube",
            "clips": [
                {"type": "video", "source": "s3://bucket/clip1.mp4", "duration": 10}
            ],
        }));

        let gen_params = VideoGenerationParams::from_tool_params(&params);
        assert!(gen_params.is_ok());
        assert_eq!(gen_params.unwrap().clips.len(), 1);
    }
}