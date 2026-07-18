//! Domain types for the auto-scaling engine.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A per-service autoscaling policy: replica bounds and the target
/// utilization the scaler tries to keep the service near.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScalingPolicy {
    pub policy_id: Uuid,
    pub service_name: String,
    pub min_replicas: u32,
    pub max_replicas: u32,
    pub target_cpu_percent: u32,
    pub target_memory_percent: u32,
}

/// A single observed utilization sample for a service.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetricSnapshot {
    pub snapshot_id: Uuid,
    pub service_name: String,
    pub cpu_percent: u32,
    pub memory_percent: u32,
    pub request_count: u64,
    pub response_time_ms: u32,
}

/// The action recommended by a scaling evaluation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScalingAction {
    ScaleUp,
    ScaleDown,
    NoChange,
}

/// The result of evaluating a service's current metrics against its policy.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScalingDecision {
    pub decision_id: Uuid,
    pub service_name: String,
    pub current_replicas: u32,
    pub desired_replicas: u32,
    pub action: ScalingAction,
    pub reason: String,
}

/// A forecast of the replica count a service is expected to need, based on
/// recent request-count history.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DemandPrediction {
    pub prediction_id: Uuid,
    pub service_name: String,
    pub predicted_replicas: u32,
    pub confidence_percent: u32,
    pub predicted_load: f32,
}
