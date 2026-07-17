pub mod error;
pub mod types;
pub mod metrics;
pub mod analytics;

/// Errors produced by the monitoring subsystem.
#[derive(Debug, Clone, thiserror::Error)]
pub enum MonitoringError {
    /// Analytics computation failed.
    #[error("analysis failed")]
    AnalysisFailed,
    /// Requested metric was not found.
    #[error("metric not found")]
    MetricNotFound,
}

/// Result type for monitoring operations.
pub type MonitoringResult<T> = std::result::Result<T, MonitoringError>;

pub use types::*;
pub use metrics::MetricsCollector;
pub use analytics::AnalyticsEngine;
