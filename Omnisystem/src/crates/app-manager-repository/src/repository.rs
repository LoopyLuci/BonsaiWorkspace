use crate::{GitHubFetcher, LocalLoader, Marketplace, Result};
use app_manager_core::{AppId, Version, Manifest};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryConfig {
    pub marketplace_url: String,
    pub cache_dir: PathBuf,
    pub github_token: Option<String>,
    pub verify_signatures: bool,
}

impl Default for RepositoryConfig {
    fn default() -> Self {
        RepositoryConfig {
            marketplace_url: "https://omnisystem.marketplace.local".to_string(),
            cache_dir: PathBuf::from("./.cache"),
            github_token: None,
            verify_signatures: true,
        }
    }
}

pub struct Repository {
    config: RepositoryConfig,
    github_fetcher: GitHubFetcher,
    local_loader: LocalLoader,
    marketplace: Marketplace,
}

impl Repository {
    pub fn new(config: RepositoryConfig) -> Self {
        let github_fetcher = GitHubFetcher::new(config.github_token.clone());
        let local_loader = LocalLoader::new(config.cache_dir.clone());
        let marketplace = Marketplace::new(config.marketplace_url.clone());

        Repository {
            config,
            github_fetcher,
            local_loader,
            marketplace,
        }
    }

    pub async fn fetch_from_github(&self, owner: &str, repo: &str, version: &Version) -> Result<Vec<u8>> {
        tracing::info!("Fetching from GitHub: {}/{} v{}", owner, repo, version);
        self.github_fetcher.fetch(owner, repo, version).await
    }

    pub async fn fetch_from_marketplace(&self, app_id: &AppId, version: &Version) -> Result<Vec<u8>> {
        tracing::info!("Fetching from marketplace: {} v{}", app_id, version);
        self.marketplace.fetch(app_id, version).await
    }

    pub async fn load_from_file(&self, path: &PathBuf) -> Result<Vec<u8>> {
        tracing::info!("Loading from file: {:?}", path);
        self.local_loader.load(path).await
    }

    pub async fn fetch_manifest(&self, app_id: &AppId, version: &Version) -> Result<Manifest> {
        tracing::info!("Fetching manifest for {} v{}", app_id, version);
        self.marketplace.fetch_manifest(app_id, version).await
    }

    pub async fn verify_checksum(&self, data: &[u8], expected: &str) -> Result<bool> {
        use sha2::{Sha256, Digest};

        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        let hex = hex::encode(result);

        Ok(hex == expected)
    }

    pub async fn list_available_versions(&self, app_id: &AppId) -> Result<Vec<Version>> {
        tracing::info!("Listing versions for {}", app_id);
        self.marketplace.list_versions(app_id).await
    }

    pub fn get_config(&self) -> &RepositoryConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repository_creation() {
        let config = RepositoryConfig::default();
        let repo = Repository::new(config);
        assert!(!repo.config.marketplace_url.is_empty());
    }

    #[tokio::test]
    async fn test_verify_checksum() {
        let config = RepositoryConfig::default();
        let repo = Repository::new(config);

        let data = b"test data";
        let expected = "916f0027a575074ce72a331777c3478d6513f786a591bd892da1a577bf2335f9";

        let result = repo.verify_checksum(data, expected).await.unwrap();
        assert!(result);
    }

    #[test]
    fn test_invalid_checksum() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let config = RepositoryConfig::default();
            let repo = Repository::new(config);

            let data = b"test data";
            let expected = "invalid";

            let result = repo.verify_checksum(data, expected).await.unwrap();
            assert!(!result);
        });
    }
}
