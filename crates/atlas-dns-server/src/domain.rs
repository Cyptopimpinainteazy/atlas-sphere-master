//! Atlas Sphere DNS Server - Domain and DNS Record Management
//!
//! Core types for DNS records, domain names, and domain management

use crate::error::{DnsError, DnsResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::str::FromStr;
use trust_dns_proto::rr::{
    rdata, DNSClass as TrustDNSClass, Name, RData, Record, RecordType as TrustDnsRecordType,
};

/// DNS Class enum (serializable wrapper around trust_dns DNSClass)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DnsClass {
    IN,
    CS,
    CH,
    HS,
    NONE,
    ANY,
}

impl Default for DnsClass {
    fn default() -> Self {
        DnsClass::IN
    }
}

impl From<TrustDNSClass> for DnsClass {
    fn from(c: TrustDNSClass) -> Self {
        match c {
            TrustDNSClass::IN => DnsClass::IN,
            TrustDNSClass::CH => DnsClass::CH,
            TrustDNSClass::HS => DnsClass::HS,
            TrustDNSClass::NONE => DnsClass::NONE,
            TrustDNSClass::ANY => DnsClass::ANY,
            _ => DnsClass::IN,
        }
    }
}

impl From<DnsClass> for TrustDNSClass {
    fn from(c: DnsClass) -> Self {
        match c {
            DnsClass::IN => TrustDNSClass::IN,
            DnsClass::CS => TrustDNSClass::IN, // CS not directly supported, fallback to IN
            DnsClass::CH => TrustDNSClass::CH,
            DnsClass::HS => TrustDNSClass::HS,
            DnsClass::NONE => TrustDNSClass::NONE,
            DnsClass::ANY => TrustDNSClass::ANY,
        }
    }
}

/// Domain name with validation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DomainName(String);

impl DomainName {
    /// Create new domain name with validation
    pub fn new(name: impl Into<String>) -> DnsResult<Self> {
        let name = name.into();
        Self::validate_name(&name)?;
        Ok(Self(name.to_lowercase()))
    }

    /// Create from trust-dns Name
    pub fn from_name(name: &Name) -> DnsResult<Self> {
        let name_str = name.to_string();
        Self::new(name_str.trim_end_matches('.'))
    }

    /// Convert to trust-dns Name
    pub fn to_name(&self) -> DnsResult<Name> {
        Name::from_str(&format!("{}.", self.0))
            .map_err(|e| DnsError::invalid_domain_name(e.to_string()))
    }

    /// Get the domain name as string
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Check if domain is valid .x3 domain
    pub fn is_x3_domain(&self) -> bool {
        self.0.ends_with(".x3") || self.0 == "x3"
    }

    /// Get parent domain
    pub fn parent(&self) -> Option<Self> {
        if let Some(pos) = self.0.find('.') {
            if pos < self.0.len() - 1 {
                return Some(Self(self.0[pos + 1..].to_string()));
            }
        }
        None
    }

    /// Check if this is a subdomain of other
    pub fn is_subdomain_of(&self, other: &DomainName) -> bool {
        if other.as_str() == "x3" {
            return self.is_x3_domain();
        }
        self.0.len() > other.as_str().len() && self.0.ends_with(&format!(".{}", other.as_str()))
    }

    /// Validate domain name format
    fn validate_name(name: &str) -> DnsResult<()> {
        if name.is_empty() {
            return Err(DnsError::invalid_domain_name("Domain name cannot be empty"));
        }

        if name.len() > 253 {
            return Err(DnsError::invalid_domain_name(
                "Domain name too long (max 253 characters)",
            ));
        }

        if name.starts_with('.') || name.ends_with('.') {
            return Err(DnsError::invalid_domain_name(
                "Domain name cannot start or end with dot",
            ));
        }

        let parts: Vec<&str> = name.split('.').collect();
        if parts.is_empty() {
            return Err(DnsError::invalid_domain_name("Invalid domain name format"));
        }

        for part in parts {
            if part.is_empty() {
                return Err(DnsError::invalid_domain_name("Empty domain label"));
            }

            if part.len() > 63 {
                return Err(DnsError::invalid_domain_name(
                    "Domain label too long (max 63 characters)",
                ));
            }

            // Check label format (alphanumeric and hyphens, but not starting or ending with hyphen)
            let mut chars = part.chars();
            let first_char = chars.next().unwrap();
            let last_char = chars.next_back();

            if !first_char.is_ascii_alphanumeric() {
                return Err(DnsError::invalid_domain_name(format!(
                    "Invalid first character '{}' in domain label",
                    first_char
                )));
            }

            if let Some(last) = last_char {
                if !last.is_ascii_alphanumeric() {
                    return Err(DnsError::invalid_domain_name(format!(
                        "Invalid last character '{}' in domain label",
                        last
                    )));
                }
            }

            for c in part.chars() {
                if !c.is_ascii_alphanumeric() && c != '-' {
                    return Err(DnsError::invalid_domain_name(format!(
                        "Invalid character '{}' in domain label",
                        c
                    )));
                }
            }
        }

        Ok(())
    }
}

