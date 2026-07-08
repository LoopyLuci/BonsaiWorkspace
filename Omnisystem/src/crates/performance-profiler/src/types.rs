use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProfilerConfig {
    pub sampling_rate_hz: u32,
    pub buffer_size: usize,
    pub auto_flush_interval_ms: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StackFrame {
    pub function_name: String,
    pub module_name: String,
    pub line_number: u32,
    pub offset: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CpuSample {
    pub sample_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub stack_trace: Vec<StackFrame>,
    pub duration_us: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PerformanceMetric {
    pub metric_id: Uuid,
    pub name: String,
    pub value: f64,
    pub unit: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProfileReport {
    pub profile_id: Uuid,
    pub total_samples: u64,
    pub duration_ms: u64,
    pub cpu_time_percent: f32,
    pub memory_peak_mb: u64,
    pub hotspots: Vec<StackFrame>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FlameGraphNode {
    pub function_name: String,
    pub time_percent: f32,
    pub sample_count: u64,
}
