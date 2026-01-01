//! Atlas Sphere DNS Server - Zone Management
//! 
//! DNS zone management for .x3 TLD and custom zones

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::config::{DnsConfig, ZoneConfig};
use crate::error::{DnsResult, DnsError};
use crate::domain::{DomainName, DomainRecord};
use log::{info, warn, error};

/// Zone Manager
pub struct ZoneManager {
    config: DnsConfig,
    zones: Arc<RwLock<HashMap<String, ManagedZone>>>,
}

#[derive(Debug, Clone)]
pub struct ManagedZone {
    pub config: ZoneConfig,
    pub domains: HashMap<DomainName, DomainRecord>,
    pub last_updated: std::time::SystemTime,
}

impl ZoneManager {
    /// Create new zone manager
    pub async fn new(config: DnsConfig) -> DnsResult<Self> {
        info!("🌐 Initializing Zone Manager...");
        
        let zones = Arc::new(RwLock::new(HashMap::new()));
        
        // Initialize .x3 zone
        let x3_zone = ManagedZone {
            config: config.zone.clone(),
            domains: HashMap::new(),
            last_updated: std::time::SystemTime::now(),
        };
        
        {
            let mut zones_mut = zones.write().await;
            zones_mut.insert("x3".to_string(), x3_zone);
        }
        
        info!("✅ Zone Manager initialized");
        Ok(Self { config, zones })
    }
    
    /// Get zone count
    pub async fn get_zone_count(&self) -> usize {
        let zones = self.zones.read().await;
        zones.len()
    }
    
    /// Add new zone
    pub async fn add_zone(&self, zone_config: ZoneConfig) -> DnsResult<()> {
        let mut zones = self.zones.write().await;
        let zone_name = zone_config.name.clone();
        
        let zone = ManagedZone {
            config: zone_config,
            domains: HashMap::new(),
            last_updated: std::time::SystemTime::now(),
        };
        
        zones.insert(zone.config.name.clone(), zone);
        info!("📍 Added zone: {}", zone_name);
        
        Ok(())
    }
    
    /// Get domain from zone
    pub async fn get_domain(&self, domain: &DomainName) -> DnsResult<Option<DomainRecord>> {
        let zones = self.zones.read().await;
        
        // Try to find in .x3 zone first
        if domain.is_x3_domain() {
            if let Some(zone) = zones.get("x3") {
                if let Some(domain_record) = zone.domains.get(domain) {
                    return Ok(Some(domain_record.clone()));
                }
            }
        }
        
        // Check other zones
        for zone in zones.values() {
            if let Some(domain_record) = zone.domains.get(domain) {
                return Ok(Some(domain_record.clone()));
            }
        }
        
        Ok(None)
    }
    
    /// Add domain to zone
    pub async fn add_domain(&self, zone_name: &str, domain_record: DomainRecord) -> DnsResult<()> {
        let mut zones = self.zones.write().await;
        let domain_name = domain_record.domain.clone();
        
        if let Some(zone) = zones.get_mut(zone_name) {
            zone.domains.insert(domain_name.clone(), domain_record);
            zone.last_updated = std::time::SystemTime::now();
            info!("📍 Added domain to zone {}: {}", zone_name, domain_name);
        } else {
            return Err(DnsError::zone_not_found(zone_name.to_string()));
        }
        
        Ok(())
    }
    
    /// List all zones
    pub async fn list_zones(&self) -> Vec<String> {
        let zones = self.zones.read().await;
        zones.keys().cloned().collect()
    }
    
    /// Get zone statistics
    pub async fn get_zone_stats(&self) -> HashMap<String, ZoneStats> {
        let zones = self.zones.read().await;
        let mut stats = HashMap::new();
        
        for (name, zone) in zones.iter() {
            stats.insert(name.clone(), ZoneStats {
                domain_count: zone.domains.len(),
                last_updated: zone.last_updated,
            });
        }
        
        stats
    }
}

#[derive(Debug, Clone)]
pub struct ZoneStats {
    pub domain_count: usize,
    pub last_updated: std::time::SystemTime,
}
