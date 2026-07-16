//! Error types for the memory optimizer.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryError {
    /// Attempted to deallocate a block id that isn't currently allocated.
    InvalidPointer,
    /// A memory pool has no free blocks left.
    PoolExhausted,
    /// A block was returned to a fixed-size pool with the wrong size.
    BlockSizeMismatch,
    /// An allocation/deallocation/lookup failed (e.g. unknown pool id).
    AllocationFailed,
    /// Any other memory error.
    Other(String),
}

impl std::fmt::Display for MemoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryError::InvalidPointer => write!(f, "invalid or already-freed block pointer"),
            MemoryError::PoolExhausted => write!(f, "memory pool has no free blocks"),
            MemoryError::BlockSizeMismatch => {
                write!(f, "returned block size does not match pool's block size")
            }
            MemoryError::AllocationFailed => write!(f, "allocation failed"),
            MemoryError::Other(msg) => write!(f, "memory error: {}", msg),
        }
    }
}

impl std::error::Error for MemoryError {}

/// Result type used throughout the memory optimizer.
pub type MemoryResult<T> = std::result::Result<T, MemoryError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_messages() {
        assert_eq!(
            MemoryError::PoolExhausted.to_string(),
            "memory pool has no free blocks"
        );
    }
}
