//! Convert safetensors to .bkp format

use crate::error::{ConverterError, ConverterResult};
use crate::ConversionConfig;
use std::path::Path;

/// Convert safetensors to BKP format
///
/// Converts safetensors to GGUF first, then packages into .bkp.
pub async fn convert_safetensors_to_bkp<P: AsRef<Path>>(
    safetensors_path: P,
    output_path: P,
    config: ConversionConfig,
) -> ConverterResult<()> {
    let safetensors_path = safetensors_path.as_ref();
    let output_path = output_path.as_ref();

    if !safetensors_path.exists() {
        return Err(ConverterError::NotFound(safetensors_path.to_path_buf()));
    }

    tracing::info!(
        "Converting safetensors {} to BKP {} (via GGUF)",
        safetensors_path.display(),
        output_path.display()
    );

    // Create temporary GGUF file
    let temp_gguf = tempfile::NamedTempFile::new()
        .map_err(|e| ConverterError::Io(e))?;
    let temp_gguf_path = temp_gguf.path().to_path_buf();

    // Step 1: Convert safetensors to GGUF
    crate::converters::convert_safetensors_to_gguf(safetensors_path, &temp_gguf_path, config.clone())
        .await
        .map_err(|e| e.with_context("Failed to convert safetensors to GGUF"))?;

    // Step 2: Package GGUF into BKP
    crate::converters::convert_gguf_to_bkp(&temp_gguf_path, output_path, config)
        .await
        .map_err(|e| e.with_context("Failed to package GGUF into BKP"))?;

    // Cleanup
    let _ = std::fs::remove_file(&temp_gguf_path);

    tracing::info!(
        "Successfully converted safetensors to BKP at {}",
        output_path.display()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_safetensors_to_bkp_not_found() {
        let result = convert_safetensors_to_bkp(
            "/nonexistent/file.safetensors",
            "/tmp/output.bkp",
            ConversionConfig::default(),
        )
        .await;

        assert!(result.is_err());
    }
}
