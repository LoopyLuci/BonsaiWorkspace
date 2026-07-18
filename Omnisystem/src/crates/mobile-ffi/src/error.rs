//! Error types for mobile-ffi.

#[derive(Debug, Clone)]
pub enum Error {
    /// Requested codec/config combination isn't supported.
    InvalidConfiguration(String),
    /// Requested codec MIME type has no matching implementation.
    CodecNotAvailable(String),
    /// Decoder input buffer error.
    InputBufferError(String),
    /// Decoder was used before initialization completed.
    DecoderNotInitialized,
    /// Decoder output buffer error (e.g. queue full).
    OutputBufferError(String),
    /// JNI call failed.
    Jni(String),
    /// Catch-all for other errors.
    Other(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidConfiguration(msg) => write!(f, "invalid configuration: {}", msg),
            Error::CodecNotAvailable(msg) => write!(f, "codec not available: {}", msg),
            Error::InputBufferError(msg) => write!(f, "input buffer error: {}", msg),
            Error::DecoderNotInitialized => write!(f, "decoder not initialized"),
            Error::OutputBufferError(msg) => write!(f, "output buffer error: {}", msg),
            Error::Jni(msg) => write!(f, "JNI error: {}", msg),
            Error::Other(msg) => write!(f, "error: {}", msg),
        }
    }
}

impl std::error::Error for Error {}

/// Result type used throughout mobile-ffi.
pub type Result<T> = std::result::Result<T, Error>;
