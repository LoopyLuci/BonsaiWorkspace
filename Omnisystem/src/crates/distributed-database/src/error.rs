//! Error types

#[derive(Debug, Clone)]
pub enum DatabaseError {
    /// Referenced table does not exist
    TableNotFound,
    /// Referenced shard does not exist
    ShardNotFound,
    /// Query routing failed: no shards are registered for the table
    RoutingFailed,
    /// Other error
    Other(String),
}

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseError::TableNotFound => write!(f, "table not found"),
            DatabaseError::ShardNotFound => write!(f, "shard not found"),
            DatabaseError::RoutingFailed => write!(f, "cannot route query: no shards registered for this table"),
            DatabaseError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for DatabaseError {}

/// Result type
pub type DatabaseResult<T> = std::result::Result<T, DatabaseError>;
