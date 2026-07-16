//! Error types for the kernel-snapshot crate.

#[derive(Debug, Clone)]
pub enum KernelError {
    /// The vault registry was asked to do something inconsistent with its
    /// current state (e.g. register a vault id that already exists).
    InvalidState(String),
    /// No vault is registered under the given id.
    VaultNotFound(u64),
    /// Catch-all for anything else.
    Other(String),
}

impl std::fmt::Display for KernelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KernelError::InvalidState(msg) => write!(f, "invalid kernel state: {}", msg),
            KernelError::VaultNotFound(id) => write!(f, "vault not found: {}", id),
            KernelError::Other(msg) => write!(f, "kernel-snapshot error: {}", msg),
        }
    }
}

impl std::error::Error for KernelError {}

/// Result type used throughout the kernel-snapshot crate.
pub type Result<T> = std::result::Result<T, KernelError>;

// Backwards-compatible alias (matches the original stub's naming).
pub type Error = KernelError;
