use crate::ir::BuirModule;
use anyhow::Result;

pub fn serialize_to_bytes(module: &BuirModule) -> Result<Vec<u8>> {
    Ok(serde_json::to_vec(module)?)
}

pub fn deserialize_from_bytes(bytes: &[u8]) -> Result<BuirModule> {
    Ok(serde_json::from_slice(bytes)?)
}
