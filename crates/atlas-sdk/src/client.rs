//! Atlas Sphere SDK client for blockchain interaction.
//!
//! Provides a high-level interface for connecting to Atlas Sphere nodes,
//! submitting Comit transactions, and querying blockchain state.

use crate::comit::ComitBuilder;
use crate::error::{AtlasError, Result};
use crate::rpc::{HttpRpcClient, WsRpcClient};
use crate::types::{AccountInfo, AssetMetadata, BlockHeader, ComitPayload, ComitResult};
use sp_core::{crypto::Pair, sr25519, H256};
use std::sync::Arc;

// ============================================================================
// Client Configuration
// ============================================================================

/// Configuration for AtlasClient.
#[derive(Clone, Debug)]
pub struct ClientConfig {
    /// HTTP RPC endpoint.
    pub http_endpoint: String,
    /// WebSocket RPC endpoint.
    pub ws_endpoint: Option<String>,
    /// Request timeout in seconds.
    pub timeout_secs: u64,
    /// Maximum retry attempts.
    pub max_retries: u32,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            http_endpoint: crate::DEFAULT_HTTP_ENDPOINT.to_string(),
            ws_endpoint: Some(crate::DEFAULT_WS_ENDPOINT.to_string()),
            timeout_secs: 30,
            max_retries: 3,
        }
    }
}

impl ClientConfig {
    /// Create config with HTTP endpoint only.
    pub fn http_only(endpoint: impl Into<String>) -> Self {
        Self {
            http_endpoint: endpoint.into(),
            ws_endpoint: None,
            ..Default::default()
        }
    }

    /// Create config with both HTTP and WebSocket endpoints.
    pub fn with_ws(http_endpoint: impl Into<String>, ws_endpoint: impl Into<String>) -> Self {
        Self {
            http_endpoint: http_endpoint.into(),
            ws_endpoint: Some(ws_endpoint.into()),
            ..Default::default()
        }
    }

    /// Set request timeout.
    pub fn timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    /// Set max retries.
    pub fn retries(mut self, max: u32) -> Self {
        self.max_retries = max;
        self
    }
}

// ============================================================================
// Signer
// ============================================================================

/// Signing capabilities for transactions.
pub trait Signer: Send + Sync {
    /// Get the account ID (public key).
    fn account_id(&self) -> [u8; 32];

    /// Sign a message.
    fn sign(&self, message: &[u8]) -> [u8; 64];
}

/// Sr25519 keypair signer.
pub struct Sr25519Signer {
    pair: sr25519::Pair,
}

impl Sr25519Signer {
    /// Create from seed phrase.
    pub fn from_phrase(phrase: &str, password: Option<&str>) -> Result<Self> {
        let (pair, _) = sr25519::Pair::from_phrase(phrase, password)
            .map_err(|e| AtlasError::InvalidKey(format!("Invalid seed phrase: {:?}", e)))?;
        Ok(Self { pair })
    }

    /// Create from raw seed.
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        let pair = sr25519::Pair::from_seed(seed);
        Self { pair }
    }

    /// Generate a new random keypair.
    pub fn generate() -> Self {
        let (pair, _, _) = sr25519::Pair::generate_with_phrase(None);
        Self { pair }
    }

    /// Get the public key.
    pub fn public(&self) -> sr25519::Public {
        self.pair.public()
    }
}

impl Signer for Sr25519Signer {
    fn account_id(&self) -> [u8; 32] {
        self.pair.public().0
    }

    fn sign(&self, message: &[u8]) -> [u8; 64] {
        self.pair.sign(message).0
    }
}

// ============================================================================
// Atlas Client
// ============================================================================

/// High-level client for interacting with Atlas Sphere blockchain.
pub struct AtlasClient {
    config: ClientConfig,
    http: HttpRpcClient,
    ws: Option<WsRpcClient>,
    signer: Option<Arc<dyn Signer>>,
}

impl AtlasClient {
    /// Create a new client with default configuration.
    pub fn new() -> Self {
        Self::with_config(ClientConfig::default())
    }

    /// Create a new client with custom configuration.
    pub fn with_config(config: ClientConfig) -> Self {
        let http = HttpRpcClient::new(&config.http_endpoint);

        Self {
            config,
            http,
            ws: None,
            signer: None,
        }
    }

