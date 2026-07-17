//! Error types for traffic management

use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum TrafficError {
    #[error("routing policy not found")]
    PolicyNotFound,

    #[error("invalid weight: must be 0-100")]
    InvalidWeight,

    #[error("canary deployment not found")]
    CanaryDeploymentFailed,

    #[error("no destinations registered for this service")]
    NoDestinationsAvailable,

    #[error("other error: {0}")]
    Other(String),
}

pub type TrafficResult<T> = std::result::Result<T, TrafficError>;
