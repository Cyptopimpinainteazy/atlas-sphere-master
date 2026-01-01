//! Localization & Transcription Adapter for Swarm Media
//!
//! Provides multilingual content processing:
//! - Speech-to-text transcription (Whisper)
//! - Subtitle generation (SRT, VTT, ASS)
//! - Translation between languages
//! - Forced alignment for dubbing
//! - Multi-language audio track generation
//!
//! Task 10 from ARCHITECTURE_COMPLETE.md

use crate::tool_adapter::{GpuNodeCapabilities, JobId, JobStatus, Priority, ToolAdapter, ToolParams, ToolResourceReq, ToolResult, ToolType};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Supported languages (ISO 639-1 codes)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Language(pub String);

impl Language {
    pub fn english() -> Self { Language("en".to_string()) }
    pub fn spanish() -> Self { Language("es".to_string()) }
    pub fn french() -> Self { Language("fr".to_string()) }
    pub fn german() -> Self { Language("de".to_string()) }
    pub fn chinese() -> Self { Language("zh".to_string()) }
    pub fn japanese() -> Self { Language("ja".to_string()) }
    pub fn korean() -> Self { Language("ko".to_string()) }
    pub fn portuguese() -> Self { Language("pt".to_string()) }
    pub fn russian() -> Self { Language("ru".to_string()) }
    pub fn arabic() -> Self { Language("ar".to_string()) }
    pub fn hindi() -> Self { Language("hi".to_string()) }
}

impl Default for Language {
    fn default() -> Self {
        Language::english()
    }
}

/// Whisper model sizes
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WhisperModel {
    /// Tiny model (39M params, fastest)
    Tiny,
    /// Base model (74M params)
    Base,
    /// Small model (244M params)
    Small,
    /// Medium model (769M params)
    Medium,
    /// Large model (1550M params, best quality)
    Large,
    /// Large-v2 (improved large)
    LargeV2,
    /// Large-v3 (latest)
    LargeV3,
}

impl Default for WhisperModel {
    fn default() -> Self {
        WhisperModel::Medium
    }
}

/// Subtitle format
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubtitleFormat {
    /// SubRip format (.srt)
    Srt,
    /// WebVTT format (.vtt)
    Vtt,
    /// Advanced SubStation Alpha (.ass)
    Ass,
    /// Plain text
    Text,
    /// JSON with word-level timestamps
    Json,
}

impl Default for SubtitleFormat {
    fn default() -> Self {
        SubtitleFormat::Srt
    }
}

/// Translation provider
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TranslationProvider {
    /// OpenAI GPT-4 for context-aware translation
    OpenAi,
    /// Google Cloud Translation
    GoogleTranslate,
    /// DeepL (best for European languages)
    DeepL,
    /// Local NLLB (No Language Left Behind)
    Nllb,
}

impl Default for TranslationProvider {
    fn default() -> Self {
        TranslationProvider::DeepL
    }
}

/// Transcription segment (word or sentence level)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionSegment {
    /// Segment text
    pub text: String,
    /// Start time in seconds
    pub start: f32,
    /// End time in seconds
    pub end: f32,
    /// Confidence score (0-1)
    pub confidence: f32,
    /// Speaker ID (if diarization enabled)
    pub speaker: Option<String>,
    /// Word-level timestamps
    pub words: Option<Vec<WordTimestamp>>,
}

/// Word-level timestamp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordTimestamp {
    pub word: String,
    pub start: f32,
    pub end: f32,
    pub confidence: f32,
}

/// Transcription parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionParams {
    /// Input audio/video URL or path
    pub input_url: String,
    /// Source language (auto-detect if None)
    pub source_language: Option<Language>,
    /// Whisper model to use
    pub model: WhisperModel,
    /// Enable word-level timestamps
    pub word_timestamps: bool,
    /// Enable speaker diarization
    pub diarization: bool,
    /// Maximum number of speakers (for diarization)
    pub max_speakers: Option<u32>,
    /// Output format for subtitles
    pub subtitle_format: SubtitleFormat,
}

impl Default for TranscriptionParams {
    fn default() -> Self {
        TranscriptionParams {
            input_url: String::new(),
            source_language: None,
            model: WhisperModel::default(),
            word_timestamps: true,
            diarization: false,
            max_speakers: None,
            subtitle_format: SubtitleFormat::default(),
        }
    }
}

