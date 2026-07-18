//! Error types for the runtime.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    /// A `ResourcePool` could not satisfy an allocation request.
    ResourceExhausted(String),
    /// A task failed while executing.
    TaskFailed(String),
    /// A task with the given id could not be found.
    TaskNotFound(String),
    /// Any other runtime error.
    Other(String),
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeError::ResourceExhausted(msg) => write!(f, "resource exhausted: {}", msg),
            RuntimeError::TaskFailed(msg) => write!(f, "task failed: {}", msg),
            RuntimeError::TaskNotFound(msg) => write!(f, "task not found: {}", msg),
            RuntimeError::Other(msg) => write!(f, "runtime error: {}", msg),
        }
    }
}

impl std::error::Error for RuntimeError {}

/// Result type used throughout the runtime.
pub type Result<T> = std::result::Result<T, RuntimeError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_messages() {
        assert_eq!(
            RuntimeError::ResourceExhausted("need 5".into()).to_string(),
            "resource exhausted: need 5"
        );
        assert_eq!(
            RuntimeError::TaskNotFound("t1".into()).to_string(),
            "task not found: t1"
        );
    }
}
