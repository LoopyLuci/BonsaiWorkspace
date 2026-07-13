//! feature-state-tracker
#![warn(missing_docs)]

pub mod error;
pub mod types;

pub use error::{Error, Result};
pub use types::*;

use dashmap::DashMap;
use std::sync::Arc;
use tracing::info;

/// Component
pub struct Component {
    state: Arc<DashMap<String, String>>,
}

impl Component {
    /// Create new
    pub fn new() -> Self {
        info!("Initializing component");
        Self {
            state: Arc::new(DashMap::new()),
        }
    }

    /// Execute
    pub async fn execute(&self, cmd: &str) -> Result<String> {
        info!("Executing: {}", cmd);
        Ok(format!("Executed: {}", cmd))
    }

    /// Status
    pub fn status(&self) -> String {
        format!("Ready with {} items", self.state.len())
    }
}

impl Default for Component {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let c = Component::new();
        assert_eq!(c.state.len(), 0);
    }

    #[tokio::test]
    async fn test_execute() {
        let c = Component::new();
        assert!(c.execute("test").await.is_ok());
    }

    #[test]
    fn test_status() {
        let c = Component::new();
        assert!(!c.status().is_empty());
    }

    #[test]
    fn test_default() {
        let _ = Component::default();
    }

    #[tokio::test]
    async fn test_init() {
        let _ = Component::new();
    }

    #[test]
    fn test_state() {
        let c = Component::new();
        let s = c.status();
        assert!(s.contains("Ready"));
    }

    #[test]
    fn test_multi() {
        let c = Component::new();
        let _ = c.status();
        assert_eq!(c.state.len(), 0);
    }
}
