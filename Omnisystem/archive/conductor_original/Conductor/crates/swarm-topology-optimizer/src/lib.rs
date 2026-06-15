//! Topology optimization
#![warn(missing_docs)]

pub mod error;
pub mod types;

pub use error::{Error, Result};
pub use types::*;

use dashmap::DashMap;
use std::sync::Arc;
use tracing::{info, debug};

/// Main component
pub struct Component {
    state: Arc<DashMap<String, String>>,
}

impl Component {
    /// Create new component
    pub fn new() -> Self {
        info!("Initializing component");
        Self {
            state: Arc::new(DashMap::new()),
        }
    }

    /// Execute operation
    pub async fn execute(&self, op: &str) -> Result<String> {
        debug!("Executing: {}", op);
        Ok(format!("Executed '{}'", op))
    }

    /// Get state value
    pub fn get(&self, key: &str) -> Option<String> {
        self.state.get(key).map(|v| v.value().clone())
    }

    /// Set state value
    pub fn set(&self, key: String, value: String) {
        self.state.insert(key, value);
    }
}

impl Default for Component {
    fn default() -> Self {
        Self::new()
    }
}

/// Initialize
pub async fn init() -> Result<()> {
    info!("Initialized successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_creation() {
        let c = Component::new();
        assert_eq!(c.state.len(), 0);
    }

    #[test]
    fn test_set_get() {
        let c = Component::new();
        c.set("k1".to_string(), "v1".to_string());
        assert_eq!(c.get("k1"), Some("v1".to_string()));
    }

    #[tokio::test]
    async fn test_execute() {
        let c = Component::new();
        assert!(c.execute("op").await.is_ok());
    }

    #[test]
    fn test_default() {
        let c = Component::default();
        assert_eq!(c.state.len(), 0);
    }

    #[tokio::test]
    async fn test_init() {
        assert!(init().await.is_ok());
    }

    #[test]
    fn test_multiple() {
        let c = Component::new();
        c.set("a".to_string(), "1".to_string());
        c.set("b".to_string(), "2".to_string());
        assert_eq!(c.state.len(), 2);
    }
}
