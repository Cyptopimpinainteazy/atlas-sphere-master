//! Configuration for the gateway.

use crate::error::Result;
use serde::Deserialize;

/// Gateway configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct GatewayConfig {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub cors: CorsConfig,
}

/// HTTP server configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

/// Database configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
    #[serde(default = "default_min_connections")]
    pub min_connections: u32,
}

/// CORS configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct CorsConfig {
    #[serde(default = "default_cors_origins")]
    pub allowed_origins: Vec<String>,
    #[serde(default = "default_cors_methods")]
    pub allowed_methods: Vec<String>,
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    8080
}

fn default_max_connections() -> u32 {
    10
}

fn default_min_connections() -> u32 {
    1
}

fn default_cors_origins() -> Vec<String> {
    vec!["*".to_string()]
}

fn default_cors_methods() -> Vec<String> {
    vec!["GET".to_string(), "POST".to_string(), "OPTIONS".to_string()]
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: default_host(),
                port: default_port(),
            },
            database: DatabaseConfig {
                url: "postgres://gateway:gateway@localhost:5432/atlas_indexer".to_string(),
                max_connections: default_max_connections(),
                min_connections: default_min_connections(),
            },
            cors: CorsConfig {
                allowed_origins: default_cors_origins(),
                allowed_methods: default_cors_methods(),
            },
        }
    }
}

impl GatewayConfig {
    /// Load configuration from a file.
    pub fn load(path: &str) -> Result<Self> {
        let settings = config::Config::builder()
            .add_source(config::File::with_name(path))
            .add_source(config::Environment::with_prefix("GATEWAY"))
            .build()
            .map_err(|e| crate::error::GatewayError::Config(e.to_string()))?;

        settings
            .try_deserialize()
            .map_err(|e| crate::error::GatewayError::Config(e.to_string()))
    }
}
