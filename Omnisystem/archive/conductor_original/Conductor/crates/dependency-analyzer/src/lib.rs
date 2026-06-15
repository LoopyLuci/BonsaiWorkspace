//! Dependency Analyzer Analytics
#![warn(missing_docs)]
pub mod error;
pub mod types;
pub use error::{Error, Result};
pub use types::*;
use dashmap::DashMap;
use std::sync::Arc;
use tracing::info;

pub struct DEPENDENCY_ANALYZER {
    data: Arc<DashMap<String, f64>>,
}

impl DEPENDENCY_ANALYZER {
    pub fn new() -> Self {
        info!("Analytics engine initialized");
        Self {
            data: Arc::new(DashMap::new()),
        }
    }

    pub fn record(&self, key: &str, value: f64) {
        self.data.insert(key.to_string(), value);
    }

    pub fn get(&self, key: &str) -> Option<f64> {
        self.data.get(key).map(|e| *e.value())
    }

    pub fn analyze(&self) -> Option<f64> {
        if self.data.is_empty() { return None; }
        let sum: f64 = self.data.iter().map(|e| *e.value()).sum();
        Some(sum / self.data.len() as f64)
    }
}

impl Default for DEPENDENCY_ANALYZER {
    fn default() -> Self { Self::new() }
}

pub async fn init() -> Result<()> { info!("Initialized"); Ok(()) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_creation() { let e = DEPENDENCY_ANALYZER::new(); assert_eq!(e.data.len(), 0); }
    #[test]
    fn test_record() { let e = DEPENDENCY_ANALYZER::new(); e.record("k", 42.0); assert!(e.get("k").is_some()); }
    #[test]
    fn test_analyze() { let e = DEPENDENCY_ANALYZER::new(); e.record("a", 10.0); e.record("b", 20.0); assert_eq!(e.analyze(), Some(15.0)); }
    #[tokio::test]
    async fn test_init() { assert!(init().await.is_ok()); }
    #[test]
    fn test_default() { let _ = DEPENDENCY_ANALYZER::default(); }
}
