//! Atlas Sphere DNS Server - Configuration Management
//!
//! Configuration handling for DNS server, API, database, and blockchain integration

use crate::error::{DnsError, DnsResult};
use config::Config;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub bind_address: SocketAddr,
    pub tcp_enabled: bool,
    pub udp_enabled: bool,
    pub max_connections: usize,
    pub connection_timeout: u64,
    pub query_timeout: u64,
    pub cache_size: usize,
    pub zone_transfer_enabled: bool,
    pub recursion_enabled: bool,
    pub dnssec_enabled: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0:8053".parse().unwrap(),
            tcp_enabled: true,
            udp_enabled: true,
            max_connections: 10000,
            connection_timeout: 30,
            query_timeout: 10,
            cache_size: 10000,
            zone_transfer_enabled: false,
            recursion_enabled: false,
            dnssec_enabled: true,
        }
    }
}

/// API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub bind_address: SocketAddr,
    pub enabled: bool,
    pub cors_enabled: bool,
    pub api_key_required: bool,
    pub rate_limit_enabled: bool,
    pub rate_limit_per_hour: u32,
    pub admin_endpoints_enabled: bool,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1:8080".parse().unwrap(),
            enabled: true,
            cors_enabled: true,
            api_key_required: false,
            rate_limit_enabled: true,
            rate_limit_per_hour: 1000,
            admin_endpoints_enabled: true,
        }
    }
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub connection_timeout: u64,
    pub enable_wal_mode: bool,
    pub enable_foreign_keys: bool,
    pub cache_size: i32,
    pub backup_enabled: bool,
    pub backup_interval: u64,
    pub backup_retention_days: u32,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "sqlite:./data/atlas_dns.db".to_string(),
            max_connections: 10,
            connection_timeout: 30,
            enable_wal_mode: true,
            enable_foreign_keys: true,
            cache_size: 2000,
            backup_enabled: true,
            backup_interval: 3600, // 1 hour
            backup_retention_days: 30,
        }
    }
}

/// Blockchain integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainConfig {
    pub enabled: bool,
    pub rpc_url: String,
    pub ws_url: String,
    pub chain_id: u32,
    pub registry_contract: String,
    pub domain_contract: String,
    pub private_key: Option<String>,
    pub poll_interval: u64,
    pub confirmations_required: u32,
    pub gas_price: Option<u64>,
    pub gas_limit: Option<u64>,
}

impl Default for BlockchainConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            rpc_url: "http://127.0.0.1:9944".to_string(),
            ws_url: "ws://127.0.0.1:9944".to_string(),
            chain_id: 42,
            registry_contract: "0x0000000000000000000000000000000000000000".to_string(),
            domain_contract: "0x0000000000000000000000000000000000000000".to_string(),
            private_key: None,
            poll_interval: 30,
            confirmations_required: 1,
            gas_price: None,
            gas_limit: None,
        }
    }
}

/// Zone configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneConfig {
    pub name: String,
    pub admin_email: String,
    pub refresh_interval: u32,
    pub retry_interval: u32,
    pub expire_interval: u32,
    pub minimum_ttl: u32,
    pub default_ttl: u32,
    pub authoritative_servers: Vec<String>,
    pub zone_file_path: Option<PathBuf>,
}

impl Default for ZoneConfig {
    fn default() -> Self {
        Self {
            name: "x3".to_string(),
            admin_email: "admin@atlas-sphere.io".to_string(),
            refresh_interval: 3600,
            retry_interval: 1800,
            expire_interval: 604800,
            minimum_ttl: 3600,
            default_ttl: 300,
            authoritative_servers: vec![
                "ns1.atlas-sphere.io".to_string(),
                "ns2.atlas-sphere.io".to_string(),
            ],
            zone_file_path: None,
        }
    }
}

/// DNSSEC configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsSecConfig {
    pub enabled: bool,
    pub algorithm: String,
    pub key_size: u32,
    pub rollover_interval: u32,
    pub signature_validity: u32,
    pub key_directory: PathBuf,
    pub zone_signing_enabled: bool,
    pub automatic_rollover: bool,
}

