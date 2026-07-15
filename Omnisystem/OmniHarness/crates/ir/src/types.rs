//! Lightweight status snapshot type, returned by `IrCompiler::stats`.

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct State {
    pub timestamp: u64,
    pub status: String,
}

impl State {
    pub fn now(status: impl Into<String>) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self { timestamp, status: status.into() }
    }
}
