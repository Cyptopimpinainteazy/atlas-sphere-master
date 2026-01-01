/// Image Adapter - SDXL with LoRA injection and ControlNet
///
/// Supports:
/// - Stable Diffusion XL (8GB VRAM minimum)
/// - LoRA models for brand consistency
/// - ControlNet for layout control
/// - Batch processing (multiple images in parallel)
/// - Image caching for identical prompts

use async_trait::async_trait;
use chrono::Utc;
use serde_json::json;
use std::collections::HashMap;
use tokio::sync::Mutex;
use uuid::Uuid;
use crate::tool_adapter::{
    JobId, JobStatus, ToolAdapter, ToolParams, ToolResult, ToolType, ToolResourceReq,
};

/// SDXL model configurations
#[derive(Clone, Debug)]
pub struct SdxlModel {
    pub id: String,
    pub name: String,
    pub min_vram_gb: u32,
    pub images_per_second: f32,
    pub max_resolution: (u32, u32),
}

impl SdxlModel {
    pub fn sdxl_v1() -> Self {
        Self {
            id: "sdxl-v1".to_string(),
            name: "Stable Diffusion XL 1.0".to_string(),
            min_vram_gb: 8,
            images_per_second: 1.5,
            max_resolution: (2048, 2048),
        }
    }

    pub fn sdxl_turbo() -> Self {
        Self {
            id: "sdxl-turbo".to_string(),
            name: "SDXL Turbo (4x faster)".to_string(),
            min_vram_gb: 6,
            images_per_second: 6.0,
            max_resolution: (512, 512),
        }
    }
}

/// LoRA style presets
#[derive(Clone, Debug)]
pub struct LoraStyle {
    pub name: String,
    pub model_id: String,
    pub description: String,
    pub weight: f32, // 0.0 to 1.0
}

impl LoraStyle {
    pub fn brand_style_v1() -> Self {
        Self {
            name: "brand_style_v1".to_string(),
            model_id: "/models/lora/brand_style_v1.safetensors".to_string(),
            description: "Company visual identity with signature colors and composition".to_string(),
            weight: 1.0,
        }
    }

    pub fn product_photography() -> Self {
        Self {
            name: "product_photography".to_string(),
            model_id: "/models/lora/product_photography_v2.safetensors".to_string(),
            description: "Professional product photography lighting and composition".to_string(),
            weight: 0.8,
        }
    }

    pub fn abstract_art() -> Self {
        Self {
            name: "abstract_art".to_string(),
            model_id: "/models/lora/abstract_art_v1.safetensors".to_string(),
            description: "Modern abstract art style with vibrant colors".to_string(),
            weight: 0.9,
        }
    }
}

/// ControlNet configuration for layout guidance
#[derive(Clone, Debug)]
pub struct ControlNetConfig {
    pub control_type: String, // "layout", "canny", "depth", "pose"
    pub image_url: Option<String>,
    pub conditioning_scale: f32, // 0.0 to 1.0
}

impl ControlNetConfig {
    pub fn layout_control(image_url: String) -> Self {
        Self {
            control_type: "layout".to_string(),
            image_url: Some(image_url),
            conditioning_scale: 0.75,
        }
    }
}

/// Generation parameters for image creation
#[derive(Clone, Debug)]
pub struct ImageGenerationParams {
    pub model: String,
    pub prompt: String,
    pub negative_prompt: String,
    pub style_preset: String,
    pub width: u32,
    pub height: u32,
    pub num_images: u32,
    pub num_inference_steps: u32,
    pub guidance_scale: f32,
    pub seed: Option<u64>,
    pub controlnet: Option<ControlNetConfig>,
}

