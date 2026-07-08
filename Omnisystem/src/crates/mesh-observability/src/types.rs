use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Trace {
    pub trace_id: Uuid,
    pub service_name: String,
    pub operation_name: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub duration_ms: u64,
    pub spans: Vec<Span>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Span {
    pub span_id: Uuid,
    pub parent_span_id: Option<Uuid>,
    pub operation_name: String,
    pub service_name: String,
    pub start_time: DateTime<Utc>,
    pub duration_ms: u64,
    pub tags: Vec<(String, String)>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrafficMetric {
    pub metric_id: Uuid,
    pub source_service: String,
    pub destination_service: String,
    pub request_count: u64,
    pub error_count: u64,
    pub avg_latency_ms: f32,
    pub success_rate: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServiceTopology {
    pub topology_id: Uuid,
    pub services: Vec<String>,
    pub connections: Vec<(String, String)>,
    pub total_services: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthMetrics {
    pub health_id: Uuid,
    pub service_name: String,
    pub uptime_percent: f32,
    pub error_rate: f32,
    pub latency_p50_ms: f32,
    pub latency_p99_ms: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TraceVisualization {
    pub viz_id: Uuid,
    pub trace_id: Uuid,
    pub visualization: String,
    pub format: VisualizationFormat,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum VisualizationFormat {
    GraphML,
    JSON,
    Mermaid,
}
