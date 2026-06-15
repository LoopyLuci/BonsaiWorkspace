use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Copy)]
pub enum ProbeType {
    Http,
    Tcp,
    Exec,
    Grpc,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProbeConfig {
    pub probe_type: ProbeType,
    pub path: String,
    pub port: u16,
    pub interval_secs: u64,
    pub timeout_secs: u64,
    pub initial_delay_secs: u64,
    pub failure_threshold: u32,
    pub success_threshold: u32,
}

impl Default for ProbeConfig {
    fn default() -> Self {
        Self {
            probe_type: ProbeType::Http,
            path: "/health".to_string(),
            port: 8080,
            interval_secs: 10,
            timeout_secs: 5,
            initial_delay_secs: 0,
            failure_threshold: 3,
            success_threshold: 2,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Copy)]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthCheckEvent {
    pub service_id: String,
    pub endpoint_id: String,
    pub status: HealthStatus,
    pub timestamp: DateTime<Utc>,
    pub duration_ms: u32,
    pub error_message: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthMonitor {
    pub service_id: String,
    pub status: HealthStatus,
    pub probe_config: ProbeConfig,
    pub consecutive_failures: u32,
    pub consecutive_successes: u32,
    pub last_check_time: DateTime<Utc>,
    pub last_status_change: DateTime<Utc>,
    pub check_history: Vec<HealthCheckEvent>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthMetrics {
    pub total_checks: u64,
    pub successful_checks: u64,
    pub failed_checks: u64,
    pub uptime_percentage: f64,
    pub average_response_time_ms: u32,
    pub min_response_time_ms: u32,
    pub max_response_time_ms: u32,
    pub last_status_change: DateTime<Utc>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Copy)]
pub enum ContextPropagationType {
    TraceContext,
    Baggage,
    Correlation,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DistributedContext {
    pub trace_id: String,
    pub span_id: String,
    pub correlation_id: String,
    pub parent_span_id: Option<String>,
    pub baggage: HashMap<String, String>,
    pub timestamp: DateTime<Utc>,
    pub propagation_type: ContextPropagationType,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContextPropagationConfig {
    pub enabled: bool,
    pub propagation_types: Vec<ContextPropagationType>,
    pub include_baggage: bool,
    pub max_baggage_size: usize,
    pub sampling_rate: f64,
}

impl Default for ContextPropagationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            propagation_types: vec![ContextPropagationType::TraceContext],
            include_baggage: true,
            max_baggage_size: 1024,
            sampling_rate: 1.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthCheckMetadata {
    pub check_count: u64,
    pub last_status: HealthStatus,
    pub status_change_count: u32,
    pub uptime_seconds: u64,
    pub downtime_seconds: u64,
    pub metadata: HashMap<String, String>,
}
