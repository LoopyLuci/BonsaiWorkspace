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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_blocked_without_user_confirmation() {
        let dir = tempfile::tempdir().unwrap();
        let result = fetch_model(
            "model.bin",
            "https://example.invalid/model.bin",
            dir.path(),
            false,
        )
        .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("user_confirmed"));
    }

    #[tokio::test]
    async fn test_fetch_returns_cached_file_without_network() {
        let dir = tempfile::tempdir().unwrap();
        tokio::fs::write(dir.path().join("model.bin"), b"cached weights")
            .await
            .unwrap();

        // Bogus URL: if this were actually hit, the test would fail on
        // network error rather than returning the cached path.
        let result = fetch_model(
            "model.bin",
            "https://example.invalid/does-not-exist",
            dir.path(),
            true,
        )
        .await
        .unwrap();

        assert_eq!(result, dir.path().join("model.bin"));
    }

    #[tokio::test]
    async fn test_list_cached() {
        let dir = tempfile::tempdir().unwrap();
        tokio::fs::write(dir.path().join("a.bin"), b"1").await.unwrap();
        tokio::fs::write(dir.path().join("b.bin"), b"2").await.unwrap();

        let mut names = list_cached(dir.path()).await.unwrap();
        names.sort();
        assert_eq!(names, vec!["a.bin".to_string(), "b.bin".to_string()]);
    }
}
