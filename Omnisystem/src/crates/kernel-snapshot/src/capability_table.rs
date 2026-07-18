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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_serialize_deserialize() {
        let mut table = CapabilityTable::new();
        table.add_capability("net.connect".to_string());
        table.add_capability("fs.read".to_string());

        let bytes = table.serialize().unwrap();
        let restored = CapabilityTable::deserialize(&bytes).unwrap();

        assert_eq!(restored.capabilities, table.capabilities);
        assert_eq!(restored.capabilities.len(), 2);
    }

    #[test]
    fn test_deserialize_invalid_data_errors() {
        assert!(CapabilityTable::deserialize(b"not json").is_err());
    }
}
