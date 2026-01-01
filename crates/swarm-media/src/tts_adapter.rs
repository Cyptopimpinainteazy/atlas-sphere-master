//! Text-to-Speech (TTS) & Voice Cloning Adapter for Swarm Media
//!
//! Provides speech synthesis capabilities with support for:
//! - XTTS-v2 (multilingual TTS with voice cloning)
//! - Piper (lightweight TTS for resource-constrained nodes)
//! - ElevenLabs (cloud fallback for premium quality)
//!
//! Task 7 from ARCHITECTURE_COMPLETE.md

use crate::tool_adapter::{GpuNodeCapabilities, JobId, JobStatus, Priority, ToolAdapter, ToolParams, ToolResourceReq, ToolResult, ToolType};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Supported TTS models
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TtsModel {
    /// XTTS-v2: Multilingual TTS with voice cloning, 4-6GB VRAM
    XttsV2,
    /// Piper: Lightweight TTS, CPU-only capable
    Piper,
    /// ElevenLabs: Cloud API for premium quality (fallback)
    ElevenLabs,
    /// Bark: High quality text-to-speech with prosody control
    Bark,
}

impl Default for TtsModel {
    fn default() -> Self {
        TtsModel::XttsV2
    }
}

/// Voice preset for consistent narrator voice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoicePreset {
    /// Unique identifier for the voice
    pub voice_id: String,
    /// Human-readable name
    pub name: String,
    /// Reference audio samples for voice cloning (paths or URLs)
    pub reference_samples: Vec<String>,
    /// Language code (e.g., "en", "es", "fr")
    pub language: String,
    /// Speaking rate multiplier (0.5 - 2.0)
    pub speaking_rate: f32,
    /// Pitch adjustment (-20 to +20 semitones)
    pub pitch_semitones: i8,
}

impl Default for VoicePreset {
    fn default() -> Self {
        VoicePreset {
            voice_id: "default".to_string(),
            name: "Atlas Default Voice".to_string(),
            reference_samples: vec![],
            language: "en".to_string(),
            speaking_rate: 1.0,
            pitch_semitones: 0,
        }
    }
}

/// TTS generation parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsParams {
    /// Text to synthesize
    pub text: String,
    /// TTS model to use
    pub model: TtsModel,
    /// Voice preset (for cloning or predefined voices)
    pub voice: Option<VoicePreset>,
    /// Output format: "mp3", "wav", "ogg", "flac"
    pub output_format: String,
    /// Sample rate in Hz (22050, 44100, 48000)
    pub sample_rate: u32,
    /// Enable emotion/prosody control (Bark only)
    pub prosody_control: bool,
    /// Language code override
    pub language: Option<String>,
    /// Maximum duration in seconds (truncate if exceeded)
    pub max_duration_secs: Option<u32>,
}

impl Default for TtsParams {
    fn default() -> Self {
        TtsParams {
            text: String::new(),
            model: TtsModel::default(),
            voice: None,
            output_format: "mp3".to_string(),
            sample_rate: 22050,
            prosody_control: false,
            language: None,
            max_duration_secs: Some(300), // 5 minutes max
        }
    }
}

/// TTS job tracking
#[derive(Debug, Clone)]
struct TtsJob {
    job_id: JobId,
    params: TtsParams,
    status: JobStatus,
    created_at: i64,
    started_at: Option<i64>,
    completed_at: Option<i64>,
    result: Option<TtsResult>,
    assigned_node: Option<Uuid>,
}

/// Result of TTS generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsResult {
    /// URL or path to the generated audio file
    pub audio_url: String,
    /// Duration of generated audio in seconds
    pub duration_secs: f32,
    /// Character count processed
    pub chars_processed: usize,
    /// Model used for generation
    pub model_used: TtsModel,
    /// Content hash for caching
    pub content_hash: String,
    /// Generation time in milliseconds
    pub generation_time_ms: u64,
    /// Sample rate of output
    pub sample_rate: u32,
}

/// TTS Adapter Configuration
#[derive(Debug, Clone)]
pub struct TtsAdapterConfig {
    /// Local XTTS server URL
    pub xtts_server_url: String,
    /// Local Piper server URL
    pub piper_server_url: String,
    /// ElevenLabs API key (for cloud fallback)
    pub elevenlabs_api_key: Option<String>,
    /// Maximum text length per request
    pub max_text_length: usize,
    /// Cache directory for voice samples
    pub voice_cache_dir: String,
    /// Default voice preset
    pub default_voice: VoicePreset,
}

