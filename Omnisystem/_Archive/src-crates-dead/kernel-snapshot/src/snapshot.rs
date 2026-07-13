//! Snapshot creation and serialization

use crate::capability_table::CapabilityTable;
use blake3::Hash;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub vault_id: u64,
    pub binary_hash: String,
    pub capabilities: Vec<u8>,
    pub timestamp: u64,
    pub memory_data: Vec<u8>,
}

impl Snapshot {
    pub fn new(vault_id: u64, binary_hash: &str, capabilities: &CapabilityTable) -> Self {
        Self {
            vault_id,
            binary_hash: binary_hash.to_string(),
            capabilities: capabilities.serialize().unwrap_or_default(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            memory_data: Vec::new(),
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        serde_json::to_vec(self).unwrap_or_default()
    }

    pub fn get_hash(&self) -> Hash {
        blake3::hash(&self.serialize())
    }
}
