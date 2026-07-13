//! Aggregator-specific error types.

use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum AggregatorError {
    #[error("aggregation failed: {0}")]
    AggregationFailed(String),
    #[error("metric not found: {0}")]
    MetricNotFound(String),
}

pub type AggregatorResult<T> = std::result::Result<T, AggregatorError>;
