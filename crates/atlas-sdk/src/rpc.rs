//! RPC client for Atlas Sphere blockchain.
//!
//! Provides HTTP and WebSocket JSON-RPC clients for interacting
//! with Atlas Sphere nodes.

use crate::error::{AtlasError, Result};
use crate::types::{AccountInfo, BlockHeader, ComitResult};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

// ============================================================================
// JSON-RPC Types
// ============================================================================

/// JSON-RPC request.
#[derive(Debug, Serialize)]
pub struct RpcRequest<T: Serialize> {
    pub jsonrpc: &'static str,
    pub id: u64,
    pub method: String,
    pub params: T,
}

impl<T: Serialize> RpcRequest<T> {
    pub fn new(id: u64, method: impl Into<String>, params: T) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            method: method.into(),
            params,
        }
    }
}

/// JSON-RPC response.
#[derive(Debug, Deserialize)]
pub struct RpcResponse<T> {
    pub jsonrpc: String,
    pub id: u64,
    pub result: Option<T>,
    pub error: Option<RpcError>,
}

/// JSON-RPC error.
#[derive(Debug, Deserialize)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
}

impl std::fmt::Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RPC error {}: {}", self.code, self.message)
    }
}

// ============================================================================
// HTTP RPC Client
// ============================================================================

/// HTTP JSON-RPC client.
#[derive(Clone)]
pub struct HttpRpcClient {
    endpoint: String,
    client: reqwest::Client,
    request_id: std::sync::Arc<AtomicU64>,
}

