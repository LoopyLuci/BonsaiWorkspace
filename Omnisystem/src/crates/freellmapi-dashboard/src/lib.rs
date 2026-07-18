//! FreeLLMAPI Dashboard - in-memory per-tenant and per-provider usage metrics
//! aggregation with axum HTTP handlers for health and metrics endpoints.

pub mod handlers;
pub mod metrics;

pub use handlers::{health_handler, metrics_handler};
pub use metrics::DashboardMetrics;
