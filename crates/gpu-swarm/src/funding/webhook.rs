//! Webhook Bridge - n8n Integration for Funding Automation
//!
//! Bridges the GPU swarm with n8n workflows for automated outreach.
//! Supports the lane3-social-detonator and lane4-funding-magnet workflows.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Webhook configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    /// n8n webhook base URL
    pub n8n_base_url: Option<String>,
    /// Custom webhook URLs by lane
    pub lane_urls: HashMap<String, String>,
    /// Request timeout
    pub timeout: Duration,
    /// Max retries
    pub max_retries: u8,
    /// Retry delay
    pub retry_delay: Duration,
    /// Enable dry run mode
    pub dry_run: bool,
    /// Rate limit (requests per minute)
    pub rate_limit_rpm: u32,
}

impl Default for WebhookConfig {
    fn default() -> Self {
        let mut lane_urls = HashMap::new();
        lane_urls.insert(
            "lane3-social-detonator".to_string(),
            "http://localhost:5678/webhook/lane3-social".to_string(),
        );
        lane_urls.insert(
            "lane4-funding-magnet".to_string(),
            "http://localhost:5678/webhook/lane4-funding".to_string(),
        );

        Self {
            n8n_base_url: Some("http://localhost:5678".to_string()),
            lane_urls,
            timeout: Duration::from_secs(30),
            max_retries: 3,
            retry_delay: Duration::from_secs(2),
            dry_run: true, // Default to dry run for safety
            rate_limit_rpm: 60,
        }
    }
}

/// Webhook payload structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    /// Trigger type
    pub trigger: String,
    /// Project name
    pub project: String,
    /// Campaign ID
    pub campaign_id: String,
    /// Campaign type
    pub campaign_type: String,
    /// Target lane (workflow)
    pub lane: String,
    /// Prospect information
    pub prospect: Option<ProspectPayload>,
    /// Content payload
    pub content: Option<ContentPayload>,
    /// Metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Timestamp
    pub created_at: String,
}

/// Prospect data for webhook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProspectPayload {
    pub id: String,
    pub name: String,
    pub email: Option<String>,
    pub twitter: Option<String>,
    pub description: Option<String>,
}

/// Content data for webhook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentPayload {
    pub subject: Option<String>,
    pub body: String,
    pub variant_id: String,
    pub novaflux_script: Option<String>,
    pub captions: Option<Vec<CaptionPayload>>,
}

/// Caption data for webhook
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptionPayload {
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
}

/// Result of webhook dispatch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookResult {
    /// Whether dispatch succeeded
    pub success: bool,
    /// Response status code
    pub status_code: Option<u16>,
    /// Response body (if any)
    pub response_body: Option<String>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Retry count
    pub retry_count: u8,
    /// Duration of request
    pub duration_ms: u64,
    /// Whether this was a dry run
    pub dry_run: bool,
}

/// Rate limiter state
struct RateLimiter {
    requests_this_minute: u32,
    minute_started: Instant,
    limit: u32,
}

impl RateLimiter {
    fn new(limit: u32) -> Self {
        Self {
            requests_this_minute: 0,
            minute_started: Instant::now(),
            limit,
        }
    }

    fn check_and_increment(&mut self) -> bool {
        if self.minute_started.elapsed() > Duration::from_secs(60) {
            self.requests_this_minute = 0;
            self.minute_started = Instant::now();
        }

        if self.requests_this_minute < self.limit {
            self.requests_this_minute += 1;
            true
        } else {
            false
        }
    }
}

/// Webhook Bridge for n8n integration
pub struct WebhookBridge {
    config: WebhookConfig,
    rate_limiter: Arc<RwLock<RateLimiter>>,
    /// Dispatch history
    history: Arc<RwLock<Vec<DispatchRecord>>>,
    /// Pending queue
    pending: Arc<RwLock<Vec<WebhookPayload>>>,
}

/// Record of a dispatch attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchRecord {
    pub payload_id: String,
    pub lane: String,
    pub result: WebhookResult,
    pub timestamp: u64,
}

