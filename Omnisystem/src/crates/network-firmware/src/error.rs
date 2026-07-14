//! Error types for network-firmware.

#[derive(Debug, Clone)]
pub enum NetworkError {
    /// Packet processing failure.
    PacketError(String),
    /// MAC address lookup failure.
    MACLookupFailed(String),
    /// Routing table failure.
    RoutingError(String),
    /// Catch-all for other errors.
    Other(String),
}

impl std::fmt::Display for NetworkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetworkError::PacketError(msg) => write!(f, "packet error: {}", msg),
            NetworkError::MACLookupFailed(msg) => write!(f, "MAC lookup failed: {}", msg),
            NetworkError::RoutingError(msg) => write!(f, "routing error: {}", msg),
            NetworkError::Other(msg) => write!(f, "network error: {}", msg),
        }
    }
}

impl std::error::Error for NetworkError {}

/// Result type used throughout network-firmware.
pub type Result<T> = std::result::Result<T, NetworkError>;
