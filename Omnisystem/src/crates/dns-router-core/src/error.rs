//! DNS-specific error types.

use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum DnsError {
    #[error("zone not found: {0}")]
    ZoneNotFound(String),
    #[error("zone already exists: {0}")]
    ZoneAlreadyExists(String),
    #[error("record not found: {0}")]
    RecordNotFound(String),
    #[error("no healthy servers available for: {0}")]
    NoHealthyServers(String),
    #[error("geo location not found for domain: {0}")]
    GeoLocationNotFound(String),
    #[error("dynamic update failed: {0}")]
    DynamicUpdateFailed(String),
}

pub type DnsResult<T> = std::result::Result<T, DnsError>;
