/// LLM Adapter - Local LLaMA.cpp server with fine-tuned models
///
/// Supports:
/// - Llama-2-70B, Mistral-7B, DeepSeek-67B (local)
/// - GPT-3.5-turbo, Claude-3-opus (cloud fallback)
/// - Prompt templates for founder voice, tech writing, investor tone
/// - Token counting and cost tracking

use async_trait::async_trait;
use chrono::Utc;
use serde_json::json;
use std::collections::HashMap;
use uuid::Uuid;
use crate::tool_adapter::{
    JobId, JobStatus, ToolAdapter, ToolParams, ToolResult, ToolType, ToolResourceReq,
};

/// LLM Model configurations
#[derive(Clone, Debug)]
pub struct LlmModel {
    pub id: String,
    pub name: String,
    pub min_vram_gb: u32,
    pub tokens_per_second: f32,
    pub context_window: usize,
    pub is_local: bool,
    pub cost_per_1k_tokens: f32, // USD
}

impl LlmModel {
    pub fn llama2_70b() -> Self {
        Self {
            id: "llama2-70b".to_string(),
            name: "Llama 2 70B".to_string(),
            min_vram_gb: 40,
            tokens_per_second: 5.0,
            context_window: 4096,
            is_local: true,
            cost_per_1k_tokens: 0.0,
        }
    }

    pub fn mistral_7b() -> Self {
        Self {
            id: "mistral-7b".to_string(),
            name: "Mistral 7B".to_string(),
            min_vram_gb: 8,
            tokens_per_second: 15.0,
            context_window: 8192,
            is_local: true,
            cost_per_1k_tokens: 0.0,
        }
    }

    pub fn deepseek_67b() -> Self {
        Self {
            id: "deepseek-67b".to_string(),
            name: "DeepSeek 67B".to_string(),
            min_vram_gb: 35,
            tokens_per_second: 6.0,
            context_window: 4096,
            is_local: true,
            cost_per_1k_tokens: 0.0,
        }
    }

    pub fn gpt35_turbo() -> Self {
        Self {
            id: "gpt-3.5-turbo".to_string(),
            name: "GPT-3.5 Turbo (OpenAI)".to_string(),
            min_vram_gb: 0,
            tokens_per_second: 100.0,
            context_window: 4096,
            is_local: false,
            cost_per_1k_tokens: 0.002,
        }
    }

    pub fn claude_opus() -> Self {
        Self {
            id: "claude-opus".to_string(),
            name: "Claude 3 Opus (Anthropic)".to_string(),
            min_vram_gb: 0,
            tokens_per_second: 100.0,
            context_window: 200000,
            is_local: false,
            cost_per_1k_tokens: 0.015,
        }
    }
}

/// Prompt templates for consistent voice/style
#[derive(Clone, Debug)]
pub struct PromptTemplate {
    pub name: String,
    pub system_message: String,
    pub prefix: String,
    pub suffix: String,
}

impl PromptTemplate {
    pub fn founder_voice() -> Self {
        Self {
            name: "founder_voice".to_string(),
            system_message: "You are a visionary tech founder with deep domain expertise. \
                Your writing is direct, passionate, and backed by data. You challenge assumptions \
                and think strategically about the future.".to_string(),
            prefix: "".to_string(),
            suffix: "\n\nKeep it concise and actionable. Focus on impact.".to_string(),
        }
    }

    pub fn tech_writing() -> Self {
        Self {
            name: "tech_writing".to_string(),
            system_message: "You are a technical writer with the ability to explain complex \
                concepts to both engineers and non-technical stakeholders. Your writing is clear, \
                well-organized, and includes practical examples.".to_string(),
            prefix: "".to_string(),
            suffix: "\n\nInclude code examples where relevant. Use bullet points for clarity.".to_string(),
        }
    }

