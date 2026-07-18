//! Error types

#[derive(Debug, Clone)]
pub enum SchedulerError {
    /// No ready thread was available to schedule
    SchedulingFailed,
    /// Referenced thread does not exist
    ThreadNotFound,
    /// Other error
    Other(String),
}

impl std::fmt::Display for SchedulerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchedulerError::SchedulingFailed => write!(f, "no ready thread available to schedule"),
            SchedulerError::ThreadNotFound => write!(f, "thread not found"),
            SchedulerError::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for SchedulerError {}

/// Result type
pub type SchedulerResult<T> = std::result::Result<T, SchedulerError>;
