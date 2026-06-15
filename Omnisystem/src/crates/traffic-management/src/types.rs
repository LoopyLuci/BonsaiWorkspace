use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoutingPolicy {
    pub policy_id: Uuid,
    pub service_name: String,
    pub routing_strategy: RoutingStrategy,
    pub timeout_ms: u32,
    pub retries: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum RoutingStrategy {
    RoundRobin,
    LeastConnections,
    WeightedDistribution,
    Random,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WeightedDestination {
    pub destination_id: Uuid,
    pub service_name: String,
    pub version: String,
    pub weight: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CanaryDeployment {
    pub deployment_id: Uuid,
    pub service_name: String,
    pub stable_version: String,
    pub canary_version: String,
    pub canary_traffic_percent: u32,
    pub status: DeploymentStatus,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum DeploymentStatus {
    Planning,
    InProgress,
    Stable,
    RolledBack,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrafficShapingPolicy {
    pub policy_id: Uuid,
    pub service_name: String,
    pub max_requests_per_second: u32,
    pub burst_size: u32,
    pub delay_ms: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RouteConfig {
    pub route_id: Uuid,
    pub destination: String,
    pub timeout_ms: u32,
    pub retry_count: u32,
    pub circuit_breaker_threshold: f32,
}