    pub fn investor_tone() -> Self {
        Self {
            name: "investor_tone".to_string(),
            system_message: "You are a seasoned venture capitalist analyzing investment opportunities. \
                You think about market size, competitive advantage, team quality, and path to profitability. \
                Your communication is analytical, data-driven, and professional.".to_string(),
            prefix: "".to_string(),
            suffix: "\n\nQuantify impact wherever possible. Highlight competitive moats.".to_string(),
        }
    }
}

/// Generation parameters
#[derive(Clone, Debug)]
pub struct GenerationParams {
    pub model: String,
    pub temperature: f32,
    pub max_tokens: usize,
    pub top_p: f32,
    pub presence_penalty: f32,
    pub style: String,
}

impl GenerationParams {
    pub fn from_tool_params(params: &ToolParams) -> Result<Self, String> {
        let temperature = params
            .get("temperature")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.7) as f32;

        if temperature < 0.0 || temperature > 2.0 {
            return Err("temperature must be between 0 and 2".to_string());
        }

        let max_tokens = params
            .get("max_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(500) as usize;

        if max_tokens > 4096 {
            return Err("max_tokens cannot exceed 4096".to_string());
        }

        Ok(GenerationParams {
            model: params
                .get("model")
                .and_then(|v| v.as_str())
                .unwrap_or("mistral-7b")
                .to_string(),
            temperature,
            max_tokens,
            top_p: params
                .get("top_p")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.9) as f32,
            presence_penalty: params
                .get("presence_penalty")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.1) as f32,
            style: params
                .get("style")
                .and_then(|v| v.as_str())
                .unwrap_or("founder_voice")
                .to_string(),
        })
    }
}

/// LLM job state tracking
#[derive(Clone, Debug)]
struct LlmJob {
    job_id: JobId,
    status: JobStatus,
    prompt: String,
    model: String,
    result: Option<String>,
    tokens_used: Option<u32>,
    error: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
}

/// LLM Adapter implementation
pub struct LlmAdapter {
    local_server_url: String,
    openai_api_key: Option<String>,
    anthropic_api_key: Option<String>,
    jobs: HashMap<JobId, LlmJob>,
    models: HashMap<String, LlmModel>,
}

impl LlmAdapter {
    pub fn new(
        local_server_url: String,
        openai_api_key: Option<String>,
        anthropic_api_key: Option<String>,
    ) -> Self {
        let mut models = HashMap::new();
        models.insert("llama2-70b".to_string(), LlmModel::llama2_70b());
        models.insert("mistral-7b".to_string(), LlmModel::mistral_7b());
        models.insert("deepseek-67b".to_string(), LlmModel::deepseek_67b());
        models.insert("gpt-3.5-turbo".to_string(), LlmModel::gpt35_turbo());
        models.insert("claude-opus".to_string(), LlmModel::claude_opus());

        Self {
            local_server_url,
            openai_api_key,
            anthropic_api_key,
            jobs: HashMap::new(),
            models,
        }
    }

    fn apply_template(prompt: &str, template: &PromptTemplate) -> String {
        format!(
            "{}{}{}",
            template.prefix,
            prompt,
            template.suffix
        )
    }

    fn get_template(style: &str) -> PromptTemplate {
        match style {
            "founder_voice" => PromptTemplate::founder_voice(),
            "tech_writing" => PromptTemplate::tech_writing(),
            "investor_tone" => PromptTemplate::investor_tone(),
            _ => PromptTemplate::founder_voice(),
        }
    }

    async fn call_local_server(
        &self,
        prompt: &str,
        params: &GenerationParams,
    ) -> Result<(String, u32), String> {
        use reqwest::Client;
        
        let client = Client::new();
        
        let request = json!({
            "prompt": prompt,
            "n_predict": params.max_tokens,
            "temperature": params.temperature,
            "top_p": params.top_p,
            "stream": false
        });

        let response = client
            .post(&format!("{}/completion", self.local_server_url))
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Failed to connect to local server: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Local server error: {}", response.status()));
        }

        let response_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        let content = response_json
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or("Invalid response format from local server")?;

