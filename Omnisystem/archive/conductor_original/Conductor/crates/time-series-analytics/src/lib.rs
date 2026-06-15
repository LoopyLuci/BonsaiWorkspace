//! Time Series Analytics Engine
//!
//! Provides time-series data analysis and historical trend tracking

#![warn(missing_docs)]

pub mod error;
pub mod types;

pub use error::{Error, Result};
pub use types::*;

use dashmap::DashMap;
use std::sync::Arc;
use chrono::{DateTime, Utc};
use tracing::{info, debug};

/// Time series analytics engine
pub struct TimeSeriesAnalytics {
    data_points: Arc<DashMap<String, Vec<(DateTime<Utc>, f64)>>>,
}

impl TimeSeriesAnalytics {
    /// Create new analytics engine
    pub fn new() -> Self {
        info!("Initializing Time Series Analytics");
        Self {
            data_points: Arc::new(DashMap::new()),
        }
    }

    /// Add data point to series
    pub fn add_point(&self, series: &str, value: f64) {
        debug!("Adding data point to series: {}", series);
        let timestamp = Utc::now();
        self.data_points
            .entry(series.to_string())
            .or_insert_with(Vec::new)
            .push((timestamp, value));
    }

    /// Get series data
    pub fn get_series(&self, series: &str) -> Option<Vec<(DateTime<Utc>, f64)>> {
        self.data_points.get(series).map(|entry| entry.clone())
    }

    /// Calculate trend
    pub fn calculate_trend(&self, series: &str) -> Option<f64> {
        debug!("Calculating trend for series: {}", series);
        self.get_series(series).and_then(|points| {
            if points.len() < 2 {
                return None;
            }
            let first = points.first().unwrap().1;
            let last = points.last().unwrap().1;
            Some((last - first) / first * 100.0)
        })
    }

    /// Get average over time period
    pub fn get_average(&self, series: &str) -> Option<f64> {
        debug!("Calculating average for series: {}", series);
        self.get_series(series).and_then(|points| {
            if points.is_empty() {
                return None;
            }
            let sum: f64 = points.iter().map(|(_, v)| v).sum();
            Some(sum / points.len() as f64)
        })
    }

    /// Clear data
    pub fn clear(&self) {
        debug!("Clearing all data");
        self.data_points.clear();
    }
}

impl Default for TimeSeriesAnalytics {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn init() -> Result<()> {
    info!("Time Series Analytics initialized");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_creation() {
        let engine = TimeSeriesAnalytics::new();
        assert_eq!(engine.data_points.len(), 0);
    }

    #[test]
    fn test_add_point() {
        let engine = TimeSeriesAnalytics::new();
        engine.add_point("cpu", 45.5);
        assert!(engine.get_series("cpu").is_some());
    }

    #[test]
    fn test_get_series() {
        let engine = TimeSeriesAnalytics::new();
        engine.add_point("memory", 512.0);
        let series = engine.get_series("memory");
        assert!(series.is_some());
        assert_eq!(series.unwrap().len(), 1);
    }

    #[test]
    fn test_calculate_trend() {
        let engine = TimeSeriesAnalytics::new();
        engine.add_point("temp", 100.0);
        engine.add_point("temp", 120.0);
        let trend = engine.calculate_trend("temp");
        assert!(trend.is_some());
    }

    #[test]
    fn test_get_average() {
        let engine = TimeSeriesAnalytics::new();
        engine.add_point("latency", 10.0);
        engine.add_point("latency", 20.0);
        engine.add_point("latency", 30.0);
        let avg = engine.get_average("latency");
        assert_eq!(avg, Some(20.0));
    }

    #[test]
    fn test_clear() {
        let engine = TimeSeriesAnalytics::new();
        engine.add_point("test", 42.0);
        engine.clear();
        assert_eq!(engine.data_points.len(), 0);
    }

    #[test]
    fn test_default() {
        let engine = TimeSeriesAnalytics::default();
        assert_eq!(engine.data_points.len(), 0);
    }

    #[tokio::test]
    async fn test_init() {
        assert!(init().await.is_ok());
    }

    #[test]
    fn test_multiple_series() {
        let engine = TimeSeriesAnalytics::new();
        engine.add_point("cpu", 45.0);
        engine.add_point("memory", 512.0);
        engine.add_point("disk", 1024.0);
        assert_eq!(engine.data_points.len(), 3);
    }
}
