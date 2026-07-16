//! Error types for the buddy crate.

#[derive(Debug, Clone)]
pub enum BuddyError {
    /// A requested capability isn't registered.
    CapabilityNotFound(String),
    /// Catch-all for anything else.
    Other(String),
}

impl std::fmt::Display for BuddyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuddyError::CapabilityNotFound(name) => write!(f, "capability not found: {}", name),
            BuddyError::Other(msg) => write!(f, "buddy error: {}", msg),
        }
    }
}

impl std::error::Error for BuddyError {}

/// Result type used throughout the buddy crate.
pub type Result<T> = std::result::Result<T, BuddyError>;

// Backwards-compatible alias (matches the original stub's naming).
pub type Error = BuddyError;
