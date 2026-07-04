use crate::ir::{BuirModule, BuirFunction};

pub fn hash_buir(module: &BuirModule) -> String {
    let json = serde_json::to_vec(module).expect("BUIR serialization failed");
    let hash = blake3::hash(&json);
    hash.to_hex().to_string()
}

pub fn hash_function(function: &BuirFunction) -> FunctionHash {
    let json = serde_json::to_vec(function).expect("Function serialization failed");
    let hash = blake3::hash(&json);
    FunctionHash(*hash.as_bytes())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct FunctionHash(pub [u8; 32]);