impl Default for TtsAdapterConfig {
    fn default() -> Self {
        TtsAdapterConfig {
            xtts_server_url: "http://localhost:8002".to_string(),
            piper_server_url: "http://localhost:8003".to_string(),
            elevenlabs_api_key: None,
            max_text_length: 10000,
            voice_cache_dir: "/var/cache/tts/voices".to_string(),
            default_voice: VoicePreset::default(),
        }
    }
}

/// Text-to-Speech Adapter
/// 
/// Implements the ToolAdapter trait for speech synthesis with support for
/// multiple TTS engines and voice cloning capabilities.
pub struct TtsAdapter {
    config: TtsAdapterConfig,
    jobs: Arc<RwLock<HashMap<JobId, TtsJob>>>,
    voice_presets: Arc<RwLock<HashMap<String, VoicePreset>>>,
    content_cache: Arc<RwLock<HashMap<String, TtsResult>>>,
}

impl TtsAdapter {
    pub fn new(config: TtsAdapterConfig) -> Self {
        let mut presets = HashMap::new();
        presets.insert("default".to_string(), config.default_voice.clone());
        
        // Add built-in voice presets
        presets.insert("narrator".to_string(), VoicePreset {
            voice_id: "narrator".to_string(),
            name: "Professional Narrator".to_string(),
            reference_samples: vec![],
            language: "en".to_string(),
            speaking_rate: 0.95,
            pitch_semitones: -2,
        });
        
        presets.insert("founder".to_string(), VoicePreset {
            voice_id: "founder".to_string(),
            name: "Founder Voice (Confident)".to_string(),
            reference_samples: vec![],
            language: "en".to_string(),
            speaking_rate: 1.05,
            pitch_semitones: 0,
        });
        
        presets.insert("tech_explainer".to_string(), VoicePreset {
            voice_id: "tech_explainer".to_string(),
            name: "Technical Explainer".to_string(),
            reference_samples: vec![],
            language: "en".to_string(),
            speaking_rate: 1.1,
            pitch_semitones: 1,
        });

        TtsAdapter {
            config,
            jobs: Arc::new(RwLock::new(HashMap::new())),
            voice_presets: Arc::new(RwLock::new(presets)),
            content_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Generate content hash for caching
    fn compute_content_hash(params: &TtsParams) -> String {
        let mut hasher = Sha256::new();
        hasher.update(params.text.as_bytes());
        hasher.update(format!("{:?}", params.model).as_bytes());
        hasher.update(params.output_format.as_bytes());
        hasher.update(&params.sample_rate.to_le_bytes());
        if let Some(ref voice) = params.voice {
            hasher.update(voice.voice_id.as_bytes());
        }
        let result = hasher.finalize();
        hex::encode(result)
    }

    /// Estimate audio duration from text length
    fn estimate_duration(text: &str, speaking_rate: f32) -> f32 {
        // Average speaking rate: ~150 words per minute
        // Average word length: ~5 characters
        let words = text.len() as f32 / 5.0;
        let minutes = words / (150.0 * speaking_rate);
        minutes * 60.0
    }

    /// Call local XTTS server
    async fn call_xtts(&self, params: &TtsParams) -> Result<TtsResult, String> {
        let client = reqwest::Client::new();
        
        let voice = params.voice.clone().unwrap_or_else(|| self.config.default_voice.clone());
        
        let request_body = serde_json::json!({
            "text": params.text,
            "language": params.language.clone().unwrap_or(voice.language.clone()),
            "speaker_wav": voice.reference_samples.first().cloned().unwrap_or_default(),
            "sample_rate": params.sample_rate,
        });

        let start_time = std::time::Instant::now();
        
        let response = client
            .post(&format!("{}/tts", self.config.xtts_server_url))
            .json(&request_body)
            .timeout(std::time::Duration::from_secs(120))
            .send()
            .await
            .map_err(|e| format!("XTTS request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("XTTS server error: {}", response.status()));
        }

        let audio_bytes = response
            .bytes()
            .await
            .map_err(|e| format!("Failed to read audio: {}", e))?;

        let generation_time_ms = start_time.elapsed().as_millis() as u64;

        // Save to storage (in production, upload to S3/storage)
        let output_path = format!(
            "/tmp/tts_{}.{}",
            Uuid::new_v4(),
            params.output_format
        );
        
        tokio::fs::write(&output_path, &audio_bytes)
            .await
            .map_err(|e| format!("Failed to save audio: {}", e))?;

        let duration_secs = Self::estimate_duration(&params.text, voice.speaking_rate);

        Ok(TtsResult {
            audio_url: output_path,
            duration_secs,
            chars_processed: params.text.len(),
            model_used: TtsModel::XttsV2,
            content_hash: Self::compute_content_hash(params),
            generation_time_ms,
            sample_rate: params.sample_rate,
        })
    }

    /// Call local Piper server (lightweight fallback)
    async fn call_piper(&self, params: &TtsParams) -> Result<TtsResult, String> {
        let client = reqwest::Client::new();
        
        let request_body = serde_json::json!({
            "text": params.text,
            "output_file": format!("/tmp/piper_{}.{}", Uuid::new_v4(), params.output_format),
        });

        let start_time = std::time::Instant::now();
        
        let response = client
            .post(&format!("{}/synthesize", self.config.piper_server_url))
            .json(&request_body)
            .timeout(std::time::Duration::from_secs(60))
            .send()
            .await
            .map_err(|e| format!("Piper request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Piper server error: {}", response.status()));
        }

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse Piper response: {}", e))?;

        let generation_time_ms = start_time.elapsed().as_millis() as u64;

        let audio_url = result["output_file"]
            .as_str()
            .unwrap_or("/tmp/piper_output.wav")
            .to_string();

        Ok(TtsResult {
            audio_url,
            duration_secs: Self::estimate_duration(&params.text, 1.0),
            chars_processed: params.text.len(),
            model_used: TtsModel::Piper,
            content_hash: Self::compute_content_hash(params),
            generation_time_ms,
            sample_rate: 22050,
        })
    }

    /// Call ElevenLabs API (cloud fallback)
    async fn call_elevenlabs(&self, params: &TtsParams) -> Result<TtsResult, String> {
        let api_key = self.config.elevenlabs_api_key.as_ref()
            .ok_or("ElevenLabs API key not configured")?;

        let client = reqwest::Client::new();
        
        let voice_id = params.voice.as_ref()
            .map(|v| v.voice_id.clone())
            .unwrap_or_else(|| "21m00Tcm4TlvDq8ikWAM".to_string()); // Default Rachel voice

        let request_body = serde_json::json!({
            "text": params.text,
            "model_id": "eleven_monolingual_v1",
            "voice_settings": {
                "stability": 0.75,
                "similarity_boost": 0.75,
            }
        });

        let start_time = std::time::Instant::now();

        let response = client
            .post(&format!(
                "https://api.elevenlabs.io/v1/text-to-speech/{}",
                voice_id
            ))
            .header("xi-api-key", api_key)
            .header("Content-Type", "application/json")
            .json(&request_body)
            .timeout(std::time::Duration::from_secs(60))
            .send()
            .await
            .map_err(|e| format!("ElevenLabs request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("ElevenLabs API error: {}", response.status()));
        }

        let audio_bytes = response
            .bytes()
            .await
            .map_err(|e| format!("Failed to read audio: {}", e))?;

        let generation_time_ms = start_time.elapsed().as_millis() as u64;

        let output_path = format!("/tmp/elevenlabs_{}.mp3", Uuid::new_v4());
        
        tokio::fs::write(&output_path, &audio_bytes)
            .await
            .map_err(|e| format!("Failed to save audio: {}", e))?;

        Ok(TtsResult {
            audio_url: output_path,
            duration_secs: Self::estimate_duration(&params.text, 1.0),
            chars_processed: params.text.len(),
            model_used: TtsModel::ElevenLabs,
            content_hash: Self::compute_content_hash(params),
            generation_time_ms,
            sample_rate: 44100,
        })
    }

    /// Execute TTS generation with fallback chain
    async fn generate_speech(&self, params: &TtsParams) -> Result<TtsResult, String> {
        // Check cache first
        let cache_key = Self::compute_content_hash(params);
        {
            let cache = self.content_cache.read().await;
            if let Some(cached) = cache.get(&cache_key) {
                return Ok(cached.clone());
            }
        }

        // Try requested model first, then fallback
        let result = match params.model {
            TtsModel::XttsV2 => {
                match self.call_xtts(params).await {
                    Ok(r) => Ok(r),
                    Err(_) => {
                        // Fallback to Piper
                        match self.call_piper(params).await {
                            Ok(r) => Ok(r),
                            Err(_) => {
                                // Final fallback to cloud
                                self.call_elevenlabs(params).await
                            }
                        }
                    }
                }
            }
            TtsModel::Piper => {
                match self.call_piper(params).await {
                    Ok(r) => Ok(r),
                    Err(_) => self.call_elevenlabs(params).await,
                }
            }
            TtsModel::ElevenLabs => self.call_elevenlabs(params).await,
            TtsModel::Bark => {
                // Bark not yet implemented, fallback to XTTS
                self.call_xtts(params).await
            }
        }?;

        // Cache the result
        {
            let mut cache = self.content_cache.write().await;
            cache.insert(cache_key, result.clone());
        }

        Ok(result)
    }

    /// Register a custom voice preset
    pub async fn register_voice_preset(&self, preset: VoicePreset) -> Result<(), String> {
        let mut presets = self.voice_presets.write().await;
        presets.insert(preset.voice_id.clone(), preset);
        Ok(())
    }

    /// Get available voice presets
    pub async fn list_voice_presets(&self) -> Vec<VoicePreset> {
        let presets = self.voice_presets.read().await;
        presets.values().cloned().collect()
    }
}

#[async_trait]
impl ToolAdapter for TtsAdapter {
    fn tool_type(&self) -> ToolType {
        ToolType::TextToSpeech
    }

    async fn validate_params(&self, params: &ToolParams) -> Result<(), String> {
        let tts_params: TtsParams = serde_json::from_value(params.params.clone())
            .map_err(|e| format!("Invalid TTS params: {}", e))?;

        if tts_params.text.is_empty() {
            return Err("Text cannot be empty".to_string());
        }

        if tts_params.text.len() > self.config.max_text_length {
            return Err(format!(
                "Text exceeds maximum length of {} characters",
                self.config.max_text_length
            ));
        }

        let valid_formats = ["mp3", "wav", "ogg", "flac"];
        if !valid_formats.contains(&tts_params.output_format.as_str()) {
            return Err(format!(
                "Invalid output format. Supported: {:?}",
                valid_formats
            ));
        }

        let valid_sample_rates = [22050, 44100, 48000];
        if !valid_sample_rates.contains(&tts_params.sample_rate) {
            return Err(format!(
                "Invalid sample rate. Supported: {:?}",
                valid_sample_rates
            ));
        }

        Ok(())
    }

    async fn invoke(&self, params: ToolParams) -> Result<JobId, String> {
        let tts_params: TtsParams = serde_json::from_value(params.params.clone())
            .map_err(|e| format!("Invalid TTS params: {}", e))?;

        let job_id = Uuid::new_v4();
        let job = TtsJob {
            job_id,
            params: tts_params,
            status: JobStatus::Queued,
            created_at: chrono::Utc::now().timestamp(),
            started_at: None,
            completed_at: None,
            result: None,
            assigned_node: None,
        };

        {
            let mut jobs = self.jobs.write().await;
            jobs.insert(job_id, job);
        }

        // Execute in background
        let adapter = self.clone();
        let job_id_clone = job_id;
        tokio::spawn(async move {
            adapter.execute_job(job_id_clone).await;
        });

        Ok(job_id)
    }

    async fn get_status(&self, job_id: JobId) -> Result<JobStatus, String> {
        let jobs = self.jobs.read().await;
        jobs.get(&job_id)
            .map(|j| j.status.clone())
            .ok_or_else(|| format!("Job {} not found", job_id))
    }

    async fn get_result(&self, job_id: JobId) -> Result<ToolResult, String> {
        let jobs = self.jobs.read().await;
        let job = jobs
            .get(&job_id)
            .ok_or_else(|| format!("Job {} not found", job_id))?;

        match &job.status {
            JobStatus::Completed => {
                let result = job.result.as_ref()
                    .ok_or("Result not available")?;
                
                Ok(ToolResult {
                    job_id,
                    tool_type: ToolType::TextToSpeech,
                    output: serde_json::to_value(result).unwrap(),
                    execution_time_ms: result.generation_time_ms as u32,
                    content_hash: Some(result.content_hash.clone()),
                    executed_by_node: job.assigned_node.unwrap_or(Uuid::nil()),
                })
            }
            JobStatus::Failed(err) => Err(format!("Job failed: {}", err)),
            _ => Err("Job not yet completed".to_string()),
        }
    }

    async fn cancel_job(&self, job_id: JobId) -> Result<(), String> {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.get_mut(&job_id) {
            match job.status {
                JobStatus::Queued | JobStatus::Assigned => {
                    job.status = JobStatus::Cancelled;
                    Ok(())
                }
                JobStatus::Running => Err("Cannot cancel running job".to_string()),
                _ => Err("Job already completed or cancelled".to_string()),
            }
        } else {
            Err(format!("Job {} not found", job_id))
        }
    }

    fn resource_requirements(&self, params: &ToolParams) -> ToolResourceReq {
        let tts_params: TtsParams = serde_json::from_value(params.params.clone())
            .unwrap_or_default();

        match tts_params.model {
            TtsModel::XttsV2 => ToolResourceReq {
                min_vram_gb: 6,
                preferred_latency_ms: 5000,
                supports_batching: false,
            },
            TtsModel::Piper => ToolResourceReq {
                min_vram_gb: 0, // CPU only
                preferred_latency_ms: 2000,
                supports_batching: true,
            },
            TtsModel::ElevenLabs => ToolResourceReq {
                min_vram_gb: 0, // Cloud API
                preferred_latency_ms: 3000,
                supports_batching: false,
            },
            TtsModel::Bark => ToolResourceReq {
                min_vram_gb: 8,
                preferred_latency_ms: 10000,
                supports_batching: false,
            },
        }
    }
}

impl TtsAdapter {
    async fn execute_job(&self, job_id: JobId) {
        // Update status to running
        {
            let mut jobs = self.jobs.write().await;
            if let Some(job) = jobs.get_mut(&job_id) {
                job.status = JobStatus::Running;
                job.started_at = Some(chrono::Utc::now().timestamp());
            }
        }

        // Get job params
        let params = {
            let jobs = self.jobs.read().await;
            jobs.get(&job_id).map(|j| j.params.clone())
        };

        let Some(params) = params else {
            return;
        };

        // Execute generation
        let result = self.generate_speech(&params).await;

        // Update job with result
        {
            let mut jobs = self.jobs.write().await;
            if let Some(job) = jobs.get_mut(&job_id) {
                match result {
                    Ok(tts_result) => {
                        job.status = JobStatus::Completed;
                        job.result = Some(tts_result);
                    }
                    Err(err) => {
                        job.status = JobStatus::Failed(err);
                    }
                }
                job.completed_at = Some(chrono::Utc::now().timestamp());
            }
        }
    }
}

impl Clone for TtsAdapter {
    fn clone(&self) -> Self {
        TtsAdapter {
            config: self.config.clone(),
            jobs: Arc::clone(&self.jobs),
            voice_presets: Arc::clone(&self.voice_presets),
            content_cache: Arc::clone(&self.content_cache),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tts_params_default() {
        let params = TtsParams::default();
        assert_eq!(params.output_format, "mp3");
        assert_eq!(params.sample_rate, 22050);
    }

    #[test]
    fn test_voice_preset_default() {
        let preset = VoicePreset::default();
        assert_eq!(preset.language, "en");
        assert_eq!(preset.speaking_rate, 1.0);
    }

    #[test]
    fn test_content_hash_consistency() {
        let params1 = TtsParams {
            text: "Hello world".to_string(),
            ..Default::default()
        };
        let params2 = TtsParams {
            text: "Hello world".to_string(),
            ..Default::default()
        };

        let hash1 = TtsAdapter::compute_content_hash(&params1);
        let hash2 = TtsAdapter::compute_content_hash(&params2);

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_duration_estimation() {
        // 150 words at 5 chars each = 750 chars per minute
        // 750 chars = 1 minute at speaking rate 1.0
        let duration = TtsAdapter::estimate_duration("a".repeat(750).as_str(), 1.0);
        assert!((duration - 60.0).abs() < 1.0);
    }

    #[tokio::test]
    async fn test_adapter_creation() {
        let config = TtsAdapterConfig::default();
        let adapter = TtsAdapter::new(config);

        let presets = adapter.list_voice_presets().await;
        assert!(presets.len() >= 3); // default, narrator, founder, tech_explainer
    }

    #[tokio::test]
    async fn test_param_validation() {
        let config = TtsAdapterConfig::default();
        let adapter = TtsAdapter::new(config);

        // Empty text should fail
        let params = ToolParams::new(serde_json::json!({
            "text": "",
            "model": "XttsV2",
            "output_format": "mp3",
            "sample_rate": 22050
        }));

        let result = adapter.validate_params(&params).await;
        assert!(result.is_err());

        // Valid params should pass
        let params = ToolParams::new(serde_json::json!({
            "text": "Hello, this is a test.",
            "model": "XttsV2",
            "output_format": "mp3",
            "sample_rate": 22050
        }));

        let result = adapter.validate_params(&params).await;
        assert!(result.is_ok());
    }
}
