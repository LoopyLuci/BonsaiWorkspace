use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use uuid::Uuid;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct ZoneId(pub Uuid);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct DomainName(pub String);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct RecordId(pub Uuid);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Copy)]
pub enum RecordType {
    A,
    Aaaa,
    Cname,
    Mx,
    Txt,
    Ns,
    Soa,
    Ptr,
    Srv,
    Caa,
}

impl RecordType {
    pub fn to_string(&self) -> &'static str {
        match self {
            RecordType::A => "A",
            RecordType::Aaaa => "AAAA",
            RecordType::Cname => "CNAME",
            RecordType::Mx => "MX",
            RecordType::Txt => "TXT",
            RecordType::Ns => "NS",
            RecordType::Soa => "SOA",
            RecordType::Ptr => "PTR",
            RecordType::Srv => "SRV",
            RecordType::Caa => "CAA",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RecordData {
    A(Ipv4Addr),
    Aaaa(Ipv6Addr),
    Cname(String),
    Mx { preference: u16, exchange: String },
    Txt(String),
    Ns(String),
    Ptr(String),
    Srv {
        priority: u16,
        weight: u16,
        port: u16,
        target: String,
    },
    Caa {
        flags: u8,
        tag: String,
        value: String,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DnsRecord {
    pub id: RecordId,
    pub name: String,
    pub record_type: RecordType,
    pub data: RecordData,
    pub ttl: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl DnsRecord {
    pub fn new(
        name: String,
        record_type: RecordType,
        data: RecordData,
        ttl: u32,
    ) -> Self {
        Self {
            id: RecordId(Uuid::new_v4()),
            name,
            record_type,
            data,
            ttl,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ZoneType {
    Primary,
    Secondary,
    Stub,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Zone {
    pub id: ZoneId,
    pub name: DomainName,
    pub zone_type: ZoneType,
    pub serial: u32,
    pub refresh: u32,
    pub retry: u32,
    pub expire: u32,
    pub minimum_ttl: u32,
    pub nameservers: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Zone {
    pub fn new(name: DomainName) -> Self {
        Self {
            id: ZoneId(Uuid::new_v4()),
            name,
            zone_type: ZoneType::Primary,
            serial: 1,
            refresh: 86400,
            retry: 3600,
            expire: 604800,
            minimum_ttl: 3600,
            nameservers: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn with_nameservers(mut self, nameservers: Vec<String>) -> Self {
        self.nameservers = nameservers;
        self
    }

    pub fn increment_serial(&mut self) {
        self.serial = self.serial.wrapping_add(1);
        self.updated_at = Utc::now();
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DnsQuery {
    pub domain: DomainName,
    pub record_type: RecordType,
    pub client_ip: Option<IpAddr>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DnsResponse {
    pub query: DnsQuery,
    pub records: Vec<DnsRecord>,
    pub authoritative: bool,
    pub recursion_available: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GeoLocation {
    pub latitude: f64,
    pub longitude: f64,
    pub country: String,
    pub region: String,
    pub city: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GeoRoute {
    pub location: GeoLocation,
    pub target: String,
    pub priority: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthCheck {
    pub interval: u32,
    pub timeout: u32,
    pub retries: u32,
    pub healthy_threshold: u32,
    pub unhealthy_threshold: u32,
}

impl Default for HealthCheck {
    fn default() -> Self {
        Self {
            interval: 30,
            timeout: 10,
            retries: 3,
            healthy_threshold: 2,
            unhealthy_threshold: 3,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerHealth {
    pub server: String,
    pub status: HealthStatus,
    pub last_check: Option<DateTime<Utc>>,
    pub check_count: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DynamicUpdate {
    pub zone: DomainName,
    pub records: Vec<DnsRecord>,
    pub timestamp: DateTime<Utc>,
    pub client_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DnsMetrics {
    pub total_queries: u64,
    pub successful_queries: u64,
    pub failed_queries: u64,
    pub average_response_time_ms: f64,
    pub zones_count: u64,
    pub records_count: u64,
}

impl Default for DnsMetrics {
    fn default() -> Self {
        Self {
            total_queries: 0,
            successful_queries: 0,
            failed_queries: 0,
            average_response_time_ms: 0.0,
            zones_count: 0,
            records_count: 0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub queries_per_second: u32,
    pub burst_size: u32,
    pub block_duration_secs: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            queries_per_second: 1000,
            burst_size: 5000,
            block_duration_secs: 60,
        }
    }
}
