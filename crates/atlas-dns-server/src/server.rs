//! Atlas Sphere DNS Server - Core Server Implementation
//! 
//! Main DNS server that handles queries and serves .x3 domains
//! Using a simple UDP-based DNS server without the complex Server trait API

use std::sync::Arc;
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tokio::sync::RwLock;
use trust_dns_proto::op::{Message, MessageType, OpCode, ResponseCode};
use trust_dns_proto::rr::{RecordType, Record, Name, RData, rdata};
use trust_dns_proto::serialize::binary::{BinDecodable, BinEncodable};
use crate::config::DnsConfig;
use crate::error::{DnsResult, DnsError};
use crate::domain::{DomainName, DomainRecord, DnsRecordType};
use crate::zone::ZoneManager;
use crate::cache::DnsCache;
use crate::registry::DomainRegistry;
use log::{info, warn, error, debug};

/// Main Atlas DNS Server
pub struct AtlasDnsServer {
    config: DnsConfig,
    zone_manager: Arc<RwLock<ZoneManager>>,
    domain_registry: Arc<RwLock<DomainRegistry>>,
    cache: Arc<RwLock<DnsCache>>,
    running: Arc<RwLock<bool>>,
    stats: Arc<RwLock<ServerStats>>,
}

/// DNS Server Statistics
#[derive(Debug, Clone)]
pub struct ServerStats {
    pub total_queries: u64,
    pub cached_responses: u64,
    pub authoritative_responses: u64,
    pub nxdomain_responses: u64,
    pub error_responses: u64,
    pub tcp_connections: u64,
    pub udp_packets: u64,
    pub average_response_time_ms: f64,
    pub start_time: std::time::SystemTime,
}

impl Default for ServerStats {
    fn default() -> Self {
        Self {
            total_queries: 0,
            cached_responses: 0,
            authoritative_responses: 0,
            nxdomain_responses: 0,
            error_responses: 0,
            tcp_connections: 0,
            udp_packets: 0,
            average_response_time_ms: 0.0,
            start_time: std::time::SystemTime::now(),
        }
    }
}

impl AtlasDnsServer {
    /// Create new DNS server instance
    pub async fn new(config: DnsConfig) -> DnsResult<Self> {
        info!("🚀 Initializing Atlas DNS Server...");
        
        // Initialize components
        let zone_manager = Arc::new(RwLock::new(ZoneManager::new(config.clone()).await?));
        let domain_registry = Arc::new(RwLock::new(DomainRegistry::new(config.clone()).await?));
        let cache = Arc::new(RwLock::new(DnsCache::new(config.clone()).await?));
        let running = Arc::new(RwLock::new(false));
        let stats = Arc::new(RwLock::new(ServerStats::default()));
        
        // Initialize .x3 zone with default records
        Self::initialize_x3_zone(&zone_manager, &domain_registry).await?;
        
        info!("✅ Atlas DNS Server initialized successfully");
        
        Ok(Self {
            config,
            zone_manager,
            domain_registry,
            cache,
            running,
            stats,
        })
    }
    
    /// Initialize the .x3 zone with default Atlas Sphere services
    async fn initialize_x3_zone(
        _zone_manager: &Arc<RwLock<ZoneManager>>,
        domain_registry: &Arc<RwLock<DomainRegistry>>,
    ) -> DnsResult<()> {
        let mut domain_registry = domain_registry.write().await;
        
        info!("🌐 Initializing .x3 zone with Atlas Sphere services...");
        
        // Define default Atlas Sphere services
        let default_services = vec![
            ("blockexplorer", "10.0.1.100"),
            ("api", "10.0.1.200"),
            ("rpc", "10.0.1.200"),
            ("xchange", "10.0.2.100"),
            ("wallet", "10.0.2.200"),
            ("dashboard", "10.0.2.250"),
            ("explorer", "10.0.2.251"),
            ("docs", "10.0.3.100"),
            ("status", "10.0.3.200"),
            ("governance", "10.0.3.250"),
        ];
        
        let service_count = default_services.len();
        for (service, ip) in default_services {
            let domain = DomainName::new(format!("{}.x3", service))?;
            
            // Create domain record
            let mut domain_record = DomainRecord::new(domain.clone(), None);
            
            // Add A record
            let ip_addr = ip.parse::<std::net::Ipv4Addr>()
                .map_err(|_| DnsError::invalid_domain_name("Invalid default IP address"))?;
            let a_record = crate::domain::DnsRecord::a(
                domain.clone(), 
                ip_addr, 
                Some(300) // 5 minutes TTL for services
            );
            domain_record.add_record(a_record)?;
            
            // Register domain
            domain_registry.register_domain(domain_record).await?;
            
            info!("  📍 Registered {}.x3 -> {}", service, ip);
        }
        
        info!("✅ .x3 zone initialized with {} services", service_count);
        Ok(())
    }
    