impl std::fmt::Display for DomainName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::ops::Deref for DomainName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// DNS Record Types supported by Atlas Sphere DNS
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DnsRecordType {
    A(Ipv4Addr),
    AAAA(Ipv6Addr),
    CNAME(String),
    MX(MxRecord),
    NS(String),
    TXT(String),
    SRV(SrvRecord),
    CAA(String), // Certification Authority Authorization
    HINFO(HInfoRecord),
}

impl DnsRecordType {
    /// Convert to trust-dns RecordType
    pub fn to_trust_dns_type(&self) -> TrustDnsRecordType {
        match self {
            DnsRecordType::A(_) => TrustDnsRecordType::A,
            DnsRecordType::AAAA(_) => TrustDnsRecordType::AAAA,
            DnsRecordType::CNAME(_) => TrustDnsRecordType::CNAME,
            DnsRecordType::MX(_) => TrustDnsRecordType::MX,
            DnsRecordType::NS(_) => TrustDnsRecordType::NS,
            DnsRecordType::TXT(_) => TrustDnsRecordType::TXT,
            DnsRecordType::SRV(_) => TrustDnsRecordType::SRV,
            DnsRecordType::CAA(_) => TrustDnsRecordType::CAA,
            DnsRecordType::HINFO(_) => TrustDnsRecordType::HINFO,
        }
    }

    /// Check if record type supports multiple values
    pub fn supports_multiple(&self) -> bool {
        !matches!(self, DnsRecordType::CNAME(_) | DnsRecordType::HINFO(_))
    }

    /// Get TTL recommendation for this record type
    pub fn recommended_ttl(&self) -> u32 {
        match self {
            DnsRecordType::A(_) | DnsRecordType::AAAA(_) => 300, // 5 minutes
            DnsRecordType::CNAME(_) => 3600,                     // 1 hour
            DnsRecordType::MX(_) => 3600,                        // 1 hour
            DnsRecordType::NS(_) => 86400,                       // 1 day
            DnsRecordType::TXT(_) => 3600,                       // 1 hour
            DnsRecordType::SRV(_) => 300,                        // 5 minutes
            DnsRecordType::CAA(_) => 86400,                      // 1 day
            DnsRecordType::HINFO(_) => 86400,                    // 1 day
        }
    }
}

/// MX (Mail Exchange) record
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MxRecord {
    pub priority: u16,
    pub exchange: DomainName,
}

impl MxRecord {
    /// Create new MX record
    pub fn new(priority: u16, exchange: DomainName) -> Self {
        Self { priority, exchange }
    }
}

/// SRV (Service) record
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SrvRecord {
    pub priority: u16,
    pub weight: u16,
    pub port: u16,
    pub target: DomainName,
}

impl SrvRecord {
    /// Create new SRV record
    pub fn new(priority: u16, weight: u16, port: u16, target: DomainName) -> Self {
        Self {
            priority,
            weight,
            port,
            target,
        }
    }
}

/// HINFO (Host Information) record
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HInfoRecord {
    pub cpu: String,
    pub os: String,
}

impl HInfoRecord {
    /// Create new HINFO record
    pub fn new(cpu: String, os: String) -> Self {
        Self { cpu, os }
    }
}

/// Domain Status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DomainStatus {
    Active,
    Pending,
    Expired,
    Suspended,
    Deleted,
}

impl std::fmt::Display for DomainStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DomainStatus::Active => write!(f, "Active"),
            DomainStatus::Pending => write!(f, "Pending"),
            DomainStatus::Expired => write!(f, "Expired"),
            DomainStatus::Suspended => write!(f, "Suspended"),
            DomainStatus::Deleted => write!(f, "Deleted"),
        }
    }
}

impl Default for DomainStatus {
    fn default() -> Self {
        DomainStatus::Active
    }
}