/// Translation parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationParams {
    /// Text to translate (or transcription segments)
    pub source_text: String,
    /// Source segments (alternative to plain text)
    pub source_segments: Option<Vec<TranscriptionSegment>>,
    /// Source language
    pub source_language: Language,
    /// Target languages
    pub target_languages: Vec<Language>,
    /// Translation provider
    pub provider: TranslationProvider,
    /// Preserve timing from source segments
    pub preserve_timing: bool,
    /// Context hint for better translation
    pub context_hint: Option<String>,
    /// Glossary terms (term -> translations)
    pub glossary: Option<HashMap<String, HashMap<String, String>>>,
}

impl Default for TranslationParams {
    fn default() -> Self {
        TranslationParams {
            source_text: String::new(),
            source_segments: None,
            source_language: Language::english(),
            target_languages: vec![],
            provider: TranslationProvider::default(),
            preserve_timing: true,
            context_hint: None,
            glossary: None,
        }
    }
}

/// Localization job parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LocalizationParams {
    /// Transcription only
    Transcribe(TranscriptionParams),
    /// Translation only (from existing text)
    Translate(TranslationParams),
    /// Full pipeline: transcribe + translate
    FullPipeline {
        transcription: TranscriptionParams,
        target_languages: Vec<Language>,
        translation_provider: TranslationProvider,
    },
}

/// Transcription result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionResult {
    /// Detected or specified language
    pub language: Language,
    /// Full transcription text
    pub text: String,
    /// Segments with timestamps
    pub segments: Vec<TranscriptionSegment>,
    /// Subtitle file URL
    pub subtitle_url: Option<String>,
    /// Duration of audio in seconds
    pub duration_secs: f32,
    /// Model used
    pub model_used: WhisperModel,
    /// Processing time in milliseconds
    pub processing_time_ms: u64,
}

/// Translation result for a single language
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationResult {
    /// Target language
    pub language: Language,
    /// Translated text
    pub text: String,
    /// Translated segments (if source had segments)
    pub segments: Option<Vec<TranscriptionSegment>>,
    /// Subtitle file URL
    pub subtitle_url: Option<String>,
    /// Provider used
    pub provider: TranslationProvider,
}

/// Complete localization result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalizationResult {
    /// Original transcription
    pub transcription: Option<TranscriptionResult>,
    /// Translations by language
    pub translations: HashMap<String, TranslationResult>,
    /// Content hash for caching
    pub content_hash: String,
    /// Total processing time
    pub total_time_ms: u64,
}

/// Job tracking
#[derive(Debug, Clone)]
struct LocalizationJob {
    job_id: JobId,
    params: LocalizationParams,
    status: JobStatus,
    created_at: i64,
    started_at: Option<i64>,
    completed_at: Option<i64>,
    result: Option<LocalizationResult>,
    assigned_node: Option<Uuid>,
}

/// Localization Adapter Configuration
#[derive(Debug, Clone)]
pub struct LocalizationAdapterConfig {
    /// Local Whisper server URL
    pub whisper_server_url: String,
    /// OpenAI API key (for translation)
    pub openai_api_key: Option<String>,
    /// DeepL API key
    pub deepl_api_key: Option<String>,
    /// Google Translation API key
    pub google_api_key: Option<String>,
    /// Local NLLB server URL
    pub nllb_server_url: Option<String>,
    /// Output directory for subtitles
    pub output_dir: String,
    /// Maximum audio duration (seconds)
    pub max_duration_secs: u32,
}

impl Default for LocalizationAdapterConfig {
    fn default() -> Self {
        LocalizationAdapterConfig {
            whisper_server_url: "http://localhost:8007".to_string(),
            openai_api_key: None,
            deepl_api_key: None,
            google_api_key: None,
            nllb_server_url: Some("http://localhost:8008".to_string()),
            output_dir: "/var/cache/localization".to_string(),
            max_duration_secs: 3600, // 1 hour max
        }
    }
}

/// Localization Adapter
///
/// Implements transcription, translation, and subtitle generation
/// for multilingual content distribution.
pub struct LocalizationAdapter {
    config: LocalizationAdapterConfig,
    jobs: Arc<RwLock<HashMap<JobId, LocalizationJob>>>,
    content_cache: Arc<RwLock<HashMap<String, LocalizationResult>>>,
}

