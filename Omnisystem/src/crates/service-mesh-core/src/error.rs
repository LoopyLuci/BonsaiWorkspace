//! Error types

/// Errors produced by the service mesh core (registry, load balancer,
/// circuit breaker, and rate limiter)
#[derive(Debug, Clone)]
pub enum MeshError {
    /// No service (or sidecar) is registered under the given id
    ServiceNotFound(String),
    /// No sidecar proxy is registered under the given id
    SidecarNotFound(String),
    /// No healthy endpoint is available to route a request to
    EndpointUnavailable,
    /// The circuit is open for the given service and is rejecting requests
    CircuitBreakerOpen(String),
    /// The supplied configuration is invalid
    InvalidConfiguration(String),
    /// Other error
    Other(String),
}

impl std::fmt::Display for MeshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MeshError::ServiceNotFound(id) => write!(f, "service not found: {}", id),
            MeshError::SidecarNotFound(id) => write!(f, "sidecar not found: {}", id),
            MeshError::EndpointUnavailable => write!(f, "no healthy endpoint available"),
            MeshError::CircuitBreakerOpen(id) => write!(f, "circuit breaker open for: {}", id),
            MeshError::InvalidConfiguration(msg) => write!(f, "invalid configuration: {}", msg),
            MeshError::Other(msg) => write!(f, "error: {}", msg),
        }
    }
}

impl std::error::Error for MeshError {}

/// Result type
pub type MeshResult<T> = std::result::Result<T, MeshError>;