impl Default for DnsSecConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            algorithm: "RSASHA256".to_string(),
            key_size: 2048,
            rollover_interval: 2592000, // 30 days
            signature_validity: 604800, // 7 days
            key_directory: PathBuf::from("./keys"),
            zone_signing_enabled: true,
            automatic_rollover: true,
        }
    }
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub enabled: bool,
    pub max_size: usize,
    pub ttl_default: u32,
    pub ttl_max: u32,
    pub cleanup_interval: u32,
    pub compression_enabled: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_size: 10000,
            ttl_default: 300,
            ttl_max: 3600,
            cleanup_interval: 300,
            compression_enabled: false,
        }
    }
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub enable_rate_limiting: bool,
    pub rate_limit_per_ip: u32,
    pub rate_limit_window: u32,
    pub block_malicious_ips: bool,
    pub enable_query_logging: bool,
    pub log_retention_days: u32,
    pub enable_ddos_protection: bool,
    pub max_queries_per_second: u32,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_rate_limiting: true,
            rate_limit_per_ip: 100,
            rate_limit_window: 60,
            block_malicious_ips: false,
            enable_query_logging: false,
            log_retention_days: 7,
            enable_ddos_protection: false,
            max_queries_per_second: 1000,
        }
    }
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    pub enabled: bool,
    pub prometheus_enabled: bool,
    pub metrics_port: u16,
    pub health_check_enabled: bool,
    pub health_check_interval: u32,
    pub alert_webhook_url: Option<String>,
    pub performance_monitoring: bool,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            prometheus_enabled: true,
            metrics_port: 9090,
            health_check_enabled: true,
            health_check_interval: 30,
            alert_webhook_url: None,
            performance_monitoring: true,
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
    pub output: String,
    pub file_path: Option<PathBuf>,
    pub max_file_size: u64,
    pub max_files: u32,
    pub enable_structured_logging: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "json".to_string(),
            output: "stdout".to_string(),
            file_path: Some(PathBuf::from("./logs/dns-server.log")),
            max_file_size: 100 * 1024 * 1024, // 100MB
            max_files: 10,
            enable_structured_logging: true,
        }
    }
}

/// Main DNS Server Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsConfig {
    pub server: ServerConfig,
    pub api: ApiConfig,
    pub database: DatabaseConfig,
    pub blockchain: BlockchainConfig,
    pub zone: ZoneConfig,
    pub dnssec: DnsSecConfig,
    pub cache: CacheConfig,
    pub security: SecurityConfig,
    pub monitoring: MonitoringConfig,
    pub logging: LoggingConfig,
    pub custom_zones: HashMap<String, ZoneConfig>,
    pub environment: String,
}

impl DnsConfig {
    /// Load configuration from environment variables and files
    pub fn load_from_env() -> DnsResult<Self> {
        let mut config = Config::builder()
            // Add default configuration
            .add_source(config::Environment::with_prefix("ATLAS_DNS"))
            .add_source(config::File::with_name("config/dns-server"))
            .add_source(config::File::with_name("config/default"))
            .add_source(config::File::with_name("atlas-dns-config"))
            .set_default("server.bind_address", "0.0.0.0:53")?
            .set_default("server.tcp_enabled", true)?
            .set_default("server.udp_enabled", true)?
            .set_default("api.enabled", true)?
            .set_default("database.url", "sqlite:./data/atlas_dns.db")?
            .set_default("blockchain.enabled", false)?
            .set_default("zone.name", "x3")?
            .set_default("dnssec.enabled", true)?
            .set_default("cache.enabled", true)?
            .set_default("security.enable_rate_limiting", true)?
            .set_default("monitoring.enabled", true)?
            .set_default("logging.level", "info")?
            .set_default("environment", "development")?
            .build()?;

        let mut dns_config: DnsConfig = config
            .try_deserialize()
            .map_err(|e| DnsError::config(format!("Failed to deserialize config: {}", e)))?;

        // Apply environment-specific overrides
        dns_config.apply_environment_overrides();

        // Validate configuration
        dns_config.validate()?;

        Ok(dns_config)
    }

    /// Load configuration from file path
    pub fn load_from_file<P: AsRef<std::path::Path>>(path: P) -> DnsResult<Self> {
        let config = Config::builder()
            .add_source(config::File::with_name(path.as_ref().to_str().unwrap()))
            .add_source(config::Environment::with_prefix("ATLAS_DNS"))
            .build()?;

        let mut dns_config: DnsConfig = config
            .try_deserialize()
            .map_err(|e| DnsError::config(format!("Failed to deserialize config: {}", e)))?;

        dns_config.apply_environment_overrides();
        dns_config.validate()?;

        Ok(dns_config)
    }

