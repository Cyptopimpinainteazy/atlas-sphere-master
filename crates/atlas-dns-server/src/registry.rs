//! Atlas Sphere DNS Server - Domain Registry
//! 
//! Domain registration and management system with blockchain integration

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::config::DnsConfig;
use crate::error::{DnsResult, DnsError};
use crate::domain::{DomainName, DomainRecord, DomainStatus};
use crate::blockchain::BlockchainClient;
use log::{info, warn, error};

/// Domain Registry
pub struct DomainRegistry {
    config: DnsConfig,
    domains: Arc<RwLock<HashMap<DomainName, DomainRecord>>>,
    blockchain_client: Option<BlockchainClient>,
}

impl DomainRegistry {
    /// Create new domain registry
    pub async fn new(config: DnsConfig) -> DnsResult<Self> {
        info!("📋 Initializing Domain Registry...");
        
        let blockchain_client = if config.blockchain.enabled {
            Some(BlockchainClient::new(config.clone()).await?)
        } else {
            None
        };
        
        let domains = Arc::new(RwLock::new(HashMap::new()));
        
        info!("✅ Domain Registry initialized");
        Ok(Self {
            config,
            domains,
            blockchain_client,
        })
    }
    
    /// Register new domain
    pub async fn register_domain(&mut self, domain_record: DomainRecord) -> DnsResult<()> {
        let domain_name = domain_record.domain.clone();
        
        // Validate domain name
        if !domain_name.is_x3_domain() {
            return Err(DnsError::invalid_domain_name(
                "Only .x3 domains are supported".to_string()
            ));
        }
        
        // Check if domain already exists
        let mut domains = self.domains.write().await;
        if domains.contains_key(&domain_name) {
            return Err(DnsError::domain_not_found(
                format!("Domain {} already registered", domain_name)
            ));
        }
        
        // Register on blockchain if enabled
        if let Some(ref client) = self.blockchain_client {
            if let Some(ref owner) = domain_record.owner_address {
                client.register_domain(&domain_name, owner).await?;
            }
        }
        
        // Store domain
        domains.insert(domain_name.clone(), domain_record);
        info!("📋 Registered domain: {}", domain_name);
        
        Ok(())
    }
    
    /// Get domain by name
    pub async fn get_domain(&self, domain_name: &DomainName) -> DnsResult<Option<DomainRecord>> {
        let domains = self.domains.read().await;
        Ok(domains.get(domain_name).cloned())
    }
    
    /// Update domain record
    pub async fn update_domain(&self, domain_name: &DomainName, updated_record: DomainRecord) -> DnsResult<()> {
        let mut domains = self.domains.write().await;
        
        if domains.contains_key(domain_name) {
            domains.insert(domain_name.clone(), updated_record);
            info!("📋 Updated domain: {}", domain_name);
        } else {
            return Err(DnsError::domain_not_found(domain_name.to_string()));
        }
        
        Ok(())
    }
    
    /// Delete domain
    pub async fn delete_domain(&self, domain_name: &DomainName) -> DnsResult<()> {
        let mut domains = self.domains.write().await;
        
        if let Some(domain_record) = domains.remove(domain_name) {
            // Remove from blockchain if enabled
            if let Some(ref client) = self.blockchain_client {
                if let Some(ref owner) = domain_record.owner_address {
                    client.delete_domain(domain_name, owner).await?;
                }
            }
            
            info!("📋 Deleted domain: {}", domain_name);
        } else {
            return Err(DnsError::domain_not_found(domain_name.to_string()));
        }
        
        Ok(())
    }
    
    /// List all domains
    pub async fn list_domains(&self) -> Vec<DomainName> {
        let domains = self.domains.read().await;
        domains.keys().cloned().collect()
    }
    
    /// Get domain count
    pub async fn get_domain_count(&self) -> usize {
        let domains = self.domains.read().await;
        domains.len()
    }
    
    /// Search domains by pattern
    pub async fn search_domains(&self, pattern: &str) -> DnsResult<Vec<DomainRecord>> {
        let domains = self.domains.read().await;
        let pattern = pattern.to_lowercase();
        
        let mut results = Vec::new();
        for domain_record in domains.values() {
            if domain_record.domain.as_str().contains(&pattern) {
                results.push(domain_record.clone());
            }
        }
        
        Ok(results)
    }
    
    /// Verify domain ownership on blockchain
    pub async fn verify_domain_ownership(&self, domain_name: &DomainName, owner_address: &str) -> DnsResult<bool> {
        if let Some(ref client) = self.blockchain_client {
            return client.verify_ownership(domain_name, owner_address).await;
        }
        
        // If blockchain is not enabled, assume ownership verification passes
        Ok(true)
    }
    
    /// Get domains by owner address
    pub async fn get_domains_by_owner(&self, owner_address: &str) -> Vec<DomainRecord> {
        let domains = self.domains.read().await;
        let mut results = Vec::new();
        
        for domain_record in domains.values() {
            if domain_record.owner_address.as_ref() == Some(&owner_address.to_string()) {
                results.push(domain_record.clone());
            }
        }
        
        results
    }
    
    /// Update domain status
    pub async fn update_domain_status(&self, domain_name: &DomainName, status: DomainStatus) -> DnsResult<()> {
        let mut domains = self.domains.write().await;
        
        if let Some(domain_record) = domains.get_mut(domain_name) {
            let status_debug = format!("{:?}", status);
            domain_record.status = status;
            info!("📋 Updated domain {} status to {}", domain_name, status_debug);
        } else {
            return Err(DnsError::domain_not_found(domain_name.to_string()));
        }
        
        Ok(())
    }
    
    /// Get registry statistics
    pub async fn get_registry_stats(&self) -> RegistryStats {
        let domains = self.domains.read().await;
        
        let mut stats = RegistryStats {
            total_domains: 0,
            active_domains: 0,
            pending_domains: 0,
            expired_domains: 0,
            blockchain_verified: 0,
        };
        
        for domain_record in domains.values() {
            stats.total_domains += 1;
            
            match domain_record.status {
                DomainStatus::Active => stats.active_domains += 1,
                DomainStatus::Pending => stats.pending_domains += 1,
                DomainStatus::Expired => stats.expired_domains += 1,
                _ => {}
            }
            
            if domain_record.blockchain_verified {
                stats.blockchain_verified += 1;
            }
        }
        
        stats
    }
}

#[derive(Debug, Clone)]
pub struct RegistryStats {
    pub total_domains: usize,
    pub active_domains: usize,
    pub pending_domains: usize,
    pub expired_domains: usize,
    pub blockchain_verified: usize,
}
