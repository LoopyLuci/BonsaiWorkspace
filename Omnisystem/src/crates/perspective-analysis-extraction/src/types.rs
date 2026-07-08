//! Types

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct State {
    pub status: String,
    pub data: String,
}
