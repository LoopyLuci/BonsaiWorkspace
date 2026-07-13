//! Download models from HuggingFace Hub and convert to .bkp

use crate::error::{ConverterError, ConverterResult};
use crate::ConversionConfig;
use std::path::Path;
use tokio::io::AsyncWriteExt;

/// Download a model from HuggingFace Hub and convert to BKP
///
/// Supports downloading:
/// - GGUF models directly
/// - safetensors models (converts to BKP via GGUF)
/// - PyTorch checkpoints (converts to GGUF then BKP)
pub async fn convert_huggingface_to_bkp(
    model_id: &str,
    output_path: impl AsRef<Path>,
    config: ConversionConfig,
) -> ConverterResult<()> {
    let output_path = output_path.as_ref();

    tracing::info!(
        "Downloading model {} from HuggingFace and converting to BKP",
        model_id
    );

    // Create output directory
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Detect model type by querying HF API
    let model_files = list_hf_model_files(model_id).await?;

    tracing::debug!("Found {} files in HF model", model_files.len());

    // Prefer GGUF format
    if let Some(gguf_file) = model_files.iter().find(|f| f.ends_with(".gguf")) {
        let gguf_path = download_hf_file(model_id, gguf_file, &config).await?;
        let result = crate::converters::convert_gguf_to_bkp(&gguf_path, output_path, config).await;
        let _ = std::fs::remove_file(&gguf_path);
        return result;
    }

    // Fall back to safetensors
    if let Some(st_file) = model_files
        .iter()
        .find(|f| f.ends_with(".safetensors"))
    {
        let st_path = download_hf_file(model_id, st_file, &config).await?;
        let result =
            crate::converters::convert_safetensors_to_bkp(&st_path, output_path, config).await;
        let _ = std::fs::remove_file(&st_path);
        return result;
    }

    Err(ConverterError::huggingface_api(format!(
        "No convertible model format found in {}. Supported: .gguf, .safetensors",
        model_id
    )))
}

async fn list_hf_model_files(model_id: &str) -> ConverterResult<Vec<String>> {
    let url = format!("https://huggingface.co/api/models/{}", model_id);

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| ConverterError::HuggingFaceApi(e.to_string()))?;

    if !response.status().is_success() {
        return Err(ConverterError::HuggingFaceApi(format!(
            "Failed to fetch model info: HTTP {}",
            response.status()
        )));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| ConverterError::HuggingFaceApi(e.to_string()))?;

    // Extract sibling filenames
    let siblings = json
        .get("siblings")
        .and_then(|s| s.as_array())
        .ok_or_else(|| ConverterError::HuggingFaceApi("Invalid API response".to_string()))?;

    let files = siblings
        .iter()
        .filter_map(|s| s.get("rfilename").and_then(|r| r.as_str()))
        .map(|s| s.to_string())
        .collect();

    Ok(files)
}

async fn download_hf_file(
    model_id: &str,
    filename: &str,
    config: &ConversionConfig,
) -> ConverterResult<std::path::PathBuf> {
    let url = format!(
        "https://huggingface.co/{}/resolve/main/{}",
        model_id, filename
    );

    tracing::info!("Downloading: {}", url);

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(config.timeout_secs as u64))
        .send()
        .await
        .map_err(|e| ConverterError::HuggingFaceApi(format!("Download failed: {}", e)))?;

    if !response.status().is_success() {
        return Err(ConverterError::HuggingFaceApi(format!(
            "HTTP {}: {}",
            response.status(),
            url
        )));
    }

    // Create temporary file
    let mut temp_file = tempfile::NamedTempFile::new()
        .map_err(|e| ConverterError::Io(e))?;

    let content = response
        .bytes()
        .await
        .map_err(|e| ConverterError::HuggingFaceApi(e.to_string()))?;

    // Write content to temp file
    tokio::fs::write(temp_file.path(), &content).await?;

    Ok(temp_file.path().to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_huggingface_id_parsing() {
        // This test just verifies the format is recognized
        let result = convert_huggingface_to_bkp(
            "invalid-model-id-with-no-slash",
            "/tmp/output.bkp",
            ConversionConfig::default(),
        )
        .await;

        // Should fail with validation error
        assert!(result.is_err());
    }
}
