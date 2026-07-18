//! Error types for app-manager-security

#[derive(Debug, Clone)]
pub enum SecurityError {
    /// Caller has no policy / is not allowed to access the resource
    AccessDenied(String),
    /// Caller has a policy but lacks the specific permission requested
    PermissionDenied(String),
    /// A sandbox rule was violated
    SandboxViolation(String),
    /// A cryptographic operation (hashing/HMAC) failed
    CryptoError(String),
}

impl std::fmt::Display for SecurityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecurityError::AccessDenied(msg) => write!(f, "access denied: {}", msg),
            SecurityError::PermissionDenied(msg) => write!(f, "permission denied: {}", msg),
            SecurityError::SandboxViolation(msg) => write!(f, "sandbox violation: {}", msg),
            SecurityError::CryptoError(msg) => write!(f, "crypto error: {}", msg),
        }
    }
}

impl std::error::Error for SecurityError {}

/// Result type
pub type Result<T> = std::result::Result<T, SecurityError>;
