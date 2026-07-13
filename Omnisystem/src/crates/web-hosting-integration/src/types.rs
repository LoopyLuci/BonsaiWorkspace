use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Hash)]
pub struct ServiceId(pub String);

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Copy)]
pub enum ServiceType {
    WebHosting,
    DnsRouter,
    FtpSftp,
}

impl ServiceType {
    pub fn to_string(&self) -> &'static str {
        match self {
            ServiceType::WebHosting => "web-hosting",
            ServiceType::DnsRouter => "dns-router",
            ServiceType::FtpSftp => "ftp-sftp",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    pub service_id: ServiceId,
    pub service_type: ServiceType,
    pub host: String,
    pub port: u16,
    pub tls_enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServiceHealth {
    pub service_id: ServiceId,
    pub status: HealthStatus,
    pub last_check: DateTime<Utc>,
    pub response_time_ms: f64,
    pub error_count: u64,
    pub success_count: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecurityPolicy {
    pub require_tls: bool,
    pub min_tls_version: String,
    pub allowed_ciphers: Vec<String>,
    pub max_session_duration_secs: u64,
    pub rate_limit_requests_per_sec: u32,
    pub require_authentication: bool,
    pub require_authorization: bool,
    pub enforce_https_redirect: bool,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            require_tls: true,
            min_tls_version: "1.3".to_string(),
            allowed_ciphers: vec![
                "TLS_AES_256_GCM_SHA384".to_string(),
                "TLS_CHACHA20_POLY1305_SHA256".to_string(),
            ],
            max_session_duration_secs: 3600,
            rate_limit_requests_per_sec: 1000,
            require_authentication: true,
            require_authorization: true,
            enforce_https_redirect: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub service_id: ServiceId,
    pub action: String,
    pub user: String,
    pub result: String,
    pub details: HashMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FailoverConfig {
    pub enable_failover: bool,
    pub primary_endpoint: ServiceEndpoint,
    pub backup_endpoints: Vec<ServiceEndpoint>,
    pub health_check_interval_secs: u64,
    pub failover_threshold_failures: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub service_id: ServiceId,
    pub request_count: u64,
    pub average_response_time_ms: f64,
    pub p95_response_time_ms: f64,
    pub p99_response_time_ms: f64,
    pub error_rate: f64,
    pub throughput_requests_per_sec: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecurityAudit {
    pub audit_id: String,
    pub timestamp: DateTime<Utc>,
    pub services_checked: u32,
    pub vulnerabilities_found: u32,
    pub compliance_score: f64,
    pub recommendations: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntegrationConfig {
    pub services: Vec<ServiceEndpoint>,
    pub security_policy: SecurityPolicy,
    pub failover_config: FailoverConfig,
    pub enable_audit_logging: bool,
    pub enable_metrics_collection: bool,
    pub backup_interval_secs: u64,
}

impl Default for IntegrationConfig {
    fn default() -> Self {
        Self {
            services: Vec::new(),
            security_policy: SecurityPolicy::default(),
            failover_config: FailoverConfig {
                enable_failover: true,
                primary_endpoint: ServiceEndpoint {
                    service_id: ServiceId("primary".to_string()),
                    service_type: ServiceType::WebHosting,
                    host: "localhost".to_string(),
                    port: 443,
                    tls_enabled: true,
                },
                backup_endpoints: Vec::new(),
                health_check_interval_secs: 30,
                failover_threshold_failures: 3,
            },
            enable_audit_logging: true,
            enable_metrics_collection: true,
            backup_interval_secs: 3600,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CacheEntry<T> {
    pub value: T,
    pub created_at: DateTime<Utc>,
    pub ttl_secs: u64,
}

impl<T> CacheEntry<T> {
    pub fn is_expired(&self) -> bool {
        let age = (Utc::now() - self.created_at).num_seconds() as u64;
        age > self.ttl_secs
    }
}
