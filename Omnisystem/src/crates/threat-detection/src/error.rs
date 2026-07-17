//! Error types for threat detection

use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum ThreatError {
    #[error("incident not found")]
    IncidentNotFound,

    #[error("other error: {0}")]
    Other(String),
}

pub type ThreatResult<T> = std::result::Result<T, ThreatError>;
