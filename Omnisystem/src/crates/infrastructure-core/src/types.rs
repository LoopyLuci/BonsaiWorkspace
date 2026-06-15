use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ServiceId(pub String);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct InstanceId(pub Uuid);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServiceInstance {
    pub id: InstanceId,
    pub service_id: ServiceId,
    pub host: String,
    pub port: u16,
    pub tags: HashMap<String, String>,
    pub metadata: HashMap<String, String>,
    pub health_status: HealthStatus,
    pub registered_at: DateTime<Utc>,
    pub last_heartbeat: Option<DateTime<Utc>>,
}

impl ServiceInstance {
    pub fn new(
        service_id: ServiceId,
        host: String,
        port: u16,
    ) -> Self {
        Self {
            id: InstanceId(Uuid::new_v4()),
            service_id,
            host,
            port,
            tags: HashMap::new(),
            metadata: HashMap::new(),
            health_status: HealthStatus::Unknown,
            registered_at: Utc::now(),
            last_heartbeat: None,
        }
    }

    pub fn with_tags(mut self, tags: HashMap<String, String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_metadata(mut self, metadata: HashMap<String, String>) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn is_healthy(&self) -> bool {
        self.health_status == HealthStatus::Healthy
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Copy)]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServiceDefinition {
    pub id: ServiceId,
    pub name: String,
    pub protocol: String,
    pub port: u16,
    pub tags: Vec<String>,
    pub health_check: HealthCheckConfig,
    pub load_balancer_policy: LoadBalancerPolicy,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    pub enabled: bool,
    pub interval_secs: u64,
    pub timeout_secs: u64,
    pub unhealthy_threshold: u32,
    pub healthy_threshold: u32,
    pub http_path: Option<String>,
    pub tcp_port: Option<u16>,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_secs: 10,
            timeout_secs: 3,
            unhealthy_threshold: 3,
            healthy_threshold: 2,
            http_path: Some("/health".to_string()),
            tcp_port: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Copy, PartialEq, Eq)]
pub enum LoadBalancerPolicy {
    RoundRobin,
    LeastConnections,
    Random,
    IpHash,
    WeightedRoundRobin,
}

impl Default for LoadBalancerPolicy {
    fn default() -> Self {
        Self::RoundRobin
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegistrySnapshot {
    pub services: HashMap<String, Vec<ServiceInstance>>,
    pub timestamp: DateTime<Utc>,
    pub total_instances: usize,
    pub healthy_instances: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetricsPoint {
    pub timestamp: DateTime<Utc>,
    pub name: String,
    pub value: f64,
    pub tags: HashMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthCheckResult {
    pub instance_id: InstanceId,
    pub service_id: ServiceId,
    pub status: HealthStatus,
    pub timestamp: DateTime<Utc>,
    pub response_time_ms: u64,
    pub details: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_instance_creation() {
        let svc = ServiceId("api-svc".to_string());
        let instance = ServiceInstance::new(svc.clone(), "localhost".to_string(), 8080);

        assert_eq!(instance.service_id, svc);
        assert_eq!(instance.host, "localhost");
        assert_eq!(instance.port, 8080);
        assert_eq!(instance.health_status, HealthStatus::Unknown);
    }

    #[test]
    fn test_service_instance_address() {
        let instance = ServiceInstance::new(
            ServiceId("svc".to_string()),
            "192.168.1.1".to_string(),
            3000,
        );
        assert_eq!(instance.address(), "192.168.1.1:3000");
    }

    #[test]
    fn test_service_instance_is_healthy() {
        let mut instance = ServiceInstance::new(
            ServiceId("svc".to_string()),
            "localhost".to_string(),
            8080,
        );

        assert!(!instance.is_healthy());
        instance.health_status = HealthStatus::Healthy;
        assert!(instance.is_healthy());
    }

    #[test]
    fn test_health_check_config_defaults() {
        let config = HealthCheckConfig::default();
        assert!(config.enabled);
        assert_eq!(config.interval_secs, 10);
        assert_eq!(config.healthy_threshold, 2);
    }

    #[test]
    fn test_load_balancer_policy_default() {
        assert_eq!(LoadBalancerPolicy::default(), LoadBalancerPolicy::RoundRobin);
    }
}