        // Estimate tokens (llama.cpp usually returns token count)
        let tokens_used = response_json
            .get("tokens_predicted")
            .and_then(|v| v.as_u64())
            .unwrap_or((content.len() / 4) as u64) as u32;

        Ok((content.to_string(), tokens_used))
    }

    async fn call_openai(
        &self,
        prompt: &str,
        params: &GenerationParams,
    ) -> Result<(String, u32), String> {
        use reqwest::Client;
        
        let api_key = self.openai_api_key.as_ref()
            .ok_or("OpenAI API key not configured")?;

        let client = Client::new();
        
        let request = json!({
            "model": "gpt-3.5-turbo",
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "temperature": params.temperature,
            "max_tokens": params.max_tokens,
            "top_p": params.top_p
        });

        let response = client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("OpenAI API request failed: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("OpenAI API error: {} - {}", status, error_text));
        }

        let response_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse OpenAI response: {}", e))?;

        let content = response_json
            .get("choices")
            .and_then(|choices| choices.as_array())
            .and_then(|choices| choices.first())
            .and_then(|choice| choice.get("message"))
            .and_then(|message| message.get("content"))
            .and_then(|content| content.as_str())
            .ok_or("Invalid response format from OpenAI")?;

        // Get token usage
        let tokens_used = response_json
            .get("usage")
            .and_then(|usage| usage.get("total_tokens"))
            .and_then(|tokens| tokens.as_u64())
            .unwrap_or((content.len() / 4) as u64);
        let tokens_used = u32::try_from(tokens_used).unwrap_or(u32::MAX);

        Ok((content.to_string(), tokens_used))
    }

    async fn call_anthropic(
        &self,
        prompt: &str,
        params: &GenerationParams,
    ) -> Result<(String, u32), String> {
        use reqwest::Client;
        
        let api_key = self.anthropic_api_key.as_ref()
            .ok_or("Anthropic API key not configured")?;

        let client = Client::new();
        
        let request = json!({
            "model": "claude-3-opus-20240229",
            "max_tokens": params.max_tokens,
            "temperature": params.temperature,
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ]
        });

        let response = client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Anthropic API request failed: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(format!("Anthropic API error: {} - {}", status, error_text));
        }

        let response_json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse Anthropic response: {}", e))?;

        let content = response_json
            .get("content")
            .and_then(|content| content.as_array())
            .and_then(|content| content.first())
            .and_then(|item| item.get("text"))
            .and_then(|text| text.as_str())
            .ok_or("Invalid response format from Anthropic")?;

        // Get token usage
        let input_tokens = response_json
            .get("usage")
            .and_then(|usage| usage.get("input_tokens"))
            .and_then(|tokens| tokens.as_u64())
            .unwrap_or(0);
        let output_tokens = response_json
            .get("usage")
            .and_then(|usage| usage.get("output_tokens"))
            .and_then(|tokens| tokens.as_u64())
            .unwrap_or(0);
        let tokens_used = input_tokens.saturating_add(output_tokens);
        let tokens_used = u32::try_from(tokens_used).unwrap_or(u32::MAX);

        Ok((content.to_string(), tokens_used))
    }
}

#[async_trait]
impl ToolAdapter for LlmAdapter {
    fn tool_type(&self) -> ToolType {
        ToolType::TextGeneration
    }

    async fn validate_params(&self, params: &ToolParams) -> Result<(), String> {
        // Validate prompt exists
        let _prompt = params
            .get("prompt")
            .and_then(|v| v.as_str())
            .ok_or("prompt parameter required")?;

        // Validate model
        let model = params
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or("mistral-7b");

        if !self.models.contains_key(model) {
            return Err(format!("Unknown model: {}", model));
        }

        // Validate generation parameters
        GenerationParams::from_tool_params(params)?;

        Ok(())
    }

