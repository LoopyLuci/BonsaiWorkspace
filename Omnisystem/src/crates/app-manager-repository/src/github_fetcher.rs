use crate::{Result, RepositoryError};
use app_manager_core::{Version, Manifest};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub name: String,
    pub body: String,
    pub draft: bool,
    pub prerelease: bool,
    pub published_at: String,
}

pub struct GitHubFetcher {
    github_token: Option<String>,
}

impl GitHubFetcher {
    pub fn new(token: Option<String>) -> Self {
        GitHubFetcher { github_token: token }
    }

    pub async fn fetch(&self, owner: &str, repo: &str, version: &Version) -> Result<Vec<u8>> {
        tracing::info!("Fetching from GitHub: {}/{} v{}", owner, repo, version);

        let url = format!(
            "https://github.com/{}/{}/releases/download/v{}/package.tar.gz",
            owner, repo, version
        );

        let client = reqwest::Client::new();
        let mut request = client.get(&url);

        if let Some(token) = &self.github_token {
            request = request.header("Authorization", format!("token {}", token));
        }

        let response = request
            .send()
            .await
            .map_err(|e| RepositoryError::GitHubError(e.to_string()))?;

        if response.status() == 404 {
            return Err(RepositoryError::NotFound(format!("{}@{}", repo, version)));
        }

        response
            .bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| RepositoryError::NetworkError(e.to_string()))
    }

    pub async fn fetch_releases(&self, owner: &str, repo: &str) -> Result<Vec<GitHubRelease>> {
        tracing::info!("Fetching releases: {}/{}", owner, repo);

        let url = format!("https://api.github.com/repos/{}/{}/releases", owner, repo);

        let client = reqwest::Client::new();
        let mut request = client.get(&url);

        if let Some(token) = &self.github_token {
            request = request.header("Authorization", format!("token {}", token));
        }

        let response = request
            .send()
            .await
            .map_err(|e| RepositoryError::GitHubError(e.to_string()))?;

        response
            .json()
            .await
            .map_err(|e| RepositoryError::NetworkError(e.to_string()))
    }

    pub async fn fetch_manifest(&self, owner: &str, repo: &str, version: &Version) -> Result<Manifest> {
        tracing::info!("Fetching manifest: {}/{} v{}", owner, repo, version);

        let url = format!(
            "https://raw.githubusercontent.com/{}/{}/v{}/omni.manifest.json",
            owner, repo, version
        );

        let response = reqwest::get(&url)
            .await
            .map_err(|e| RepositoryError::NetworkError(e.to_string()))?;

        response
            .json()
            .await
            .map_err(|e| RepositoryError::NetworkError(e.to_string()))
    }

    pub fn get_token(&self) -> Option<&str> {
        self.github_token.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_github_fetcher_creation() {
        let fetcher = GitHubFetcher::new(None);
        assert!(fetcher.get_token().is_none());

        let fetcher_with_token = GitHubFetcher::new(Some("token123".to_string()));
        assert!(fetcher_with_token.get_token().is_some());
    }
}
