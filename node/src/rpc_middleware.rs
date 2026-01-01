/// RPC Rate Limiting and Security Middleware for Atlas Sphere
///
/// Provides per-connection and per-method rate limiting to prevent DoS attacks
/// and abuse of RPC endpoints.
///
/// Note: The RpcServiceBuilder and RpcServiceT integration requires jsonrpsee 0.20+
/// which conflicts with substrate's pinned version. The standalone RateLimiter and
/// CorsConfig are still usable for manual rate limiting checks.
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Rate limit configuration
#[derive(Clone, Debug)]
pub struct RateLimitConfig {
    /// Maximum requests per second per connection
    pub requests_per_second: u32,
    /// Maximum concurrent subscriptions per connection
    pub max_subscriptions_per_connection: u32,
    /// Burst allowance (requests can spike up to this)
    pub burst_size: u32,
    /// Per-method rate limits (method -> requests per minute)
    pub method_limits: HashMap<String, u32>,
    /// Default requests per minute for unlisted methods
    pub default_method_limit: u32,
    /// Ban duration after rate limit exceeded
    pub ban_duration: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        let mut method_limits = HashMap::new();
        // Heavy methods get stricter limits
        method_limits.insert("eth_call".to_string(), 100); // 100/min
        method_limits.insert("eth_estimateGas".to_string(), 60); // 60/min
        method_limits.insert("atlasKernel_getCanonicalBalance".to_string(), 300); // 300/min
        method_limits.insert("atomicTrade_simulate".to_string(), 30); // 30/min (expensive)

        Self {
            requests_per_second: 50,
            max_subscriptions_per_connection: 10,
            burst_size: 100,
            method_limits,
            default_method_limit: 600, // 10/sec default
            ban_duration: Duration::from_secs(60),
        }
    }
}

/// Per-connection rate limit state
struct ConnectionState {
    /// Token bucket for overall rate limiting
    tokens: AtomicU64,
    /// Last token refill time
    last_refill: RwLock<Instant>,
    /// Per-method request counts (method -> (count, window_start))
    method_counts: RwLock<HashMap<String, (u32, Instant)>>,
    /// Active subscription count
    subscriptions: AtomicU64,
    /// Ban expiry time (if banned)
    banned_until: RwLock<Option<Instant>>,
}

impl ConnectionState {
    fn new(burst_size: u32) -> Self {
        Self {
            tokens: AtomicU64::new(burst_size as u64),
            last_refill: RwLock::new(Instant::now()),
            method_counts: RwLock::new(HashMap::new()),
            subscriptions: AtomicU64::new(0),
            banned_until: RwLock::new(None),
        }
    }
}

/// Rate limiter state shared across connections
pub struct RateLimiter {
    config: RateLimitConfig,
    connections: RwLock<HashMap<SocketAddr, Arc<ConnectionState>>>,
    /// Global request counter for metrics
    total_requests: AtomicU64,
    /// Global rejected counter for metrics
    total_rejected: AtomicU64,
}