/// Complete DNS Record
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DnsRecord {
    pub domain: DomainName,
    pub record_type: DnsRecordType,
    pub ttl: u32,
    pub class: DnsClass,
    pub data: DnsRecordType,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub blockchain_verified: bool,
    pub signature: Option<String>,
}

impl DnsRecord {
    /// Create new DNS record
    pub fn new(
        domain: DomainName,
        record_type: DnsRecordType,
        ttl: Option<u32>,
        blockchain_verified: bool,
    ) -> Self {
        let now = chrono::Utc::now();
        let ttl = ttl.unwrap_or_else(|| record_type.recommended_ttl());

        Self {
            domain,
            record_type: record_type.clone(),
            ttl,
            class: DnsClass::IN,
            data: record_type,
            created_at: now,
            updated_at: now,
            blockchain_verified,
            signature: None,
        }
    }

    /// Create A record
    pub fn a(domain: DomainName, ip: Ipv4Addr, ttl: Option<u32>) -> Self {
        Self::new(domain, DnsRecordType::A(ip), ttl, false)
    }

    /// Create AAAA record
    pub fn aaaa(domain: DomainName, ip: Ipv6Addr, ttl: Option<u32>) -> Self {
        Self::new(domain, DnsRecordType::AAAA(ip), ttl, false)
    }

    /// Create CNAME record
    pub fn cname(domain: DomainName, target: String, ttl: Option<u32>) -> Self {
        Self::new(domain, DnsRecordType::CNAME(target), ttl, false)
    }

    /// Create MX record
    pub fn mx(domain: DomainName, priority: u16, exchange: DomainName, ttl: Option<u32>) -> Self {
        Self::new(
            domain,
            DnsRecordType::MX(MxRecord::new(priority, exchange)),
            ttl,
            false,
        )
    }

    /// Create NS record
    pub fn ns(domain: DomainName, nameserver: String, ttl: Option<u32>) -> Self {
        Self::new(domain, DnsRecordType::NS(nameserver), ttl, false)
    }

    /// Create TXT record
    pub fn txt(domain: DomainName, text: String, ttl: Option<u32>) -> Self {
        Self::new(domain, DnsRecordType::TXT(text), ttl, false)
    }

    /// Create SRV record
    pub fn srv(
        domain: DomainName,
        priority: u16,
        weight: u16,
        port: u16,
        target: DomainName,
        ttl: Option<u32>,
    ) -> Self {
        Self::new(
            domain,
            DnsRecordType::SRV(SrvRecord::new(priority, weight, port, target)),
            ttl,
            false,
        )
    }

    /// Convert to trust-dns Record
    pub fn to_trust_dns_record(&self) -> DnsResult<Record> {
        let mut record = Record::new();
        record
            .set_name(self.domain.to_name()?)
            .set_ttl(self.ttl)
            .set_dns_class(TrustDNSClass::from(self.class))
            .set_record_type(self.record_type.to_trust_dns_type());

        // Set record data based on type
        match &self.data {
            DnsRecordType::A(ip) => {
                record.set_data(Some(RData::A(rdata::A(*ip))));
            }
            DnsRecordType::AAAA(ip) => {
                record.set_data(Some(RData::AAAA(rdata::AAAA(*ip))));
            }
            DnsRecordType::CNAME(target) => {
                let target_name = Name::from_str(target)
                    .map_err(|e| DnsError::invalid_domain_name(e.to_string()))?;
                record.set_data(Some(RData::CNAME(rdata::CNAME(target_name))));
            }
            DnsRecordType::MX(mx) => {
                let exchange_name = mx.exchange.to_name()?;
                record.set_data(Some(RData::MX(rdata::MX::new(mx.priority, exchange_name))));
            }
            DnsRecordType::NS(nameserver) => {
                let ns_name = Name::from_str(nameserver)
                    .map_err(|e| DnsError::invalid_domain_name(e.to_string()))?;
                record.set_data(Some(RData::NS(rdata::NS(ns_name))));
            }
            DnsRecordType::TXT(text) => {
                record.set_data(Some(RData::TXT(rdata::TXT::new(vec![text.clone()]))));
            }
            DnsRecordType::SRV(srv) => {
                let target_name = srv.target.to_name()?;
                record.set_data(Some(RData::SRV(rdata::SRV::new(
                    srv.priority,
                    srv.weight,
                    srv.port,
                    target_name,
                ))));
            }
            DnsRecordType::CAA(caa) => {
                // Parse CAA record format: [flags] [tag] [value]
                let parts: Vec<&str> = caa.splitn(3, ' ').collect();
                if parts.len() >= 3 {
                    let flags = parts[0].parse::<u8>().unwrap_or(0);
                    let _tag = parts[1];
                    let value = parts[2];
                    // Use CAA::new_issue with proper types
                    let name = Name::from_str(value).ok();
                    record.set_data(Some(RData::CAA(rdata::CAA::new_issue(
                        flags & 0x80 != 0,
                        name,
                        vec![],
                    ))));
                } else {
                    return Err(DnsError::Validation(
                        "Invalid CAA record format".to_string(),
                    ));
                }
            }
            DnsRecordType::HINFO(hinfo) => {
                record.set_data(Some(RData::HINFO(rdata::HINFO::new(
                    hinfo.cpu.clone(),
                    hinfo.os.clone(),
                ))));
            }
        }

        Ok(record)
    }

