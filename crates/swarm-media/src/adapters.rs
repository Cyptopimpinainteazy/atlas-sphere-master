//! Example implementations of tool adapters
//! 
//! This module shows how to implement specific tool adapters using the
//! ToolAdapter trait. Copy these patterns for your own tools.

use crate::tool_adapter::*;
use async_trait::async_trait;
use uuid::Uuid;

// ============================================================================
// EXAMPLE 1: Mock Adapter (for testing)
// ============================================================================
//
// A mock adapter that succeeds immediately (useful for unit tests)

pub struct MockAdapter {
    tool_type: ToolType,
}

impl MockAdapter {
    pub fn new(tool_type: ToolType) -> Self {
        Self { tool_type }
    }
}

#[async_trait]
impl ToolAdapter for MockAdapter {
    fn tool_type(&self) -> ToolType {
        self.tool_type.clone()
    }

    async fn validate_params(&self, _params: &ToolParams) -> Result<(), String> {
        // Mocks always accept params
        Ok(())
    }

    async fn invoke(&self, _params: ToolParams) -> Result<JobId, String> {
        // Just return a random job ID
        Ok(Uuid::new_v4())
    }

    async fn get_status(&self, _job_id: JobId) -> Result<JobStatus, String> {
        // Mocks are always done
        Ok(JobStatus::Completed)
    }

    async fn get_result(&self, job_id: JobId) -> Result<ToolResult, String> {
        Ok(ToolResult {
            job_id,
            tool_type: self.tool_type.clone(),
            output: serde_json::json!({"status": "mock_success"}),
            execution_time_ms: 100,
            content_hash: Some("mock_hash".to_string()),
            executed_by_node: Uuid::new_v4(),
        })
    }

    async fn cancel_job(&self, _job_id: JobId) -> Result<(), String> {
        Ok(())
    }
}

// ============================================================================
// EXAMPLE 2: Template for a Real Tool Adapter
// ============================================================================
//
// This is the pattern for implementing a real tool like LLM, image gen, etc.

pub struct MyToolAdapter {
    // Whatever connection/config your tool needs
    api_endpoint: String,
    api_key: String,
}

impl MyToolAdapter {
    pub fn new(api_endpoint: String, api_key: String) -> Self {
        Self {
            api_endpoint,
            api_key,
        }
    }
}

#[async_trait]
impl ToolAdapter for MyToolAdapter {
    fn tool_type(&self) -> ToolType {
        // Return the tool type this adapter handles
        ToolType::Custom("my_tool".to_string())
    }

    async fn validate_params(&self, params: &ToolParams) -> Result<(), String> {
        // 1. Check required fields exist
        if params.get("prompt").is_none() {
            return Err("Missing required field: prompt".to_string());
        }

        // 2. Validate field values (e.g., max length, valid options)
        if let Some(prompt) = params.get("prompt") {
            if let Some(text) = prompt.as_str() {
                if text.len() > 10000 {
                    return Err("prompt exceeds max length of 10000".to_string());
                }
            }
        }

        // 3. Return Ok if all validation passes
        Ok(())
    }

    async fn invoke(&self, params: ToolParams) -> Result<JobId, String> {
        // 1. Validate first
        self.validate_params(&params).await?;

        // 2. Call your actual tool
        let prompt = params
            .get("prompt")
            .and_then(|v| v.as_str())
            .unwrap_or("default");

        // 3. Call API (example)
        let response = reqwest::Client::new()
            .post(&self.api_endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&serde_json::json!({
                "prompt": prompt,
                "params": params.params
            }))
            .send()
            .await
            .map_err(|e| format!("API call failed: {}", e))?;

        // 4. Parse response and extract job ID
        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        if let Some(job_id_str) = data.get("job_id").and_then(|v| v.as_str()) {
            Ok(Uuid::parse_str(job_id_str)
                .map_err(|e| format!("Invalid job ID format: {}", e))?)
        } else {
            Err("No job_id in response".to_string())
        }
    }

    async fn get_status(&self, job_id: JobId) -> Result<JobStatus, String> {
        // 1. Query your tool's API for job status
        let response = reqwest::Client::new()
            .get(format!("{}/status/{}", self.api_endpoint, job_id))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| format!("Status query failed: {}", e))?;

        // 2. Parse response
        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        // 3. Map tool-specific status to our JobStatus enum
        match data.get("status").and_then(|v| v.as_str()) {
            Some("running") => Ok(JobStatus::Running),
            Some("completed") => Ok(JobStatus::Completed),
            Some("failed") => {
                let reason = data
                    .get("error")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown error");
                Ok(JobStatus::Failed(reason.to_string()))
            }
            Some("queued") => Ok(JobStatus::Queued),
            _ => Err("Unknown status from tool".to_string()),
        }
    }

    async fn get_result(&self, job_id: JobId) -> Result<ToolResult, String> {
        // 1. Query your tool for results
        let response = reqwest::Client::new()
            .get(format!("{}/result/{}", self.api_endpoint, job_id))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| format!("Result query failed: {}", e))?;

        // 2. Parse response
        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        // 3. Extract result details
        let execution_time_ms = data
            .get("execution_time_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;

        let output = data
            .get("output")
            .cloned()
            .unwrap_or(serde_json::Value::Null);

        // 4. Compute content hash for reproducibility (simple hash for now)
        let output_str = output.to_string();
        let hash_value = format!("{:x}", output_str.len()); // Simple deterministic hash
        let content_hash = Some(format!("sha256::{}", hash_value));

        // 5. Return ToolResult
        Ok(ToolResult {
            job_id,
            tool_type: self.tool_type(),
            output,
            execution_time_ms,
            content_hash,
            executed_by_node: Uuid::new_v4(), // Should be set by dispatcher
        })
    }

    async fn cancel_job(&self, job_id: JobId) -> Result<(), String> {
        // 1. Call tool's cancel endpoint
        reqwest::Client::new()
            .post(format!("{}/cancel/{}", self.api_endpoint, job_id))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|e| format!("Cancel request failed: {}", e))?;

        Ok(())
    }

    fn resource_requirements(&self, params: &ToolParams) -> ToolResourceReq {
        // Based on params, determine resource needs
        // Example: larger prompts need more VRAM
        let prompt_size = params
            .get("prompt")
            .and_then(|v| v.as_str())
            .map(|s| s.len())
            .unwrap_or(0);

        let min_vram_gb = if prompt_size > 5000 { 16 } else { 8 };

        ToolResourceReq {
            min_vram_gb,
            preferred_latency_ms: 2000,
            supports_batching: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_adapter() {
        let adapter = MockAdapter::new(ToolType::TextGeneration);
        assert_eq!(adapter.tool_type(), ToolType::TextGeneration);
    }
}
