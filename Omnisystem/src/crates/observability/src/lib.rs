//! observability
//!
//! Metrics collection (Prometheus/JSON export), alerting, SLA tracking,
//! tracing initialization, and dashboard configuration (Grafana export),
//! unified behind [`ObservabilityStack`].

pub mod alerts;
pub mod core;
pub mod dashboard;
pub mod error;
pub mod metrics;
pub mod sla;
pub mod stack;
pub mod tracing;
pub mod types;

pub use alerts::{Alert, AlertComparison, AlertEngine, AlertRule, AlertSeverity};
pub use core::Core;
pub use dashboard::{DashboardConfig, Panel, PanelType, Position};
pub use error::{Error, Result};
pub use metrics::{MetricPoint, MetricsCollector};
pub use sla::{SLACompliance, SLAObservation, SLATarget, SLATracker};
pub use stack::ObservabilityStack;
pub use tracing::{init_jaeger, init_tracing, JaegerConfig};
pub use types::State;