    /// Start the DNS server
    pub async fn start(self: Arc<Self>) -> DnsResult<()> {
        info!("🌟 Starting Atlas DNS Server on {}:{}", 
              self.config.server.bind_address.ip(),
              self.config.server.bind_address.port());
        
        // Set running flag
        {
            let mut running = self.running.write().await;
            *running = true;
        }
        
        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.start_time = std::time::SystemTime::now();
        }
        
        // Start UDP server
        if self.config.server.udp_enabled {
            self.start_udp_server().await?;
        }
        
        Ok(())
    }
    
    /// Stop the DNS server
    pub async fn stop(&self) -> DnsResult<()> {
        info!("🛑 Stopping Atlas DNS Server...");
        
        // Set running flag to false
        {
            let mut running = self.running.write().await;
            *running = false;
        }
        
        info!("✅ Atlas DNS Server stopped");
        Ok(())
    }
    
    /// Start the UDP DNS server
    async fn start_udp_server(self: &Arc<Self>) -> DnsResult<()> {
        let socket = UdpSocket::bind(&self.config.server.bind_address)
            .await
            .map_err(|e| DnsError::Network(format!("Failed to bind UDP socket: {}", e)))?;
        
        info!("🌐 Atlas DNS Server listening on UDP {}", self.config.server.bind_address);
        
        let server = Arc::clone(self);
        
        tokio::spawn(async move {
            let mut buf = vec![0u8; 4096];
            
            loop {
                // Check if we should stop
                {
                    let running = server.running.read().await;
                    if !*running {
                        break;
                    }
                }
                
                match socket.recv_from(&mut buf).await {
                    Ok((len, src)) => {
                        let packet = buf[..len].to_vec();
                        let server_clone = Arc::clone(&server);
                        let socket_clone = socket.local_addr().ok();
                        
                        tokio::spawn(async move {
                            if let Err(e) = server_clone.handle_udp_request(&packet, src, socket_clone).await {
                                warn!("Error handling DNS request: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        error!("UDP receive error: {}", e);
                    }
                }
            }
        });
        
        Ok(())
    }
    
    /// Handle a single UDP DNS request
    async fn handle_udp_request(
        &self,
        packet: &[u8],
        src: SocketAddr,
        _local_addr: Option<SocketAddr>,
    ) -> DnsResult<()> {
        let start_time = std::time::Instant::now();
        
        // Parse the DNS message
        let request = Message::from_bytes(packet)
            .map_err(|e| DnsError::Protocol(e))?;
        
        debug!("📨 DNS request from {}: {:?}", src, request.queries());
        
        // Update query statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_queries += 1;
            stats.udp_packets += 1;
        }
        
        // Process the request
        let response = self.process_dns_request(&request).await;
        
        // Update stats based on response
        {
            let mut stats = self.stats.write().await;
            let response_time = start_time.elapsed().as_millis() as f64;
            if stats.total_queries > 0 {
                stats.average_response_time_ms = 
                    (stats.average_response_time_ms * (stats.total_queries - 1) as f64 + response_time) 
                    / stats.total_queries as f64;
            }
        }
        
        // Send response back
        let response_bytes = response.to_bytes()
            .map_err(|e| DnsError::Protocol(e))?;
        
        // Create a new socket to send the response
        let send_socket = UdpSocket::bind("0.0.0.0:0").await
            .map_err(|e| DnsError::Network(format!("Failed to create send socket: {}", e)))?;
        
        send_socket.send_to(&response_bytes, src).await
            .map_err(|e| DnsError::Network(format!("Failed to send response: {}", e)))?;
        
        debug!("✅ Response sent in {}ms", start_time.elapsed().as_millis());
        Ok(())
    }
    
    /// Process DNS request and build response
    async fn process_dns_request(&self, request: &Message) -> Message {
        let mut response = Message::new();
        response.set_id(request.id());
        response.set_message_type(MessageType::Response);
        response.set_op_code(OpCode::Query);
        response.set_authoritative(true);
        response.set_recursion_desired(request.recursion_desired());
        response.set_recursion_available(false);
        
        // Copy queries to response
        for query in request.queries() {
            response.add_query(query.clone());
        }
        
        // Process each query
        for query in request.queries() {
            let query_name = query.name().to_string();
            let query_name_clean = query_name.trim_end_matches('.');
            
            debug!("🔍 Processing query for: {} (type: {:?})", query_name_clean, query.query_type());
            
            // Check if this is an .x3 domain
            if !query_name_clean.ends_with(".x3") && query_name_clean != "x3" {
                response.set_response_code(ResponseCode::Refused);
                continue;
            }
            
            // Look up the domain
            let domain_result = DomainName::new(query_name_clean.to_string());
            let domain_name = match domain_result {
                Ok(d) => d,
                Err(_) => {
                    response.set_response_code(ResponseCode::FormErr);
                    continue;
                }
            };
            
            // Try to get the domain from the registry
            let domain_record = {
                let registry = self.domain_registry.read().await;
                registry.get_domain(&domain_name).await
            };
            
            match domain_record {
                Ok(Some(domain)) => {
                    // Found the domain - add answers based on query type
                    let answers = self.get_records_for_query(&domain, query.query_type(), query.name());
                    for record in answers {
                        response.add_answer(record);
                    }
                    
                    if response.answers().is_empty() {
                        response.set_response_code(ResponseCode::NoError);
                    } else {
                        response.set_response_code(ResponseCode::NoError);
                    }
                    
                    // Update stats
                    {
                        let mut stats = self.stats.write().await;
                        stats.authoritative_responses += 1;
                    }
                }
                Ok(None) => {
                    // Domain not found
                    response.set_response_code(ResponseCode::NXDomain);
                    
                    // Update stats
                    {
                        let mut stats = self.stats.write().await;
                        stats.nxdomain_responses += 1;
                    }
                }
                Err(e) => {
                    error!("Error looking up domain: {}", e);
                    response.set_response_code(ResponseCode::ServFail);
                    
                    // Update stats
                    {
                        let mut stats = self.stats.write().await;
                        stats.error_responses += 1;
                    }
                }
            }
        }
        
        response
    }
    
    /// Get records matching the query type from a domain record
    fn get_records_for_query(&self, domain: &DomainRecord, query_type: RecordType, query_name: &Name) -> Vec<Record> {
        let mut answers = Vec::new();
        
        for dns_record in &domain.records {
            let record_type = dns_record.record_type.to_trust_dns_type();
            
            // Match query type or return all if ANY
            if query_type != RecordType::ANY && query_type != record_type {
                continue;
            }
            
            let mut record = Record::new();
            record.set_name(query_name.clone());
            record.set_ttl(dns_record.ttl);
            record.set_record_type(record_type);
            
            // Set the record data based on type
            match &dns_record.data {
                DnsRecordType::A(ip) => {
                    record.set_data(Some(RData::A(rdata::A(*ip))));
                }
                DnsRecordType::AAAA(ip) => {
                    record.set_data(Some(RData::AAAA(rdata::AAAA(*ip))));
                }
                DnsRecordType::CNAME(target) => {
                    if let Ok(name) = Name::from_ascii(target) {
                        record.set_data(Some(RData::CNAME(rdata::CNAME(name))));
                    }
                }
                DnsRecordType::TXT(text) => {
                    record.set_data(Some(RData::TXT(rdata::TXT::new(vec![text.clone()]))));
                }
                DnsRecordType::MX(mx) => {
                    if let Ok(name) = mx.exchange.to_name() {
                        record.set_data(Some(RData::MX(rdata::MX::new(mx.priority, name))));
                    }
                }
                DnsRecordType::NS(nameserver) => {
                    if let Ok(name) = Name::from_ascii(nameserver) {
                        record.set_data(Some(RData::NS(rdata::NS(name))));
                    }
                }
                DnsRecordType::SRV(srv) => {
                    if let Ok(target) = srv.target.to_name() {
                        record.set_data(Some(RData::SRV(rdata::SRV::new(
                            srv.priority, srv.weight, srv.port, target
                        ))));
                    }
                }
                DnsRecordType::CAA(_) | DnsRecordType::HINFO(_) => {
                    // Skip complex types for now
                    continue;
                }
            }
            
            answers.push(record);
        }
        
        answers
    }
    
    /// Get number of managed zones
    pub async fn get_zone_count(&self) -> usize {
        let zone_manager = self.zone_manager.read().await;
        zone_manager.get_zone_count().await
    }
    
    /// Get server statistics
    pub async fn get_stats(&self) -> ServerStats {
        let stats = self.stats.read().await;
        stats.clone()
    }
    
    /// Check if server is running
    pub async fn is_running(&self) -> bool {
        let running = self.running.read().await;
        *running
    }
}
