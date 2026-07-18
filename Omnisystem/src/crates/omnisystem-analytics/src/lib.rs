//! omnisystem-analytics: a small in-process metrics collector with real
//! min/max/avg aggregation, plus an ASCII-art status dashboard.

pub mod collector;
pub mod dashboard;
pub mod metrics;

pub use collector::MetricsCollector;
pub use dashboard::Dashboard;
pub use metrics::{MetricPoint, MetricType, Metrics};