    /// Connect to HTTP endpoint.
    pub async fn connect(endpoint: impl Into<String>) -> Result<Self> {
        let config = ClientConfig::http_only(endpoint);
        let client = Self::with_config(config);

        // Verify connection
        client.http.health().await?;

        Ok(client)
    }

    /// Connect to both HTTP and WebSocket endpoints.
    pub async fn connect_with_ws(
        http_endpoint: impl Into<String>,
        ws_endpoint: impl Into<String>,
    ) -> Result<Self> {
        let http_ep = http_endpoint.into();
        let ws_ep = ws_endpoint.into();

        let config = ClientConfig::with_ws(&http_ep, &ws_ep);
        let http = HttpRpcClient::new(&http_ep);
        let ws = Some(WsRpcClient::connect(&ws_ep).await?);

        // Verify connection
        http.health().await?;

        Ok(Self {
            config,
            http,
            ws,
            signer: None,
        })
    }

    /// Set the signer for transactions.
    pub fn with_signer(mut self, signer: impl Signer + 'static) -> Self {
        self.signer = Some(Arc::new(signer));
        self
    }

    /// Get the current signer's account ID.
    pub fn account_id(&self) -> Option<[u8; 32]> {
        self.signer.as_ref().map(|s| s.account_id())
    }

    /// Check if client has a signer.
    pub fn has_signer(&self) -> bool {
        self.signer.is_some()
    }

    /// Get the HTTP endpoint.
    pub fn http_endpoint(&self) -> &str {
        &self.config.http_endpoint
    }

    /// Get the WebSocket endpoint if configured.
    pub fn ws_endpoint(&self) -> Option<&str> {
        self.config.ws_endpoint.as_deref()
    }

    // =========================================================================
    // Chain Info
    // =========================================================================

    /// Get chain name.
    pub async fn chain_name(&self) -> Result<String> {
        self.http.chain_id().await
    }

    /// Get node name.
    pub async fn node_name(&self) -> Result<String> {
        self.http.node_name().await
    }

    /// Get node version.
    pub async fn node_version(&self) -> Result<String> {
        self.http.node_version().await
    }

    /// Get latest block hash.
    pub async fn latest_block_hash(&self) -> Result<H256> {
        let hash_str = self.http.chain_get_head().await?;
        parse_h256(&hash_str)
    }

    /// Get finalized block hash.
    pub async fn finalized_block_hash(&self) -> Result<H256> {
        let hash_str = self.http.chain_get_finalized_head().await?;
        parse_h256(&hash_str)
    }

    /// Get block header by hash.
    pub async fn block_header(&self, hash: Option<H256>) -> Result<BlockHeader> {
        let hash_str = hash.map(|h| format!("0x{:x}", h));
        self.http.chain_get_header(hash_str.as_deref()).await
    }

    /// Get block hash by number.
    pub async fn block_hash(&self, number: u64) -> Result<H256> {
        let hash_str = self.http.chain_get_block_hash(Some(number)).await?;
        parse_h256(&hash_str)
    }

    // =========================================================================
    // Account Queries
    // =========================================================================

    /// Get account info.
    pub async fn account_info(&self, account: &str) -> Result<AccountInfo> {
        self.http.get_account_info(account).await
    }

    /// Get canonical balance for an asset.
    pub async fn balance(&self, account: &str, asset_id: u32) -> Result<u128> {
        let balance_str = self.http.get_canonical_balance(account, asset_id).await?;
        balance_str
            .parse()
            .map_err(|e| AtlasError::Decoding(format!("Failed to parse balance: {}", e)))
    }

    /// Get native token balance.
    pub async fn native_balance(&self, account: &str) -> Result<u128> {
        self.balance(account, 0).await
    }

    /// Check if account is authorized for Comit transactions.
    pub async fn is_authorized(&self, account: &str) -> Result<bool> {
        self.http.is_authorized(account).await
    }

    /// Get asset metadata.
    pub async fn asset_metadata(&self, asset_id: u32) -> Result<AssetMetadata> {
        self.http.get_asset_metadata(asset_id).await
    }

    // =========================================================================
    // Comit Transactions
    // =========================================================================

    /// Create a new Comit transaction builder.
    pub fn comit(&self) -> ComitBuilder {
        ComitBuilder::new()
    }

