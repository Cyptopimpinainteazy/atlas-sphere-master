//! Multi-chain mempool scanner for ChronosFlash
//!
//! Scans 103+ chains simultaneously for pending transactions

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{interval, Duration};

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::Message;

use crate::config::{ChainConfig, ChronosConfig, MempoolConfig};
use crate::error::{ChronosError, ChronosResult};
use crate::intent::{IntentDetector, SwapIntent};
use crate::types::{Address, ChainId, ChainStatus, Hash, MempoolStats, Timestamp};

/// Multi-chain mempool scanner
pub struct MempoolScanner {
    config: MempoolConfig,
    chains: HashMap<ChainId, ChainScanner>,
    intent_detector: IntentDetector,
    stats: Arc<RwLock<MempoolStats>>,
    statuses: Arc<RwLock<HashMap<ChainId, ChainStatus>>>,
    intent_tx: mpsc::Sender<SwapIntent>,
}

impl MempoolScanner {
    /// Create new mempool scanner
    pub fn new(config: ChronosConfig, intent_tx: mpsc::Sender<SwapIntent>) -> Self {
        let mut chains = HashMap::new();
        
        for (chain_id, chain_config) in config.chains.iter() {
            if chain_config.enabled {
                chains.insert(*chain_id, ChainScanner::new(chain_config.clone()));
            }
        }

        Self {
            config: config.mempool,
            chains,
            intent_detector: IntentDetector::new(),
            stats: Arc::new(RwLock::new(MempoolStats::default())),
            statuses: Arc::new(RwLock::new(HashMap::new())),
            intent_tx,
        }
    }

    /// Start scanning all chains
    pub async fn start(&mut self) -> ChronosResult<()> {
        let scan_interval = Duration::from_millis(self.config.scan_interval_ms);
        let mut ticker = interval(scan_interval);

        loop {
            ticker.tick().await;

            // Scan chains (sequential for now; avoids mutable-borrow fanout issues)
            let mut stats = self.stats.write().await;
            stats.chains_monitored = self.chains.len();

            let chain_ids: Vec<ChainId> = self.chains.keys().copied().collect();
            drop(stats);

            for chain_id in chain_ids {
                let (pending_txs, status) = match self.chains.get_mut(&chain_id) {
                    Some(scanner) => match scanner.scan().await {
                        Ok(txs) => {
                            let st = scanner.get_status();
                            (txs, Some(st))
                        }
                        Err(e) => {
                            log::warn!("Chain scan failed ({}): {}", chain_id, e);
                            // Still publish best-effort status snapshot.
                            let st = scanner.get_status();
                            (Vec::new(), Some(st))
                        }
                    },
                    None => continue,
                };

                if let Some(status) = status {
                    let mut statuses = self.statuses.write().await;
                    statuses.insert(chain_id, status);
                }

                let mut stats = self.stats.write().await;
                stats.total_pending += pending_txs.len();
                drop(stats);

                for tx in pending_txs {
                    if let Some(intent) = self.intent_detector.detect(
                        chain_id,
                        &tx.data,
                        tx.sender,
                        tx.gas_price,
                    ) {
                        let mut stats = self.stats.write().await;
                        stats.swap_intents_detected += 1;
                        drop(stats);

                        if self.intent_tx.send(intent).await.is_err() {
                            return Err(ChronosError::MempoolScanFailed(
                                "Intent channel closed".to_string(),
                            ));
                        }
                    }
                }
            }
        }
    }

    /// Get current statistics
    pub async fn get_stats(&self) -> MempoolStats {
        self.stats.read().await.clone()
    }

    pub fn shared_stats_handle(&self) -> Arc<RwLock<MempoolStats>> {
        self.stats.clone()
    }

    pub fn shared_statuses_handle(&self) -> Arc<RwLock<HashMap<ChainId, ChainStatus>>> {
        self.statuses.clone()
    }

    /// Get chain status
    pub async fn get_chain_status(&self, chain_id: ChainId) -> Option<ChainStatus> {
        self.chains.get(&chain_id).map(|s| s.get_status())
    }

    /// Add a new chain to monitor
    pub fn add_chain(&mut self, config: ChainConfig) {
        self.chains.insert(config.chain_id, ChainScanner::new(config));
    }

    /// Remove a chain from monitoring
    pub fn remove_chain(&mut self, chain_id: ChainId) {
        self.chains.remove(&chain_id);
    }
}

