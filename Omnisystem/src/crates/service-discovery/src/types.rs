use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// A generic timestamped record (kept from the original stub; used by [`crate::Manager`]).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Record {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
}

/// Health status of a registered service instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceStatus {
    Healthy,
    Unhealthy,
    Unknown,
}

/// A single running instance of a named service.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServiceInstance {
    pub instance_id: String,
    pub service_name: String,
    pub host: String,
    pub port: u16,
    pub status: ServiceStatus,
    pub registered_at: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
    pub metadata: HashMap<String, String>,
}

/// A snapshot view of all instances registered for a service.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServiceRegistry {
    pub service_name: String,
    pub instances: Vec<ServiceInstance>,
    pub total_instances: u32,
    pub healthy_instances: u32,
}

/// Configuration describing how a [`crate::LoadBalancer`] should pick between
/// instances of a service.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LoadBalancingPolicy {
    pub policy_type: String,
    pub weight_map: HashMap<String, u32>,
}
