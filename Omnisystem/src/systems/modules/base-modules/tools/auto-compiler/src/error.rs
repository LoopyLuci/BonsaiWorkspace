//! Error types for auto-compiler

use thiserror::Error;

#[derive(Error, Debug)]
pub enum AutoCompileError {
    #[error("Project detection failed: {0}")]
    DetectionFailed(String),

    #[error("Build configuration invalid: {0}")]
    InvalidConfig(String),

    #[error("Compilation failed: {0}")]
    CompilationFailed(String),

    #[error("Dependency resolution failed: {0}")]
    DependencyResolution(String),

    #[error("File watching failed: {0}")]
    WatchingFailed(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Command execution failed: {0}")]
    CommandFailed(String),

    #[error("Cache error: {0}")]
    CacheError(String),

    #[error("Hot reload failed: {0}")]
    HotReloadFailed(String),

    #[error("Distributed build failed: {0}")]
    DistributedBuildFailed(String),

    #[error("No projects found")]
    NoProjectsFound,

    #[error("Invalid project type: {0}")]
    InvalidProjectType(String),

    #[error("Build timeout")]
    BuildTimeout,

    #[error("Insufficient resources: {0}")]
    InsufficientResources(String),
}

pub type Result<T> = std::result::Result<T, AutoCompileError>;
