//! Error types for usee-search.

#[derive(Debug, Clone)]
pub enum SearchError {
    /// Requested resource was not found.
    NotFound,
    /// Query parsing/validation failure.
    QueryError(String),
    /// Indexing/sharding failure.
    IndexError(String),
    /// Catch-all for other errors.
    Other(String),
}

impl std::fmt::Display for SearchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchError::NotFound => write!(f, "not found"),
            SearchError::QueryError(msg) => write!(f, "query error: {}", msg),
            SearchError::IndexError(msg) => write!(f, "index error: {}", msg),
            SearchError::Other(msg) => write!(f, "search error: {}", msg),
        }
    }
}

impl std::error::Error for SearchError {}

/// Result type used throughout usee-search.
pub type Result<T> = std::result::Result<T, SearchError>;
