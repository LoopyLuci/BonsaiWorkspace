//! Model format detection and identification

use crate::error::{ConverterError, ConverterResult};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Supported model formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelFormat {
    /// GGUF format (llama.cpp quantized models)
    Gguf,
    /// safetensors format (Hugging Face standard)
    Safetensors,
    /// Bonsai Knowledge Package (.bkp archive)
    Bkp,
    /// Hugging Face Hub remote model
    HuggingFace,
    /// PyTorch format (.pth, .pt)
    PyTorch,
    /// ONNX format
    Onnx,
    /// Generic checkpoint directory
    Checkpoint,
}

impl std::fmt::Display for ModelFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Gguf => write!(f, "GGUF"),
            Self::Safetensors => write!(f, "safetensors"),
            Self::Bkp => write!(f, "BKP"),
            Self::HuggingFace => write!(f, "HuggingFace"),
            Self::PyTorch => write!(f, "PyTorch"),
            Self::Onnx => write!(f, "ONNX"),
            Self::Checkpoint => write!(f, "Checkpoint"),
        }
    }
}

/// Format detector with magic byte checking and heuristics
pub struct FormatDetector;

impl FormatDetector {
    /// Detect format from file path and optional magic bytes
    pub fn detect<P: AsRef<Path>>(path: P) -> ConverterResult<ModelFormat> {
        let path = path.as_ref();

        // First, try extension-based detection
        if let Some(ext) = path.extension() {
            let ext_str = ext
                .to_str()
                .ok_or_else(|| ConverterError::format("Invalid path encoding"))?
                .to_lowercase();

            match ext_str.as_str() {
                "gguf" => return Ok(ModelFormat::Gguf),
                "safetensors" => return Ok(ModelFormat::Safetensors),
                "bkp" => return Ok(ModelFormat::Bkp),
                "pth" | "pt" => return Ok(ModelFormat::PyTorch),
                "onnx" => return Ok(ModelFormat::Onnx),
                _ => {}
            }
        }

        // Try magic byte detection for files
        if path.is_file() {
            return Self::detect_from_magic_bytes(path);
        }

        // For directories, look for checkpoint indicators
        if path.is_dir() {
            if Self::looks_like_checkpoint(path) {
                return Ok(ModelFormat::Checkpoint);
            }
        }

        Err(ConverterError::format(format!(
            "Cannot detect format for: {}",
            path.display()
        )))
    }

    /// Detect format from HuggingFace model ID
    pub fn detect_huggingface_id(model_id: &str) -> ConverterResult<ModelFormat> {
        // HF model IDs have format: owner/model-name
        if model_id.contains('/') && !model_id.contains('.') && !model_id.contains('\\') {
            Ok(ModelFormat::HuggingFace)
        } else {
            Err(ConverterError::format(
                "Invalid HuggingFace model ID format (expected: owner/model-name)",
            ))
        }
    }

    fn detect_from_magic_bytes(path: &Path) -> ConverterResult<ModelFormat> {
        use std::io::Read;

        let mut file = std::fs::File::open(path)
            .map_err(|e| ConverterError::NotFound(path.to_path_buf()).with_context(e))?;

        let mut magic = [0u8; 16];
        file.read_exact(&mut magic)
            .ok(); // Some formats may not have full magic bytes

        // GGUF: starts with 0x67 0x67 0x75 0x66 (little-endian "gguf")
        if &magic[0..4] == b"gguf" {
            return Ok(ModelFormat::Gguf);
        }

        // safetensors: starts with specific binary format
        // The first 8 bytes are a little-endian u64 header size
        if magic.len() >= 8 {
            let header_size = u64::from_le_bytes([
                magic[0], magic[1], magic[2], magic[3], magic[4], magic[5], magic[6], magic[7],
            ]);
            // safetensors headers are typically 100-10000 bytes
            if header_size < 100_000 && header_size > 0 {
                // Peek further to check for JSON
                let mut json_buf = [0u8; 1];
                file.read_exact(&mut json_buf).ok();
                if json_buf[0] == b'{' {
                    return Ok(ModelFormat::Safetensors);
                }
            }
        }

        // ZIP/BKP detection
        if &magic[0..2] == b"PK" {
            return Ok(ModelFormat::Bkp);
        }

        // Zstandard frame magic
        if &magic[0..4] == &[0x28, 0xb5, 0x2f, 0xfd] {
            // Could be compressed GGUF or safetensors
            // For now, return generic compressed indication
            return Err(ConverterError::format(
                "Compressed format detected; specify explicit format",
            ));
        }

        Err(ConverterError::format(format!(
            "Unknown format for file: {}",
            path.display()
        )))
    }

    fn looks_like_checkpoint(dir: &Path) -> bool {
        // Check for typical checkpoint files
        let indicators = ["config.json", "model.safetensors", "pytorch_model.bin"];
        indicators.iter().any(|name| dir.join(name).exists())
    }
}

/// Detect format from path or model ID
pub fn detect_format<P: AsRef<Path>>(path: P) -> ConverterResult<ModelFormat> {
    let path = path.as_ref();
    let path_str = path.to_string_lossy();

    // Check if it looks like a HuggingFace model ID (not a filesystem path)
    if !path_str.contains('/') || path_str.contains('\\') || path.exists() {
        // It's a file or directory path
        FormatDetector::detect(path)
    } else {
        // Might be a HuggingFace model ID
        FormatDetector::detect_huggingface_id(&path_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_huggingface_id_detection() {
        assert_eq!(
            FormatDetector::detect_huggingface_id("meta-llama/Llama-2-7b").unwrap(),
            ModelFormat::HuggingFace
        );
        assert_eq!(
            FormatDetector::detect_huggingface_id("gpt2").is_err(),
            true
        );
    }

    #[test]
    fn test_extension_detection() {
        use std::path::PathBuf;
        assert_eq!(
            FormatDetector::detect(PathBuf::from("model.gguf")).unwrap(),
            ModelFormat::Gguf
        );
        assert_eq!(
            FormatDetector::detect(PathBuf::from("model.safetensors")).unwrap(),
            ModelFormat::Safetensors
        );
    }
}
