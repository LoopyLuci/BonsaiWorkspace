//! Convert .bkp (Bonsai Knowledge Package) to GGUF format

use crate::error::{ConverterError, ConverterResult};
use crate::ConversionConfig;
use std::path::Path;
use package::PackageReader;

/// Convert BKP to GGUF format
///
/// Extracts the base model (GGUF) from the .bkp package.
pub async fn convert_bkp_to_gguf<P: AsRef<Path>>(
    bkp_path: P,
    output_path: P,
    config: ConversionConfig,
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

    // Open BKP package
    let reader = PackageReader::open(bkp_path)
        .map_err(|e| ConverterError::validation(format!("Failed to open BKP: {}", e)))?;

    // Get manifest to locate base model
    let manifest = reader
        .get_manifest()
        .map_err(|e| ConverterError::validation(format!("Failed to read manifest: {}", e)))?;

    tracing::debug!(
        "BKP manifest: {} v{}",
        manifest.name,
        manifest.version
    );

    // Extract base model file
    let base_model_path = &manifest.base_model.path_in_package;

    let extracted = reader
        .extract_file(base_model_path)
        .map_err(|e| ConverterError::validation(format!(
            "Failed to extract base model {}: {}",
            base_model_path, e
        )))?;

    // Create output directory if needed
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Copy extracted GGUF to output path
    std::fs::copy(&extracted, output_path)?;

    // Cleanup temporary file
    let _ = std::fs::remove_file(extracted);

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
}
