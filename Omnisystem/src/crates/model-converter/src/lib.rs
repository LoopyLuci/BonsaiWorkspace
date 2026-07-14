//! model-converter: converts model files between GGUF, safetensors,
//! HuggingFace Hub, and the Bonsai Knowledge Package (.bkp) format.

pub mod converters;
pub mod error;
pub mod format;
pub mod progress;
pub mod validation;

pub use converters::convert_batch;
pub use error::{ConverterError, ConverterResult};
pub use format::{detect_format, ModelFormat};

/// Shared configuration for a single conversion (or batch of conversions).
#[derive(Debug, Clone)]
pub struct ConversionConfig {
    /// Model context length in tokens (used for metadata where applicable).
    pub context_length: u32,
    /// Override model name in generated metadata.
    pub model_name: Option<String>,
    /// Model author, recorded in package metadata.
    pub author: Option<String>,
    /// License identifier, recorded in package metadata.
    pub license: Option<String>,
    /// Free-text description, recorded in package metadata.
    pub description: String,
    /// Whether to verify a roundtrip conversion produces identical output.
    pub verify_roundtrip: bool,
    /// Whether to compress .bkp package contents.
    pub compress_bkp: bool,
    /// Number of parallel jobs to use for batch conversions.
    pub parallel_jobs: usize,
    /// Timeout (seconds) for network operations (HuggingFace Hub, etc).
    pub timeout_secs: u64,
    /// HuggingFace Hub auth token (falls back to the HF_TOKEN env var if unset).
    pub hf_token: Option<String>,
}

impl Default for ConversionConfig {
    fn default() -> Self {
        Self {
            context_length: 4096,
            model_name: None,
            author: None,
            license: None,
            description: String::new(),
            verify_roundtrip: false,
            compress_bkp: true,
            parallel_jobs: 4,
            timeout_secs: 300,
            hf_token: None,
        }
    }
}
