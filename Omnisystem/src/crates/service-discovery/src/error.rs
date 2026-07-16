//! Error types for the service discovery crate.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryError {
    /// A service instance with the same id is already registered.
    ServiceAlreadyRegistered,
    /// No service (or no instance) found under the given name/id.
    ServiceNotFound,
    /// Load balancing could not select an instance (e.g. empty instance list).
    LoadBalancingFailed,
    /// Catch-all for anything else.
    Other(String),
}

impl std::fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiscoveryError::ServiceAlreadyRegistered => {
                write!(f, "service instance is already registered")
            }
            DiscoveryError::ServiceNotFound => write!(f, "service not found"),
            DiscoveryError::LoadBalancingFailed => {
                write!(f, "load balancing failed: no instances available")
            }
            DiscoveryError::Other(msg) => write!(f, "service discovery error: {}", msg),
        }
    }
}

impl std::error::Error for DiscoveryError {}

/// Result type used throughout the service discovery crate.
pub type DiscoveryResult<T> = std::result::Result<T, DiscoveryError>;

// Backwards-compatible aliases (matches the original stub's naming used by `manager.rs`).
pub type Error = DiscoveryError;
pub type Result<T> = DiscoveryResult<T>;
