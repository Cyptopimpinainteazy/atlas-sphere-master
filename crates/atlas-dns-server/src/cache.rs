//! Atlas Sphere DNS Server - DNS Cache
//! 
//! High-performance DNS response caching for improved performance

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::config::DnsConfig;
use crate::error::DnsResult;
use log::debug;

/// DNS Cache
pub struct DnsCache {
    config: DnsConfig,
    cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
    stats: Arc<RwLock<CacheStats>>,
}

/// Cached DNS response data  
#[derive(Debug, Clone)]
pub struct CachedResponse {
    pub data: Vec<u8>,
    pub record_count: usize,
}

#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub response: CachedResponse,
    pub expires_at: std::time::SystemTime,
    pub created_at: std::time::SystemTime,
    pub hits: u64,
}

#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub total_entries: u64,
    pub size_bytes: u64,
}

impl DnsCache {
    /// Create new DNS cache
    pub async fn new(config: DnsConfig) -> DnsResult<Self> {
        if !config.cache.enabled {
            debug!("🚫 DNS cache is disabled");
            return Ok(Self {
                config,
                cache: Arc::new(RwLock::new(HashMap::new())),
                stats: Arc::new(RwLock::new(CacheStats::default())),
            });
        }
        
        debug!("💾 Initializing DNS cache (max size: {} entries)", config.cache.max_size);
        
        let cache = Arc::new(RwLock::new(HashMap::new()));
        let stats = Arc::new(RwLock::new(CacheStats::default()));
        
        // Start cleanup task
        Self::start_cleanup_task(cache.clone(), stats.clone(), config.clone());
        
        Ok(Self {
            config,
            cache,
            stats,
        })
    }
    
    /// Get cached response
    pub async fn get(&self, key: &str) -> DnsResult<Option<CachedResponse>> {
        if !self.config.cache.enabled {
            return Ok(None);
        }
        
        let mut cache = self.cache.write().await;
        
        if let Some(entry) = cache.get_mut(key) {
            // Check if entry is expired
            if std::time::SystemTime::now() > entry.expires_at {
                cache.remove(key);
                self.update_stats(|stats| stats.misses += 1).await;
                return Ok(None);
            }
            
            // Update hit count and return response
            entry.hits += 1;
            self.update_stats(|stats| stats.hits += 1).await;
            
            debug!("📦 Cache hit for key: {}", key);
            return Ok(Some(entry.response.clone()));
        }
        
        self.update_stats(|stats| stats.misses += 1).await;
        debug!("📦 Cache miss for key: {}", key);
        Ok(None)
    }
    
    /// Set cached response
    pub async fn set(&self, key: &str, response: CachedResponse, ttl: u32) -> DnsResult<()> {
        if !self.config.cache.enabled {
            return Ok(());
        }
        
        let mut cache = self.cache.write().await;
        
        // Check cache size limit
        if cache.len() >= self.config.cache.max_size {
            self.evict_oldest(&mut cache).await?;
        }
        
        let now = std::time::SystemTime::now();
        let expires_at = now + std::time::Duration::from_secs(ttl as u64);
        
        let entry = CacheEntry {
            response,
            expires_at,
            created_at: now,
            hits: 0,
        };
        
        cache.insert(key.to_string(), entry);
        self.update_stats(|stats| {
            stats.total_entries = cache.len() as u64;
        }).await;
        
        debug!("💾 Cached response with TTL: {} seconds", ttl);
        Ok(())
    }
    
    /// Remove entry from cache
    pub async fn remove(&self, key: &str) -> DnsResult<()> {
        if !self.config.cache.enabled {
            return Ok(());
        }
        
        let mut cache = self.cache.write().await;
        cache.remove(key);
        self.update_stats(|stats| {
            stats.total_entries = cache.len() as u64;
        }).await;
        
        Ok(())
    }
    
    /// Clear entire cache
    pub async fn clear(&self) -> DnsResult<()> {
        let mut cache = self.cache.write().await;
        cache.clear();
        self.update_stats(|stats| {
            stats.total_entries = 0;
            stats.hits = 0;
            stats.misses = 0;
        }).await;
        
        debug!("🗑️  Cleared DNS cache");
        Ok(())
    }
    
    /// Get cache statistics
    pub async fn get_stats(&self) -> CacheStats {
        let stats = self.stats.read().await;
        stats.clone()
    }
    
    /// Start background cleanup task
    fn start_cleanup_task(
        cache: Arc<RwLock<HashMap<String, CacheEntry>>>,
        stats: Arc<RwLock<CacheStats>>,
        config: DnsConfig,
    ) {
        let interval = std::time::Duration::from_secs(config.cache.cleanup_interval as u64);
        
        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);
            
            loop {
                interval_timer.tick().await;
                Self::cleanup_expired(&cache, &stats).await;
            }
        });
    }
    
    /// Cleanup expired entries
    async fn cleanup_expired(
        cache: &Arc<RwLock<HashMap<String, CacheEntry>>>,
        stats: &Arc<RwLock<CacheStats>>,
    ) {
        let now = std::time::SystemTime::now();
        let mut cache_mut = cache.write().await;
        let mut stats_mut = stats.write().await;
        
        let before_count = cache_mut.len();
        
        // Remove expired entries
        cache_mut.retain(|_, entry| now < entry.expires_at);
        
        let removed = before_count - cache_mut.len();
        if removed > 0 {
            stats_mut.evictions += removed as u64;
            stats_mut.total_entries = cache_mut.len() as u64;
            debug!("🧹 Cleaned up {} expired cache entries", removed);
        }
    }
    
    /// Evict oldest entry when cache is full
    async fn evict_oldest(&self, cache: &mut HashMap<String, CacheEntry>) -> DnsResult<()> {
        let oldest_key = cache
            .iter()
            .min_by_key(|(_, entry)| entry.created_at)
            .map(|(key, _)| key.clone());
            
        if let Some(key) = oldest_key {
            cache.remove(&key);
            self.update_stats(|stats| stats.evictions += 1).await;
            debug!("📦 Evicted oldest cache entry: {}", key);
        }
        
        Ok(())
    }
    
    /// Update cache statistics
    async fn update_stats<F>(&self, updater: F)
    where
        F: FnOnce(&mut CacheStats),
    {
        let mut stats = self.stats.write().await;
        updater(&mut stats);
    }
    
    /// Get cache size
    pub async fn size(&self) -> usize {
        let cache = self.cache.read().await;
        cache.len()
    }
    
    /// Check if cache is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.cache.enabled
    }
    
    /// Get cache hit rate
    pub async fn hit_rate(&self) -> f64 {
        let stats = self.stats.read().await;
        let total = stats.hits + stats.misses;
        if total > 0 {
            stats.hits as f64 / total as f64
        } else {
            0.0
        }
    }
}
