use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ComponentSpec {
    pub name: String,
    /// HTTP URL to probe. If None, only TCP port is checked.
    pub health_url: Option<String>,
    /// TCP port to wait on (always checked, even when health_url is set).
    pub health_port: u16,
    /// Other component names that must be Ready before this one starts.
    pub dependencies: Vec<String>,
    pub retries: u32,
    pub retry_delay_ms: u64,
    pub timeout_secs: u64,
    /// If true, failure aborts the entire launch sequence.
    pub required: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentState {
    Pending,
    Starting,
    Ready,
    Failed(String),
    Skipped,
}

impl ComponentState {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Ready | Self::Failed(_) | Self::Skipped)
    }
}
