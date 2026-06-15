//! Model validation and integrity checking

use crate::error::{ConverterError, ConverterResult};
use crate::format::ModelFormat;
use std::path::Path;

/// Validation result summary
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub format: ModelFormat,
    pub file_size: u64,
    pub hash: String,
    pub issues: Vec<String>,
    pub warnings: Vec<String>,
}

/// Validate a model file for integrity and correctness
pub fn validate_model<P: AsRef<Path>>(path: P) -> ConverterResult<ValidationResult> {
    let path = path.as_ref();

    if !path.exists() {
        return Err(ConverterError::NotFound(path.to_path_buf()));
    }

    let metadata = std::fs::metadata(path)?;
    let file_size = metadata.len();

    // Detect format
    let format = crate::format::FormatDetector::detect(path)?;

    // Compute hash
    let hash = compute_hash(path)?;

    let mut issues = Vec::new();
    let mut warnings = Vec::new();

    // Format-specific validation
    match format {
        ModelFormat::Gguf => {
            validate_gguf(path, &mut issues, &mut warnings)?;
        }
        ModelFormat::Safetensors => {
            validate_safetensors(path, &mut issues, &mut warnings)?;
        }
        ModelFormat::Bkp => {
            validate_bkp(path, &mut issues, &mut warnings)?;
        }
        _ => {
            warnings.push(format!("Limited validation available for {} format", format));
        }
    }

    // Check file size sanity
    if file_size == 0 {
        issues.push("File is empty".to_string());
    } else if file_size < 1_000_000 {
        warnings.push("File is very small (< 1MB)".to_string());
    }

    let is_valid = issues.is_empty();

    Ok(ValidationResult {
        is_valid,
        format,
        file_size,
        hash,
        issues,
        warnings,
    })
}

/// Verify that a roundtrip conversion produces bit-identical output
pub async fn validate_roundtrip(
    original_path: &Path,
    converted_path: &Path,
) -> ConverterResult<bool> {
    let original_hash = compute_hash(original_path)?;
    let converted_hash = compute_hash(converted_path)?;

    Ok(original_hash == converted_hash)
}

fn compute_hash<P: AsRef<Path>>(path: P) -> ConverterResult<String> {
    use std::io::Read;

    let mut file = std::fs::File::open(path.as_ref())?;
    let mut hasher = blake3::Hasher::new();

    let mut buffer = [0; 65536]; // 64KB chunks
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }

    Ok(hasher.finalize().to_hex().to_string())
}

fn validate_gguf(path: &Path, issues: &mut Vec<String>, warnings: &mut Vec<String>) -> ConverterResult<()> {
    use std::io::Read;

    let mut file = std::fs::File::open(path)?;
    let mut magic = [0u8; 4];
    file.read_exact(&mut magic)?;

    if &magic != b"gguf" {
        issues.push("Invalid GGUF magic bytes".to_string());
        return Ok(());
    }

    // Check version
    let mut version = [0u8; 4];
    file.read_exact(&mut version)?;
    let version = u32::from_le_bytes(version);

    if version > 3 {
        warnings.push(format!(
            "GGUF version {} may not be fully supported",
            version
        ));
    }

    // Check for basic header structure
    let mut num_kv = [0u8; 8];
    file.read_exact(&mut num_kv).ok();

    Ok(())
}

fn validate_safetensors(
    path: &Path,
    issues: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> ConverterResult<()> {
    use std::io::Read;

    let mut file = std::fs::File::open(path)?;
    let mut header_size_bytes = [0u8; 8];
    file.read_exact(&mut header_size_bytes)?;

    let header_size = u64::from_le_bytes(header_size_bytes);

    if header_size > 1_000_000_000 {
        issues.push(format!(
            "Invalid header size: {} (> 1GB)",
            header_size
        ));
        return Ok(());
    }

    // Read and validate JSON header
    let mut json_buf = vec![0; header_size as usize];
    if file.read(&mut json_buf).is_err() {
        issues.push("Failed to read header".to_string());
        return Ok(());
    }

    match serde_json::from_slice::<serde_json::Value>(&json_buf) {
        Ok(_) => {
            // Header is valid JSON
        }
        Err(e) => {
            issues.push(format!("Invalid JSON header: {}", e));
        }
    }

    Ok(())
}

fn validate_bkp(path: &Path, issues: &mut Vec<String>, warnings: &mut Vec<String>) -> ConverterResult<()> {
    // BKP is a ZIP file
    let file = std::fs::File::open(path)?;
    match zip::ZipArchive::new(file) {
        Ok(mut archive) => {
            // Check for required files
            let has_manifest = (0..archive.len())
                .any(|i| {
                    archive
                        .by_index(i)
                        .map(|f| f.name() == "manifest.json")
                        .unwrap_or(false)
                })
                .then_some(());

            if has_manifest.is_none() {
                issues.push("Missing manifest.json in BKP".to_string());
            }

            // Check for base model
            let has_base_model = (0..archive.len())
                .any(|i| {
                    archive
                        .by_index(i)
                        .map(|f| f.name().starts_with("base_model/"))
                        .unwrap_or(false)
                })
                .then_some(());

            if has_base_model.is_none() {
                warnings.push("No base model found in BKP".to_string());
            }
        }
        Err(e) => {
            issues.push(format!("Invalid ZIP archive: {}", e));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_compute_hash() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"test data").unwrap();
        file.flush().unwrap();

        let hash1 = compute_hash(file.path()).unwrap();
        let hash2 = compute_hash(file.path()).unwrap();

        assert_eq!(hash1, hash2);
    }
}
