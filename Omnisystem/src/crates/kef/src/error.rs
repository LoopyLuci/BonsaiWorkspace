//! Error types for the Knowledge Extraction Fabric (kef).

#[derive(Debug)]
pub enum KefError {
    /// Underlying I/O failure.
    Io(std::io::Error),
    /// JSON (de)serialization failure.
    SerdeJson(serde_json::Error),
    /// Compression/decompression failure.
    Compression(String),
    /// Embedding dimension mismatch.
    DimensionMismatch { expected: usize, got: usize },
    /// Ingestion pipeline failure.
    IngestionFailed(String),
    /// Model scanning/format-detection failure.
    ModelScan(String),
    /// Activation/attention clustering failure.
    ClusteringFailed(String),
    /// General extraction failure.
    ExtractionFailed(String),
    /// Catch-all for other errors.
    Other(String),
}

impl std::fmt::Display for KefError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KefError::Io(e) => write!(f, "I/O error: {}", e),
            KefError::SerdeJson(e) => write!(f, "serialization error: {}", e),
            KefError::Compression(msg) => write!(f, "compression error: {}", msg),
            KefError::DimensionMismatch { expected, got } => write!(
                f,
                "embedding dimension mismatch: expected {}, got {}",
                expected, got
            ),
            KefError::IngestionFailed(msg) => write!(f, "ingestion failed: {}", msg),
            KefError::ModelScan(msg) => write!(f, "model scan failed: {}", msg),
            KefError::ClusteringFailed(msg) => write!(f, "clustering failed: {}", msg),
            KefError::ExtractionFailed(msg) => write!(f, "extraction failed: {}", msg),
            KefError::Other(msg) => write!(f, "kef error: {}", msg),
        }
    }
}

impl std::error::Error for KefError {}

impl From<std::io::Error> for KefError {
    fn from(err: std::io::Error) -> Self {
        KefError::Io(err)
    }
}

impl From<serde_json::Error> for KefError {
    fn from(err: serde_json::Error) -> Self {
        KefError::SerdeJson(err)
    }
}

/// Result type used throughout kef.
pub type Result<T> = std::result::Result<T, KefError>;
