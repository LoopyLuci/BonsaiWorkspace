//! Upload .bkp models to HuggingFace Hub

use crate::error::{ConverterError, ConverterResult};
use crate::ConversionConfig;
use std::path::Path;

/// Upload a BKP model to HuggingFace Hub
///
/// Converts BKP to the appropriate format for HF Hub (preferring safetensors for compatibility)
/// and uploads using the HF Hub API with an authentication token.
pub async fn convert_bkp_to_huggingface(
    bkp_path: impl AsRef<Path>,
    repo_id: &str,
    token: Option<String>,
    config: ConversionConfig,
) -> ConverterResult<()> {
    let bkp_path = bkp_path.as_ref();

    if !bkp_path.exists() {
        return Err(ConverterError::NotFound(bkp_path.to_path_buf()));
    }

    // Validate repo_id format (owner/repo)
    if !repo_id.contains('/') {
        return Err(ConverterError::invalid_model(
            "repo_id must be in format: owner/repo-name",
        ));
    }

    let auth_token = token.or_else(|| {
        // Try to load from HF_TOKEN environment variable
        std::env::var("HF_TOKEN").ok()
    });

    if auth_token.is_none() {
        return Err(ConverterError::validation(
            "HuggingFace token required. Set HF_TOKEN env var or provide token parameter.",
        ));
    }

    tracing::info!(
        "Converting BKP {} to safetensors for upload to {}",
        bkp_path.display(),
        repo_id
    );

    // Create temporary safetensors file
    let temp_safetensors = tempfile::NamedTempFile::new()
        .map_err(|e| ConverterError::Io(e))?;
    let temp_safetensors_path = temp_safetensors.path().to_path_buf();

    // Convert BKP to safetensors
    crate::converters::convert_bkp_to_safetensors(bkp_path, &temp_safetensors_path, config)
        .await?;

    // Upload to HF Hub
    upload_to_hf_hub(&temp_safetensors_path, repo_id, auth_token.as_deref().unwrap()).await?;

    // Cleanup
    let _ = std::fs::remove_file(&temp_safetensors_path);

    tracing::info!("Successfully uploaded model to {}", repo_id);

    Ok(())
}

async fn upload_to_hf_hub(
    file_path: &Path,
    repo_id: &str,
    token: &str,
) -> ConverterResult<()> {
    let filename = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("model.safetensors");

    let upload_url = format!(
        "https://huggingface.co/{}/blob/main/{}",
        repo_id, filename
    );

    tracing::debug!("Uploading to: {}", upload_url);

    // Read file content
    let file_content = tokio::fs::read(file_path).await?;

    let client = reqwest::Client::new();

    // First, check if repo exists and create if needed
    let repo_check_url = format!("https://huggingface.co/api/repos/{}?type=model", repo_id);

    let response = client
        .get(&repo_check_url)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| ConverterError::HuggingFaceApi(format!("Failed to check repo: {}", e)))?;

    if response.status() == 404 {
        // Repository doesn't exist, try to create it
        create_hf_repo(repo_id, token).await?;
    } else if !response.status().is_success() {
        return Err(ConverterError::HuggingFaceApi(format!(
            "HTTP {} from HF API",
            response.status()
        )));
    }

    // Upload the file using the HF Hub API
    let upload_endpoint = format!(
        "https://huggingface.co/api/upload/{}",
        repo_id
    );

    let multipart = reqwest::multipart::Form::new()
        .part("files", reqwest::multipart::Part::bytes(file_content)
            .file_name(filename.to_string()));

    let response = client
        .post(&upload_endpoint)
        .bearer_auth(token)
        .multipart(multipart)
        .send()
        .await
        .map_err(|e| ConverterError::HuggingFaceApi(format!("Upload failed: {}", e)))?;

    if !response.status().is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(ConverterError::HuggingFaceApi(format!(
            "Upload failed: {} - {}",
            response.status(),
            error_text
        )));
    }

    tracing::info!("File uploaded successfully to {}", repo_id);

    Ok(())
}

async fn create_hf_repo(repo_id: &str, token: &str) -> ConverterResult<()> {
    let (org, name) = repo_id
        .split_once('/')
        .ok_or_else(|| ConverterError::invalid_model("Invalid repo_id format"))?;

    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "repo_id": repo_id,
        "type": "model"
    });

    let response = client
        .post("https://huggingface.co/api/repos/create")
        .bearer_auth(token)
        .json(&body)
        .send()
        .await
        .map_err(|e| ConverterError::HuggingFaceApi(format!("Failed to create repo: {}", e)))?;

    if !response.status().is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        tracing::warn!(
            "Could not create repo (it may already exist): {}",
            error_text
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_invalid_repo_id() {
        let result = convert_bkp_to_huggingface(
            "/tmp/model.bkp",
            "invalid-repo-id",
            Some("token".to_string()),
            ConversionConfig::default(),
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_missing_token() {
        // Clear HF_TOKEN if set
        std::env::remove_var("HF_TOKEN");

        let result = convert_bkp_to_huggingface(
            "/tmp/model.bkp",
            "owner/repo",
            None,
            ConversionConfig::default(),
        )
        .await;

        assert!(result.is_err());
    }
}
