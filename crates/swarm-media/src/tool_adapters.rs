// ============================================================================
// X3 ATLAS SPHERE - EXTERNAL TOOL ADAPTERS
// LLM, Image Generation, Video, TTS, and Scheduling API Integrations
// ============================================================================

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

// ============================================================================
// ERROR TYPES
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolError {
    RateLimited { retry_after: Duration },
    ApiError { code: u16, message: String },
    NetworkError { message: String },
    InvalidInput { field: String, message: String },
    ContentFiltered { reason: String },
    QuotaExceeded,
    AuthenticationFailed,
    ServiceUnavailable,
    Timeout,
    Unknown { message: String },
}

impl std::fmt::Display for ToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolError::RateLimited { retry_after } => {
                write!(f, "Rate limited, retry after {:?}", retry_after)
            }
            ToolError::ApiError { code, message } => {
                write!(f, "API error {}: {}", code, message)
            }
            ToolError::NetworkError { message } => write!(f, "Network error: {}", message),
            ToolError::InvalidInput { field, message } => {
                write!(f, "Invalid input for '{}': {}", field, message)
            }
            ToolError::ContentFiltered { reason } => write!(f, "Content filtered: {}", reason),
            ToolError::QuotaExceeded => write!(f, "API quota exceeded"),
            ToolError::AuthenticationFailed => write!(f, "Authentication failed"),
            ToolError::ServiceUnavailable => write!(f, "Service unavailable"),
            ToolError::Timeout => write!(f, "Request timed out"),
            ToolError::Unknown { message } => write!(f, "Unknown error: {}", message),
        }
    }
}

