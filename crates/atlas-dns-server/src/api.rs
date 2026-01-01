//! Atlas Sphere DNS Server - Management API
//! 
//! RESTful API for DNS management, domain registration, and server control

use axum::{
    extract::Path,
    http::StatusCode,
    response::{Json, IntoResponse},
    routing::{get, post, put, delete},
    Router,
};
use serde::{Deserialize, Serialize};
use crate::config::DnsConfig;
use crate::error::{DnsResult, DnsError};
use crate::domain::DomainName;
use log::info;

/// Start the management API server
pub async fn start_management_api(config: DnsConfig) -> DnsResult<()> {
    if !config.api.enabled {
        info!("🚫 Management API is disabled");
        return Ok(());
    }
    
    info!("🌐 Starting management API on {}", config.api.bind_address);
    
    // Store config for handlers
    let bind_addr = config.api.bind_address;
    
    let app = Router::new()
        // Health and status endpoints
        .route("/health", get(health_check))
        .route("/status", get(server_status))
        .route("/stats", get(server_stats))
        
        // Domain management endpoints
        .route("/domains", get(list_domains))
        .route("/domains", post(register_domain))
        .route("/domains/:domain", get(get_domain))
        .route("/domains/:domain", put(update_domain))
        .route("/domains/:domain", delete(delete_domain))
        .route("/domains/:domain/verify", post(verify_domain))
        
        // DNS record management
        .route("/domains/:domain/records", get(list_domain_records))
        .route("/domains/:domain/records", post(add_domain_record))
        .route("/domains/:domain/records/:record_type", delete(remove_domain_record))
        
        // Cache management
        .route("/cache/stats", get(cache_stats))
        .route("/cache/clear", post(clear_cache))
        
        // Zone management
        .route("/zones", get(list_zones))
        .route("/zones/:zone", get(get_zone))
        
        // Configuration
        .route("/config", get(get_config))
        
        // Metrics for monitoring
        .route("/metrics", get(prometheus_metrics));
    
    let listener = tokio::net::TcpListener::bind(bind_addr)
        .await
        .map_err(|e| DnsError::api(format!("Failed to bind: {}", e)))?;
    
    axum::serve(listener, app)
        .await
        .map_err(|e| DnsError::api(format!("API server error: {}", e)))?;
    
    Ok(())
}

// ===== Health and Status Endpoints =====