    /// Submit a Comit transaction.
    pub async fn submit_comit(&self, payload: ComitPayload) -> Result<ComitResult> {
        let signer = self.signer.as_ref().ok_or(AtlasError::NoSigner)?;

        // Validate payload sizes
        let evm_size = payload.evm_payload.as_ref().map(|p| p.len()).unwrap_or(0);
        let svm_size = payload.svm_payload.as_ref().map(|p| p.len()).unwrap_or(0);

        if evm_size > crate::MAX_PAYLOAD_SIZE {
            return Err(AtlasError::PayloadTooLarge(evm_size));
        }
        if svm_size > crate::MAX_PAYLOAD_SIZE {
            return Err(AtlasError::PayloadTooLarge(svm_size));
        }

        // Sign the transaction
        let message = payload_to_signing_message(&payload);
        let signature = signer.sign(&message);

        // Submit via RPC
        let params = SubmitComitParams {
            payload,
            signature: signature.to_vec(),
            sender: signer.account_id().to_vec(),
        };

        self.http.request("atlasKernel_submitComit", params).await
    }

    /// Submit a pre-built and signed Comit transaction.
    pub async fn submit_signed_comit(
        &self,
        payload: ComitPayload,
        signature: &[u8],
        sender: &[u8],
    ) -> Result<ComitResult> {
        let params = SubmitComitParams {
            payload,
            signature: signature.to_vec(),
            sender: sender.to_vec(),
        };

        self.http.request("atlasKernel_submitComit", params).await
    }

    // =========================================================================
    // EVM Operations
    // =========================================================================

    /// Get EVM account balance.
    pub async fn evm_balance(&self, address: &str) -> Result<u128> {
        let balance_hex = self.http.eth_get_balance(address, "latest").await?;
        parse_u128_hex(&balance_hex)
    }

    /// Get EVM nonce.
    pub async fn evm_nonce(&self, address: &str) -> Result<u64> {
        let nonce_hex = self
            .http
            .eth_get_transaction_count(address, "latest")
            .await?;
        parse_u64_hex(&nonce_hex)
    }

    /// Call EVM contract (read-only).
    pub async fn evm_call(&self, to: &str, data: &[u8]) -> Result<Vec<u8>> {
        let call = crate::evm::EvmCallRequest {
            from: None,
            to: to.to_string(),
            data: Some(format!("0x{}", hex::encode(data))),
            value: None,
            gas: None,
            gas_price: None,
        };

        let result = self.http.eth_call(&call, "latest").await?;
        crate::utils::from_hex(&result)
    }

    /// Estimate EVM gas.
    pub async fn evm_estimate_gas(&self, to: &str, data: &[u8]) -> Result<u64> {
        let call = crate::evm::EvmCallRequest {
            from: None,
            to: to.to_string(),
            data: Some(format!("0x{}", hex::encode(data))),
            value: None,
            gas: None,
            gas_price: None,
        };

        let gas_hex = self.http.eth_estimate_gas(&call).await?;
        parse_u64_hex(&gas_hex)
    }

    // =========================================================================
    // Subscriptions (WebSocket)
    // =========================================================================

    /// Subscribe to new block headers.
    /// Returns None if WebSocket is not configured.
    pub async fn subscribe_blocks(
        &self,
    ) -> Result<Option<tokio::sync::mpsc::UnboundedReceiver<crate::rpc::SubscriptionMessage>>> {
        match &self.ws {
            Some(ws) => Ok(Some(ws.subscribe_new_heads().await?)),
            None => Ok(None),
        }
    }

    /// Subscribe to finalized block headers.
    /// Returns None if WebSocket is not configured.
    pub async fn subscribe_finalized(
        &self,
    ) -> Result<Option<tokio::sync::mpsc::UnboundedReceiver<crate::rpc::SubscriptionMessage>>> {
        match &self.ws {
            Some(ws) => Ok(Some(ws.subscribe_finalized_heads().await?)),
            None => Ok(None),
        }
    }
}