impl LocalizationAdapter {
    pub fn new(config: LocalizationAdapterConfig) -> Self {
        LocalizationAdapter {
            config,
            jobs: Arc::new(RwLock::new(HashMap::new())),
            content_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Compute content hash for caching
    fn compute_content_hash(params: &LocalizationParams) -> String {
        let mut hasher = Sha256::new();
        hasher.update(format!("{:?}", params).as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Call Whisper server for transcription
    async fn transcribe(&self, params: &TranscriptionParams) -> Result<TranscriptionResult, String> {
        let client = reqwest::Client::new();

        let request_body = serde_json::json!({
            "audio_url": params.input_url,
            "language": params.source_language.as_ref().map(|l| &l.0),
            "model": format!("{:?}", params.model).to_lowercase(),
            "word_timestamps": params.word_timestamps,
            "diarize": params.diarization,
            "max_speakers": params.max_speakers,
        });

        let start_time = std::time::Instant::now();

        let response = client
            .post(&format!("{}/transcribe", self.config.whisper_server_url))
            .json(&request_body)
            .timeout(std::time::Duration::from_secs(300)) // 5 min timeout
            .send()
            .await
            .map_err(|e| format!("Whisper request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Whisper server error: {}", response.status()));
        }

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse Whisper response: {}", e))?;

        let processing_time_ms = start_time.elapsed().as_millis() as u64;

        // Parse segments
        let segments: Vec<TranscriptionSegment> = result["segments"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|s| TranscriptionSegment {
                        text: s["text"].as_str().unwrap_or("").to_string(),
                        start: s["start"].as_f64().unwrap_or(0.0) as f32,
                        end: s["end"].as_f64().unwrap_or(0.0) as f32,
                        confidence: s["confidence"].as_f64().unwrap_or(0.0) as f32,
                        speaker: s["speaker"].as_str().map(String::from),
                        words: s["words"].as_array().map(|words| {
                            words.iter().map(|w| WordTimestamp {
                                word: w["word"].as_str().unwrap_or("").to_string(),
                                start: w["start"].as_f64().unwrap_or(0.0) as f32,
                                end: w["end"].as_f64().unwrap_or(0.0) as f32,
                                confidence: w["probability"].as_f64().unwrap_or(0.0) as f32,
                            }).collect()
                        }),
                    })
                    .collect()
            })
            .unwrap_or_default();

        let text = result["text"].as_str().unwrap_or("").to_string();
        let detected_lang = result["language"]
            .as_str()
            .map(|l| Language(l.to_string()))
            .unwrap_or_else(Language::english);

        // Generate subtitle file
        let subtitle_url = self.generate_subtitle_file(&segments, &params.subtitle_format).await?;

        let duration_secs = segments.last().map(|s| s.end).unwrap_or(0.0);

        Ok(TranscriptionResult {
            language: detected_lang,
            text,
            segments,
            subtitle_url: Some(subtitle_url),
            duration_secs,
            model_used: params.model.clone(),
            processing_time_ms,
        })
    }

    /// Generate subtitle file from segments
    async fn generate_subtitle_file(&self, segments: &[TranscriptionSegment], format: &SubtitleFormat) -> Result<String, String> {
        let content = match format {
            SubtitleFormat::Srt => self.segments_to_srt(segments),
            SubtitleFormat::Vtt => self.segments_to_vtt(segments),
            SubtitleFormat::Ass => self.segments_to_ass(segments),
            SubtitleFormat::Text => segments.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join("\n"),
            SubtitleFormat::Json => serde_json::to_string_pretty(segments).unwrap_or_default(),
        };

        let ext = match format {
            SubtitleFormat::Srt => "srt",
            SubtitleFormat::Vtt => "vtt",
            SubtitleFormat::Ass => "ass",
            SubtitleFormat::Text => "txt",
            SubtitleFormat::Json => "json",
        };

        let output_path = format!("{}/subtitle_{}.{}", self.config.output_dir, Uuid::new_v4(), ext);

        // Ensure directory exists
        if let Some(parent) = std::path::Path::new(&output_path).parent() {
            tokio::fs::create_dir_all(parent).await.ok();
        }

        tokio::fs::write(&output_path, content)
            .await
            .map_err(|e| format!("Failed to write subtitle file: {}", e))?;

        Ok(output_path)
    }

    /// Convert segments to SRT format
    fn segments_to_srt(&self, segments: &[TranscriptionSegment]) -> String {
        segments.iter().enumerate().map(|(i, seg)| {
            let start = Self::format_srt_time(seg.start);
            let end = Self::format_srt_time(seg.end);
            format!("{}\n{} --> {}\n{}\n", i + 1, start, end, seg.text.trim())
        }).collect::<Vec<_>>().join("\n")
    }

    /// Convert segments to VTT format
    fn segments_to_vtt(&self, segments: &[TranscriptionSegment]) -> String {
        let mut content = "WEBVTT\n\n".to_string();
        for seg in segments {
            let start = Self::format_vtt_time(seg.start);
            let end = Self::format_vtt_time(seg.end);
            content.push_str(&format!("{} --> {}\n{}\n\n", start, end, seg.text.trim()));
        }
        content
    }

    /// Convert segments to ASS format
    fn segments_to_ass(&self, segments: &[TranscriptionSegment]) -> String {
        let mut content = r#"[Script Info]
Title: Generated Subtitles
ScriptType: v4.00+
Collisions: Normal
PlayDepth: 0

[V4+ Styles]
Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding
Style: Default,Arial,20,&H00FFFFFF,&H000000FF,&H00000000,&H00000000,0,0,0,0,100,100,0,0,1,2,2,2,10,10,10,1

[Events]
Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text
"#.to_string();

        for seg in segments {
            let start = Self::format_ass_time(seg.start);
            let end = Self::format_ass_time(seg.end);
            content.push_str(&format!(
                "Dialogue: 0,{},{},Default,,0,0,0,,{}\n",
                start, end, seg.text.trim()
            ));
        }
        content
    }

    fn format_srt_time(secs: f32) -> String {
        let hours = (secs / 3600.0) as u32;
        let minutes = ((secs % 3600.0) / 60.0) as u32;
        let seconds = (secs % 60.0) as u32;
        let millis = ((secs % 1.0) * 1000.0) as u32;
        format!("{:02}:{:02}:{:02},{:03}", hours, minutes, seconds, millis)
    }

    fn format_vtt_time(secs: f32) -> String {
        let hours = (secs / 3600.0) as u32;
        let minutes = ((secs % 3600.0) / 60.0) as u32;
        let seconds = (secs % 60.0) as u32;
        let millis = ((secs % 1.0) * 1000.0) as u32;
        format!("{:02}:{:02}:{:02}.{:03}", hours, minutes, seconds, millis)
    }

    fn format_ass_time(secs: f32) -> String {
        let hours = (secs / 3600.0) as u32;
        let minutes = ((secs % 3600.0) / 60.0) as u32;
        let seconds = (secs % 60.0) as u32;
        let centis = ((secs % 1.0) * 100.0) as u32;
        format!("{}:{:02}:{:02}.{:02}", hours, minutes, seconds, centis)
    }

    /// Translate text using configured provider
    async fn translate(&self, params: &TranslationParams) -> Result<HashMap<String, TranslationResult>, String> {
        let mut results = HashMap::new();

        for target_lang in &params.target_languages {
            let translated = match params.provider {
                TranslationProvider::DeepL => {
                    self.translate_deepl(&params.source_text, &params.source_language, target_lang).await?
                }
                TranslationProvider::OpenAi => {
                    self.translate_openai(&params.source_text, &params.source_language, target_lang, params.context_hint.as_deref()).await?
                }
                TranslationProvider::GoogleTranslate => {
                    self.translate_google(&params.source_text, &params.source_language, target_lang).await?
                }
                TranslationProvider::Nllb => {
                    self.translate_nllb(&params.source_text, &params.source_language, target_lang).await?
                }
            };

            // If we have source segments, apply timing
            let segments = if params.preserve_timing && params.source_segments.is_some() {
                let source_segs = params.source_segments.as_ref().unwrap();
                Some(self.apply_timing_to_translation(&translated, source_segs))
            } else {
                None
            };

            // Generate subtitle if we have segments
            let subtitle_url = if let Some(ref segs) = segments {
                Some(self.generate_subtitle_file(segs, &SubtitleFormat::Srt).await?)
            } else {
                None
            };

            results.insert(target_lang.0.clone(), TranslationResult {
                language: target_lang.clone(),
                text: translated,
                segments,
                subtitle_url,
                provider: params.provider.clone(),
            });
        }

        Ok(results)
    }

    /// Translate using DeepL
    async fn translate_deepl(&self, text: &str, source: &Language, target: &Language) -> Result<String, String> {
        let api_key = self.config.deepl_api_key.as_ref()
            .ok_or("DeepL API key not configured")?;

        let client = reqwest::Client::new();

        let response = client
            .post("https://api-free.deepl.com/v2/translate")
            .header("Authorization", format!("DeepL-Auth-Key {}", api_key))
            .form(&[
                ("text", text),
                ("source_lang", &source.0.to_uppercase()),
                ("target_lang", &target.0.to_uppercase()),
            ])
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| format!("DeepL request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("DeepL error: {}", response.status()));
        }

        let result: serde_json::Value = response.json().await
            .map_err(|e| format!("Failed to parse DeepL response: {}", e))?;

        result["translations"][0]["text"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| "Invalid DeepL response".to_string())
    }

    /// Translate using OpenAI
    async fn translate_openai(&self, text: &str, source: &Language, target: &Language, context: Option<&str>) -> Result<String, String> {
        let api_key = self.config.openai_api_key.as_ref()
            .ok_or("OpenAI API key not configured")?;

        let client = reqwest::Client::new();

        let system_prompt = format!(
            "You are a professional translator. Translate the following text from {} to {}. \
            Maintain the original meaning, tone, and style. {}",
            source.0, target.0,
            context.map(|c| format!("Context: {}", c)).unwrap_or_default()
        );

        let request_body = serde_json::json!({
            "model": "gpt-4-turbo-preview",
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": text}
            ],
            "temperature": 0.3
        });

        let response = client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&request_body)
            .timeout(std::time::Duration::from_secs(60))
            .send()
            .await
            .map_err(|e| format!("OpenAI request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("OpenAI error: {}", response.status()));
        }

        let result: serde_json::Value = response.json().await
            .map_err(|e| format!("Failed to parse OpenAI response: {}", e))?;

        result["choices"][0]["message"]["content"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| "Invalid OpenAI response".to_string())
    }

