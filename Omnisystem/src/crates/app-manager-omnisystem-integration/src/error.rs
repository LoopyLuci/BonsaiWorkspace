//! Error types for app-manager-omnisystem-integration
//!
//! Wraps the error types of the crates this integration layer orchestrates
//! (`app-manager-repository`, `app-manager-installer`, `app-manager-core`)
//! plus a small set of orchestration-specific error conditions.

use app_manager_core::error::AppManagerError;
use app_manager_installer::InstallerError;
use app_manager_repository::RepositoryError;

#[derive(Debug)]
pub enum AppIntegrationError {
    /// A repository-level operation (fetch/verify/list) failed.
    Repository(RepositoryError),
    /// An installer-level operation (install/uninstall/update/rollback) failed.
    Installer(InstallerError),
    /// A module lifecycle state-machine operation failed.
    Lifecycle(AppManagerError),
    /// `start_application` was called on an application already in the `Running` state.
    AlreadyRunning(String),
    /// An operation that requires `initialize()` to have run was attempted first.
    NotInitialized,
    /// A filesystem operation performed directly by the integration layer failed.
    Io(std::io::Error),
}

impl std::fmt::Display for AppIntegrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppIntegrationError::Repository(e) => write!(f, "repository error: {}", e),
            AppIntegrationError::Installer(e) => write!(f, "installer error: {}", e),
            AppIntegrationError::Lifecycle(e) => write!(f, "lifecycle error: {}", e),
            AppIntegrationError::AlreadyRunning(id) => {
                write!(f, "application already running: {}", id)
            }
            AppIntegrationError::NotInitialized => {
                write!(f, "application manager has not been initialized")
            }
            AppIntegrationError::Io(e) => write!(f, "I/O error: {}", e),
        }
    }
}

impl std::error::Error for AppIntegrationError {}

impl From<RepositoryError> for AppIntegrationError {
    fn from(e: RepositoryError) -> Self {
        AppIntegrationError::Repository(e)
    }
}

impl From<InstallerError> for AppIntegrationError {
    fn from(e: InstallerError) -> Self {
        AppIntegrationError::Installer(e)
    }
}

impl From<AppManagerError> for AppIntegrationError {
    fn from(e: AppManagerError) -> Self {
        AppIntegrationError::Lifecycle(e)
    }
}

impl From<std::io::Error> for AppIntegrationError {
    fn from(e: std::io::Error) -> Self {
        AppIntegrationError::Io(e)
    }
}

/// Result type
pub type Result<T> = std::result::Result<T, AppIntegrationError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_variants() {
        assert!(AppIntegrationError::NotInitialized.to_string().contains("not been initialized"));
        assert!(AppIntegrationError::AlreadyRunning("x".into())
            .to_string()
            .contains("already running"));
    }

    #[test]
    fn test_from_repository_error() {
        let e: AppIntegrationError = RepositoryError::NotFound("x".into()).into();
        assert!(matches!(e, AppIntegrationError::Repository(_)));
    }

    #[test]
    fn test_from_installer_error() {
        let e: AppIntegrationError = InstallerError::InstallationFailed("x".into()).into();
        assert!(matches!(e, AppIntegrationError::Installer(_)));
    }

    #[test]
    fn test_from_lifecycle_error() {
        let e: AppIntegrationError = AppManagerError::ModuleNotLoaded("x".into()).into();
        assert!(matches!(e, AppIntegrationError::Lifecycle(_)));
    }

    #[test]
    fn test_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "missing");
        let e: AppIntegrationError = io_err.into();
        assert!(matches!(e, AppIntegrationError::Io(_)));
    }
}
