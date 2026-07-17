pub mod error;
pub mod types;
pub mod authentication;
pub mod authorization;

/// Errors produced by the authentication/authorization subsystem.
#[derive(Debug, Clone, thiserror::Error)]
pub enum AuthError {
    /// The supplied credentials were invalid.
    #[error("invalid credentials")]
    InvalidCredentials,
    /// No user matched the request.
    #[error("user not found")]
    UserNotFound,
    /// The auth token has expired.
    #[error("token expired")]
    TokenExpired,
    /// The auth token is invalid.
    #[error("token invalid")]
    TokenInvalid,
    /// The action was not permitted.
    #[error("permission denied")]
    PermissionDenied,
}

/// Result type for auth operations.
pub type AuthResult<T> = std::result::Result<T, AuthError>;

pub use types::*;
pub use authentication::AuthenticationManager;
pub use authorization::AuthorizationManager;
