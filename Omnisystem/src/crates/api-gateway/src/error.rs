//! Error types for the api-gateway crate.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GatewayError {
    /// No route/rule is registered for the requested path.
    RouteNotFound,
    /// Catch-all for anything else.
    Other(String),
}

impl std::fmt::Display for GatewayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GatewayError::RouteNotFound => write!(f, "route not found"),
            GatewayError::Other(msg) => write!(f, "api-gateway error: {}", msg),
        }
    }
}

impl std::error::Error for GatewayError {}

/// Result type used throughout the api-gateway crate.
pub type GatewayResult<T> = std::result::Result<T, GatewayError>;

// Backwards-compatible aliases (matches the original stub's naming).
pub type Error = GatewayError;
pub type Result<T> = GatewayResult<T>;
