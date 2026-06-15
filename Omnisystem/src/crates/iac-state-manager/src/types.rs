//! Types

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Config {
    pub enabled: bool,
    pub name: String,
}