impl RateLimiter {
    /// Construct a shared rate limiter with the given configuration.
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            connections: RwLock::new(HashMap::new()),
            total_requests: AtomicU64::new(0),
            total_rejected: AtomicU64::new(0),
        }
    }

    /// Get or create connection state
    fn get_connection_state(&self, addr: SocketAddr) -> Arc<ConnectionState> {
        let mut conns = self.connections.write().unwrap();
        conns
            .entry(addr)
            .or_insert_with(|| Arc::new(ConnectionState::new(self.config.burst_size)))
            .clone()
    }

    /// Check if a request should be allowed
    pub fn check_request(&self, addr: SocketAddr, method: &str) -> Result<(), RateLimitError> {
        self.total_requests.fetch_add(1, Ordering::Relaxed);

        let state = self.get_connection_state(addr);

        // Check if banned
        if let Some(banned_until) = *state.banned_until.read().unwrap() {
            if Instant::now() < banned_until {
                self.total_rejected.fetch_add(1, Ordering::Relaxed);
                return Err(RateLimitError::Banned);
            } else {
                *state.banned_until.write().unwrap() = None;
            }
        }

        // Refill tokens based on elapsed time
        self.refill_tokens(&state);

        // Try to consume a token
        let tokens = state.tokens.load(Ordering::Relaxed);
        if tokens == 0 {
            // Rate limited - apply ban
            *state.banned_until.write().unwrap() = Some(Instant::now() + self.config.ban_duration);
            self.total_rejected.fetch_add(1, Ordering::Relaxed);
            return Err(RateLimitError::TooManyRequests);
        }
        state.tokens.fetch_sub(1, Ordering::Relaxed);

        // Check per-method limit
        self.check_method_limit(&state, method)?;

        Ok(())
    }

    /// Refill tokens based on elapsed time
    fn refill_tokens(&self, state: &ConnectionState) {
        let mut last_refill = state.last_refill.write().unwrap();
        let now = Instant::now();
        let elapsed = now.duration_since(*last_refill);

        if elapsed >= Duration::from_millis(100) {
            // Refill tokens (rate per 100ms)
            let refill_rate = self.config.requests_per_second as u64 / 10;
            let periods = elapsed.as_millis() as u64 / 100;
            let new_tokens = periods * refill_rate;

            let current = state.tokens.load(Ordering::Relaxed);
            let max = self.config.burst_size as u64;
            state
                .tokens
                .store((current + new_tokens).min(max), Ordering::Relaxed);
            *last_refill = now;
        }
    }

    /// Check per-method rate limit
    fn check_method_limit(
        &self,
        state: &ConnectionState,
        method: &str,
    ) -> Result<(), RateLimitError> {
        let limit = self
            .config
            .method_limits
            .get(method)
            .copied()
            .unwrap_or(self.config.default_method_limit);

        let mut counts = state.method_counts.write().unwrap();
        let now = Instant::now();
        let window = Duration::from_secs(60);

        let (count, window_start) = counts.entry(method.to_string()).or_insert((0, now));

        // Reset window if expired
        if now.duration_since(*window_start) >= window {
            *count = 0;
            *window_start = now;
        }

        if *count >= limit {
            self.total_rejected.fetch_add(1, Ordering::Relaxed);
            return Err(RateLimitError::MethodLimitExceeded);
        }

        *count += 1;
        Ok(())
    }

    /// Track subscription count
    pub fn add_subscription(&self, addr: SocketAddr) -> Result<(), RateLimitError> {
        let state = self.get_connection_state(addr);
        let current = state.subscriptions.fetch_add(1, Ordering::Relaxed);

        if current >= self.config.max_subscriptions_per_connection as u64 {
            state.subscriptions.fetch_sub(1, Ordering::Relaxed);
            return Err(RateLimitError::TooManySubscriptions);
        }

        Ok(())
    }

    /// Remove subscription tracking
    pub fn remove_subscription(&self, addr: SocketAddr) {
        let state = self.get_connection_state(addr);
        state.subscriptions.fetch_sub(1, Ordering::Relaxed);
    }

    /// Get metrics snapshot
    pub fn metrics(&self) -> RateLimitMetrics {
        RateLimitMetrics {
            total_requests: self.total_requests.load(Ordering::Relaxed),
            total_rejected: self.total_rejected.load(Ordering::Relaxed),
            active_connections: self.connections.read().unwrap().len(),
        }
    }

    /// Cleanup stale connections (call periodically)
    pub fn cleanup_stale_connections(&self, max_age: Duration) {
        let mut conns = self.connections.write().unwrap();
        let now = Instant::now();

        conns.retain(|_, state| {
            let last = *state.last_refill.read().unwrap();
            now.duration_since(last) < max_age
        });
    }
}

