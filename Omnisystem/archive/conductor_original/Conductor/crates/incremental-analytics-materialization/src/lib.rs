//! Result materialization
#![warn(missing_docs)]

pub mod error;
pub mod types;

pub use error::{Error, Result};
pub use types::*;

use dashmap::DashMap;
use std::sync::Arc;
use tracing::info;

/// Analytics component
pub struct Analytics {
    data: Arc<DashMap<String, Vec<f64>>>,
}

impl Analytics {
    /// Create new analytics
    pub fn new() -> Self {
        info!("Initializing analytics");
        Self {
            data: Arc::new(DashMap::new()),
        }
    }

    /// Add data point
    pub fn add_point(&self, key: &str, value: f64) {
        let mut entry = self.data.entry(key.to_string()).or_insert_with(Vec::new);
        entry.push(value);
    }

    /// Analyze data
    pub async fn analyze(&self, key: &str) -> Result<String> {
        match self.data.get(key) {
            Some(values) => {
                let count = values.len();
                let sum: f64 = values.iter().sum();
                let avg = if count > 0 { sum / count as f64 } else { 0.0 };
                Ok(format!("Count: {}, Avg: {}", count, avg))
            }
            None => Ok("No data".to_string()),
        }
    }

    /// Get insights
    pub fn get_insights(&self) -> String {
        format!("Analytics ready, {} datasets", self.data.len())
    }
}

impl Default for Analytics {
    fn default() -> Self {
        Self::new()
    }
}

/// Initialize
pub async fn init() -> Result<()> {
    info!("Analytics initialized");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let a = Analytics::new();
        assert_eq!(a.data.len(), 0);
    }

    #[test]
    fn test_add_point() {
        let a = Analytics::new();
        a.add_point("data", 42.0);
        assert_eq!(a.data.len(), 1);
    }

    #[tokio::test]
    async fn test_analyze() {
        let a = Analytics::new();
        a.add_point("test", 10.0);
        a.add_point("test", 20.0);
        assert!(a.analyze("test").await.is_ok());
    }

    #[test]
    fn test_insights() {
        let a = Analytics::new();
        a.add_point("data", 1.0);
        let insights = a.get_insights();
        assert!(insights.contains("ready"));
    }

    #[test]
    fn test_default() {
        let a = Analytics::default();
        assert_eq!(a.data.len(), 0);
    }

    #[tokio::test]
    async fn test_init() {
        assert!(init().await.is_ok());
    }

    #[test]
    fn test_multiple_datasets() {
        let a = Analytics::new();
        a.add_point("set1", 1.0);
        a.add_point("set2", 2.0);
        a.add_point("set3", 3.0);
        assert_eq!(a.data.len(), 3);
    }
}
