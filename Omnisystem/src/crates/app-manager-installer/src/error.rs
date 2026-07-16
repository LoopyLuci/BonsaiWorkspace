//! Error types for app-manager-installer

#[derive(Debug)]
pub enum InstallerError {
    /// Registering/downloading/installing a module through its lifecycle failed
    InstallationFailed(String),
    /// Fetching the package or its manifest failed
    DownloadFailed(String),
    /// Package verification failed
    VerificationFailed(String),
    /// Snapshot creation or restoration failed
    RollbackFailed(String),
    /// Dependency resolution failed
    DependencyResolutionFailed(String),
    /// Local filesystem I/O error
    IoError(std::io::Error),
    /// Internal invariant violation
    Internal(String),
}

impl std::fmt::Display for InstallerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstallerError::InstallationFailed(msg) => write!(f, "installation failed: {}", msg),
            InstallerError::DownloadFailed(msg) => write!(f, "download failed: {}", msg),
            InstallerError::VerificationFailed(msg) => write!(f, "verification failed: {}", msg),
            InstallerError::RollbackFailed(msg) => write!(f, "rollback failed: {}", msg),
            InstallerError::DependencyResolutionFailed(msg) => {
                write!(f, "dependency resolution failed: {}", msg)
            }
            InstallerError::IoError(e) => write!(f, "I/O error: {}", e),
            InstallerError::Internal(msg) => write!(f, "internal error: {}", msg),
        }
    }
}

impl std::error::Error for InstallerError {}

impl From<std::io::Error> for InstallerError {
    fn from(e: std::io::Error) -> Self {
        InstallerError::IoError(e)
    }
}

/// Result type
pub type Result<T> = std::result::Result<T, InstallerError>;
