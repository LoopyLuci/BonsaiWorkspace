use serde::{Serialize, Deserialize};
use crate::ToolExecutionMode;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub output_schema: Option<serde_json::Value>,
    pub required_capabilities: Vec<String>,
    pub execution_mode: ToolExecutionMode,
    pub binary: String,
    pub timeout_ms: u64,
    pub max_memory_mb: u64,
}