/// Rate limit error types
#[derive(Debug, Clone)]
pub enum RateLimitError {
    /// Too many requests from a single source.
    TooManyRequests,
    /// Specific RPC method exceeded its rate cap.
    MethodLimitExceeded,
    /// Subscription count limit exceeded for the session.
    TooManySubscriptions,
    /// Temporarily banned due to repeated violations.
    Banned,
}

impl std::fmt::Display for RateLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooManyRequests => write!(f, "Rate limit exceeded"),
            Self::MethodLimitExceeded => write!(f, "Method rate limit exceeded"),
            Self::TooManySubscriptions => write!(f, "Too many active subscriptions"),
            Self::Banned => write!(f, "Temporarily banned due to rate limit violation"),
        }
    }
}

/// Metrics for monitoring
#[derive(Debug, Clone)]
pub struct RateLimitMetrics {
    /// Total RPC requests evaluated by the rate limiter.
    pub total_requests: u64,
    /// RPC requests rejected due to rate limiting.
    pub total_rejected: u64,
    /// Current number of active RPC connections.
    pub active_connections: usize,
}

/// CORS configuration for RPC server
#[derive(Clone, Debug)]
pub struct CorsConfig {
    /// Allowed origins (None = no CORS, Some(vec![]) = block all, Some(origins) = allow listed)
    pub allowed_origins: Option<Vec<String>>,
    /// Allow credentials
    pub allow_credentials: bool,
    /// Max age for preflight cache
    pub max_age: Option<u32>,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            // Default: only allow localhost for development
            allowed_origins: Some(vec![
                "http://localhost:3000".to_string(),
                "http://127.0.0.1:3000".to_string(),
                "https://explorer.atlas-sphere.io".to_string(),
                "https://dex.atlas-sphere.io".to_string(),
            ]),
            allow_credentials: false,
            max_age: Some(3600),
        }
    }
}

impl CorsConfig {
    /// Production config with strict origins
    pub fn production(allowed_origins: Vec<String>) -> Self {
        Self {
            allowed_origins: Some(allowed_origins),
            allow_credentials: false,
            max_age: Some(86400),
        }
    }

    /// Development config allowing localhost
    pub fn development() -> Self {
        Self::default()
    }

    /// Check if origin is allowed
    pub fn is_origin_allowed(&self, origin: &str) -> bool {
        match &self.allowed_origins {
            None => true,                                 // No CORS restrictions
            Some(origins) if origins.is_empty() => false, // Block all
            Some(origins) => origins.iter().any(|o| o == origin || o == "*"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_basic() {
        let limiter = RateLimiter::new(RateLimitConfig {
            requests_per_second: 10,
            burst_size: 5,
            ..Default::default()
        });

        let addr: SocketAddr = "127.0.0.1:12345".parse().unwrap();

        // First 5 requests should succeed (burst)
        for _ in 0..5 {
            assert!(limiter.check_request(addr, "test_method").is_ok());
        }

        // 6th should fail (exceeded burst)
        assert!(limiter.check_request(addr, "test_method").is_err());
    }

    #[test]
    fn test_cors_config() {
        let cors = CorsConfig::default();
        assert!(cors.is_origin_allowed("http://localhost:3000"));
        assert!(!cors.is_origin_allowed("http://evil.com"));
    }

    #[test]
    fn test_subscription_limits() {
        let limiter = RateLimiter::new(RateLimitConfig {
            max_subscriptions_per_connection: 2,
            ..Default::default()
        });

        let addr: SocketAddr = "127.0.0.1:12345".parse().unwrap();

        assert!(limiter.add_subscription(addr).is_ok());
        assert!(limiter.add_subscription(addr).is_ok());
        assert!(limiter.add_subscription(addr).is_err()); // 3rd should fail

        limiter.remove_subscription(addr);
        assert!(limiter.add_subscription(addr).is_ok()); // Now should work
    }
}
