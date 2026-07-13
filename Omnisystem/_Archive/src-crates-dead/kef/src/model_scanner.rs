//! Model format detection and introspection

use crate::{ExtractionMethod, KefError, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::fs;

/// Supported model types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelType {
    /// Large Language Model (transformer-based)
    Llm,
    /// Text embedding model
    Embedding,
    /// Vision transformer
    Vision,
    /// Mixture of Experts model
    Moe,
    /// Other/unknown
    Other,
}

impl std::fmt::Display for ModelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModelType::Llm => write!(f, "llm"),
            ModelType::Embedding => write!(f, "embedding"),
            ModelType::Vision => write!(f, "vision"),
            ModelType::Moe => write!(f, "moe"),
            ModelType::Other => write!(f, "other"),
        }
    }
}

/// Report from scanning a model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelReport {
    /// Type of model
    pub model_type: ModelType,
    /// Total parameter count
    pub parameter_count: u64,
    /// Number of layers/blocks
    pub layers: usize,
    /// Hidden dimension size
    pub hidden_size: usize,
    /// Context window length
    pub context_window: usize,
    /// Quantization type if applicable
    pub quantization: Option<String>,
    /// Applicable extraction methods for this model
    pub applicable_methods: Vec<ExtractionMethod>,
    /// File format (gguf, safetensors, pytorch, onnx)
    pub file_format: String,
    /// Model name/identifier
    pub model_name: String,
    /// Vocabulary size
    pub vocab_size: usize,
}

impl ModelReport {
    /// Create a minimal report
    pub fn new(model_type: ModelType) -> Self {
        Self {
            model_type,
            parameter_count: 0,
            layers: 0,
            hidden_size: 768,
            context_window: 2048,
            quantization: None,
            applicable_methods: Self::default_methods(model_type),
            file_format: "unknown".to_string(),
            model_name: String::new(),
            vocab_size: 0,
        }
    }

    /// Get default extraction methods for a model type
    fn default_methods(model_type: ModelType) -> Vec<ExtractionMethod> {
        match model_type {
            ModelType::Llm => vec![
                ExtractionMethod::Synthetic,
                ExtractionMethod::Activation,
                ExtractionMethod::Attention,
                ExtractionMethod::MembershipInference,
            ],
            ModelType::Embedding => vec![
                ExtractionMethod::Activation,
                ExtractionMethod::MembershipInference,
            ],
            ModelType::Vision => vec![ExtractionMethod::Activation],
            ModelType::Moe => vec![
                ExtractionMethod::Synthetic,
                ExtractionMethod::Activation,
                ExtractionMethod::MembershipInference,
            ],
            ModelType::Other => vec![ExtractionMethod::Activation],
        }
    }
}

/// Scans models for format and architecture
pub struct ModelScanner;

impl ModelScanner {
    /// Scan a model file and return a report
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or format is unrecognized
    pub async fn scan(path: &Path) -> Result<ModelReport> {
        // Check file exists
        if !path.exists() {
            return Err(KefError::ModelScan(format!(
                "model file not found: {}",
                path.display()
            )));
        }

        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        // Detect format from extension and magic bytes
        let format = Self::detect_format(path)?;

        // Create initial report
        let mut report = match format.as_str() {
            "gguf" => Self::scan_gguf(path)?,
            "safetensors" => Self::scan_safetensors(path)?,
            "pytorch" => Self::scan_pytorch(path)?,
            "onnx" => Self::scan_onnx(path)?,
            _ => ModelReport::new(ModelType::Other),
        };

        report.file_format = format;
        report.model_name = filename.to_string();

        // For small models, enable all methods; for large models, selective
        if report.parameter_count < 1_000_000_000 {
            // < 1B: all methods enabled
            report.applicable_methods = match report.model_type {
                ModelType::Llm => vec![
                    ExtractionMethod::Synthetic,
                    ExtractionMethod::Activation,
                    ExtractionMethod::Attention,
                    ExtractionMethod::MembershipInference,
                ],
                _ => report.applicable_methods,
            };
        } else if report.parameter_count > 10_000_000_000 {
            // > 10B: focus on efficient methods
            report.applicable_methods = vec![
                ExtractionMethod::Synthetic,
                ExtractionMethod::MembershipInference,
            ];
        }

        Ok(report)
    }