    /// Update the record
    pub fn update(&mut self, new_data: DnsRecordType) -> DnsResult<()> {
        self.data = new_data;
        self.updated_at = chrono::Utc::now();
        Ok(())
    }

    /// Set blockchain verification
    pub fn set_blockchain_verified(&mut self, verified: bool) {
        self.blockchain_verified = verified;
    }

    /// Set signature
    pub fn set_signature(&mut self, signature: String) {
        self.signature = Some(signature);
    }

    /// Check if record is expired
    pub fn is_expired(&self) -> bool {
        // This is a simplified check - in production, you'd check against the zone's expire time
        false
    }

    /// Get record size estimate
    pub fn estimated_size(&self) -> usize {
        match &self.data {
            DnsRecordType::A(_) => 16,
            DnsRecordType::AAAA(_) => 28,
            DnsRecordType::CNAME(target) => 16 + target.len(),
            DnsRecordType::MX(mx) => 18 + mx.exchange.as_str().len(),
            DnsRecordType::NS(ns) => 16 + ns.len(),
            DnsRecordType::TXT(txt) => 16 + txt.len(),
            DnsRecordType::SRV(srv) => 20 + srv.target.as_str().len(),
            DnsRecordType::CAA(caa) => 16 + caa.len(),
            DnsRecordType::HINFO(hinfo) => 16 + hinfo.cpu.len() + hinfo.os.len(),
        }
    }
}

/// Domain Record - High-level domain information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DomainRecord {
    pub domain: DomainName,
    pub records: Vec<DnsRecord>,
    pub registered_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    pub owner_address: Option<String>,
    pub blockchain_verified: bool,
    pub status: DomainStatus,
    pub custom_data: HashMap<String, String>,
}

impl DomainRecord {
    /// Create new domain record
    pub fn new(domain: DomainName, owner_address: Option<String>) -> Self {
        let now = chrono::Utc::now();
        Self {
            domain,
            records: Vec::new(),
            registered_at: now,
            expires_at: None,
            owner_address,
            blockchain_verified: false,
            status: DomainStatus::Active,
            custom_data: HashMap::new(),
        }
    }

    /// Add DNS record
    pub fn add_record(&mut self, record: DnsRecord) -> DnsResult<()> {
        // Check for duplicate records
        let record_type = &record.record_type;

        if !record_type.supports_multiple() {
            // Remove existing records of this type
            let trust_type = record_type.to_trust_dns_type();
            self.records
                .retain(|r| r.record_type.to_trust_dns_type() != trust_type);
        }

        self.records.push(record);
        Ok(())
    }

    /// Get records by type
    pub fn get_records_by_type(&self, record_type: &DnsRecordType) -> Vec<&DnsRecord> {
        let trust_dns_type = record_type.to_trust_dns_type();
        self.records
            .iter()
            .filter(|r| r.record_type.to_trust_dns_type() == trust_dns_type)
            .collect()
    }

    /// Get single record by type (first match)
    pub fn get_record_by_type(&self, record_type: &DnsRecordType) -> Option<&DnsRecord> {
        let trust_dns_type = record_type.to_trust_dns_type();
        self.records
            .iter()
            .find(|r| r.record_type.to_trust_dns_type() == trust_dns_type)
    }

    /// Remove record
    pub fn remove_record(&mut self, record_index: usize) -> Option<DnsRecord> {
        if record_index < self.records.len() {
            Some(self.records.remove(record_index))
        } else {
            None
        }
    }
}
