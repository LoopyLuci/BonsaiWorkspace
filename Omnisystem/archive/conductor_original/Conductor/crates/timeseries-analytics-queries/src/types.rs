//! Types

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Data {
    pub timestamp: u64,
    pub value: f64,
}