impl HttpRpcClient {
    /// Create a new HTTP RPC client.
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            client: reqwest::Client::new(),
            request_id: std::sync::Arc::new(AtomicU64::new(1)),
        }
    }

    /// Create with custom reqwest client.
    pub fn with_client(endpoint: impl Into<String>, client: reqwest::Client) -> Self {
        Self {
            endpoint: endpoint.into(),
            client,
            request_id: std::sync::Arc::new(AtomicU64::new(1)),
        }
    }

    /// Get the next request ID.
    fn next_id(&self) -> u64 {
        self.request_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Send a JSON-RPC request.
    pub async fn request<P, R>(&self, method: &str, params: P) -> Result<R>
    where
        P: Serialize,
        R: DeserializeOwned,
    {
        let request = RpcRequest::new(self.next_id(), method, params);

        let response = self
            .client
            .post(&self.endpoint)
            .json(&request)
            .send()
            .await
            .map_err(|e| AtlasError::Rpc(format!("HTTP request failed: {}", e)))?;

        let rpc_response: RpcResponse<R> = response
            .json()
            .await
            .map_err(|e| AtlasError::Rpc(format!("Failed to parse response: {}", e)))?;

        if let Some(error) = rpc_response.error {
            return Err(AtlasError::Rpc(error.to_string()));
        }

        rpc_response
            .result
            .ok_or_else(|| AtlasError::Rpc("No result in response".to_string()))
    }

    // =========================================================================
    // Standard RPC Methods
    // =========================================================================

    /// Get chain ID.
    pub async fn chain_id(&self) -> Result<String> {
        self.request("system_chain", ()).await
    }

    /// Get node name.
    pub async fn node_name(&self) -> Result<String> {
        self.request("system_name", ()).await
    }

    /// Get node version.
    pub async fn node_version(&self) -> Result<String> {
        self.request("system_version", ()).await
    }

    /// Get health status.
    pub async fn health(&self) -> Result<serde_json::Value> {
        self.request("system_health", ()).await
    }

    /// Get latest block hash.
    pub async fn chain_get_head(&self) -> Result<String> {
        self.request("chain_getHead", ()).await
    }

    /// Get block hash by number.
    pub async fn chain_get_block_hash(&self, block_number: Option<u64>) -> Result<String> {
        self.request("chain_getBlockHash", (block_number,)).await
    }

    /// Get block header by hash.
    pub async fn chain_get_header(&self, hash: Option<&str>) -> Result<BlockHeader> {
        self.request("chain_getHeader", (hash,)).await
    }

    /// Get finalized head.
    pub async fn chain_get_finalized_head(&self) -> Result<String> {
        self.request("chain_getFinalizedHead", ()).await
    }

    // =========================================================================
    // Atlas Kernel RPC Methods
    // =========================================================================

    /// Get canonical balance for an account.
    pub async fn get_canonical_balance(&self, account: &str, asset_id: u32) -> Result<String> {
        self.request("atlasKernel_getCanonicalBalance", (account, asset_id))
            .await
    }

    /// Check if an account is authorized.
    pub async fn is_authorized(&self, account: &str) -> Result<bool> {
        self.request("atlasKernel_isAuthorized", (account,)).await
    }

    /// Get asset metadata.
    pub async fn get_asset_metadata(&self, asset_id: u32) -> Result<crate::types::AssetMetadata> {
        self.request("atlasKernel_getAssetMetadata", (asset_id,))
            .await
    }

    /// Get account info.
    pub async fn get_account_info(&self, account: &str) -> Result<AccountInfo> {
        self.request("atlasKernel_getAccountInfo", (account,)).await
    }

    // =========================================================================
    // EVM RPC Methods (Frontier compatibility)
    // =========================================================================

    /// Get EVM chain ID.
    pub async fn eth_chain_id(&self) -> Result<String> {
        self.request("eth_chainId", ()).await
    }

    /// Get EVM account balance.
    pub async fn eth_get_balance(&self, address: &str, block: &str) -> Result<String> {
        self.request("eth_getBalance", (address, block)).await
    }

    /// Get EVM account nonce.
    pub async fn eth_get_transaction_count(&self, address: &str, block: &str) -> Result<String> {
        self.request("eth_getTransactionCount", (address, block))
            .await
    }

    /// Send raw EVM transaction.
    pub async fn eth_send_raw_transaction(&self, data: &str) -> Result<String> {
        self.request("eth_sendRawTransaction", (data,)).await
    }

    /// Call EVM contract (read-only).
    pub async fn eth_call(&self, call: &crate::evm::EvmCallRequest, block: &str) -> Result<String> {
        self.request("eth_call", (call, block)).await
    }

    /// Estimate EVM gas.
    pub async fn eth_estimate_gas(&self, call: &crate::evm::EvmCallRequest) -> Result<String> {
        self.request("eth_estimateGas", (call,)).await
    }

    /// Get EVM transaction receipt.
    pub async fn eth_get_transaction_receipt(
        &self,
        tx_hash: &str,
    ) -> Result<Option<serde_json::Value>> {
        self.request("eth_getTransactionReceipt", (tx_hash,)).await
    }

    /// Get EVM block by number.
    pub async fn eth_get_block_by_number(
        &self,
        block: &str,
        full_tx: bool,
    ) -> Result<Option<serde_json::Value>> {
        self.request("eth_getBlockByNumber", (block, full_tx)).await
    }

    /// Get EVM logs.
    pub async fn eth_get_logs(&self, filter: &serde_json::Value) -> Result<Vec<serde_json::Value>> {
        self.request("eth_getLogs", (filter,)).await
    }
}

// ============================================================================
// WebSocket RPC Client
// ============================================================================

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex;

/// WebSocket subscription message.
#[derive(Debug, Clone)]
pub enum SubscriptionMessage {
    /// New block header.
    NewHead(BlockHeader),
    /// Finalized block header.
    FinalizedHead(BlockHeader),
    /// New Comit result.
    ComitResult(ComitResult),
    /// Custom subscription data.
    Custom(serde_json::Value),
}

/// Pending RPC requests waiting for response.
type PendingRequests = Arc<Mutex<HashMap<u64, tokio::sync::oneshot::Sender<serde_json::Value>>>>;

/// WebSocket RPC client.
pub struct WsRpcClient {
    endpoint: String,
    request_id: AtomicU64,
    pending: PendingRequests,
    subscriptions: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<SubscriptionMessage>>>>,
    _handle: Option<tokio::task::JoinHandle<()>>,
}

