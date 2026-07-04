//! Error types and handling for UnixCC

use std::fmt;
use std::io;
use std::path::PathBuf;

/// Result type alias for UnixCC operations
pub type Result<T> = std::result::Result<T, Error>;

/// UnixCC error types
#[derive(Debug)]
pub enum Error {
    /// Configuration error
    Config(String),

    /// Language detection failed
    LanguageDetection {
        path: PathBuf,
        reason: String,
    },

    /// Compiler not found or not installed
    CompilerNotFound {
        compiler: String,
        language: String,
    },

    /// Compilation error from compiler
    CompilationFailed {
        language: String,
        message: String,
        output: String,
    },

    /// Cache operation error
    CacheError(String),

    /// I/O error
    Io(io::Error),

    /// Serialization error
    Serialization(String),

    /// Build error
    BuildError(String),

    /// Dependency error
    DependencyError {
        message: String,
        depends_on: String,
    },

    /// Parallelization error
    ParallelizationError(String),

    /// Invalid target
    InvalidTarget(String),

    /// Distributed build error
    DistributedBuildError {
        worker: String,
        error: String,
    },

    /// Generic error
    Other(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Config(msg) => write!(f, "Configuration error: {}", msg),
            Error::LanguageDetection { path, reason } => {
                write!(f, "Failed to detect language for {}: {}", path.display(), reason)
            }
            Error::CompilerNotFound { compiler, language } => {
                write!(f, "{} compiler '{}' not found. Please install it.", language, compiler)
            }
            Error::CompilationFailed {
                language,
                message,
                ..
            } => {
                write!(f, "{} compilation failed: {}", language, message)
            }
            Error::CacheError(msg) => write!(f, "Cache error: {}", msg),
            Error::Io(err) => write!(f, "I/O error: {}", err),
            Error::Serialization(msg) => write!(f, "Serialization error: {}", msg),
            Error::BuildError(msg) => write!(f, "Build error: {}", msg),
            Error::DependencyError {
                message,
                depends_on,
            } => {
                write!(
                    f,
                    "Dependency error: {} (depends on {})",
                    message, depends_on
                )
            }
            Error::ParallelizationError(msg) => write!(f, "Parallelization error: {}", msg),
            Error::InvalidTarget(target) => write!(f, "Invalid target: {}", target),
            Error::DistributedBuildError { worker, error } => {
                write!(f, "Distributed build error on {}: {}", worker, error)
            }
            Error::Other(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for Error {}

// Conversions from standard library types
impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Serialization(err.to_string())
    }
}

impl From<toml::ser::Error> for Error {
    fn from(err: toml::ser::Error) -> Self {
        Error::Serialization(err.to_string())
    }
}

impl From<toml::de::Error> for Error {
    fn from(err: toml::de::Error) -> Self {
        Error::Serialization(err.to_string())
    }
}

/// Helper to create config errors
pub fn config_error(msg: impl Into<String>) -> Error {
    Error::Config(msg.into())
}

/// Helper to create compilation errors
pub fn compile_error(language: impl Into<String>, message: impl Into<String>) -> Error {
    Error::CompilationFailed {
        language: language.into(),
        message: message.into(),
        output: String::new(),
    }
}

/// Helper to create build errors
pub fn build_error(msg: impl Into<String>) -> Error {
    Error::BuildError(msg.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = Error::Config("test error".to_string());
        assert_eq!(err.to_string(), "Configuration error: test error");
    }

    #[test]
    fn test_error_from_io() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let err = Error::from(io_err);
        assert!(matches!(err, Error::Io(_)));
    }
}
