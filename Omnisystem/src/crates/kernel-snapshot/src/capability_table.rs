//! Capability table management

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CapabilityTable {
    capabilities: Vec<String>,
}

impl CapabilityTable {
    pub fn new() -> Self {
        Self {
            capabilities: Vec::new(),
        }
    }

    pub fn add_capability(&mut self, cap: String) {
        self.capabilities.push(cap);
    }

    pub fn serialize(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    pub fn deserialize(data: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(data)
    }
}

impl Default for CapabilityTable {
    fn default() -> Self {
        Self::new()
    }
}
