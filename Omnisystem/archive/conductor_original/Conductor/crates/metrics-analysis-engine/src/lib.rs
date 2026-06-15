//! Metrics analysis
#![warn(missing_docs)]

pub mod error;
pub mod types;

pub use error::{Error, Result};
pub use types::*;

use dashmap::DashMap;
use std::sync::Arc;
use tracing::info;

/// Operations component
pub struct Operations {
    config: Arc<DashMap<String, String>>,
}

impl Operations {
    /// Create new operations
    pub fn new() -> Self {
        info!("Initializing operations");
        Self {
            config: Arc::new(DashMap::new()),
        }
    }

    /// Execute operation
    pub async fn execute(&self, op: &str) -> Result<String> {
        Ok(format!("Executed '{}'", op))
    }

    /// Get config
    pub fn get_config(&self, key: &str) -> Option<String> {
        self.config.get(key).map(|v| v.value().clone())
    }

    /// Set config
    pub fn set_config(&self, key: String, value: String) {
        self.config.insert(key, value);
    }
}

impl Default for Operations {
    fn default() -> Self {
        Self::new()
    }
}

/// Initialize
pub async fn init() -> Result<()> {
    info!("Operations initialized");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let ops = Operations::new();
        assert_eq!(ops.config.len(), 0);
    }

    #[test]
    fn test_set_get() {
        let ops = Operations::new();
        ops.set_config("k".to_string(), "v".to_string());
        assert_eq!(ops.get_config("k"), Some("v".to_string()));
    }

    #[tokio::test]
    async fn test_execute() {
        let ops = Operations::new();
        assert!(ops.execute("op").await.is_ok());
    }

    #[test]
    fn test_default() {
        let ops = Operations::default();
        assert_eq!(ops.config.len(), 0);
    }

    #[tokio::test]
    async fn test_init() {
        assert!(init().await.is_ok());
    }

    #[test]
    fn test_multiple() {
        let ops = Operations::new();
        ops.set_config("a".to_string(), "1".to_string());
        ops.set_config("b".to_string(), "2".to_string());
        assert_eq!(ops.config.len(), 2);
    }

    #[test]
    fn test_get_missing() {
        let ops = Operations::new();
        assert_eq!(ops.get_config("missing"), None);
    }
}