impl WsRpcClient {
    /// Connect to WebSocket endpoint.
    pub async fn connect(endpoint: impl Into<String>) -> Result<Self> {
        let endpoint = endpoint.into();
        let pending: PendingRequests = Arc::new(Mutex::new(HashMap::new()));
        let subscriptions: Arc<Mutex<HashMap<String, mpsc::UnboundedSender<SubscriptionMessage>>>> =
            Arc::new(Mutex::new(HashMap::new()));

        // Note: In production, we would establish actual WebSocket connection here
        // using tokio-tungstenite. For now, this is a placeholder structure.

        Ok(Self {
            endpoint,
            request_id: AtomicU64::new(1),
            pending,
            subscriptions,
            _handle: None,
        })
    }

    /// Get the next request ID.
    fn next_id(&self) -> u64 {
        self.request_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Get endpoint URL.
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Subscribe to new block headers.
    pub async fn subscribe_new_heads(
        &self,
    ) -> Result<mpsc::UnboundedReceiver<SubscriptionMessage>> {
        let (tx, rx) = mpsc::unbounded_channel();
        let mut subs = self.subscriptions.lock().await;
        subs.insert("newHeads".to_string(), tx);
        Ok(rx)
    }

    /// Subscribe to finalized heads.
    pub async fn subscribe_finalized_heads(
        &self,
    ) -> Result<mpsc::UnboundedReceiver<SubscriptionMessage>> {
        let (tx, rx) = mpsc::unbounded_channel();
        let mut subs = self.subscriptions.lock().await;
        subs.insert("finalizedHead".to_string(), tx);
        Ok(rx)
    }

    /// Unsubscribe from a subscription.
    pub async fn unsubscribe(&self, subscription_id: &str) -> Result<bool> {
        let mut subs = self.subscriptions.lock().await;
        Ok(subs.remove(subscription_id).is_some())
    }

    /// Close the WebSocket connection.
    pub async fn close(&self) -> Result<()> {
        // Clean up pending requests and subscriptions
        self.pending.lock().await.clear();
        self.subscriptions.lock().await.clear();
        Ok(())
    }
}

// ============================================================================
// RPC Provider Abstraction
// ============================================================================

/// Unified RPC provider that can be either HTTP or WebSocket.
pub enum RpcProvider {
    Http(HttpRpcClient),
    Ws(WsRpcClient),
}

impl RpcProvider {
    /// Create HTTP provider.
    pub fn http(endpoint: impl Into<String>) -> Self {
        Self::Http(HttpRpcClient::new(endpoint))
    }

    /// Create WebSocket provider.
    pub async fn ws(endpoint: impl Into<String>) -> Result<Self> {
        Ok(Self::Ws(WsRpcClient::connect(endpoint).await?))
    }

    /// Check if this is an HTTP provider.
    pub fn is_http(&self) -> bool {
        matches!(self, Self::Http(_))
    }

    /// Check if this is a WebSocket provider.
    pub fn is_ws(&self) -> bool {
        matches!(self, Self::Ws(_))
    }

    /// Get endpoint URL.
    pub fn endpoint(&self) -> &str {
        match self {
            Self::Http(client) => &client.endpoint,
            Self::Ws(client) => client.endpoint(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rpc_request_serialization() {
        let request = RpcRequest::new(1, "test_method", vec!["param1", "param2"]);
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("jsonrpc"));
        assert!(json.contains("test_method"));
    }

    #[test]
    fn test_http_client_creation() {
        let client = HttpRpcClient::new("http://localhost:9944");
        assert_eq!(client.endpoint, "http://localhost:9944");
    }

    #[test]
    fn test_rpc_provider_http() {
        let provider = RpcProvider::http("http://localhost:9944");
        assert!(provider.is_http());
        assert!(!provider.is_ws());
    }

    #[tokio::test]
    async fn test_ws_client_creation() {
        let client = WsRpcClient::connect("ws://localhost:9944").await.unwrap();
        assert_eq!(client.endpoint(), "ws://localhost:9944");
    }
}