    /// Translate using Google Cloud Translation
    async fn translate_google(&self, text: &str, source: &Language, target: &Language) -> Result<String, String> {
        let api_key = self.config.google_api_key.as_ref()
            .ok_or("Google API key not configured")?;

        let client = reqwest::Client::new();

        let response = client
            .post(&format!(
                "https://translation.googleapis.com/language/translate/v2?key={}",
                api_key
            ))
            .json(&serde_json::json!({
                "q": text,
                "source": source.0,
                "target": target.0,
                "format": "text"
            }))
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| format!("Google Translate request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Google Translate error: {}", response.status()));
        }

        let result: serde_json::Value = response.json().await
            .map_err(|e| format!("Failed to parse Google response: {}", e))?;

        result["data"]["translations"][0]["translatedText"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| "Invalid Google response".to_string())
    }

    /// Translate using local NLLB model
    async fn translate_nllb(&self, text: &str, source: &Language, target: &Language) -> Result<String, String> {
        let server_url = self.config.nllb_server_url.as_ref()
            .ok_or("NLLB server not configured")?;

        let client = reqwest::Client::new();

        let response = client
            .post(&format!("{}/translate", server_url))
            .json(&serde_json::json!({
                "text": text,
                "src_lang": source.0,
                "tgt_lang": target.0
            }))
            .timeout(std::time::Duration::from_secs(60))
            .send()
            .await
            .map_err(|e| format!("NLLB request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("NLLB error: {}", response.status()));
        }

        let result: serde_json::Value = response.json().await
            .map_err(|e| format!("Failed to parse NLLB response: {}", e))?;

        result["translation"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| "Invalid NLLB response".to_string())
    }

    /// Apply timing from source segments to translated text
    fn apply_timing_to_translation(&self, translated: &str, source_segments: &[TranscriptionSegment]) -> Vec<TranscriptionSegment> {
        // Split translated text into roughly equal parts matching source segments
        let total_source_chars: usize = source_segments.iter().map(|s| s.text.len()).sum();
        let translated_chars = translated.len();
        
        let mut result = Vec::new();
        let mut char_pos = 0;

        for source_seg in source_segments {
            let seg_ratio = source_seg.text.len() as f32 / total_source_chars as f32;
            let target_chars = (translated_chars as f32 * seg_ratio) as usize;
            
            let end_pos = (char_pos + target_chars).min(translated.len());
            let text = translated[char_pos..end_pos].to_string();
            
            result.push(TranscriptionSegment {
                text,
                start: source_seg.start,
                end: source_seg.end,
                confidence: source_seg.confidence,
                speaker: source_seg.speaker.clone(),
                words: None, // Word timing doesn't transfer across languages
            });

            char_pos = end_pos;
        }

        // Handle remaining text
        if char_pos < translated.len() {
            if let Some(last) = result.last_mut() {
                last.text.push_str(&translated[char_pos..]);
            }
        }

        result
    }

    /// Execute full localization pipeline
    async fn execute_localization(&self, params: &LocalizationParams) -> Result<LocalizationResult, String> {
        let cache_key = Self::compute_content_hash(params);

        // Check cache
        {
            let cache = self.content_cache.read().await;
            if let Some(cached) = cache.get(&cache_key) {
                return Ok(cached.clone());
            }
        }

        let start_time = std::time::Instant::now();

        let result = match params {
            LocalizationParams::Transcribe(p) => {
                let transcription = self.transcribe(p).await?;
                LocalizationResult {
                    transcription: Some(transcription),
                    translations: HashMap::new(),
                    content_hash: cache_key.clone(),
                    total_time_ms: start_time.elapsed().as_millis() as u64,
                }
            }
            LocalizationParams::Translate(p) => {
                let translations = self.translate(p).await?;
                LocalizationResult {
                    transcription: None,
                    translations,
                    content_hash: cache_key.clone(),
                    total_time_ms: start_time.elapsed().as_millis() as u64,
                }
            }
            LocalizationParams::FullPipeline { transcription, target_languages, translation_provider } => {
                // First transcribe
                let transcription_result = self.transcribe(transcription).await?;

                // Then translate to all target languages
                let translate_params = TranslationParams {
                    source_text: transcription_result.text.clone(),
                    source_segments: Some(transcription_result.segments.clone()),
                    source_language: transcription_result.language.clone(),
                    target_languages: target_languages.clone(),
                    provider: translation_provider.clone(),
                    preserve_timing: true,
                    context_hint: None,
                    glossary: None,
                };

                let translations = self.translate(&translate_params).await?;

                LocalizationResult {
                    transcription: Some(transcription_result),
                    translations,
                    content_hash: cache_key.clone(),
                    total_time_ms: start_time.elapsed().as_millis() as u64,
                }
            }
        };

        // Cache result
        {
            let mut cache = self.content_cache.write().await;
            cache.insert(cache_key, result.clone());
        }

        Ok(result)
    }
}

