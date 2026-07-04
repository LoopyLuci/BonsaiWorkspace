//! Snapshot restoration

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RestoreContext {
    pub vault_id: u64,
    pub instruction_pointer: u64,
    pub register_state: Vec<u64>,
}

impl RestoreContext {
    pub fn new(vault_id: u64) -> Self {
        Self {
            vault_id,
            instruction_pointer: 0,
            register_state: vec![0; 16],
        }
    }
}
