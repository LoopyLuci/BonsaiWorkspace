//! Error types for omnisystem-cluster.

#[derive(Debug, Clone)]
pub enum ClusterError {
    /// Consensus/election protocol failure.
    Consensus(String),
    /// Networking, encoding, or I/O-adjacent failure.
    Network(String),
    /// Catch-all for other cluster errors.
    Other(String),
}

impl std::fmt::Display for ClusterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClusterError::Consensus(msg) => write!(f, "consensus error: {}", msg),
            ClusterError::Network(msg) => write!(f, "network error: {}", msg),
            ClusterError::Other(msg) => write!(f, "cluster error: {}", msg),
        }
    }
}

impl std::error::Error for ClusterError {}

impl From<serde_json::Error> for ClusterError {
    fn from(err: serde_json::Error) -> Self {
        ClusterError::Other(format!("serialization error: {}", err))
    }
}

/// Result type used throughout omnisystem-cluster.
pub type Result<T> = std::result::Result<T, ClusterError>;