impl WebhookBridge {
    /// Create a new webhook bridge
    pub fn new(config: WebhookConfig) -> Self {
        let rate_limiter = Arc::new(RwLock::new(RateLimiter::new(config.rate_limit_rpm)));
        Self {
            config,
            rate_limiter,
            history: Arc::new(RwLock::new(Vec::new())),
            pending: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create with default configuration
    pub fn default_bridge() -> Self {
        Self::new(WebhookConfig::default())
    }

    /// Get URL for a lane
    fn get_lane_url(&self, lane: &str) -> Option<String> {
        self.config.lane_urls.get(lane).cloned().or_else(|| {
            self.config
                .n8n_base_url
                .as_ref()
                .map(|base| format!("{}/webhook/{}", base, lane))
        })
    }

    /// Dispatch a webhook payload
    pub async fn dispatch(&self, payload: WebhookPayload) -> WebhookResult {
        let start = Instant::now();
        let lane = payload.lane.clone();
        let payload_id = format!("{}-{}", payload.campaign_id, payload.created_at);

        // Check rate limit
        {
            let mut limiter = self.rate_limiter.write().await;
            if !limiter.check_and_increment() {
                return WebhookResult {
                    success: false,
                    status_code: None,
                    response_body: None,
                    error: Some("Rate limit exceeded".to_string()),
                    retry_count: 0,
                    duration_ms: start.elapsed().as_millis() as u64,
                    dry_run: self.config.dry_run,
                };
            }
        }

        // Dry run mode - just simulate
        if self.config.dry_run {
            let result = WebhookResult {
                success: true,
                status_code: Some(200),
                response_body: Some("{\"status\":\"dry_run\"}".to_string()),
                error: None,
                retry_count: 0,
                duration_ms: start.elapsed().as_millis() as u64,
                dry_run: true,
            };

            self.record_dispatch(&payload_id, &lane, &result).await;
            return result;
        }

        // Get URL
        let url = match self.get_lane_url(&lane) {
            Some(u) => u,
            None => {
                return WebhookResult {
                    success: false,
                    status_code: None,
                    response_body: None,
                    error: Some(format!("No URL configured for lane: {}", lane)),
                    retry_count: 0,
                    duration_ms: start.elapsed().as_millis() as u64,
                    dry_run: false,
                };
            }
        };

        // Attempt dispatch with retries
        let mut retry_count = 0;
        let mut last_error = None;

        while retry_count <= self.config.max_retries {
            match self.send_request(&url, &payload).await {
                Ok(result) => {
                    let mut final_result = result;
                    final_result.retry_count = retry_count;
                    final_result.duration_ms = start.elapsed().as_millis() as u64;
                    self.record_dispatch(&payload_id, &lane, &final_result).await;
                    return final_result;
                }
                Err(e) => {
                    last_error = Some(e);
                    retry_count += 1;
                    if retry_count <= self.config.max_retries {
                        tokio::time::sleep(self.config.retry_delay).await;
                    }
                }
            }
        }

        let result = WebhookResult {
            success: false,
            status_code: None,
            response_body: None,
            error: last_error,
            retry_count,
            duration_ms: start.elapsed().as_millis() as u64,
            dry_run: false,
        };

        self.record_dispatch(&payload_id, &lane, &result).await;
        result
    }

    /// Send HTTP request (stub - would use reqwest in production)
    async fn send_request(&self, _url: &str, _payload: &WebhookPayload) -> Result<WebhookResult, String> {
        // In production, this would use reqwest:
        // let client = reqwest::Client::new();
        // let response = client
        //     .post(url)
        //     .json(payload)
        //     .timeout(self.config.timeout)
        //     .send()
        //     .await?;

        // For now, simulate success
        Ok(WebhookResult {
            success: true,
            status_code: Some(200),
            response_body: Some("{\"status\":\"accepted\"}".to_string()),
            error: None,
            retry_count: 0,
            duration_ms: 0,
            dry_run: false,
        })
    }

    /// Record dispatch for history
    async fn record_dispatch(&self, payload_id: &str, lane: &str, result: &WebhookResult) {
        let record = DispatchRecord {
            payload_id: payload_id.to_string(),
            lane: lane.to_string(),
            result: result.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        let mut history = self.history.write().await;
        history.push(record);

        // Keep last 1000 records
        if history.len() > 1000 {
            history.drain(0..100);
        }
    }

    /// Queue a payload for later dispatch
    pub async fn queue(&self, payload: WebhookPayload) {
        let mut pending = self.pending.write().await;
        pending.push(payload);
    }

    /// Dispatch all pending payloads
    pub async fn flush_queue(&self) -> Vec<WebhookResult> {
        let payloads = {
            let mut pending = self.pending.write().await;
            std::mem::take(&mut *pending)
        };

        let mut results = vec![];
        for payload in payloads {
            let result = self.dispatch(payload).await;
            results.push(result);
        }
        results
    }

    /// Get dispatch history
    pub async fn get_history(&self, limit: usize) -> Vec<DispatchRecord> {
        let history = self.history.read().await;
        history.iter().rev().take(limit).cloned().collect()
    }

    /// Get queue size
    pub async fn queue_size(&self) -> usize {
        self.pending.read().await.len()
    }

    /// Get statistics
    pub async fn get_stats(&self) -> WebhookStats {
        let history = self.history.read().await;
        let pending = self.pending.read().await;

        let success_count = history.iter().filter(|r| r.result.success).count();
        let failure_count = history.len() - success_count;
        let dry_run_count = history.iter().filter(|r| r.result.dry_run).count();

        WebhookStats {
            total_dispatched: history.len(),
            success_count,
            failure_count,
            dry_run_count,
            pending_count: pending.len(),
            dry_run_mode: self.config.dry_run,
        }
    }

    /// Build a payload for social campaign
    pub fn build_social_payload(
        &self,
        campaign_id: &str,
        script_id: &str,
        hook: &str,
        script: &str,
        cta: &str,
        captions: Vec<(u64, u64, String)>,
    ) -> WebhookPayload {
        let caption_payloads: Vec<CaptionPayload> = captions
            .into_iter()
            .map(|(start, end, text)| CaptionPayload {
                start_ms: start,
                end_ms: end,
                text,
            })
            .collect();

        WebhookPayload {
            trigger: "social_content".to_string(),
            project: "Atlas Sphere".to_string(),
            campaign_id: campaign_id.to_string(),
            campaign_type: "SocialCampaign".to_string(),
            lane: "lane3-social-detonator".to_string(),
            prospect: None,
            content: Some(ContentPayload {
                subject: None,
                body: format!("{}\n\n{}\n\n{}", hook, script, cta),
                variant_id: script_id.to_string(),
                novaflux_script: Some(format!("Hook: \"{}\"\nScript: \"{}\"", hook, script)),
                captions: Some(caption_payloads),
            }),
            metadata: HashMap::new(),
            created_at: timestamp_now(),
        }
    }

    /// Build a payload for funding outreach
    pub fn build_funding_payload(
        &self,
        campaign_id: &str,
        prospect_id: &str,
        prospect_name: &str,
        prospect_email: Option<&str>,
        subject: &str,
        body: &str,
        variant_id: &str,
    ) -> WebhookPayload {
        WebhookPayload {
            trigger: "funding_outreach".to_string(),
            project: "Atlas Sphere".to_string(),
            campaign_id: campaign_id.to_string(),
            campaign_type: "VcOutreach".to_string(),
            lane: "lane4-funding-magnet".to_string(),
            prospect: Some(ProspectPayload {
                id: prospect_id.to_string(),
                name: prospect_name.to_string(),
                email: prospect_email.map(String::from),
                twitter: None,
                description: None,
            }),
            content: Some(ContentPayload {
                subject: Some(subject.to_string()),
                body: body.to_string(),
                variant_id: variant_id.to_string(),
                novaflux_script: None,
                captions: None,
            }),
            metadata: HashMap::new(),
            created_at: timestamp_now(),
        }
    }
}

/// Webhook statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookStats {
    pub total_dispatched: usize,
    pub success_count: usize,
    pub failure_count: usize,
    pub dry_run_count: usize,
    pub pending_count: usize,
    pub dry_run_mode: bool,
}

/// Get current timestamp as ISO string
fn timestamp_now() -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}", ts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bridge_creation() {
        let bridge = WebhookBridge::default_bridge();
        let stats = bridge.get_stats().await;