    /// Save configuration to file
    pub fn save_to_file<P: AsRef<std::path::Path>>(&self, path: P) -> DnsResult<()> {
        use std::fs;

        // Create directory if it doesn't exist
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent).map_err(|e| {
                DnsError::config(format!("Failed to create config directory: {}", e))
            })?;
        }

        let config_str = serde_yaml::to_string(self)
            .map_err(|e| DnsError::config(format!("Failed to serialize config: {}", e)))?;

        fs::write(path, config_str)
            .map_err(|e| DnsError::config(format!("Failed to write config file: {}", e)))?;

        Ok(())
    }

    /// Apply environment-specific overrides
    fn apply_environment_overrides(&mut self) {
        let env = std::env::var("ATLAS_ENV").unwrap_or_else(|_| "development".to_string());
        self.environment = env.clone();

        match env.as_str() {
            "production" => {
                self.server.bind_address = "0.0.0.0:53".parse().unwrap();
                self.api.bind_address = "127.0.0.1:8080".parse().unwrap();
                self.security.enable_rate_limiting = true;
                self.logging.level = "warn".to_string();
                self.dnssec.enabled = true;
            }
            "development" => {
                self.server.bind_address = "127.0.0.1:5353".parse().unwrap();
                self.api.bind_address = "127.0.0.1:8080".parse().unwrap();
                self.security.enable_rate_limiting = false;
                self.logging.level = "debug".to_string();
                self.dnssec.enabled = false;
            }
            "testing" => {
                self.server.bind_address = "127.0.0.1:5353".parse().unwrap();
                self.api.bind_address = "127.0.0.1:8080".parse().unwrap();
                self.database.url = "sqlite::memory:".to_string();
                self.security.enable_rate_limiting = false;
                self.logging.level = "error".to_string();
                self.monitoring.enabled = false;
            }
            _ => {}
        }
    }

    /// Validate configuration values
    fn validate(&self) -> DnsResult<()> {
        // Validate server configuration
        if self.server.bind_address.port() == 0 {
            return Err(DnsError::config("Invalid server bind address port"));
        }

        if self.server.max_connections == 0 {
            return Err(DnsError::config("Max connections must be greater than 0"));
        }

        if self.server.query_timeout == 0 {
            return Err(DnsError::config("Query timeout must be greater than 0"));
        }

        // Validate API configuration
        if self.api.enabled && self.api.bind_address.port() == 0 {
            return Err(DnsError::config("Invalid API bind address port"));
        }

        // Validate database configuration
        if self.database.url.is_empty() {
            return Err(DnsError::config("Database URL cannot be empty"));
        }

        // Validate zone configuration
        if self.zone.name.is_empty() {
            return Err(DnsError::config("Zone name cannot be empty"));
        }

        if self.zone.admin_email.is_empty() {
            return Err(DnsError::config("Zone admin email cannot be empty"));
        }

        // Validate DNSSEC configuration
        if self.dnssec.enabled {
            if self.dnssec.key_size < 1024 {
                return Err(DnsError::config(
                    "DNSSEC key size must be at least 1024 bits",
                ));
            }
        }

        // Validate cache configuration
        if self.cache.enabled && self.cache.max_size == 0 {
            return Err(DnsError::config("Cache max size must be greater than 0"));
        }

        // Validate security configuration
        if self.security.enable_rate_limiting && self.security.rate_limit_per_ip == 0 {
            return Err(DnsError::config("Rate limit per IP must be greater than 0"));
        }

        Ok(())
    }

    /// Get configuration for specific environment
    pub fn for_environment(env: &str) -> DnsResult<Self> {
        let mut config = Self::default();
        config.environment = env.to_string();
        config.apply_environment_overrides();
        config.validate()?;
        Ok(config)
    }

    /// Check if configuration is valid for production
    pub fn is_production_ready(&self) -> bool {
        self.environment == "production"
            && self.dnssec.enabled
            && self.security.enable_rate_limiting
            && self.monitoring.enabled
            && self.server.bind_address.port() == 53
    }
}

impl Default for DnsConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            api: ApiConfig::default(),
            database: DatabaseConfig::default(),
            blockchain: BlockchainConfig::default(),
            zone: ZoneConfig::default(),
            dnssec: DnsSecConfig::default(),
            cache: CacheConfig::default(),
            security: SecurityConfig::default(),
            monitoring: MonitoringConfig::default(),
            logging: LoggingConfig::default(),
            custom_zones: HashMap::new(),
            environment: "development".to_string(),
        }
    }
}

/// Configuration builder for fluent configuration setup
pub struct DnsConfigBuilder {
    config: DnsConfig,
}

impl DnsConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: DnsConfig::default(),
        }
    }

    pub fn server(mut self, server: ServerConfig) -> Self {
        self.config.server = server;
        self
    }

    pub fn api(mut self, api: ApiConfig) -> Self {
        self.config.api = api;
        self
    }

    pub fn database(mut self, database: DatabaseConfig) -> Self {
        self.config.database = database;
        self
    }

    pub fn blockchain(mut self, blockchain: BlockchainConfig) -> Self {
        self.config.blockchain = blockchain;
        self
    }

    pub fn dnssec(mut self, dnssec: DnsSecConfig) -> Self {
        self.config.dnssec = dnssec;
        self
    }

    pub fn security(mut self, security: SecurityConfig) -> Self {
        self.config.security = security;
        self
    }

    pub fn build(self) -> DnsResult<DnsConfig> {
        self.config.validate()?;
        Ok(self.config)
    }
}
