//! Chain submitter for receipt submission

use crate::receipt::ExecutionReceipt;
use serde::{Deserialize, Serialize};

/// Chain submitter
pub struct ChainSubmitter {
    /// RPC URL
    rpc_url: String,
    /// Executor key (hex)
    _executor_key: String,
    /// HTTP client
    client: reqwest::Client,
}

/// RPC request
#[derive(Serialize)]
struct RpcRequest {
    jsonrpc: &'static str,
    id: u32,
    method: String,
    params: Vec<serde_json::Value>,
}

/// RPC response
#[derive(Deserialize)]
struct RpcResponse {
    #[allow(dead_code)]
    jsonrpc: String,
    #[allow(dead_code)]
    id: u32,
    result: Option<serde_json::Value>,
    error: Option<RpcError>,
}

/// RPC error
#[derive(Deserialize)]
struct RpcError {
    code: i32,
    message: String,
}

impl ChainSubmitter {
    /// Create a new chain submitter
    pub fn new(rpc_url: String, executor_key: String) -> Self {
        Self {
            rpc_url,
            _executor_key: executor_key,
            client: reqwest::Client::new(),
        }
    }

    /// Submit a receipt to the chain
    pub async fn submit_receipt(&self, receipt: &ExecutionReceipt) -> anyhow::Result<String> {
        tracing::info!("Submitting receipt for job {}", hex::encode(receipt.job_id));

        // Encode receipt
        let receipt_bytes = receipt.encode();
        let receipt_hex = format!("0x{}", hex::encode(&receipt_bytes));

        // Create RPC call
        let request = RpcRequest {
            jsonrpc: "2.0",
            id: 1,
            method: "x3Verifier_submitReceipt".to_string(),
            params: vec![serde_json::json!(receipt_hex)],
        };

        // Send request
        let response: RpcResponse = self
            .client
            .post(&self.rpc_url)
            .json(&request)
            .send()
            .await?
            .json()
            .await?;

        // Check for errors
        if let Some(error) = response.error {
            return Err(anyhow::anyhow!(
                "RPC error {}: {}",
                error.code,
                error.message
            ));
        }

        // Extract transaction hash
        if let Some(result) = response.result {
            if let Some(tx_hash) = result.as_str() {
                return Ok(tx_hash.to_string());
            }
        }

        Err(anyhow::anyhow!("No transaction hash in response"))
    }

    /// Get job status from chain
    pub async fn get_job_status(&self, job_id: [u8; 32]) -> anyhow::Result<String> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            id: 1,
            method: "x3Verifier_getJobStatus".to_string(),
            params: vec![serde_json::json!(format!("0x{}", hex::encode(job_id)))],
        };

        let response: RpcResponse = self
            .client
            .post(&self.rpc_url)
            .json(&request)
            .send()
            .await?
            .json()
            .await?;

        if let Some(error) = response.error {
            return Err(anyhow::anyhow!(
                "RPC error {}: {}",
                error.code,
                error.message
            ));
        }

        if let Some(result) = response.result {
            return Ok(result.to_string());
        }

        Ok("unknown".to_string())
    }

    /// Check if executor is registered on chain
    pub async fn is_registered(&self, executor_pubkey: [u8; 32]) -> anyhow::Result<bool> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            id: 1,
            method: "x3Verifier_isExecutorRegistered".to_string(),
            params: vec![serde_json::json!(format!("0x{}", hex::encode(executor_pubkey)))],
        };

        let response: RpcResponse = self
            .client
            .post(&self.rpc_url)
            .json(&request)
            .send()
            .await?
            .json()
            .await?;

        if let Some(result) = response.result {
            return Ok(result.as_bool().unwrap_or(false));
        }

        Ok(false)
    }

    /// Get pending jobs from chain
    pub async fn get_pending_jobs(&self, limit: u32) -> anyhow::Result<Vec<PendingJob>> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            id: 1,
            method: "x3Verifier_getPendingJobs".to_string(),
            params: vec![serde_json::json!(limit)],
        };

        let response: RpcResponse = self
            .client
            .post(&self.rpc_url)
            .json(&request)
            .send()
            .await?
            .json()
            .await?;

        if let Some(error) = response.error {
            return Err(anyhow::anyhow!(
                "RPC error {}: {}",
                error.code,
                error.message
            ));
        }

        if let Some(result) = response.result {
            let jobs: Vec<PendingJob> = serde_json::from_value(result)?;
            return Ok(jobs);
        }

        Ok(vec![])
    }
}

/// Pending job from chain
#[derive(Debug, Clone, Deserialize)]
pub struct PendingJob {
    pub job_id: String,
    pub bytecode_hash: String,
    pub input_hash: String,
    pub gas_limit: String,
    pub reward: String,
    pub submitter: String,
}
