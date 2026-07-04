use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmulatorConfig {
    pub device_file: String,
    pub max_cycles: Option<u64>,
    pub trace: bool,
    pub trace_file: Option<String>,
    pub distributed: bool,
    pub num_nodes: u32,
}

impl Default for EmulatorConfig {
    fn default() -> Self {
        Self {
            device_file: "device.hdl".to_string(),
            max_cycles: None,
            trace: false,
            trace_file: None,
            distributed: false,
            num_nodes: 1,
        }
    }
}