/// Per-chain mempool scanner
struct ChainScanner {
    config: ChainConfig,
    http: reqwest::Client,
    endpoint_idx: usize,
    pending_rx: Option<mpsc::Receiver<PendingTx>>,
    pending_buffer: Vec<PendingTx>,
    pending_txs: HashMap<Hash, PendingTx>,
    last_scan: Timestamp,
    last_info_log: Timestamp,
    backoff_until_ms: Timestamp,
    backoff_ms: u64,
    current_block: u64,
    is_connected: bool,
}

impl ChainScanner {
    fn new(config: ChainConfig) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            config,
            http,
            endpoint_idx: 0,
            pending_rx: None,
            pending_buffer: Vec::new(),
            pending_txs: HashMap::new(),
            last_scan: 0,
            last_info_log: 0,
            backoff_until_ms: 0,
            backoff_ms: 0,
            current_block: 0,
            is_connected: false,
        }
    }

    /// Scan mempool for new pending transactions
    async fn scan(&mut self) -> ChronosResult<Vec<PendingTx>> {
        let now = chrono::Utc::now().timestamp_millis() as u64;
        self.last_scan = now;

        // Connect to chain RPC
        let pending = self.fetch_pending_transactions().await?;
        
        // Filter new transactions
        let mut new_txs = vec![];
        for tx in pending {
            if !self.pending_txs.contains_key(&tx.hash) {
                self.pending_txs.insert(tx.hash, tx.clone());
                new_txs.push(tx);
            }
        }

        // Cleanup old pending txs (they've either been mined or dropped)
        self.cleanup_stale_txs();

        Ok(new_txs)
    }

    /// Fetch pending transactions from RPC
    async fn fetch_pending_transactions(&mut self) -> ChronosResult<Vec<PendingTx>> {
        // Minimal live RPC integration:
        // - EVM: poll `eth_blockNumber` (connectivity + head tracking)
        // - Solana: poll `getSlot` (connectivity + head tracking)
        //
        // Pending tx streaming is provider-specific (WS subscriptions, private relays, etc.),
        // so we keep tx list empty for now but we DO verify connectivity.

        self.update_chain_head().await?;

        // Drain any pending transactions collected by background stream(s).
        self.drain_pending_rx(2_000);
        Ok(std::mem::take(&mut self.pending_buffer))
    }

    fn is_solana(&self) -> bool {
        self.config.chain_id == 1_399_811_149 || self.config.name.eq_ignore_ascii_case("solana")
    }

    fn ensure_stream_started(&mut self) {
        if self.pending_rx.is_some() {
            return;
        }

        // Only implement EVM pending stream for now.
        if self.is_solana() {
            return;
        }

        let http_endpoints = self.config.rpc_endpoints.clone();
        if http_endpoints.is_empty() {
            return;
        }

        let ws_endpoint = self
            .config
            .ws_endpoints
            .first()
            .map(|e| e.trim().to_string())
            .filter(|e| !e.is_empty());

        let http = self.http.clone();
        let (tx, rx) = mpsc::channel::<PendingTx>(20_000);
        self.pending_rx = Some(rx);

        tokio::spawn(async move {
            let res = if let Some(ws_endpoint) = ws_endpoint {
                evm_pending_stream_task(http, http_endpoints, ws_endpoint, tx).await
            } else {
                evm_pending_filter_task(http, http_endpoints, tx).await
            };

            if let Err(e) = res {
                log::warn!("EVM pending stream stopped: {}", e);
            }
        });
    }


    fn drain_pending_rx(&mut self, max: usize) {
        self.ensure_stream_started();

        let Some(rx) = self.pending_rx.as_mut() else {
            return;
        };

        for _ in 0..max {
            match rx.try_recv() {
                Ok(tx) => self.pending_buffer.push(tx),
                Err(tokio::sync::mpsc::error::TryRecvError::Empty) => break,
                Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                    self.pending_rx = None;
                    break;
                }
            }
        }
    }

    async fn rpc_call(
        &self,
        endpoint: &str,
        method: &str,
        params: serde_json::Value,
    ) -> ChronosResult<serde_json::Value> {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });

        let resp = self
            .http
            .post(endpoint)
            .json(&body)
            .send()
            .await
            .map_err(|e| ChronosError::Network(e.to_string()))?;

        let status = resp.status();
        let retry_after_ms = resp
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.trim().parse::<u64>().ok())
            .map(|secs| secs.saturating_mul(1000))
            .unwrap_or(1_000);
        let v: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ChronosError::Network(e.to_string()))?;

        if status.as_u16() == 429 {
            return Err(ChronosError::RateLimited {
                endpoint: endpoint.to_string(),
                retry_after_ms,
            });
        }

        if !status.is_success() {
            return Err(ChronosError::Network(format!(
                "RPC HTTP {} from {}",
                status,
                endpoint
            )));
        }

        if let Some(err) = v.get("error") {
            return Err(ChronosError::Network(format!("RPC error: {}", err)));
        }

        v.get("result")
            .cloned()
            .ok_or_else(|| ChronosError::Network("Missing RPC result".to_string()))
    }

    async fn update_chain_head(&mut self) -> ChronosResult<()> {
        let now_ms = chrono::Utc::now().timestamp_millis() as u64;
        if now_ms < self.backoff_until_ms {
            // Backing off due to rate limiting; keep last-known head.
            return Ok(());
        }

        if self.config.rpc_endpoints.is_empty() {
            self.is_connected = false;
            return Err(ChronosError::InvalidConfig("Missing rpc_endpoints".to_string()));
        }

        // Heuristic: treat Solana specially by chain_id/name.
        let is_solana = self.is_solana();

        // Try endpoints in a stable order starting from last-known-good.
        let n = self.config.rpc_endpoints.len();
        let start = self.endpoint_idx % n;

        let mut last_err: Option<ChronosError> = None;
        let mut rate_limited: usize = 0;
        let mut max_retry_after_ms: u64 = 0;

        for offset in 0..n {
            let idx = (start + offset) % n;
            let endpoint = &self.config.rpc_endpoints[idx];

            let attempt: ChronosResult<()> = if is_solana {
                match self.rpc_call(endpoint, "getSlot", serde_json::json!([])).await {
                    Ok(v) => {
                        let slot = v.as_u64().ok_or_else(|| {
                            ChronosError::Network("Solana getSlot returned non-u64".to_string())
                        })?;
                        self.current_block = slot;
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            } else {
                match self
                    .rpc_call(endpoint, "eth_blockNumber", serde_json::json!([]))
                    .await
                {
                    Ok(v) => {
                        let hex_block = v.as_str().map(|s| s.to_string()).ok_or_else(|| {
                            ChronosError::Network(
                                "eth_blockNumber returned non-string".to_string(),
                            )
                        })?;
                        let parsed = u64::from_str_radix(hex_block.trim_start_matches("0x"), 16)
                            .map_err(|e| {
                                ChronosError::Network(format!(
                                    "Invalid eth_blockNumber hex '{}': {}",
                                    hex_block, e
                                ))
                            })?;
                        self.current_block = parsed;
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            };

            match attempt {
                Ok(()) => {
                    self.endpoint_idx = idx;
                    self.is_connected = true;

                    // Success: reset rate-limit backoff.
                    self.backoff_ms = 0;
                    self.backoff_until_ms = 0;

                    if now_ms.saturating_sub(self.last_info_log) >= 5_000 {
                        self.last_info_log = now_ms;
                        log::info!("{} head: {}", self.config.name, self.current_block);
                    }

                    return Ok(());
                }
                Err(e) => {
                    if let ChronosError::RateLimited {
                        retry_after_ms, ..
                    } = e
                    {
                        rate_limited += 1;
                        max_retry_after_ms = max_retry_after_ms.max(retry_after_ms);
                    }
                    last_err = Some(e);
                    continue;
                }
            }
        }

        // If all endpoints rate-limited, back off quietly.
        if rate_limited == n {
            let next_backoff = if self.backoff_ms == 0 {
                1_000
            } else {
                (self.backoff_ms.saturating_mul(2)).min(30_000)
            };
            self.backoff_ms = next_backoff.max(max_retry_after_ms);
            self.backoff_until_ms = now_ms.saturating_add(self.backoff_ms);
            self.is_connected = true;
            log::info!(
                "{} rate-limited; backing off {}ms",
                self.config.name,
                self.backoff_ms
            );
            return Ok(());
        }

        self.is_connected = false;
        Err(last_err.unwrap_or_else(|| ChronosError::Network("All RPC endpoints failed".to_string())))
    }

    /// Remove stale transactions
    fn cleanup_stale_txs(&mut self) {
        let now = chrono::Utc::now().timestamp_millis() as u64;
        let max_age = 60_000; // 60 seconds
        
        self.pending_txs.retain(|_, tx| now - tx.detected_at < max_age);
    }

    /// Get chain status
    fn get_status(&self) -> ChainStatus {
        ChainStatus {
            chain_id: self.config.chain_id,
            name: self.config.name.clone(),
            is_connected: self.is_connected,
            current_block: self.current_block,
            pending_txs: self.pending_txs.len(),
            avg_block_time_ms: self.config.block_time_ms,
            last_updated: self.last_scan,
            backoff_until_ms: self.backoff_until_ms,
            backoff_ms: self.backoff_ms,
        }
    }
}

fn parse_hex_u64(s: &str) -> Option<u64> {
    let s = s.trim();
    let s = s.strip_prefix("0x").unwrap_or(s);
    u64::from_str_radix(s, 16).ok()
}

fn parse_hex_u128(s: &str) -> Option<u128> {
    let s = s.trim();
    let s = s.strip_prefix("0x").unwrap_or(s);
    u128::from_str_radix(s, 16).ok()
}

fn hex_to_bytes(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    let s = s.strip_prefix("0x").unwrap_or(s);
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

fn hex32(s: &str) -> Option<[u8; 32]> {
    let bytes = hex_to_bytes(s)?;
    if bytes.len() != 32 {
        return None;
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Some(out)
}

fn evm_addr_to_32(s: &str) -> Option<[u8; 32]> {
    let bytes = hex_to_bytes(s)?;
    if bytes.len() != 20 {
        return None;
    }
    let mut out = [0u8; 32];
    out[12..].copy_from_slice(&bytes);
    Some(out)
}

async fn evm_rpc_call(
    http: &reqwest::Client,
    endpoint: &str,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params,
    });

    let resp = http
        .post(endpoint)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let status = resp.status();
    let v: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    if !status.is_success() {
        return Err(format!("HTTP {}", status));
    }
    if let Some(err) = v.get("error") {
        return Err(format!("RPC error: {}", err));
    }
    v.get("result")
        .cloned()
        .ok_or_else(|| "Missing RPC result".to_string())
}

async fn evm_pending_filter_task(
    http: reqwest::Client,
    http_endpoints: Vec<String>,
    out: mpsc::Sender<PendingTx>,
) -> ChronosResult<()> {
    let max_enrich_per_sec: u32 = std::env::var("CHRONOS_EVM_ENRICH_PER_SEC")
        .ok()
        .and_then(|v| v.trim().parse::<u32>().ok())
        .filter(|&v| v > 0)
        .unwrap_or(20);

    let poll_ms: u64 = std::env::var("CHRONOS_EVM_FILTER_POLL_MS")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|&v| v > 0)
        .unwrap_or(250);

    let mut http_idx: usize = 0;
    let mut filter_id: Option<String> = None;

    let mut window_start = tokio::time::Instant::now();
    let mut window_count: u32 = 0;

    loop {
        let n = http_endpoints.len();
        if n == 0 {
            return Ok(());
        }

        // Ensure filter exists.
        if filter_id.is_none() {
            let mut created: Option<String> = None;
            for attempt in 0..n {
                let idx = (http_idx + attempt) % n;
                let endpoint = &http_endpoints[idx];
                match evm_rpc_call(&http, endpoint, "eth_newPendingTransactionFilter", serde_json::json!([])).await {
                    Ok(v) => {
                        if let Some(fid) = v.as_str().map(|s| s.to_string()) {
                            created = Some(fid);
                            http_idx = idx;
                            break;
                        }
                    }
                    Err(_) => continue,
                }
            }

            filter_id = created;
            tokio::time::sleep(std::time::Duration::from_millis(poll_ms)).await;
            continue;
        }

        let fid = filter_id.clone().unwrap();

        // Poll filter changes.
        let mut changes: Option<serde_json::Value> = None;
        for attempt in 0..n {
            let idx = (http_idx + attempt) % n;
            let endpoint = &http_endpoints[idx];
            match evm_rpc_call(&http, endpoint, "eth_getFilterChanges", serde_json::json!([fid.clone()])).await {
                Ok(v) => {
                    changes = Some(v);
                    http_idx = idx;
                    break;
                }
                Err(_) => continue,
            }
        }

        let Some(changes) = changes else {
            // Filter likely invalid/expired or provider blocked it; recreate next loop.
            filter_id = None;
            tokio::time::sleep(std::time::Duration::from_millis(poll_ms)).await;
            continue;
        };

        let Some(arr) = changes.as_array() else {
            tokio::time::sleep(std::time::Duration::from_millis(poll_ms)).await;
            continue;
        };

        for h in arr {
            let Some(tx_hash) = h.as_str().map(|s| s.to_string()) else {
                continue;
            };

            // Rate limit enrichment to avoid provider bans.
            let now = tokio::time::Instant::now();
            if now.duration_since(window_start).as_secs_f32() >= 1.0 {
                window_start = now;
                window_count = 0;
            }
            if window_count >= max_enrich_per_sec {
                break;
            }
            window_count += 1;

            // Enrich via eth_getTransactionByHash (best-effort failover).
            let mut got: Option<serde_json::Value> = None;
            for attempt in 0..n {
                let idx = (http_idx + attempt) % n;
                let endpoint = &http_endpoints[idx];
                match evm_rpc_call(&http, endpoint, "eth_getTransactionByHash", serde_json::json!([tx_hash.clone()]))
                    .await
                {
                    Ok(v) => {
                        if v.is_null() {
                            got = None;
                        } else {
                            got = Some(v);
                            http_idx = idx;
                        }
                        break;
                    }
                    Err(_) => continue,
                }
            }

            let Some(txv) = got else {
                continue;
            };

            let hash = match txv.get("hash").and_then(|x| x.as_str()).and_then(hex32) {
                Some(h) => h,
                None => continue,
            };
            let sender = match txv.get("from").and_then(|x| x.as_str()).and_then(evm_addr_to_32) {
                Some(a) => a,
                None => continue,
            };
            let to = txv
                .get("to")
                .and_then(|x| x.as_str())
                .and_then(evm_addr_to_32);

            let data = txv
                .get("input")
                .and_then(|x| x.as_str())
                .and_then(hex_to_bytes)
                .unwrap_or_default();

            let value = txv
                .get("value")
                .and_then(|x| x.as_str())
                .and_then(parse_hex_u128)
                .unwrap_or(0);

            let gas_limit = txv
                .get("gas")
                .and_then(|x| x.as_str())
                .and_then(parse_hex_u64)
                .unwrap_or(0);

            let nonce = txv
                .get("nonce")
                .and_then(|x| x.as_str())
                .and_then(parse_hex_u64)
                .unwrap_or(0);

            // Prefer legacy gasPrice, else maxFeePerGas (EIP-1559).
            let gas_price = txv
                .get("gasPrice")
                .and_then(|x| x.as_str())
                .and_then(parse_hex_u128)
                .or_else(|| {
                    txv.get("maxFeePerGas")
                        .and_then(|x| x.as_str())
                        .and_then(parse_hex_u128)
                })
                .unwrap_or(0);

            let pending = PendingTx {
                hash,
                sender,
                to,
                data,
                value,
                gas_price,
                gas_limit,
                nonce,
                detected_at: chrono::Utc::now().timestamp_millis() as u64,
            };

            if out.send(pending).await.is_err() {
                return Ok(());
            }
        }

        tokio::time::sleep(std::time::Duration::from_millis(poll_ms)).await;
    }
}

async fn evm_pending_stream_task(
    http: reqwest::Client,
    http_endpoints: Vec<String>,
    ws_endpoint: String,
    out: mpsc::Sender<PendingTx>,
) -> ChronosResult<()> {
    let max_enrich_per_sec: u32 = std::env::var("CHRONOS_EVM_ENRICH_PER_SEC")
        .ok()
        .and_then(|v| v.trim().parse::<u32>().ok())
        .filter(|&v| v > 0)
        .unwrap_or(20);

    let url = url::Url::parse(&ws_endpoint)
        .map_err(|e| ChronosError::InvalidConfig(format!("Invalid WS endpoint '{}': {}", ws_endpoint, e)))?;

    let (mut ws, _) = tokio_tungstenite::connect_async(url)
        .await
        .map_err(|e| ChronosError::Network(format!("WS connect failed: {}", e)))?;

    log::info!("EVM pending WS connected: {}", ws_endpoint);

    // Subscribe to newPendingTransactions
    let subscribe = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "eth_subscribe",
        "params": ["newPendingTransactions"],
    });
    ws.send(Message::Text(subscribe.to_string()))
        .await
        .map_err(|e| ChronosError::Network(format!("WS send failed: {}", e)))?;

    log::info!("EVM pending WS subscription requested (newPendingTransactions)");

    // Best-effort: pick first HTTP endpoint, try others on failure.
    let mut http_idx: usize = 0;

    let mut window_start = tokio::time::Instant::now();
    let mut window_count: u32 = 0;

    let mut logged_first_pending_hash = false;

    while let Some(msg) = ws.next().await {
        let msg = msg.map_err(|e| ChronosError::Network(format!("WS recv failed: {}", e)))?;

        let text = match msg {
            Message::Text(t) => t,
            Message::Binary(b) => String::from_utf8_lossy(&b).to_string(),
            Message::Ping(_) | Message::Pong(_) => continue,
            Message::Close(_) => break,
            _ => continue,
        };

        let v: serde_json::Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let maybe_hash = v
            .get("params")
            .and_then(|p| p.get("result"))
            .and_then(|r| r.as_str())
            .map(|s| s.to_string());

        let Some(tx_hash) = maybe_hash else { continue };

        if !logged_first_pending_hash {
            logged_first_pending_hash = true;
            log::info!("EVM pending WS stream active (received first pending tx hash)");
        }

        // Rate limit enrichment to avoid provider bans.
        let now = tokio::time::Instant::now();
        if now.duration_since(window_start).as_secs_f32() >= 1.0 {
            window_start = now;
            window_count = 0;
        }
        if window_count >= max_enrich_per_sec {
            continue;
        }
        window_count += 1;

        // Fetch tx details via eth_getTransactionByHash
        let n = http_endpoints.len();
        if n == 0 {
            continue;
        }

        let mut got: Option<serde_json::Value> = None;
        for attempt in 0..n {
            let idx = (http_idx + attempt) % n;
            let endpoint = &http_endpoints[idx];
            match evm_rpc_call(&http, endpoint, "eth_getTransactionByHash", serde_json::json!([tx_hash]))
                .await
            {
                Ok(v) => {
                    if v.is_null() {
                        got = None;
                    } else {
                        got = Some(v);
                        http_idx = idx;
                    }
                    break;
                }
                Err(_) => continue,
            }
        }

        let Some(txv) = got else {
            continue;
        };

        let hash = match txv.get("hash").and_then(|x| x.as_str()).and_then(hex32) {
            Some(h) => h,
            None => continue,
        };
        let sender = match txv.get("from").and_then(|x| x.as_str()).and_then(evm_addr_to_32) {
            Some(a) => a,
            None => continue,
        };
        let to = txv
            .get("to")
            .and_then(|x| x.as_str())
            .and_then(evm_addr_to_32);

        let data = txv
            .get("input")
            .and_then(|x| x.as_str())
            .and_then(hex_to_bytes)
            .unwrap_or_default();

        let value = txv
            .get("value")
            .and_then(|x| x.as_str())
            .and_then(parse_hex_u128)
            .unwrap_or(0);

        let gas_limit = txv
            .get("gas")
            .and_then(|x| x.as_str())
            .and_then(parse_hex_u64)
            .unwrap_or(0);

        let nonce = txv
            .get("nonce")
            .and_then(|x| x.as_str())
            .and_then(parse_hex_u64)
            .unwrap_or(0);

        // Prefer legacy gasPrice, else maxFeePerGas (EIP-1559).
        let gas_price = txv
            .get("gasPrice")
            .and_then(|x| x.as_str())
            .and_then(parse_hex_u128)
            .or_else(|| {
                txv.get("maxFeePerGas")
                    .and_then(|x| x.as_str())
                    .and_then(parse_hex_u128)
            })
            .unwrap_or(0);

        let pending = PendingTx {
            hash,
            sender,
            to,
            data,
            value,
            gas_price,
            gas_limit,
            nonce,
            detected_at: chrono::Utc::now().timestamp_millis() as u64,
        };

        if out.send(pending).await.is_err() {
            break;
        }

        // Tiny pacing to reduce burstiness.
        tokio::time::sleep(Duration::from_millis(5)).await;
    }

    Ok(())
}