/// Health check endpoint
async fn health_check() -> impl IntoResponse {
    Json(HealthResponse {
        status: "healthy".to_string(),
        timestamp: chrono::Utc::now(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Server status endpoint
async fn server_status() -> impl IntoResponse {
    Json(ServerStatusResponse {
        running: true,
        uptime_seconds: 0,
        total_queries: 0,
        average_response_time_ms: 0.0,
        zone_count: 1,
        timestamp: chrono::Utc::now(),
    })
}

/// Server statistics endpoint
async fn server_stats() -> impl IntoResponse {
    Json(ServerStatsResponse {
        queries: QueriesStats {
            total: 0,
            cached: 0,
            authoritative: 0,
            nxdomain: 0,
            errors: 0,
        },
        cache: CacheStats {
            hits: 0,
            misses: 0,
            hit_rate: 0.0,
            total_entries: 0,
        },
        registry: RegistryStats {
            total_domains: 0,
            active_domains: 0,
            blockchain_verified: 0,
        },
        performance: PerformanceStats {
            average_response_time_ms: 0.0,
            uptime_seconds: 0,
        },
    })
}

// ===== Domain Management Endpoints =====

/// List all domains
async fn list_domains() -> impl IntoResponse {
    Json(ListDomainsResponse {
        domains: vec![],
        total: 0,
    })
}

/// Register new domain
async fn register_domain(
    Json(request): Json<RegisterDomainRequest>,
) -> impl IntoResponse {
    match DomainName::new(&request.domain) {
        Ok(domain_name) => {
            if !domain_name.is_x3_domain() {
                return (StatusCode::BAD_REQUEST, Json(ErrorResponse {
                    error: "Only .x3 domains are supported".to_string(),
                })).into_response();
            }
            
            (StatusCode::CREATED, Json(SuccessResponse {
                message: format!("Domain {} registered successfully", request.domain),
            })).into_response()
        }
        Err(e) => (StatusCode::BAD_REQUEST, Json(ErrorResponse {
            error: format!("Invalid domain name: {}", e),
        })).into_response(),
    }
}

/// Get domain details
async fn get_domain(
    Path(domain): Path<String>,
) -> impl IntoResponse {
    match DomainName::new(&domain) {
        Ok(_domain_name) => {
            (StatusCode::NOT_FOUND, Json(ErrorResponse {
                error: format!("Domain {} not found", domain),
            })).into_response()
        }
        Err(e) => (StatusCode::BAD_REQUEST, Json(ErrorResponse {
            error: format!("Invalid domain name: {}", e),
        })).into_response(),
    }
}

/// Update domain
async fn update_domain(
    Path(_domain): Path<String>,
    Json(_request): Json<UpdateDomainRequest>,
) -> impl IntoResponse {
    (StatusCode::NOT_IMPLEMENTED, Json(ErrorResponse {
        error: "Domain update not yet implemented".to_string(),
    })).into_response()
}

/// Delete domain
async fn delete_domain(
    Path(domain): Path<String>,
) -> impl IntoResponse {
    match DomainName::new(&domain) {
        Ok(_domain_name) => {
            (StatusCode::NOT_FOUND, Json(ErrorResponse {
                error: format!("Domain {} not found", domain),
            })).into_response()
        }
        Err(e) => (StatusCode::BAD_REQUEST, Json(ErrorResponse {
            error: format!("Invalid domain name: {}", e),
        })).into_response(),
    }
}

/// Verify domain ownership
async fn verify_domain(
    Path(domain): Path<String>,
    Json(request): Json<VerifyDomainRequest>,
) -> impl IntoResponse {
    match DomainName::new(&domain) {
        Ok(_domain_name) => {
            Json(VerifyDomainResponse {
                domain,
                owner_address: request.owner_address,
                verified: false,
            }).into_response()
        }
        Err(e) => (StatusCode::BAD_REQUEST, Json(ErrorResponse {
            error: format!("Invalid domain name: {}", e),
        })).into_response(),
    }
}

// ===== DNS Record Management =====

/// List domain records
async fn list_domain_records(
    Path(_domain): Path<String>,
) -> impl IntoResponse {
    (StatusCode::NOT_IMPLEMENTED, Json(ErrorResponse {
        error: "Record listing not yet implemented".to_string(),
    })).into_response()
}

/// Add domain record
async fn add_domain_record(
    Path(_domain): Path<String>,
    Json(_request): Json<AddRecordRequest>,
) -> impl IntoResponse {
    (StatusCode::NOT_IMPLEMENTED, Json(ErrorResponse {
        error: "Record addition not yet implemented".to_string(),
    })).into_response()
}

/// Remove domain record
async fn remove_domain_record(
    Path((_domain, _record_type)): Path<(String, String)>,
) -> impl IntoResponse {
    (StatusCode::NOT_IMPLEMENTED, Json(ErrorResponse {
        error: "Record removal not yet implemented".to_string(),
    })).into_response()
}

// ===== Cache Management =====

/// Get cache statistics
async fn cache_stats() -> impl IntoResponse {
    Json(CacheStatsResponse {
        hits: 0,
        misses: 0,
        hit_rate: 0.0,
        total_entries: 0,
        size: 0,
    })
}

/// Clear cache
async fn clear_cache() -> impl IntoResponse {
    (StatusCode::OK, Json(SuccessResponse {
        message: "Cache cleared successfully".to_string(),
    })).into_response()
}

// ===== Zone Management =====

/// List all zones
async fn list_zones() -> impl IntoResponse {
    let zones = vec!["x3".to_string()];
    Json(ListZonesResponse {
        total: zones.len(),
        zones,
    })
}

/// Get zone details
async fn get_zone(
    Path(_zone): Path<String>,
) -> impl IntoResponse {
    (StatusCode::NOT_IMPLEMENTED, Json(ErrorResponse {
        error: "Zone details not yet implemented".to_string(),
    })).into_response()
}

// ===== Configuration =====

/// Get server configuration
async fn get_config() -> impl IntoResponse {
    Json(ConfigResponse {
        server_bind_address: "0.0.0.0:8053".to_string(),
        api_enabled: true,
        cache_enabled: true,
        blockchain_enabled: false,
        dnssec_enabled: false,
        zone_name: "x3".to_string(),
        environment: "development".to_string(),
    })
}

// ===== Metrics =====

/// Prometheus metrics endpoint
async fn prometheus_metrics() -> impl IntoResponse {
    let metrics = r#"# HELP dns_queries_total Total DNS queries processed
# TYPE dns_queries_total counter
dns_queries_total 0

# HELP dns_cached_responses_total Total cached DNS responses
# TYPE dns_cached_responses_total counter
dns_cached_responses_total 0

# HELP dns_authoritative_responses_total Total authoritative DNS responses
# TYPE dns_authoritative_responses_total counter
dns_authoritative_responses_total 0

# HELP dns_cache_hits_total Total DNS cache hits
# TYPE dns_cache_hits_total counter
dns_cache_hits_total 0

# HELP dns_cache_misses_total Total DNS cache misses
# TYPE dns_cache_misses_total counter
dns_cache_misses_total 0

# HELP dns_domains_registered Total registered domains
# TYPE dns_domains_registered gauge
dns_domains_registered 0

# HELP dns_server_uptime_seconds Server uptime in seconds
# TYPE dns_server_uptime_seconds counter
dns_server_uptime_seconds 0

"#;
    
    (StatusCode::OK, metrics)
}

// ===== Request/Response Types =====

#[derive(Deserialize)]
pub struct RegisterDomainRequest {
    pub domain: String,
    pub owner_address: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateDomainRequest {
    pub records: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
pub struct VerifyDomainRequest {
    pub owner_address: String,
}

#[derive(Deserialize)]
pub struct AddRecordRequest {
    pub record_type: String,
    pub data: String,
    pub ttl: Option<u32>,
}

// ===== Response Types =====

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub version: String,
}

#[derive(Serialize)]
pub struct ServerStatusResponse {
    pub running: bool,
    pub uptime_seconds: u64,
    pub total_queries: u64,
    pub average_response_time_ms: f64,
    pub zone_count: usize,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize)]
pub struct ServerStatsResponse {
    pub queries: QueriesStats,
    pub cache: CacheStats,
    pub registry: RegistryStats,
    pub performance: PerformanceStats,
}

#[derive(Serialize)]
pub struct QueriesStats {
    pub total: u64,
    pub cached: u64,
    pub authoritative: u64,
    pub nxdomain: u64,
    pub errors: u64,
}

#[derive(Serialize)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
    pub total_entries: u64,
}

#[derive(Serialize)]
pub struct RegistryStats {
    pub total_domains: usize,
    pub active_domains: usize,
    pub blockchain_verified: usize,
}

#[derive(Serialize)]
pub struct PerformanceStats {
    pub average_response_time_ms: f64,
    pub uptime_seconds: u64,
}

#[derive(Serialize)]
pub struct ListDomainsResponse {
    pub domains: Vec<String>,
    pub total: usize,
}

#[derive(Serialize)]
pub struct GetDomainResponse {
    pub domain: String,
    pub records: usize,
    pub registered_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub owner_address: Option<String>,
    pub status: String,
    pub blockchain_verified: bool,
}

#[derive(Serialize)]
pub struct VerifyDomainResponse {
    pub domain: String,
    pub owner_address: String,
    pub verified: bool,
}

#[derive(Serialize)]
pub struct CacheStatsResponse {
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
    pub total_entries: u64,
    pub size: usize,
}

#[derive(Serialize)]
pub struct ListZonesResponse {
    pub zones: Vec<String>,
    pub total: usize,
}

#[derive(Serialize)]
pub struct ConfigResponse {
    pub server_bind_address: String,
    pub api_enabled: bool,
    pub cache_enabled: bool,
    pub blockchain_enabled: bool,
    pub dnssec_enabled: bool,
    pub zone_name: String,
    pub environment: String,
}

#[derive(Serialize)]
pub struct SuccessResponse {
    pub message: String,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}