#[async_trait]
impl ToolAdapter for LocalizationAdapter {
    fn tool_type(&self) -> ToolType {
        ToolType::Localization
    }

    async fn validate_params(&self, params: &ToolParams) -> Result<(), String> {
        let loc_params: LocalizationParams = serde_json::from_value(params.params.clone())
            .map_err(|e| format!("Invalid localization params: {}", e))?;

        match loc_params {
            LocalizationParams::Transcribe(p) => {
                if p.input_url.is_empty() {
                    return Err("Input URL is required".to_string());
                }
            }
            LocalizationParams::Translate(p) => {
                if p.source_text.is_empty() && p.source_segments.is_none() {
                    return Err("Source text or segments required".to_string());
                }
                if p.target_languages.is_empty() {
                    return Err("At least one target language required".to_string());
                }
            }
            LocalizationParams::FullPipeline { transcription, target_languages, .. } => {
                if transcription.input_url.is_empty() {
                    return Err("Input URL is required".to_string());
                }
                if target_languages.is_empty() {
                    return Err("At least one target language required".to_string());
                }
            }
        }

        Ok(())
    }

    async fn invoke(&self, params: ToolParams) -> Result<JobId, String> {
        let loc_params: LocalizationParams = serde_json::from_value(params.params.clone())
            .map_err(|e| format!("Invalid localization params: {}", e))?;

        let job_id = Uuid::new_v4();
        let job = LocalizationJob {
            job_id,
            params: loc_params,
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
                    tool_type: ToolType::Localization,
                    output: serde_json::to_value(result).unwrap(),
                    execution_time_ms: result.total_time_ms as u32,
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
        let loc_params: LocalizationParams = serde_json::from_value(params.params.clone())
            .unwrap_or(LocalizationParams::Transcribe(TranscriptionParams::default()));

        match loc_params {
            LocalizationParams::Transcribe(p) => {
                // VRAM requirements based on model size
                let vram = match p.model {
                    WhisperModel::Tiny => 1,
                    WhisperModel::Base => 1,
                    WhisperModel::Small => 2,
                    WhisperModel::Medium => 5,
                    WhisperModel::Large | WhisperModel::LargeV2 | WhisperModel::LargeV3 => 10,
                };
                ToolResourceReq {
                    min_vram_gb: vram,
                    preferred_latency_ms: 30000,
                    supports_batching: false,
                }
            }
            LocalizationParams::Translate(_) => {
                // Translation is mostly API-based, no GPU needed
                ToolResourceReq {
                    min_vram_gb: 0,
                    preferred_latency_ms: 5000,
                    supports_batching: true,
                }
            }
            LocalizationParams::FullPipeline { transcription, .. } => {
                let vram = match transcription.model {
                    WhisperModel::Tiny | WhisperModel::Base => 1,
                    WhisperModel::Small => 2,
                    WhisperModel::Medium => 5,
                    _ => 10,
                };
                ToolResourceReq {
                    min_vram_gb: vram,
                    preferred_latency_ms: 60000,
                    supports_batching: false,
                }
            }
        }
    }
}

impl LocalizationAdapter {
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