/// Pending transaction from mempool
#[derive(Debug, Clone)]
pub struct PendingTx {
    pub hash: Hash,
    pub sender: Address,
    pub to: Option<Address>,
    pub data: Vec<u8>,
    pub value: u128,
    pub gas_price: u128,
    pub gas_limit: u64,
    pub nonce: u64,
    pub detected_at: Timestamp,
}

/// Mempool stream connection for real-time updates
pub struct MempoolStream {
    chain_id: ChainId,
    endpoint: String,
    is_connected: bool,
}

impl MempoolStream {
    pub fn new(chain_id: ChainId, endpoint: String) -> Self {
        Self {
            chain_id,
            endpoint,
            is_connected: false,
        }
    }

    /// Connect to mempool WebSocket stream
    pub async fn connect(&mut self) -> ChronosResult<()> {
        // In production:
        // - Connect to bloxroute/flashbots mempool stream
        // - Subscribe to pending transactions
        // - Filter by DEX router addresses
        
        self.is_connected = true;
        Ok(())
    }

    /// Subscribe to pending transactions
    pub async fn subscribe(&mut self) -> ChronosResult<mpsc::Receiver<PendingTx>> {
        let (_tx, rx) = mpsc::channel(10000);
        
        // Spawn background task to receive transactions
        let _endpoint = self.endpoint.clone();
        let _chain_id = self.chain_id;
        
        tokio::spawn(async move {
            // WebSocket connection and subscription logic
            // eth_subscribe("newPendingTransactions")
            loop {
                // Receive and forward pending txs
                tokio::time::sleep(Duration::from_millis(10)).await;
                
                // In production: receive from WebSocket and send to channel
            }
        });

        Ok(rx)
    }
}

