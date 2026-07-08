use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeNode {
    pub node_id: String,
    pub display_name: String,
    pub available_cores: u32,
    pub available_memory_mb: u64,
    pub is_online: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskType {
    Wasm,
    Inference,
    DataProcess,
    Script,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FabricTask {
    pub task_id: String,
    pub project_id: String,
    pub task_type: TaskType,
    pub payload: Vec<u8>,
    pub priority: u8,
    pub required_memory_mb: u64,
    pub required_cores: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Queued,
    Assigned { node_id: String },
    Running { node_id: String },
    Completed,
    Failed { reason: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub status: TaskStatus,
    pub output: Option<Vec<u8>>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeProject {
    pub project_id: String,
    pub name: String,
    pub tasks: Vec<FabricTask>,
}
