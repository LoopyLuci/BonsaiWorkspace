use std::fmt;

#[derive(Debug, Clone)]
pub enum ModuleError {
    NotFound(String),
    AlreadyLoaded(String),
    NotLoaded(String),
    InitializationFailed(String),
    ExecutionFailed(String),
    ShutdownFailed(String),
    DependencyNotFound(String),
    VersionMismatch(String),
    ConfigurationError(String),
    Timeout(String),
    InternalError(String),
    PermissionDenied(String),
    ResourceExhausted(String),
    InvalidState(String),
    Unknown(String),
}

impl fmt::Display for ModuleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModuleError::NotFound(msg) => write!(f, "Module not found: {}", msg),
            ModuleError::AlreadyLoaded(msg) => write!(f, "Module already loaded: {}", msg),
            ModuleError::NotLoaded(msg) => write!(f, "Module not loaded: {}", msg),
            ModuleError::InitializationFailed(msg) => write!(f, "Initialization failed: {}", msg),
            ModuleError::ExecutionFailed(msg) => write!(f, "Execution failed: {}", msg),
            ModuleError::ShutdownFailed(msg) => write!(f, "Shutdown failed: {}", msg),
            ModuleError::DependencyNotFound(msg) => write!(f, "Dependency not found: {}", msg),
            ModuleError::VersionMismatch(msg) => write!(f, "Version mismatch: {}", msg),
            ModuleError::ConfigurationError(msg) => write!(f, "Configuration error: {}", msg),
            ModuleError::Timeout(msg) => write!(f, "Timeout: {}", msg),
            ModuleError::InternalError(msg) => write!(f, "Internal error: {}", msg),
            ModuleError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            ModuleError::ResourceExhausted(msg) => write!(f, "Resource exhausted: {}", msg),
            ModuleError::InvalidState(msg) => write!(f, "Invalid state: {}", msg),
            ModuleError::Unknown(msg) => write!(f, "Unknown error: {}", msg),
        }
    }
}

impl std::error::Error for ModuleError {}

pub type Result<T> = std::result::Result<T, ModuleError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_error_not_found() {
        let err = ModuleError::NotFound("test".to_string());
        assert_eq!(err.to_string(), "Module not found: test");
    }

    #[test]
    fn test_module_error_already_loaded() {
        let err = ModuleError::AlreadyLoaded("test".to_string());
        assert_eq!(err.to_string(), "Module already loaded: test");
    }

    #[test]
    fn test_module_error_initialization_failed() {
        let err = ModuleError::InitializationFailed("test".to_string());
        assert_eq!(err.to_string(), "Initialization failed: test");
    }

    #[test]
    fn test_module_error_execution_failed() {
        let err = ModuleError::ExecutionFailed("test".to_string());
        assert_eq!(err.to_string(), "Execution failed: test");
    }

    #[test]
    fn test_module_error_shutdown_failed() {
        let err = ModuleError::ShutdownFailed("test".to_string());
        assert_eq!(err.to_string(), "Shutdown failed: test");
    }

    #[test]
    fn test_module_error_dependency_not_found() {
        let err = ModuleError::DependencyNotFound("test".to_string());
        assert_eq!(err.to_string(), "Dependency not found: test");
    }

    #[test]
    fn test_module_error_version_mismatch() {
        let err = ModuleError::VersionMismatch("test".to_string());
        assert_eq!(err.to_string(), "Version mismatch: test");
    }

    #[test]
    fn test_module_error_configuration_error() {
        let err = ModuleError::ConfigurationError("test".to_string());
        assert_eq!(err.to_string(), "Configuration error: test");
    }

    #[test]
    fn test_module_error_timeout() {
        let err = ModuleError::Timeout("test".to_string());
        assert_eq!(err.to_string(), "Timeout: test");
    }

    #[test]
    fn test_module_error_internal_error() {
        let err = ModuleError::InternalError("test".to_string());
        assert_eq!(err.to_string(), "Internal error: test");
    }

    #[test]
    fn test_module_error_permission_denied() {
        let err = ModuleError::PermissionDenied("test".to_string());
        assert_eq!(err.to_string(), "Permission denied: test");
    }

    #[test]
    fn test_module_error_resource_exhausted() {
        let err = ModuleError::ResourceExhausted("test".to_string());
        assert_eq!(err.to_string(), "Resource exhausted: test");
    }

    #[test]
    fn test_module_error_invalid_state() {
        let err = ModuleError::InvalidState("test".to_string());
        assert_eq!(err.to_string(), "Invalid state: test");
    }

    #[test]
    fn test_module_error_unknown() {
        let err = ModuleError::Unknown("test".to_string());
        assert_eq!(err.to_string(), "Unknown error: test");
    }
}
