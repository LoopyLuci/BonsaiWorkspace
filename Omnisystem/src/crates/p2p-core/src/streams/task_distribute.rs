use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub cpu_cores: f32,
    pub memory_mb: u64,
    pub gpu_required: bool,
    pub network_mbps: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDefinition {
    pub task_id: String,
    pub wasm_bytes: Vec<u8>,
    pub input_data: Vec<u8>,
    pub resource_requirements: ResourceRequirements,
    pub deadline_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub output_data: Vec<u8>,
    pub execution_time_ms: u64,
    pub energy_used_joules: f32,
    pub error: Option<String>,
}

pub struct TaskDistributeStream;

impl TaskDistributeStream {
    pub fn encode_task(task: &TaskDefinition) -> Vec<u8> {
        serde_json::to_vec(task).unwrap_or_default()
    }

    pub fn decode_task(data: &[u8]) -> Option<TaskDefinition> {
        serde_json::from_slice(data).ok()
    }

    pub fn encode_result(result: &TaskResult) -> Vec<u8> {
        serde_json::to_vec(result).unwrap_or_default()
    }

    pub fn decode_result(data: &[u8]) -> Option<TaskResult> {
        serde_json::from_slice(data).ok()
    }
}
