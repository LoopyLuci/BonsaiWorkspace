//! Error types for model-converter.

use std::path::PathBuf;

#[derive(Debug)]
pub enum ConverterError {
    /// Referenced file or directory does not exist.
    NotFound(PathBuf),
    /// Underlying I/O failure.
    Io(std::io::Error),
    /// The requested (from, to) format pair has no converter.
    ConversionNotSupported { from: String, to: String },
    /// A spawned subprocess (e.g. llama.cpp's convert.py) failed.
    Subprocess(String),
    /// Progress-reporting channel failure.
    ChannelError(String),
    /// HuggingFace Hub API failure.
    HuggingFaceApi(String),
    /// Format detection failure.
    FormatDetection(String),
    /// Model/package validation failure.
    Validation(String),
    /// Invalid model identifier or reference.
    InvalidModel(String),
    /// llama.cpp's convert.py could not be located.
    LlamaCppNotFound(String),
    /// Catch-all, typically produced by `with_context`.
    Other(String),
}

impl ConverterError {
    pub fn format(msg: impl Into<String>) -> Self {
        ConverterError::FormatDetection(msg.into())
    }

    pub fn validation(msg: impl Into<String>) -> Self {
        ConverterError::Validation(msg.into())
    }

    pub fn invalid_model(msg: impl Into<String>) -> Self {
        ConverterError::InvalidModel(msg.into())
    }

    pub fn llama_cpp_not_found(msg: impl Into<String>) -> Self {
        ConverterError::LlamaCppNotFound(msg.into())
    }

    pub fn huggingface_api(msg: impl Into<String>) -> Self {
        ConverterError::HuggingFaceApi(msg.into())
    }

    /// Wrap this error with additional human-readable context.
    pub fn with_context(self, context: impl std::fmt::Display) -> Self {
        ConverterError::Other(format!("{}: {}", context, self))
    }
}

impl std::fmt::Display for ConverterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConverterError::NotFound(path) => write!(f, "not found: {}", path.display()),
            ConverterError::Io(e) => write!(f, "I/O error: {}", e),
            ConverterError::ConversionNotSupported { from, to } => {
                write!(f, "conversion not supported: {} -> {}", from, to)
            }
            ConverterError::Subprocess(msg) => write!(f, "subprocess error: {}", msg),
            ConverterError::ChannelError(msg) => write!(f, "channel error: {}", msg),
            ConverterError::HuggingFaceApi(msg) => write!(f, "HuggingFace API error: {}", msg),
            ConverterError::FormatDetection(msg) => write!(f, "format detection error: {}", msg),
            ConverterError::Validation(msg) => write!(f, "validation error: {}", msg),
            ConverterError::InvalidModel(msg) => write!(f, "invalid model: {}", msg),
            ConverterError::LlamaCppNotFound(msg) => write!(f, "llama.cpp not found: {}", msg),
            ConverterError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for ConverterError {}

impl From<std::io::Error> for ConverterError {
    fn from(err: std::io::Error) -> Self {
        ConverterError::Io(err)
    }
}

/// Result type used throughout model-converter.
pub type ConverterResult<T> = std::result::Result<T, ConverterError>;
