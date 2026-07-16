//! performance-monitor: system metrics recording/averaging, percentile
//! and moving-average aggregation, threshold alerting, and reporting.

pub mod alerts;
pub mod error;
pub mod metrics;
pub mod monitor;
pub mod reporting;

pub use alerts::{Alert, AlertManager};
pub use error::{MonitorError, Result};
pub use metrics::MetricsAggregator;
pub use monitor::{PerformanceMonitor, SystemMetrics};
pub use reporting::{PerformanceReport, ReportGenerator};
