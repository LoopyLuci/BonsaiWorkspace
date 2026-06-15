use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize, Copy)]
pub enum ProtocolType {
    Http1,
    Http2,
    Http3,
    Grpc,
    WebSocket,
    Mqtt,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Copy)]
pub enum AdaptationStrategy {
    LatencyOptimized,
    ThroughputOptimized,
    ReliabilityOptimized,
    CostOptimized,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Copy)]
pub enum ProtocolState {
    Available,
    Degraded,
    Unavailable,
    Transitioning,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProtocolMetrics {
    pub protocol: ProtocolType,
    pub avg_latency_ms: f64,
    pub p99_latency_ms: u64,
    pub throughput_rps: f64,
    pub success_rate: f64,
    pub error_count: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProtocolCapability {
    pub protocol: ProtocolType,
    pub supports_streaming: bool,
    pub supports_multiplexing: bool,
    pub supports_server_push: bool,
    pub max_connections: u32,
    pub compression_supported: bool,
    pub tls_version: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProtocolSelection {
    pub current_protocol: ProtocolType,
    pub candidates: Vec<ProtocolType>,
    pub selected_protocol: ProtocolType,
    pub adaptation_reason: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdaptationPolicy {
    pub strategy: AdaptationStrategy,
    pub latency_threshold_ms: u64,
    pub throughput_threshold_rps: f64,
    pub error_rate_threshold: f64,
    pub check_interval_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProtocolTransition {
    pub from_protocol: ProtocolType,
    pub to_protocol: ProtocolType,
    pub reason: String,
    pub initiated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub success: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdaptationMetrics {
    pub total_adaptations: u64,
    pub successful_transitions: u64,
    pub failed_transitions: u64,
    pub avg_adaptation_latency_ms: f64,
    pub protocol_distribution: Vec<(ProtocolType, f64)>,
}