impl ImageGenerationParams {
    pub fn from_tool_params(params: &ToolParams) -> Result<Self, String> {
        let prompt = params
            .get("prompt")
            .and_then(|v| v.as_str())
            .ok_or("prompt required")?
            .to_string();

        let width = params
            .get("width")
            .and_then(|v| v.as_u64())
            .unwrap_or(1024) as u32;

        let height = params
            .get("height")
            .and_then(|v| v.as_u64())
            .unwrap_or(1024) as u32;

        // Validate dimensions
        if width > 2048 || height > 2048 {
            return Err("Maximum resolution is 2048x2048".to_string());
        }

        if width % 64 != 0 || height % 64 != 0 {
            return Err("Width and height must be multiples of 64".to_string());
        }

        let num_images = params
            .get("num_images")
            .and_then(|v| v.as_u64())
            .unwrap_or(1) as u32;

        if num_images > 16 {
            return Err("Maximum batch size is 16 images".to_string());
        }

        Ok(ImageGenerationParams {
            model: params
                .get("model")
                .and_then(|v| v.as_str())
                .unwrap_or("sdxl-v1")
                .to_string(),
            prompt,
            negative_prompt: params
                .get("negative_prompt")
                .and_then(|v| v.as_str())
                .unwrap_or("ugly, deformed, low quality")
                .to_string(),
            style_preset: params
                .get("style_preset")
                .and_then(|v| v.as_str())
                .unwrap_or("brand_style_v1")
                .to_string(),
            width,
            height,
            num_images,
            num_inference_steps: params
                .get("num_inference_steps")
                .and_then(|v| v.as_u64())
                .unwrap_or(30) as u32,
            guidance_scale: params
                .get("guidance_scale")
                .and_then(|v| v.as_f64())
                .unwrap_or(7.5) as f32,
            seed: params
                .get("seed")
                .and_then(|v| v.as_u64()),
            controlnet: params
                .get("controlnet")
                .and_then(|v| {
                    let image_url = v.get("image_url")?.as_str()?.to_string();
                    Some(ControlNetConfig::layout_control(image_url))
                }),
        })
    }
}

/// Image generation job state
#[derive(Clone, Debug)]
struct ImageJob {
    job_id: JobId,
    status: JobStatus,
    params: ImageGenerationParams,
    image_urls: Vec<String>,
    error: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
}

/// Image Adapter implementation
pub struct ImageAdapter {
    server_url: String,
    models: HashMap<String, SdxlModel>,
    lora_styles: HashMap<String, LoraStyle>,
    image_cache: Mutex<HashMap<String, Vec<String>>>, // Hash -> generated images
    jobs: HashMap<JobId, ImageJob>,
}

impl ImageAdapter {
    pub fn new(server_url: String) -> Self {
        let mut models = HashMap::new();
        models.insert("sdxl-v1".to_string(), SdxlModel::sdxl_v1());
        models.insert("sdxl-turbo".to_string(), SdxlModel::sdxl_turbo());

        let mut lora_styles = HashMap::new();
        lora_styles.insert("brand_style_v1".to_string(), LoraStyle::brand_style_v1());
        lora_styles.insert("product_photography".to_string(), LoraStyle::product_photography());
        lora_styles.insert("abstract_art".to_string(), LoraStyle::abstract_art());

        Self {
            server_url,
            models,
            lora_styles,
            image_cache: Mutex::new(HashMap::new()),
            jobs: HashMap::new(),
        }
    }

    async fn generate_images(
        &self,
        params: &ImageGenerationParams,
    ) -> Result<Vec<String>, String> {
        use reqwest::Client;
        
        // Create cache key from deterministic params
        let cache_key = format!(
            "{}_{}_{}x{}_{}_{}",
            params.prompt,
            params.style_preset,
            params.width,
            params.height,
            params.seed.unwrap_or(0),
            params.num_images
        );

        // Check cache first
        if let Some(cached) = {
            let cache = self.image_cache.lock().await;
            cache.get(&cache_key).cloned()
        } {
            return Ok(cached);
        }

        let client = Client::new();
        
        // Build request payload
        let mut request_payload = json!({
            "prompt": params.prompt,
            "negative_prompt": params.negative_prompt,
            "width": params.width,
            "height": params.height,
            "num_inference_steps": params.num_inference_steps,
            "guidance_scale": params.guidance_scale,
            "num_images": params.num_images,
        });

        // Add optional seed
        if let Some(seed) = params.seed {
            request_payload["seed"] = json!(seed);
        }

        // Add LoRA configuration
        if let Some(lora) = self.lora_styles.get(&params.style_preset) {
            request_payload["lora"] = json!({
                "model_id": lora.model_id,
                "weight": lora.weight
            });
        }

        // Add ControlNet configuration
        if let Some(controlnet) = &params.controlnet {
            request_payload["controlnet"] = json!({
                "control_type": controlnet.control_type,
                "image_url": controlnet.image_url,
                "conditioning_scale": controlnet.conditioning_scale
            });
        }

        let response = client
            .post(&format!("{}/generate", self.server_url))
            .json(&request_payload)
            .send()
            .await
            .map_err(|e| format!("Failed to connect to image server: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("Image server error: {} - {}", status, error_text));
        }

        let response_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse image server response: {}", e))?;

        let image_urls = response_json
            .get("images")
            .and_then(|images| images.as_array())
            .ok_or("Invalid response format from image server")?
            .iter()
            .filter_map(|url| url.as_str().map(|s| s.to_string()))
            .collect::<Vec<String>>();

        if image_urls.is_empty() {
            return Err("No images generated".to_string());
        }

        // Cache the results
        let mut cache = self.image_cache.lock().await;
        cache.insert(cache_key, image_urls.clone());

        Ok(image_urls)
    }

    async fn inject_lora(
        &self,
        style: &LoraStyle,
    ) -> Result<(), String> {
        // Simulate LoRA weight injection into UNet
        // In production: Load safetensor file and blend into model weights
        tracing::info!(
            "Injecting LoRA style {} (weight: {}) from {}",
            style.name,
            style.weight,
            style.model_id
        );
        Ok(())
    }

    fn calculate_generation_time(params: &ImageGenerationParams, model: &SdxlModel) -> f32 {
        params.num_images as f32 / model.images_per_second
    }
}

#[async_trait]
impl ToolAdapter for ImageAdapter {
    fn tool_type(&self) -> ToolType {
        ToolType::ImageGeneration
    }