    /// Detect model format from file
    fn detect_format(path: &Path) -> Result<String> {
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        // Check extension first
        if filename.ends_with(".gguf") {
            return Ok("gguf".to_string());
        }
        if filename.ends_with(".safetensors") {
            return Ok("safetensors".to_string());
        }
        if filename.ends_with(".pt") || filename.ends_with(".pth") {
            return Ok("pytorch".to_string());
        }
        if filename.ends_with(".onnx") {
            return Ok("onnx".to_string());
        }

        // Check magic bytes
        let mut file = fs::File::open(path).map_err(|e| KefError::ModelScan(e.to_string()))?;
        let mut magic = [0u8; 4];
        use std::io::Read;
        file.read_exact(&mut magic)
            .map_err(|e| KefError::ModelScan(e.to_string()))?;

        // GGUF magic: "GGUF"
        if &magic == b"GGUF" {
            return Ok("gguf".to_string());
        }

        // safetensors magic: "??TENSOR"
        if magic.len() >= 4 && &magic[2..4] == b"TE" {
            return Ok("safetensors".to_string());
        }

        // PyTorch models start with version indicator
        if magic[0] == 0x80 && magic[1] == 0x02 {
            // Pickle format
            return Ok("pytorch".to_string());
        }

        // ONNX models
        if &magic[..4] == b"ONNX" || (magic[0] == 0x08 && magic[1] == 0x03) {
            return Ok("onnx".to_string());
        }

        Ok("unknown".to_string())
    }

    /// Scan a GGUF format model
    fn scan_gguf(path: &Path) -> Result<ModelReport> {
        // GGUF format: header with KV pairs describing model
        // For now, return a basic report with reasonable defaults
        // In production, parse GGUF header fully

        let mut report = ModelReport::new(ModelType::Llm);
        report.file_format = "gguf".to_string();

        // Try to estimate from filename patterns
        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
            // Common patterns: model-7b, model-13b, model-70b, etc.
            if filename.contains("7b") {
                report.parameter_count = 7_000_000_000;
                report.layers = 32;
            } else if filename.contains("13b") {
                report.parameter_count = 13_000_000_000;
                report.layers = 40;
            } else if filename.contains("70b") {
                report.parameter_count = 70_000_000_000;
                report.layers = 80;
                report.hidden_size = 8192;
            } else if filename.contains("3b") || filename.contains("3.1b") {
                report.parameter_count = 3_000_000_000;
                report.layers = 26;
            } else {
                // Default medium model
                report.parameter_count = 8_000_000_000;
                report.layers = 32;
            }
        }

        Ok(report)
    }

    /// Scan a safetensors model
    fn scan_safetensors(path: &Path) -> Result<ModelReport> {
        // safetensors header: JSON metadata
        let mut report = ModelReport::new(ModelType::Embedding);
        report.file_format = "safetensors".to_string();
        report.hidden_size = 768; // Common for embeddings

        Ok(report)
    }

    /// Scan a PyTorch model
    fn scan_pytorch(path: &Path) -> Result<ModelReport> {
        let mut report = ModelReport::new(ModelType::Other);
        report.file_format = "pytorch".to_string();

        Ok(report)
    }

    /// Scan an ONNX model
    fn scan_onnx(path: &Path) -> Result<ModelReport> {
        let mut report = ModelReport::new(ModelType::Embedding);
        report.file_format = "onnx".to_string();

        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_model_type_display() {
        assert_eq!(ModelType::Llm.to_string(), "llm");
        assert_eq!(ModelType::Embedding.to_string(), "embedding");
        assert_eq!(ModelType::Vision.to_string(), "vision");
    }

    #[test]
    fn test_model_report_methods() {
        let report = ModelReport::new(ModelType::Llm);
        assert!(report.applicable_methods.contains(&ExtractionMethod::Synthetic));
        assert!(report.applicable_methods.contains(&ExtractionMethod::Activation));
    }

    #[test]
    fn test_format_detection_from_extension() -> Result<()> {
        let temp_dir = TempDir::new().map_err(|e| KefError::Io(e))?;
        let gguf_path = temp_dir.path().join("model.gguf");

        // Create file with GGUF magic
        let mut file = fs::File::create(&gguf_path).map_err(|e| KefError::Io(e))?;
        file.write_all(b"GGUF").map_err(|e| KefError::Io(e))?;
        drop(file);

        let format = ModelScanner::detect_format(&gguf_path)?;
        assert_eq!(format, "gguf");

        Ok(())
    }
}