// ============================================================================
// LLM ADAPTER TRAIT
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMRequest {
    pub request_id: Uuid,
    pub model: LLMModel,
    pub system_prompt: Option<String>,
    pub messages: Vec<ChatMessage>,
    pub max_tokens: u32,
    pub temperature: f32,
    pub top_p: Option<f32>,
    pub stop_sequences: Vec<String>,
    pub response_format: Option<ResponseFormat>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LLMModel {
    // OpenAI
    GPT4o,
    GPT4oMini,
    GPT4Turbo,
    
    // Anthropic
    Claude3Opus,
    Claude3Sonnet,
    Claude35Sonnet,
    Claude4Opus,
    Claude4Sonnet,
    
    // Open source / Others
    Llama3_70B,
    Llama3_8B,
    Mistral,
    Qwen,
}

impl LLMModel {
    pub fn provider(&self) -> LLMProvider {
        match self {
            LLMModel::GPT4o | LLMModel::GPT4oMini | LLMModel::GPT4Turbo => LLMProvider::OpenAI,
            LLMModel::Claude3Opus
            | LLMModel::Claude3Sonnet
            | LLMModel::Claude35Sonnet
            | LLMModel::Claude4Opus
            | LLMModel::Claude4Sonnet => LLMProvider::Anthropic,
            LLMModel::Llama3_70B | LLMModel::Llama3_8B => LLMProvider::Together,
            LLMModel::Mistral => LLMProvider::Mistral,
            LLMModel::Qwen => LLMProvider::Together,
        }
    }

    pub fn max_context(&self) -> u32 {
        match self {
            LLMModel::GPT4o => 128_000,
            LLMModel::GPT4oMini => 128_000,
            LLMModel::GPT4Turbo => 128_000,
            LLMModel::Claude3Opus => 200_000,
            LLMModel::Claude3Sonnet => 200_000,
            LLMModel::Claude35Sonnet => 200_000,
            LLMModel::Claude4Opus => 200_000,
            LLMModel::Claude4Sonnet => 200_000,
            LLMModel::Llama3_70B => 8192,
            LLMModel::Llama3_8B => 8192,
            LLMModel::Mistral => 32768,
            LLMModel::Qwen => 32768,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LLMProvider {
    OpenAI,
    Anthropic,
    Together,
    Mistral,
    Groq,
    Local,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChatRole {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseFormat {
    Text,
    JSON { schema: Option<serde_json::Value> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResponse {
    pub request_id: Uuid,
    pub content: String,
    pub finish_reason: FinishReason,
    pub usage: TokenUsage,
    pub latency_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FinishReason {
    Stop,
    Length,
    ContentFilter,
    ToolCall,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub estimated_cost: f64,
}

#[async_trait]
pub trait LLMAdapter: Send + Sync {
    async fn generate(&self, request: LLMRequest) -> Result<LLMResponse, ToolError>;
    async fn generate_stream(
        &self,
        request: LLMRequest,
    ) -> Result<Box<dyn tokio::io::AsyncRead + Send + Unpin>, ToolError>;
    fn supported_models(&self) -> Vec<LLMModel>;
    fn provider(&self) -> LLMProvider;
    async fn health_check(&self) -> Result<(), ToolError>;
}

// ============================================================================
// OPENAI ADAPTER
// ============================================================================

pub struct OpenAIAdapter {
    pub adapter_id: Uuid,
    pub api_key: String,
    pub organization_id: Option<String>,
    pub base_url: String,
    pub timeout: Duration,
}

impl OpenAIAdapter {
    pub fn new(api_key: String) -> Self {
        Self {
            adapter_id: Uuid::new_v4(),
            api_key,
            organization_id: None,
            base_url: "https://api.openai.com/v1".to_string(),
            timeout: Duration::from_secs(60),
        }
    }

    pub fn with_organization(mut self, org_id: String) -> Self {
        self.organization_id = Some(org_id);
        self
    }

    fn model_to_api_name(&self, model: &LLMModel) -> String {
        match model {
            LLMModel::GPT4o => "gpt-4o".to_string(),
            LLMModel::GPT4oMini => "gpt-4o-mini".to_string(),
            LLMModel::GPT4Turbo => "gpt-4-turbo".to_string(),
            _ => panic!("Model not supported by OpenAI"),
        }
    }
}

#[async_trait]
impl LLMAdapter for OpenAIAdapter {
    async fn generate(&self, request: LLMRequest) -> Result<LLMResponse, ToolError> {
        let start = std::time::Instant::now();

        // Build messages
        let mut messages: Vec<serde_json::Value> = Vec::new();

        if let Some(system) = &request.system_prompt {
            messages.push(serde_json::json!({
                "role": "system",
                "content": system
            }));
        }

        for msg in &request.messages {
            messages.push(serde_json::json!({
                "role": match msg.role {
                    ChatRole::System => "system",
                    ChatRole::User => "user",
                    ChatRole::Assistant => "assistant",
                },
                "content": msg.content
            }));
        }

        let _body = serde_json::json!({
            "model": self.model_to_api_name(&request.model),
            "messages": messages,
            "max_tokens": request.max_tokens,
            "temperature": request.temperature,
            "stop": if request.stop_sequences.is_empty() { None } else { Some(&request.stop_sequences) },
        });

        // In production, would make actual HTTP request
        // For now, return mock response
        let latency = start.elapsed().as_millis() as u64;

        Ok(LLMResponse {
            request_id: request.request_id,
            content: "[LLM Response would be generated here]".to_string(),
            finish_reason: FinishReason::Stop,
            usage: TokenUsage {
                prompt_tokens: 100,
                completion_tokens: 50,
                total_tokens: 150,
                estimated_cost: 0.002,
            },
            latency_ms: latency,
        })
    }

    async fn generate_stream(
        &self,
        _request: LLMRequest,
    ) -> Result<Box<dyn tokio::io::AsyncRead + Send + Unpin>, ToolError> {
        Err(ToolError::ApiError {
            code: 501,
            message: "Streaming not yet implemented".to_string(),
        })
    }

    fn supported_models(&self) -> Vec<LLMModel> {
        vec![LLMModel::GPT4o, LLMModel::GPT4oMini, LLMModel::GPT4Turbo]
    }

    fn provider(&self) -> LLMProvider {
        LLMProvider::OpenAI
    }

    async fn health_check(&self) -> Result<(), ToolError> {
        // Would ping API endpoint
        Ok(())
    }
}

// ============================================================================
// ANTHROPIC ADAPTER
// ============================================================================

pub struct AnthropicAdapter {
    pub adapter_id: Uuid,
    pub api_key: String,
    pub base_url: String,
    pub timeout: Duration,
}

impl AnthropicAdapter {
    pub fn new(api_key: String) -> Self {
        Self {
            adapter_id: Uuid::new_v4(),
            api_key,
            base_url: "https://api.anthropic.com/v1".to_string(),
            timeout: Duration::from_secs(120),
        }
    }

    fn model_to_api_name(&self, model: &LLMModel) -> String {
        match model {
            LLMModel::Claude3Opus => "claude-3-opus-20240229".to_string(),
            LLMModel::Claude3Sonnet => "claude-3-sonnet-20240229".to_string(),
            LLMModel::Claude35Sonnet => "claude-3-5-sonnet-20241022".to_string(),
            LLMModel::Claude4Opus => "claude-opus-4-20250514".to_string(),
            LLMModel::Claude4Sonnet => "claude-sonnet-4-20250514".to_string(),
            _ => panic!("Model not supported by Anthropic"),
        }
    }
}

#[async_trait]
impl LLMAdapter for AnthropicAdapter {
    async fn generate(&self, request: LLMRequest) -> Result<LLMResponse, ToolError> {
        let start = std::time::Instant::now();

        // Build messages for Anthropic format
        let messages: Vec<serde_json::Value> = request
            .messages
            .iter()
            .map(|msg| {
                serde_json::json!({
                    "role": match msg.role {
                        ChatRole::User => "user",
                        ChatRole::Assistant => "assistant",
                        ChatRole::System => "user", // Anthropic handles system differently
                    },
                    "content": msg.content
                })
            })
            .collect();

        let _body = serde_json::json!({
            "model": self.model_to_api_name(&request.model),
            "messages": messages,
            "max_tokens": request.max_tokens,
            "system": request.system_prompt,
        });

        // In production, would make actual HTTP request
        let latency = start.elapsed().as_millis() as u64;

        Ok(LLMResponse {
            request_id: request.request_id,
            content: "[Claude Response would be generated here]".to_string(),
            finish_reason: FinishReason::Stop,
            usage: TokenUsage {
                prompt_tokens: 120,
                completion_tokens: 60,
                total_tokens: 180,
                estimated_cost: 0.003,
            },
            latency_ms: latency,
        })
    }

    async fn generate_stream(
        &self,
        _request: LLMRequest,
    ) -> Result<Box<dyn tokio::io::AsyncRead + Send + Unpin>, ToolError> {
        Err(ToolError::ApiError {
            code: 501,
            message: "Streaming not yet implemented".to_string(),
        })
    }

    fn supported_models(&self) -> Vec<LLMModel> {
        vec![
            LLMModel::Claude3Opus,
            LLMModel::Claude3Sonnet,
            LLMModel::Claude35Sonnet,
            LLMModel::Claude4Opus,
            LLMModel::Claude4Sonnet,
        ]
    }

    fn provider(&self) -> LLMProvider {
        LLMProvider::Anthropic
    }

    async fn health_check(&self) -> Result<(), ToolError> {
        Ok(())
    }
}

// ============================================================================
// IMAGE GENERATION ADAPTER
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageGenerationRequest {
    pub request_id: Uuid,
    pub model: ImageModel,
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub width: u32,
    pub height: u32,
    pub num_images: u8,
    pub seed: Option<u64>,
    pub guidance_scale: f32,
    pub num_inference_steps: u32,
    pub style: Option<ImageStyle>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageModel {
    // Flux
    FluxPro,
    FluxDev,
    FluxSchnell,
    
    // SDXL
    SDXL,
    SDXLTurbo,
    
    // Stable Diffusion 3
    SD3,
    SD3Turbo,
    
    // DALL-E
    DallE3,
    DallE2,
    
    // Midjourney (via API)
    Midjourney,
}

impl ImageModel {
    pub fn provider(&self) -> ImageProvider {
        match self {
            ImageModel::FluxPro | ImageModel::FluxDev | ImageModel::FluxSchnell => {
                ImageProvider::Replicate
            }
            ImageModel::SDXL | ImageModel::SDXLTurbo | ImageModel::SD3 | ImageModel::SD3Turbo => {
                ImageProvider::Replicate
            }
            ImageModel::DallE3 | ImageModel::DallE2 => ImageProvider::OpenAI,
            ImageModel::Midjourney => ImageProvider::Midjourney,
        }
    }

    pub fn supports_negative_prompt(&self) -> bool {
        !matches!(self, ImageModel::DallE3 | ImageModel::DallE2)
    }

    pub fn default_steps(&self) -> u32 {
        match self {
            ImageModel::FluxSchnell | ImageModel::SDXLTurbo | ImageModel::SD3Turbo => 4,
            ImageModel::FluxDev => 28,
            ImageModel::FluxPro | ImageModel::SDXL | ImageModel::SD3 => 50,
            ImageModel::DallE3 | ImageModel::DallE2 | ImageModel::Midjourney => 1, // Not applicable
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageProvider {
    OpenAI,
    Replicate,
    Stability,
    Midjourney,
    Local,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageStyle {
    Photorealistic,
    Cinematic,
    Anime,
    Digital3D,
    Illustration,
    Abstract,
    Minimalist,
    Corporate,
    Vibrant,
    Dark,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageGenerationResponse {
    pub request_id: Uuid,
    pub images: Vec<GeneratedImage>,
    pub seed_used: Option<u64>,
    pub latency_ms: u64,
    pub cost: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedImage {
    pub image_id: Uuid,
    pub url: Option<String>,
    pub base64: Option<String>,
    pub width: u32,
    pub height: u32,
    pub revised_prompt: Option<String>,
    pub nsfw_detected: bool,
}

#[async_trait]
pub trait ImageGenerationAdapter: Send + Sync {
    async fn generate(&self, request: ImageGenerationRequest)
        -> Result<ImageGenerationResponse, ToolError>;
    fn supported_models(&self) -> Vec<ImageModel>;
    fn provider(&self) -> ImageProvider;
    async fn health_check(&self) -> Result<(), ToolError>;
}

// ============================================================================
// REPLICATE ADAPTER (Flux, SDXL, etc.)
// ============================================================================

pub struct ReplicateAdapter {
    pub adapter_id: Uuid,
    pub api_token: String,
    pub base_url: String,
    pub timeout: Duration,
}

impl ReplicateAdapter {
    pub fn new(api_token: String) -> Self {
        Self {
            adapter_id: Uuid::new_v4(),
            api_token,
            base_url: "https://api.replicate.com/v1".to_string(),
            timeout: Duration::from_secs(300),
        }
    }

    fn model_to_version(&self, model: &ImageModel) -> &str {
        match model {
            ImageModel::FluxPro => "black-forest-labs/flux-pro",
            ImageModel::FluxDev => "black-forest-labs/flux-dev",
            ImageModel::FluxSchnell => "black-forest-labs/flux-schnell",
            ImageModel::SDXL => "stability-ai/sdxl:latest",
            ImageModel::SDXLTurbo => "stability-ai/sdxl-turbo:latest",
            ImageModel::SD3 => "stability-ai/stable-diffusion-3:latest",
            ImageModel::SD3Turbo => "stability-ai/sd3-turbo:latest",
            _ => panic!("Model not supported by Replicate"),
        }
    }
}

#[async_trait]
impl ImageGenerationAdapter for ReplicateAdapter {
    async fn generate(
        &self,
        request: ImageGenerationRequest,
    ) -> Result<ImageGenerationResponse, ToolError> {
        let start = std::time::Instant::now();

        let _body = serde_json::json!({
            "version": self.model_to_version(&request.model),
            "input": {
                "prompt": request.prompt,
                "negative_prompt": request.negative_prompt,
                "width": request.width,
                "height": request.height,
                "num_outputs": request.num_images,
                "seed": request.seed,
                "guidance_scale": request.guidance_scale,
                "num_inference_steps": request.num_inference_steps,
            }
        });

        // In production, would make actual HTTP request and poll for completion
        let latency = start.elapsed().as_millis() as u64;

        let images: Vec<GeneratedImage> = (0..request.num_images)
            .map(|_| GeneratedImage {
                image_id: Uuid::new_v4(),
                url: Some("https://replicate.delivery/mock-image.png".to_string()),
                base64: None,
                width: request.width,
                height: request.height,
                revised_prompt: None,
                nsfw_detected: false,
            })
            .collect();

        Ok(ImageGenerationResponse {
            request_id: request.request_id,
            images,
            seed_used: request.seed,
            latency_ms: latency,
            cost: 0.05,
        })
    }

    fn supported_models(&self) -> Vec<ImageModel> {
        vec![
            ImageModel::FluxPro,
            ImageModel::FluxDev,
            ImageModel::FluxSchnell,
            ImageModel::SDXL,
            ImageModel::SDXLTurbo,
            ImageModel::SD3,
            ImageModel::SD3Turbo,
        ]
    }

    fn provider(&self) -> ImageProvider {
        ImageProvider::Replicate
    }

    async fn health_check(&self) -> Result<(), ToolError> {
        Ok(())
    }
}

// ============================================================================
// VIDEO GENERATION ADAPTER
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoGenerationRequest {
    pub request_id: Uuid,
    pub model: VideoModel,
    pub prompt: String,
    pub duration_seconds: f32,
    pub aspect_ratio: AspectRatio,
    pub fps: u8,
    pub seed: Option<u64>,
    
    // For image-to-video
    pub start_image: Option<String>, // URL or base64
    pub end_image: Option<String>,
    
    // Style settings
    pub motion_amount: f32, // 0.0 to 1.0
    pub camera_movement: Option<CameraMovement>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoModel {
    // Runway
    RunwayGen3Alpha,
    RunwayGen3AlphaTurbo,
    
    // Pika
    Pika1_0,
    
    // Kling
    Kling1_5,
    
    // Luma
    LumaDreamMachine,
    
    // Sora (when available)
    Sora,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AspectRatio {
    Square,      // 1:1
    Landscape,   // 16:9
    Portrait,    // 9:16
    Wide,        // 21:9
    Standard,    // 4:3
}

impl AspectRatio {
    pub fn dimensions(&self, base_height: u32) -> (u32, u32) {
        match self {
            AspectRatio::Square => (base_height, base_height),
            AspectRatio::Landscape => ((base_height as f32 * 16.0 / 9.0) as u32, base_height),
            AspectRatio::Portrait => (base_height, (base_height as f32 * 16.0 / 9.0) as u32),
            AspectRatio::Wide => ((base_height as f32 * 21.0 / 9.0) as u32, base_height),
            AspectRatio::Standard => ((base_height as f32 * 4.0 / 3.0) as u32, base_height),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CameraMovement {
    Static,
    PanLeft,
    PanRight,
    TiltUp,
    TiltDown,
    ZoomIn,
    ZoomOut,
    Orbit,
    Track,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoGenerationResponse {
    pub request_id: Uuid,
    pub video_url: Option<String>,
    pub video_id: String,
    pub status: VideoStatus,
    pub duration_seconds: f32,
    pub width: u32,
    pub height: u32,
    pub latency_ms: u64,
    pub cost: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoStatus {
    Queued,
    Processing,
    Completed,
    Failed,
}

#[async_trait]
pub trait VideoGenerationAdapter: Send + Sync {
    async fn generate(
        &self,
        request: VideoGenerationRequest,
    ) -> Result<VideoGenerationResponse, ToolError>;
    async fn check_status(&self, video_id: &str) -> Result<VideoGenerationResponse, ToolError>;
    fn supported_models(&self) -> Vec<VideoModel>;
    async fn health_check(&self) -> Result<(), ToolError>;
}

// ============================================================================
// RUNWAY ADAPTER
// ============================================================================

pub struct RunwayAdapter {
    pub adapter_id: Uuid,
    pub api_key: String,
    pub base_url: String,
    pub timeout: Duration,
}

impl RunwayAdapter {
    pub fn new(api_key: String) -> Self {
        Self {
            adapter_id: Uuid::new_v4(),
            api_key,
            base_url: "https://api.runwayml.com/v1".to_string(),
            timeout: Duration::from_secs(600),
        }
    }
}

#[async_trait]
impl VideoGenerationAdapter for RunwayAdapter {
    async fn generate(
        &self,
        request: VideoGenerationRequest,
    ) -> Result<VideoGenerationResponse, ToolError> {
        let start = std::time::Instant::now();

        let (width, height) = request.aspect_ratio.dimensions(720);

        let _body = serde_json::json!({
            "model": match request.model {
                VideoModel::RunwayGen3Alpha => "gen-3-alpha",
                VideoModel::RunwayGen3AlphaTurbo => "gen-3-alpha-turbo",
                _ => panic!("Model not supported by Runway"),
            },
            "prompt": request.prompt,
            "duration": request.duration_seconds,
            "aspect_ratio": format!("{:?}", request.aspect_ratio).to_lowercase(),
            "seed": request.seed,
            "start_image": request.start_image,
            "end_image": request.end_image,
        });

        // In production, would submit job and return video_id for polling
        let latency = start.elapsed().as_millis() as u64;

        Ok(VideoGenerationResponse {
            request_id: request.request_id,
            video_url: None, // Would be populated after processing
            video_id: Uuid::new_v4().to_string(),
            status: VideoStatus::Queued,
            duration_seconds: request.duration_seconds,
            width,
            height,
            latency_ms: latency,
            cost: request.duration_seconds as f64 * 0.50, // $0.50 per second estimate
        })
    }

    async fn check_status(&self, video_id: &str) -> Result<VideoGenerationResponse, ToolError> {
        // Would poll Runway API for status
        Ok(VideoGenerationResponse {
            request_id: Uuid::new_v4(),
            video_url: Some(format!("https://runway.ml/output/{}.mp4", video_id)),
            video_id: video_id.to_string(),
            status: VideoStatus::Completed,
            duration_seconds: 5.0,
            width: 1280,
            height: 720,
            latency_ms: 0,
            cost: 2.50,
        })
    }

    fn supported_models(&self) -> Vec<VideoModel> {
        vec![VideoModel::RunwayGen3Alpha, VideoModel::RunwayGen3AlphaTurbo]
    }

    async fn health_check(&self) -> Result<(), ToolError> {
        Ok(())
    }
}

// ============================================================================
// TEXT-TO-SPEECH ADAPTER
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TTSRequest {
    pub request_id: Uuid,
    pub model: TTSModel,
    pub text: String,
    pub voice_id: String,
    pub language: Option<String>,
    pub speed: f32,      // 0.5 to 2.0
    pub pitch: f32,      // -20 to 20
    pub output_format: AudioFormat,
    pub stability: f32,           // ElevenLabs: voice stability
    pub similarity_boost: f32,    // ElevenLabs: similarity boost
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TTSModel {
    // ElevenLabs
    ElevenLabsMultilingual,
    ElevenLabsTurbo,
    
    // OpenAI
    OpenAITTS,
    OpenAITTSHD,
    
    // Google Cloud
    GoogleWaveNet,
    GoogleNeural2,
    
    // Azure
    AzureNeural,
    
    // Coqui / Local
    CoquiYTTS,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioFormat {
    Mp3,
    Wav,
    Ogg,
    Flac,
    PCM,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TTSResponse {
    pub request_id: Uuid,
    pub audio_url: Option<String>,
    pub audio_base64: Option<String>,
    pub audio_bytes: Option<Vec<u8>>,
    pub duration_seconds: f32,
    pub format: AudioFormat,
    pub characters_used: u32,
    pub latency_ms: u64,
    pub cost: f64,
}

#[async_trait]
pub trait TTSAdapter: Send + Sync {
    async fn synthesize(&self, request: TTSRequest) -> Result<TTSResponse, ToolError>;
    fn available_voices(&self) -> Vec<Voice>;
    fn supported_models(&self) -> Vec<TTSModel>;
    async fn health_check(&self) -> Result<(), ToolError>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Voice {
    pub voice_id: String,
    pub name: String,
    pub gender: Option<String>,
    pub language: String,
    pub accent: Option<String>,
    pub preview_url: Option<String>,
    pub use_cases: Vec<String>,
}

// ============================================================================
// ELEVENLABS ADAPTER
// ============================================================================

pub struct ElevenLabsAdapter {
    pub adapter_id: Uuid,
    pub api_key: String,
    pub base_url: String,
    pub voices_cache: Vec<Voice>,
}

impl ElevenLabsAdapter {
    pub fn new(api_key: String) -> Self {
        Self {
            adapter_id: Uuid::new_v4(),
            api_key,
            base_url: "https://api.elevenlabs.io/v1".to_string(),
            voices_cache: Self::default_voices(),
        }
    }

    fn default_voices() -> Vec<Voice> {
        vec![
            Voice {
                voice_id: "21m00Tcm4TlvDq8ikWAM".to_string(),
                name: "Rachel".to_string(),
                gender: Some("female".to_string()),
                language: "en".to_string(),
                accent: Some("American".to_string()),
                preview_url: None,
                use_cases: vec!["narration".to_string(), "explainer".to_string()],
            },
            Voice {
                voice_id: "AZnzlk1XvdvUeBnXmlld".to_string(),
                name: "Domi".to_string(),
                gender: Some("female".to_string()),
                language: "en".to_string(),
                accent: Some("American".to_string()),
                preview_url: None,
                use_cases: vec!["youthful".to_string(), "energetic".to_string()],
            },
            Voice {
                voice_id: "EXAVITQu4vr4xnSDxMaL".to_string(),
                name: "Bella".to_string(),
                gender: Some("female".to_string()),
                language: "en".to_string(),
                accent: Some("American".to_string()),
                preview_url: None,
                use_cases: vec!["soft".to_string(), "calm".to_string()],
            },
            Voice {
                voice_id: "ErXwobaYiN019PkySvjV".to_string(),
                name: "Antoni".to_string(),
                gender: Some("male".to_string()),
                language: "en".to_string(),
                accent: Some("American".to_string()),
                preview_url: None,
                use_cases: vec!["well-rounded".to_string(), "calm".to_string()],
            },
            Voice {
                voice_id: "MF3mGyEYCl7XYWbV9V6O".to_string(),
                name: "Elli".to_string(),
                gender: Some("female".to_string()),
                language: "en".to_string(),
                accent: Some("American".to_string()),
                preview_url: None,
                use_cases: vec!["young".to_string(), "clear".to_string()],
            },
        ]
    }
}

#[async_trait]
impl TTSAdapter for ElevenLabsAdapter {
    async fn synthesize(&self, request: TTSRequest) -> Result<TTSResponse, ToolError> {
        let start = std::time::Instant::now();

        let _body = serde_json::json!({
            "text": request.text,
            "model_id": match request.model {
                TTSModel::ElevenLabsMultilingual => "eleven_multilingual_v2",
                TTSModel::ElevenLabsTurbo => "eleven_turbo_v2",
                _ => panic!("Model not supported by ElevenLabs"),
            },
            "voice_settings": {
                "stability": request.stability,
                "similarity_boost": request.similarity_boost,
            }
        });

        // Estimate duration (rough: 150 wpm average)
        let word_count = request.text.split_whitespace().count();
        let estimated_duration = (word_count as f32 / 150.0) * 60.0 / request.speed;

        let latency = start.elapsed().as_millis() as u64;

        Ok(TTSResponse {
            request_id: request.request_id,
            audio_url: Some("https://elevenlabs.io/mock-audio.mp3".to_string()),
            audio_base64: None,
            audio_bytes: None,
            duration_seconds: estimated_duration,
            format: request.output_format,
            characters_used: request.text.len() as u32,
            latency_ms: latency,
            cost: (request.text.len() as f64) * 0.00003, // ~$30 per million characters
        })
    }

    fn available_voices(&self) -> Vec<Voice> {
        self.voices_cache.clone()
    }

    fn supported_models(&self) -> Vec<TTSModel> {
        vec![TTSModel::ElevenLabsMultilingual, TTSModel::ElevenLabsTurbo]
    }

    async fn health_check(&self) -> Result<(), ToolError> {
        Ok(())
    }
}

// ============================================================================
// SOCIAL MEDIA SCHEDULING API ADAPTER
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulePostRequest {
    pub request_id: Uuid,
    pub platform: SchedulingPlatform,
    pub scheduled_time: DateTime<Utc>,
    pub content: ScheduledContent,
    pub auto_publish: bool,
    pub notification_email: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SchedulingPlatform {
    Buffer,
    Hootsuite,
    Later,
    SproutSocial,
    Internal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledContent {
    pub text: String,
    pub media_urls: Vec<String>,
    pub link: Option<String>,
    pub hashtags: Vec<String>,
    pub target_platforms: Vec<String>, // "twitter", "instagram", etc.
    pub profile_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulePostResponse {
    pub request_id: Uuid,
    pub post_id: String,
    pub scheduled_time: DateTime<Utc>,
    pub status: ScheduleStatus,
    pub platform_post_ids: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScheduleStatus {
    Scheduled,
    Pending,
    Published,
    Failed,
    Cancelled,
}

#[async_trait]
pub trait SchedulingAdapter: Send + Sync {
    async fn schedule_post(&self, request: SchedulePostRequest)
        -> Result<SchedulePostResponse, ToolError>;
    async fn cancel_post(&self, post_id: &str) -> Result<(), ToolError>;
    async fn reschedule(&self, post_id: &str, new_time: DateTime<Utc>)
        -> Result<SchedulePostResponse, ToolError>;
    async fn get_scheduled_posts(&self, profile_id: &str) -> Result<Vec<SchedulePostResponse>, ToolError>;
    async fn health_check(&self) -> Result<(), ToolError>;
}

// ============================================================================
// TOOL REGISTRY
// ============================================================================

/// Central registry for all tool adapters
pub struct ToolRegistry {
    pub registry_id: Uuid,
    pub llm_adapters: HashMap<LLMProvider, Box<dyn LLMAdapter>>,
    pub image_adapters: HashMap<ImageProvider, Box<dyn ImageGenerationAdapter>>,
    pub video_adapters: HashMap<String, Box<dyn VideoGenerationAdapter>>,
    pub tts_adapters: HashMap<String, Box<dyn TTSAdapter>>,
    pub scheduling_adapters: HashMap<SchedulingPlatform, Box<dyn SchedulingAdapter>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            registry_id: Uuid::new_v4(),
            llm_adapters: HashMap::new(),
            image_adapters: HashMap::new(),
            video_adapters: HashMap::new(),
            tts_adapters: HashMap::new(),
            scheduling_adapters: HashMap::new(),
        }
    }

    pub fn register_llm(&mut self, adapter: Box<dyn LLMAdapter>) {
        self.llm_adapters.insert(adapter.provider(), adapter);
    }

    pub fn register_image(&mut self, adapter: Box<dyn ImageGenerationAdapter>) {
        self.image_adapters.insert(adapter.provider(), adapter);
    }

    pub fn register_video(&mut self, name: String, adapter: Box<dyn VideoGenerationAdapter>) {
        self.video_adapters.insert(name, adapter);
    }

    pub fn register_tts(&mut self, name: String, adapter: Box<dyn TTSAdapter>) {
        self.tts_adapters.insert(name, adapter);
    }

    /// Get best LLM adapter for a given model
    pub fn get_llm_for_model(&self, model: &LLMModel) -> Option<&dyn LLMAdapter> {
        self.llm_adapters.get(&model.provider()).map(|b| b.as_ref())
    }

    /// Get best image adapter for a given model
    pub fn get_image_for_model(&self, model: &ImageModel) -> Option<&dyn ImageGenerationAdapter> {
        self.image_adapters.get(&model.provider()).map(|b| b.as_ref())
    }

    /// Health check all adapters
    pub async fn health_check_all(&self) -> HashMap<String, Result<(), ToolError>> {
        let mut results = HashMap::new();

        for (provider, adapter) in &self.llm_adapters {
            let key = format!("llm:{:?}", provider);
            results.insert(key, adapter.health_check().await);
        }

        for (provider, adapter) in &self.image_adapters {
            let key = format!("image:{:?}", provider);
            results.insert(key, adapter.health_check().await);
        }

        for (name, adapter) in &self.video_adapters {
            let key = format!("video:{}", name);
            results.insert(key, adapter.health_check().await);
        }

        for (name, adapter) in &self.tts_adapters {
            let key = format!("tts:{}", name);
            results.insert(key, adapter.health_check().await);
        }

        results
    }
}

// ============================================================================
// CONVENIENCE BUILDER
// ============================================================================

/// Builder for creating a fully configured tool registry
pub struct ToolRegistryBuilder {
    openai_key: Option<String>,
    anthropic_key: Option<String>,
    replicate_key: Option<String>,
    runway_key: Option<String>,
    elevenlabs_key: Option<String>,
}

impl ToolRegistryBuilder {
    pub fn new() -> Self {
        Self {
            openai_key: None,
            anthropic_key: None,
            replicate_key: None,
            runway_key: None,
            elevenlabs_key: None,
        }
    }

    pub fn with_openai(mut self, api_key: String) -> Self {
        self.openai_key = Some(api_key);
        self
    }

    pub fn with_anthropic(mut self, api_key: String) -> Self {
        self.anthropic_key = Some(api_key);
        self
    }

    pub fn with_replicate(mut self, api_token: String) -> Self {
        self.replicate_key = Some(api_token);
        self
    }

    pub fn with_runway(mut self, api_key: String) -> Self {
        self.runway_key = Some(api_key);
        self
    }

    pub fn with_elevenlabs(mut self, api_key: String) -> Self {
        self.elevenlabs_key = Some(api_key);
        self
    }

    pub fn build(self) -> ToolRegistry {
        let mut registry = ToolRegistry::new();

        if let Some(key) = self.openai_key {
            registry.register_llm(Box::new(OpenAIAdapter::new(key)));
        }

        if let Some(key) = self.anthropic_key {
            registry.register_llm(Box::new(AnthropicAdapter::new(key)));
        }

        if let Some(key) = self.replicate_key {
            registry.register_image(Box::new(ReplicateAdapter::new(key)));
        }

        if let Some(key) = self.runway_key {
            registry.register_video("runway".to_string(), Box::new(RunwayAdapter::new(key)));
        }

        if let Some(key) = self.elevenlabs_key {
            registry.register_tts("elevenlabs".to_string(), Box::new(ElevenLabsAdapter::new(key)));
        }

        registry
    }
}

// ============================================================================
// UNIT TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_model_provider() {
        assert_eq!(LLMModel::GPT4o.provider(), LLMProvider::OpenAI);
        assert_eq!(LLMModel::Claude35Sonnet.provider(), LLMProvider::Anthropic);
        assert_eq!(LLMModel::Llama3_70B.provider(), LLMProvider::Together);
    }

    #[test]
    fn test_llm_request_creation() {
        let request = LLMRequest {
            request_id: Uuid::new_v4(),
            model: LLMModel::Claude35Sonnet,
            system_prompt: Some("You are a helpful assistant.".to_string()),
            messages: vec![ChatMessage {
                role: ChatRole::User,
                content: "Hello!".to_string(),
                name: None,
            }],
            max_tokens: 1024,
            temperature: 0.7,
            top_p: None,
            stop_sequences: vec![],
            response_format: None,
        };

        assert_eq!(request.model, LLMModel::Claude35Sonnet);
        assert_eq!(request.messages.len(), 1);
    }

    #[test]
    fn test_image_model_provider() {
        assert_eq!(ImageModel::FluxPro.provider(), ImageProvider::Replicate);
        assert_eq!(ImageModel::DallE3.provider(), ImageProvider::OpenAI);
    }

    #[test]
    fn test_aspect_ratio_dimensions() {
        let (w, h) = AspectRatio::Landscape.dimensions(720);
        assert_eq!(h, 720);
        assert_eq!(w, 1280);

        let (w, h) = AspectRatio::Square.dimensions(512);
        assert_eq!(w, 512);
        assert_eq!(h, 512);
    }

    #[test]
    fn test_tool_registry_builder() {
        let registry = ToolRegistryBuilder::new()
            .with_openai("test-key".to_string())
            .build();

        assert!(registry.llm_adapters.contains_key(&LLMProvider::OpenAI));
    }

    #[tokio::test]
    async fn test_openai_adapter_generate() {
        let adapter = OpenAIAdapter::new("test-key".to_string());

        let request = LLMRequest {
            request_id: Uuid::new_v4(),
            model: LLMModel::GPT4o,
            system_prompt: None,
            messages: vec![ChatMessage {
                role: ChatRole::User,
                content: "Hello".to_string(),
                name: None,
            }],
            max_tokens: 100,
            temperature: 0.7,
            top_p: None,
            stop_sequences: vec![],
            response_format: None,
        };

        let result = adapter.generate(request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_replicate_adapter_generate() {
        let adapter = ReplicateAdapter::new("test-token".to_string());

        let request = ImageGenerationRequest {
            request_id: Uuid::new_v4(),
            model: ImageModel::FluxSchnell,
            prompt: "A beautiful sunset".to_string(),
            negative_prompt: None,
            width: 1024,
            height: 1024,
            num_images: 1,
            seed: None,
            guidance_scale: 7.5,
            num_inference_steps: 4,
            style: None,
        };

        let result = adapter.generate(request).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().images.len(), 1);
    }

    #[tokio::test]
    async fn test_elevenlabs_adapter_synthesize() {
        let adapter = ElevenLabsAdapter::new("test-key".to_string());

        let request = TTSRequest {
            request_id: Uuid::new_v4(),
            model: TTSModel::ElevenLabsMultilingual,
            text: "Hello, this is a test.".to_string(),
            voice_id: "21m00Tcm4TlvDq8ikWAM".to_string(),
            language: Some("en".to_string()),
            speed: 1.0,
            pitch: 0.0,
            output_format: AudioFormat::Mp3,
            stability: 0.5,
            similarity_boost: 0.5,
        };

        let result = adapter.synthesize(request).await;
        assert!(result.is_ok());
        assert!(result.unwrap().duration_seconds > 0.0);
    }

    #[test]
    fn test_elevenlabs_available_voices() {
        let adapter = ElevenLabsAdapter::new("test-key".to_string());
        let voices = adapter.available_voices();

        assert!(!voices.is_empty());
        assert!(voices.iter().any(|v| v.name == "Rachel"));
    }
}
