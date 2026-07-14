//! Convert .bkp (Bonsai Knowledge Package) to GGUF format

use crate::error::{ConverterError, ConverterResult};
use crate::ConversionConfig;
use package::PackageReader;
use std::path::Path;

/// Convert BKP to GGUF format
///
/// Extracts the base model (GGUF) from the .bkp package.
pub async fn convert_bkp_to_gguf<P1: AsRef<Path>, P2: AsRef<Path>>(
    bkp_path: P1,
    output_path: P2,
    _config: ConversionConfig,
) -> ConverterResult<()> {
    let bkp_path = bkp_path.as_ref();
    let output_path = output_path.as_ref();

    if !bkp_path.exists() {
        return Err(ConverterError::NotFound(bkp_path.to_path_buf()));
    }

    tracing::info!(
        "Converting BKP {} to GGUF {}",
        bkp_path.display(),
        output_path.display()
    );

    // Open BKP package (this also parses and validates the manifest)
    let mut reader = PackageReader::open(bkp_path)
        .map_err(|e| ConverterError::validation(format!("Failed to open BKP: {}", e)))?;

    tracing::debug!(
        "BKP manifest: {} v{}",
        reader.manifest.name,
        reader.manifest.version
    );

    // Create output directory if needed
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Extract base model file directly to the requested output path
    let base_model_path = reader.manifest.base_model.path_in_package.clone();
    reader
        .extract_entry(&base_model_path, output_path)
        .map_err(|e| {
            ConverterError::validation(format!(
                "Failed to extract base model {}: {}",
                base_model_path, e
            ))
        })?;

    tracing::info!("Successfully extracted GGUF to {}", output_path.display());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bkp_to_gguf_not_found() {
        let result = convert_bkp_to_gguf(
            "/nonexistent/file.bkp",
            "/tmp/output.gguf",
            ConversionConfig::default(),
        )
        .await;

        assert!(result.is_err());
        match result {
            Err(ConverterError::NotFound(_)) => {}
            _ => panic!("Expected NotFound error"),
        }
    }

    #[tokio::test]
    async fn test_roundtrip_gguf_bkp_gguf() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        let mut gguf_file = NamedTempFile::new().unwrap();
        gguf_file.write_all(b"gguf-roundtrip-test-data").unwrap();
        gguf_file.flush().unwrap();

        let bkp_file = NamedTempFile::new().unwrap();
        crate::converters::convert_gguf_to_bkp(
            gguf_file.path(),
            bkp_file.path(),
            ConversionConfig::default(),
        )
        .await
        .unwrap();

        let extracted = NamedTempFile::new().unwrap();
        convert_bkp_to_gguf(
            bkp_file.path(),
            extracted.path(),
            ConversionConfig::default(),
        )
        .await
        .unwrap();

        let original = std::fs::read(gguf_file.path()).unwrap();
        let roundtripped = std::fs::read(extracted.path()).unwrap();
        assert_eq!(original, roundtripped);
    }
}
