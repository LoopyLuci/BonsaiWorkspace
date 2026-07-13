//! Error types

#[derive(Debug, Clone)]
pub enum DatabaseError {
    /// Database not found
    DatabaseNotFound(String),
    /// Database already exists
    DatabaseAlreadyExists(String),
    /// Connection failed
    ConnectionFailed(String),
    /// Invalid configuration
    InvalidConfiguration(String),
    /// Replication failed
    ReplicationFailed(String),
    /// Connection pool exhausted
    PoolExhausted,
    /// Other error
    Other(String),
}

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseError::DatabaseNotFound(id) => write!(f, "database not found: {}", id),
            DatabaseError::DatabaseAlreadyExists(name) => write!(f, "database already exists: {}", name),
            DatabaseError::ConnectionFailed(msg) => write!(f, "connection failed: {}", msg),
            DatabaseError::InvalidConfiguration(msg) => write!(f, "invalid configuration: {}", msg),
            DatabaseError::ReplicationFailed(msg) => write!(f, "replication failed: {}", msg),
            DatabaseError::PoolExhausted => write!(f, "connection pool exhausted"),
            DatabaseError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for DatabaseError {}

/// Result type
pub type DatabaseResult<T> = std::result::Result<T, DatabaseError>;
