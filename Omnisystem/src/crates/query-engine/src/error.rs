//! Error types for the query engine

use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum QueryError {
    #[error("query execution failed: query not found")]
    ExecutionFailed,

    #[error("query planning failed: plan not found")]
    PlanningFailed,

    #[error("other error: {0}")]
    Other(String),
}

pub type QueryResult<T> = std::result::Result<T, QueryError>;
