//! Type definitions for this module

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModuleState {
    pub status: String,
    pub data: String,
}
