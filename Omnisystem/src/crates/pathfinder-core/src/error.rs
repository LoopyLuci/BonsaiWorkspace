//! Error types for pathfinder-core.

#[derive(Debug, Clone)]
pub enum PathfinderError {
    /// Requested course does not exist.
    CourseNotFound(String),
    /// Requested user does not exist.
    UserNotFound(String),
    /// Progress/enrollment operation failure.
    ProgressError(String),
    /// Catch-all for other errors.
    Other(String),
}

impl std::fmt::Display for PathfinderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PathfinderError::CourseNotFound(id) => write!(f, "course not found: {}", id),
            PathfinderError::UserNotFound(id) => write!(f, "user not found: {}", id),
            PathfinderError::ProgressError(msg) => write!(f, "progress error: {}", msg),
            PathfinderError::Other(msg) => write!(f, "pathfinder error: {}", msg),
        }
    }
}

impl std::error::Error for PathfinderError {}

/// Result type used throughout pathfinder-core.
pub type Result<T> = std::result::Result<T, PathfinderError>;
