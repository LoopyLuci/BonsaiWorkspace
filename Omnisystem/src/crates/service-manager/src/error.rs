//! Error types for the Service Lifecycle Manager

/// Errors that can occur while managing a service's lifecycle
#[derive(Debug, thiserror::Error)]
pub enum SLMError {
    /// Vault creation failed on the kernel side
    #[error("Vault creation failed: {0}")]
    VaultCreationFailed(String),

    /// Operation attempted on a service that is not running
    #[error("Service not running: {0}")]
    ServiceNotRunning(String),

    /// Snapshotting a vault failed
    #[error("Snapshot failed: {0}")]
    SnapshotFailed(String),

    /// Restoring a vault from a snapshot failed
    #[error("Restore failed: {0}")]
    RestoreFailed(String),

    /// The requested service was not found in the registry
    #[error("Service not found: {0}")]
    ServiceNotFound(String),

    /// The service manifest failed validation
    #[error("Manifest error: {0}")]
    ManifestError(String),
}

/// Result type used throughout the service-manager crate
pub type Result<T> = std::result::Result<T, SLMError>;