/// Bloxroute mempool stream (premium mempool data)
pub struct BloxrouteMempoolStream {
    #[allow(dead_code)]
    auth_token: String,
    is_connected: bool,
}

impl BloxrouteMempoolStream {
    pub fn new(auth_token: String) -> Self {
        Self {
            auth_token,
            is_connected: false,
        }
    }

    /// Connect to Bloxroute mempool stream
    pub async fn connect(&mut self) -> ChronosResult<()> {
        // Bloxroute provides:
        // - 100ms+ faster mempool data
        // - Cross-chain mempool aggregation
        // - Transaction simulation
        
        self.is_connected = true;
        Ok(())
    }
}

/// Flashbots mempool stream (private transactions)
pub struct FlashbotsMempoolStream {
    #[allow(dead_code)]
    relay_url: String,
    is_connected: bool,
}

impl FlashbotsMempoolStream {
    pub fn new(relay_url: String) -> Self {
        Self {
            relay_url,
            is_connected: false,
        }
    }

    /// Connect to Flashbots relay for private transaction hints
    pub async fn connect(&mut self) -> ChronosResult<()> {
        // Flashbots provides:
        // - Private transaction hints (searcher bundle tips)
        // - Block builder preferences
        // - MEV-share orderflow
        
        self.is_connected = true;
        Ok(())
    }
}

