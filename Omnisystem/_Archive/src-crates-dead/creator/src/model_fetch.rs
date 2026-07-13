//! Model weight downloader.
//!
//! CRITICAL SECURITY CONSTRAINT: model downloads are NEVER initiated
//! automatically.  This function must only be called when the user has
//! explicitly clicked "Download" in the UI.  The daemon verifies this by
//! requiring the `user_confirmed = true` flag in the RPC params.

use anyhow::Result;
use std::path::Path;
use tracing::info;

/// Download `url` to `cache / name` if the file does not already exist.
///
/// `user_confirmed` MUST be `true`; the function returns an error otherwise
/// to prevent accidental background downloads.
pub async fn fetch_model(
    name: &str,
    url: &str,
    cache: &Path,
    user_confirmed: bool,
) -> Result<std::path::PathBuf> {
    if !user_confirmed {
        return Err(anyhow::anyhow!(
            "model download blocked: user_confirmed must be true. \
             This download must be explicitly triggered by a UI button."
        ));
    }

    let dest = cache.join(name);
    if dest.exists() {
        info!("model already cached at {}", dest.display());
        return Ok(dest);
    }

    tokio::fs::create_dir_all(cache).await?;
    info!("downloading model {name} from {url}");

    let client = reqwest::Client::new();
    let response = client.get(url).send().await?;
    let status = response.status();
    if !status.is_success() {
        return Err(anyhow::anyhow!("download failed: HTTP {status} for {url}"));
    }

    let bytes = response.bytes().await?;
    tokio::fs::write(&dest, &bytes).await?;
    info!("model saved to {}", dest.display());
    Ok(dest)
}

/// List locally cached models under `cache`.
pub async fn list_cached(cache: &Path) -> Result<Vec<String>> {
    let mut names = Vec::new();
    let mut rd = tokio::fs::read_dir(cache).await?;
    while let Some(entry) = rd.next_entry().await? {
        if let Some(n) = entry.file_name().to_str() {
            names.push(n.to_string());
        }
    }
    Ok(names)
}
