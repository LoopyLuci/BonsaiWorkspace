use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MeshService {
    pub service_id: Uuid,
    pub name: String,
    pub namespace: String,
    pub port: u16,
    pub protocol: Protocol,
    pub version: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum Protocol {
    HTTP,
    HTTPS,
    GRPC,
    TCP,
    UDP,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SidecarProxy {
    pub proxy_id: Uuid,
    pub service_id: Uuid,
    pub pod_ip: String,
    pub proxy_port: u16,
    pub admin_port: u16,
    pub status: ProxyStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum ProxyStatus {
    Initializing,
    Ready,
    Healthy,
    Unhealthy,
    Terminating,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    pub endpoint_id: Uuid,
    pub service_id: Uuid,
    pub address: String,
    pub port: u16,
    pub weight: u32,
    pub healthy: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MeshConfig {
    pub config_id: Uuid,
    pub name: String,
    pub mtls_enabled: bool,
    pub tracing_enabled: bool,
    pub service_registry_url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServiceRegistry {
    pub registry_id: Uuid,
    pub services: Vec<MeshService>,
    pub total_services: usize,
}