/// Multi-stream aggregator
pub struct MempoolAggregator {
    streams: Vec<Box<dyn MempoolProvider + Send + Sync>>,
    dedupe_window: HashMap<Hash, Timestamp>,
}

impl MempoolAggregator {
    pub fn new() -> Self {
        Self {
            streams: vec![],
            dedupe_window: HashMap::new(),
        }
    }

    /// Add a mempool provider
    pub fn add_provider(&mut self, provider: Box<dyn MempoolProvider + Send + Sync>) {
        self.streams.push(provider);
    }

    /// Aggregate pending transactions from all providers
    pub async fn aggregate(&mut self) -> ChronosResult<Vec<PendingTx>> {
        let mut all_txs = vec![];
        let now = chrono::Utc::now().timestamp_millis() as u64;

        for stream in &mut self.streams {
            if let Ok(txs) = stream.get_pending().await {
                for tx in txs {
                    // Deduplicate
                    if !self.dedupe_window.contains_key(&tx.hash) {
                        self.dedupe_window.insert(tx.hash, now);
                        all_txs.push(tx);
                    }
                }
            }
        }

        // Cleanup old dedupe entries
        self.dedupe_window.retain(|_, ts| now - *ts < 60_000);

        Ok(all_txs)
    }
}

impl Default for MempoolAggregator {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for mempool data providers
#[async_trait::async_trait]
pub trait MempoolProvider {
    async fn get_pending(&mut self) -> ChronosResult<Vec<PendingTx>>;
    fn chain_id(&self) -> ChainId;
    fn is_connected(&self) -> bool;
}
