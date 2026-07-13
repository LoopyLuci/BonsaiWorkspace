//! Convert .bkp to safetensors format

use crate::error::{ConverterError, ConverterResult};
use crate::ConversionConfig;
use std::path::Path;

/// Convert BKP to safetensors format
///
/// Extracts the base model (GGUF) from BKP, then converts to safetensors.
pub async fn convert_bkp_to_safetensors<P: AsRef<Path>>(
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
        "Converting BKP {} to safetensors {} (via GGUF)",
        bkp_path.display(),
        output_path.display()
    );

    // Create temporary GGUF file
    let temp_gguf = tempfile::NamedTempFile::new()
        .map_err(|e| ConverterError::Io(e))?;
    let temp_gguf_path = temp_gguf.path().to_path_buf();

    // Step 1: Extract BKP to GGUF
    crate::converters::convert_bkp_to_gguf(bkp_path, &temp_gguf_path, config.clone())
        .await
        .map_err(|e| e.with_context("Failed to extract GGUF from BKP"))?;

    // Step 2: Convert GGUF to safetensors
    crate::converters::convert_gguf_to_safetensors(&temp_gguf_path, output_path, config)
        .await
        .map_err(|e| e.with_context("Failed to convert GGUF to safetensors"))?;

    // Cleanup
    let _ = std::fs::remove_file(&temp_gguf_path);

    tracing::info!(
        "Successfully converted BKP to safetensors at {}",
        output_path.display()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bkp_to_safetensors_not_found() {
        let result = convert_bkp_to_safetensors(
            "/nonexistent/file.bkp",
            "/tmp/output.safetensors",
            ConversionConfig::default(),
        )
        .await;

        assert!(result.is_err());
    }
}