impl Default for AtlasClient {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Helper Types and Functions
// ============================================================================

#[derive(serde::Serialize)]
struct SubmitComitParams {
    payload: ComitPayload,
    signature: Vec<u8>,
    sender: Vec<u8>,
}

fn payload_to_signing_message(payload: &ComitPayload) -> Vec<u8> {
    use crate::utils::blake2b_256;

    let mut message = Vec::new();
    message.extend_from_slice(&payload.nonce.to_le_bytes());
    message.extend_from_slice(&payload.evm_gas_limit.to_le_bytes());
    message.extend_from_slice(&payload.svm_compute_limit.to_le_bytes());
    if let Some(ref evm) = payload.evm_payload {
        message.extend_from_slice(evm);
    }
    if let Some(ref svm) = payload.svm_payload {
        message.extend_from_slice(svm);
    }
    message.extend_from_slice(&payload.prepare_root.0);

    blake2b_256(&message).0.to_vec()
}

fn parse_h256(s: &str) -> Result<H256> {
    let clean = s.strip_prefix("0x").unwrap_or(s);
    let bytes =
        hex::decode(clean).map_err(|e| AtlasError::Decoding(format!("Invalid hex: {}", e)))?;

    if bytes.len() != 32 {
        return Err(AtlasError::Decoding(format!(
            "Expected 32 bytes, got {}",
            bytes.len()
        )));
    }

    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(H256(arr))
}

fn parse_u128_hex(s: &str) -> Result<u128> {
    let clean = s.strip_prefix("0x").unwrap_or(s);
    u128::from_str_radix(clean, 16)
        .map_err(|e| AtlasError::Decoding(format!("Invalid u128 hex: {}", e)))
}

fn parse_u64_hex(s: &str) -> Result<u64> {
    let clean = s.strip_prefix("0x").unwrap_or(s);
    u64::from_str_radix(clean, 16)
        .map_err(|e| AtlasError::Decoding(format!("Invalid u64 hex: {}", e)))
}

// ============================================================================
// Quick Connect Functions
// ============================================================================

/// Connect to local development node.
pub async fn connect_local() -> Result<AtlasClient> {
    AtlasClient::connect("http://127.0.0.1:9944").await
}

/// Connect to testnet.
pub async fn connect_testnet() -> Result<AtlasClient> {
    AtlasClient::connect(crate::TESTNET_HTTP_ENDPOINT).await
}

/// Connect to mainnet.
pub async fn connect_mainnet() -> Result<AtlasClient> {
    AtlasClient::connect(crate::MAINNET_HTTP_ENDPOINT).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_config_default() {
        let config = ClientConfig::default();
        assert!(!config.http_endpoint.is_empty());
    }

    #[test]
    fn test_client_config_http_only() {
        let config = ClientConfig::http_only("http://localhost:9944");
        assert_eq!(config.http_endpoint, "http://localhost:9944");
        assert!(config.ws_endpoint.is_none());
    }

    #[test]
    fn test_client_config_with_ws() {
        let config = ClientConfig::with_ws("http://localhost:9944", "ws://localhost:9944");
        assert_eq!(config.http_endpoint, "http://localhost:9944");
        assert_eq!(config.ws_endpoint.as_deref(), Some("ws://localhost:9944"));
    }

    #[test]
    fn test_sr25519_signer_generate() {
        let signer = Sr25519Signer::generate();
        let account_id = signer.account_id();
        assert_eq!(account_id.len(), 32);
    }

    #[test]
    fn test_sr25519_signer_from_seed() {
        let seed = [0x42u8; 32];
        let signer = Sr25519Signer::from_seed(&seed);
        let signature = signer.sign(b"test message");
        assert_eq!(signature.len(), 64);
    }

    #[test]
    fn test_client_creation() {
        let client = AtlasClient::new();
        assert!(!client.has_signer());
    }

    #[test]
    fn test_client_with_signer() {
        let signer = Sr25519Signer::generate();
        let client = AtlasClient::new().with_signer(signer);
        assert!(client.has_signer());
        assert!(client.account_id().is_some());
    }

    #[test]
    fn test_parse_h256() {
        let hash_str = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        let result = parse_h256(hash_str);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_u128_hex() {
        assert_eq!(parse_u128_hex("0x10").unwrap(), 16);
        assert_eq!(parse_u128_hex("ff").unwrap(), 255);
    }

    #[test]
    fn test_parse_u64_hex() {
        assert_eq!(parse_u64_hex("0x64").unwrap(), 100);
        assert_eq!(parse_u64_hex("1000").unwrap(), 4096);
    }
}
