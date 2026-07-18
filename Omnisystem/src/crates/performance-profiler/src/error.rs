//! Error types for the CPU profiler.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfilerError {
    /// An analysis operation was given an empty sample set.
    InvalidSampleCount,
    /// Any other profiler error.
    Other(String),
}

impl std::fmt::Display for ProfilerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProfilerError::InvalidSampleCount => {
                write!(f, "cannot analyze an empty set of CPU samples")
            }
            ProfilerError::Other(msg) => write!(f, "profiler error: {}", msg),
        }
    }
}

impl std::error::Error for ProfilerError {}

/// Result type used throughout the profiler.
pub type ProfilerResult<T> = std::result::Result<T, ProfilerError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_messages() {
        assert_eq!(
            ProfilerError::InvalidSampleCount.to_string(),
            "cannot analyze an empty set of CPU samples"
        );
    }
}