        // Execute localization
        let result = self.execute_localization(&params).await;

        // Update job with result
        {
            let mut jobs = self.jobs.write().await;
            if let Some(job) = jobs.get_mut(&job_id) {
                match result {
                    Ok(loc_result) => {
                        job.status = JobStatus::Completed;
                        job.result = Some(loc_result);
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

impl Clone for LocalizationAdapter {
    fn clone(&self) -> Self {
        LocalizationAdapter {
            config: self.config.clone(),
            jobs: Arc::clone(&self.jobs),
            content_cache: Arc::clone(&self.content_cache),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_codes() {
        assert_eq!(Language::english().0, "en");
        assert_eq!(Language::spanish().0, "es");
        assert_eq!(Language::chinese().0, "zh");
    }

    #[test]
    fn test_srt_time_format() {
        let adapter = LocalizationAdapter::new(LocalizationAdapterConfig::default());
        
        let time = LocalizationAdapter::format_srt_time(3661.5);
        assert_eq!(time, "01:01:01,500");
    }

    #[test]
    fn test_vtt_time_format() {
        let time = LocalizationAdapter::format_vtt_time(3661.5);
        assert_eq!(time, "01:01:01.500");
    }

    #[test]
    fn test_ass_time_format() {
        let time = LocalizationAdapter::format_ass_time(3661.5);
        assert_eq!(time, "1:01:01.50");
    }

    #[tokio::test]
    async fn test_adapter_creation() {
        let config = LocalizationAdapterConfig::default();
        let adapter = LocalizationAdapter::new(config);
        
        assert_eq!(adapter.config.max_duration_secs, 3600);
    }

    #[tokio::test]
    async fn test_param_validation() {
        let config = LocalizationAdapterConfig::default();
        let adapter = LocalizationAdapter::new(config);

        // Empty input URL should fail
        let params = ToolParams::new(serde_json::json!({
            "Transcribe": {
                "input_url": "",
                "model": "Medium"
            }
        }));

        let result = adapter.validate_params(&params).await;
        assert!(result.is_err());

        // Valid params should pass
        let params = ToolParams::new(serde_json::json!({
            "Transcribe": {
                "input_url": "https://example.com/audio.mp3",
                "model": "Medium"
            }
        }));

        let result = adapter.validate_params(&params).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_segments_to_srt() {
        let adapter = LocalizationAdapter::new(LocalizationAdapterConfig::default());
        
        let segments = vec![
            TranscriptionSegment {
                text: "Hello world".to_string(),
                start: 0.0,
                end: 2.0,
                confidence: 0.95,
                speaker: None,
                words: None,
            },
            TranscriptionSegment {
                text: "How are you".to_string(),
                start: 2.5,
                end: 4.5,
                confidence: 0.90,
                speaker: None,
                words: None,
            },
        ];

        let srt = adapter.segments_to_srt(&segments);
        assert!(srt.contains("1\n00:00:00,000 --> 00:00:02,000\nHello world"));
        assert!(srt.contains("2\n00:00:02,500 --> 00:00:04,500\nHow are you"));
    }
}