        assert!(stats.dry_run_mode);
        assert_eq!(stats.total_dispatched, 0);
    }

    #[tokio::test]
    async fn test_dry_run_dispatch() {
        let bridge = WebhookBridge::default_bridge();

        let payload = WebhookPayload {
            trigger: "test".to_string(),
            project: "Atlas Sphere".to_string(),
            campaign_id: "test-001".to_string(),
            campaign_type: "Test".to_string(),
            lane: "lane3-social-detonator".to_string(),
            prospect: None,
            content: None,
            metadata: HashMap::new(),
            created_at: timestamp_now(),
        };

        let result = bridge.dispatch(payload).await;

        assert!(result.success);
        assert!(result.dry_run);
        assert_eq!(result.status_code, Some(200));
    }

    #[tokio::test]
    async fn test_queue_operations() {
        let bridge = WebhookBridge::default_bridge();

        let payload = WebhookPayload {
            trigger: "test".to_string(),
            project: "Atlas Sphere".to_string(),
            campaign_id: "test-002".to_string(),
            campaign_type: "Test".to_string(),
            lane: "lane4-funding-magnet".to_string(),
            prospect: None,
            content: None,
            metadata: HashMap::new(),
            created_at: timestamp_now(),
        };

        bridge.queue(payload).await;
        assert_eq!(bridge.queue_size().await, 1);

        let results = bridge.flush_queue().await;
        assert_eq!(results.len(), 1);
        assert_eq!(bridge.queue_size().await, 0);
    }

    #[tokio::test]
    async fn test_social_payload_builder() {
        let bridge = WebhookBridge::default_bridge();

        let payload = bridge.build_social_payload(
            "social-001",
            "script-001",
            "What if one chain could run every EVM contract?",
            "Dual VM: EVM + SVM for the best of both worlds.",
            "Testnet link in bio.",
            vec![(0, 500, "Hook text".to_string())],
        );

        assert_eq!(payload.lane, "lane3-social-detonator");
        assert!(payload.content.is_some());
        assert!(payload.content.as_ref().unwrap().novaflux_script.is_some());
    }

    #[tokio::test]
    async fn test_funding_payload_builder() {
        let bridge = WebhookBridge::default_bridge();

        let payload = bridge.build_funding_payload(
            "funding-001",
            "prospect-001",
            "Alice Ventures",
            Some("alice@ventures.example"),
            "Demo request - Atlas Sphere",
            "Hi Alice, ...",
            "v1",
        );

        assert_eq!(payload.lane, "lane4-funding-magnet");
        assert!(payload.prospect.is_some());
        assert_eq!(payload.prospect.as_ref().unwrap().name, "Alice Ventures");
    }

    #[tokio::test]
    async fn test_history_tracking() {
        let bridge = WebhookBridge::default_bridge();

        let payload = WebhookPayload {
            trigger: "test".to_string(),
            project: "Atlas Sphere".to_string(),
            campaign_id: "test-003".to_string(),
            campaign_type: "Test".to_string(),
            lane: "lane3-social-detonator".to_string(),
            prospect: None,
            content: None,
            metadata: HashMap::new(),
            created_at: timestamp_now(),
        };

        bridge.dispatch(payload).await;

        let history = bridge.get_history(10).await;
        assert_eq!(history.len(), 1);
        assert!(history[0].result.success);
    }

    #[test]
    fn test_lane_url_resolution() {
        let config = WebhookConfig::default();
        let bridge = WebhookBridge::new(config);

        let url = bridge.get_lane_url("lane3-social-detonator");
        assert!(url.is_some());
        assert!(url.unwrap().contains("lane3"));
    }
}
