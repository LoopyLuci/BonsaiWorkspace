//! Error types for extension conversion.

#[derive(Debug, Clone)]
pub enum ConversionError {
    /// I/O failure (reading/writing files, extracting archives).
    Io(String),
    /// Expected manifest file (e.g. `package.json`) was not found.
    ManifestNotFound(String),
    /// Failed to parse a manifest or source file.
    ParseError(String),
}

impl std::fmt::Display for ConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConversionError::Io(msg) => write!(f, "I/O error: {msg}"),
            ConversionError::ManifestNotFound(msg) => write!(f, "manifest not found: {msg}"),
            ConversionError::ParseError(msg) => write!(f, "parse error: {msg}"),
        }
    }
}

impl std::error::Error for ConversionError {}
