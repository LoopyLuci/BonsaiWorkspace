//! Error types for the backend (BUEB) hardware-detection/allocation crate.

#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    #[error("no CPU detected on this system")]
    NoCpuDetected,
    #[error("hardware profile not initialized; call initialize() first")]
    NotInitialized,
}

/// Result type used across the crate.
pub type Result<T> = std::result::Result<T, Error>;