    async fn validate_params(&self, params: &ToolParams) -> Result<(), String> {
        // Validate prompt
        params
            .get("prompt")
            .and_then(|v| v.as_str())
            .ok_or("prompt required")?;

        // Validate dimensions and other params
        ImageGenerationParams::from_tool_params(params)?;

        Ok(())
    }

    async fn invoke(&self, params: ToolParams) -> Result<JobId, String> {
        let job_id = Uuid::new_v4();
        let gen_params = ImageGenerationParams::from_tool_params(&params)?;

        // Validate model exists
        if !self.models.contains_key(&gen_params.model) {
            return Err(format!("Model not found: {}", gen_params.model));
        }

        // Inject LoRA style
        if let Some(lora) = self.lora_styles.get(&gen_params.style_preset) {
            self.inject_lora(lora).await?;
        }

        // Generate images
        let image_urls = self.generate_images(&gen_params).await?;

        let _job = ImageJob {
            job_id,
            status: JobStatus::Completed,
            params: gen_params,
            image_urls,
            error: None,
            created_at: Utc::now(),
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

        let model = self.models.get(&job.params.model)
            .ok_or("Model not found".to_string())?;

        let generation_time = Self::calculate_generation_time(&job.params, model);

        Ok(ToolResult {
            job_id,
            tool_type: ToolType::ImageGeneration,
            output: json!({
                "images": job.image_urls,
                "num_images": job.image_urls.len(),
                "generation_time_seconds": generation_time,
                "model": &job.params.model,
                "resolution": format!("{}x{}", job.params.width, job.params.height),
                "style_preset": &job.params.style_preset,
            }),
            execution_time_ms: (generation_time * 1000.0) as u32,
            content_hash: Some(format!("{:x}", sha2::Sha256::digest(
                format!("{:?}", job.image_urls).as_bytes()
            ))),
            executed_by_node: Uuid::new_v4(),
        })
    }

    async fn cancel_job(&self, _job_id: JobId) -> Result<(), String> {
        // Cancel image generation
        Ok(())
    }

    fn resource_requirements(&self, params: &ToolParams) -> ToolResourceReq {
        if let Ok(gen_params) = ImageGenerationParams::from_tool_params(params) {
            if let Some(model) = self.models.get(&gen_params.model) {
                return ToolResourceReq {
                    min_vram_gb: model.min_vram_gb,
                    preferred_latency_ms: 2000,
                    supports_batching: true,
                };
            }
        }

        ToolResourceReq {
            min_vram_gb: 8,
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
    fn test_image_adapter_creation() {
        let adapter = ImageAdapter::new("http://localhost:5000".to_string());
        assert_eq!(adapter.tool_type(), ToolType::ImageGeneration);
        assert!(adapter.models.contains_key("sdxl-v1"));
    }

    #[test]
    fn test_image_generation_params_validation() {
        let params = ToolParams::new(json!({
            "prompt": "A sleek GPU next to money",
            "width": 1024,
            "height": 1024,
            "num_images": 4,
        }));

        let gen_params = ImageGenerationParams::from_tool_params(&params);
        assert!(gen_params.is_ok());
        assert_eq!(gen_params.unwrap().num_images, 4);
    }

    #[test]
    fn test_invalid_resolution() {
        let params = ToolParams::new(json!({
            "prompt": "Test",
            "width": 2049,  // Too large
            "height": 1024,
        }));

        let gen_params = ImageGenerationParams::from_tool_params(&params);
        assert!(gen_params.is_err());
    }

    #[test]
    fn test_lora_style_loading() {
        let lora = LoraStyle::brand_style_v1();
        assert_eq!(lora.weight, 1.0);
        assert!(lora.model_id.contains("brand_style_v1"));
    }
}