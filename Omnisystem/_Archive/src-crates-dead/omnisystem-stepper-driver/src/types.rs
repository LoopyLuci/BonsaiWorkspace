//! Types

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct State {
    pub timestamp: u64,
    pub status: String,
}