    async fn invoke(&self, params: ToolParams) -> Result<JobId, String> {
        let job_id = Uuid::new_v4();
        let prompt = params
            .get("prompt")
            .and_then(|v| v.as_str())
            .ok_or("prompt required")?
            .to_string();

        let gen_params = GenerationParams::from_tool_params(&params)?;

        // Apply template
        let template = Self::get_template(&gen_params.style);
        let templated_prompt = Self::apply_template(&prompt, &template);

        // Get model info
        let model = self.models.get(&gen_params.model).cloned()
            .ok_or(format!("Model not found: {}", gen_params.model))?;

        // Call appropriate backend (local or cloud with fallback)
        let (result, tokens_used) = if model.is_local {
            // Try local first, fallback to cloud on failure
            match self.call_local_server(&templated_prompt, &gen_params).await {
                Ok(result) => result,
                Err(_) => self.call_openai(&templated_prompt, &gen_params).await?,
            }
        } else {
            match gen_params.model.as_str() {
                "gpt-3.5-turbo" => self.call_openai(&templated_prompt, &gen_params).await?,
                "claude-opus" => self.call_anthropic(&templated_prompt, &gen_params).await?,
                _ => return Err("Unknown model".to_string()),
            }
        };

        let job = LlmJob {
            job_id,
            status: JobStatus::Completed,
            prompt,
            model: gen_params.model,
            result: Some(result),
            tokens_used: Some(tokens_used),
            error: None,
            created_at: Utc::now(),
        };

        // Note: In production, use Arc<RwLock<_>> for thread-safe mutation
        // For now, this is a simplified implementation
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

        let result_text = job
            .result
            .as_ref()
            .ok_or("Result not available")?
            .clone();

        Ok(ToolResult {
            job_id,
            tool_type: ToolType::TextGeneration,
            output: json!({
                "text": result_text.clone(),
                "tokens_used": job.tokens_used.unwrap_or(0),
                "model": &job.model,
                "generated_at": job.created_at.to_rfc3339(),
            }),
            execution_time_ms: 5000, // Placeholder
            content_hash: Some(format!("{:x}", sha2::Sha256::digest(result_text.as_bytes()))),
            executed_by_node: Uuid::new_v4(),
        })
    }

    async fn cancel_job(&self, _job_id: JobId) -> Result<(), String> {
        // In production, cancel the generation request
        Ok(())
    }

    fn resource_requirements(&self, params: &ToolParams) -> ToolResourceReq {
        if let Ok(gen_params) = GenerationParams::from_tool_params(params) {
            if let Some(model) = self.models.get(&gen_params.model) {
                return ToolResourceReq {
                    min_vram_gb: model.min_vram_gb,
                    preferred_latency_ms: if model.is_local { 2000 } else { 1000 },
                    supports_batching: true,
                };
            }
        }

        ToolResourceReq {
            min_vram_gb: 8,
            preferred_latency_ms: 2000,
            supports_batching: true,
        }
    }
}

use sha2::Digest;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_llm_adapter_creation() {
        let adapter = LlmAdapter::new(
            "http://localhost:8000".to_string(),
            None,
            None,
        );

        assert_eq!(adapter.tool_type(), ToolType::TextGeneration);
        assert!(adapter.models.contains_key("mistral-7b"));
    }

    #[tokio::test]
    async fn test_prompt_template_application() {
        let template = PromptTemplate::founder_voice();
        let prompt = "What is the future of AI?";
        let result = LlmAdapter::apply_template(prompt, &template);
        
        assert!(result.contains(prompt));
        assert!(result.contains("actionable"));
    }

    #[tokio::test]
    async fn test_generation_params_validation() {
        let params = ToolParams::new(json!({
            "prompt": "Test prompt",
            "model": "mistral-7b",
            "temperature": 0.7,
            "max_tokens": 500,
        }));

        let gen_params = GenerationParams::from_tool_params(&params);
        assert!(gen_params.is_ok());
    }

    #[tokio::test]
    async fn test_invalid_temperature() {
        let params = ToolParams::new(json!({
            "prompt": "Test",
            "temperature": 3.0,  // Invalid: > 2.0
        }));

        let gen_params = GenerationParams::from_tool_params(&params);
        assert!(gen_params.is_err());
    }
}