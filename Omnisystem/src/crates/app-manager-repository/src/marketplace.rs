use crate::{Result, RepositoryError};
use app_manager_core::{AppId, Version, Manifest};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppListing {
    pub app_id: AppId,
    pub name: String,
    pub description: String,
    pub version: Version,
    pub author: String,
    pub rating: f32,
    pub downloads: u64,
    pub url: String,
    pub repository: Option<String>,
}

pub struct Marketplace {
    base_url: String,
}

impl Marketplace {
    pub fn new(base_url: String) -> Self {
        Marketplace { base_url }
    }

    pub async fn search(&self, query: &str) -> Result<Vec<AppListing>> {
        tracing::info!("Searching marketplace: {}", query);

        let url = format!("{}/api/search?q={}", self.base_url, query);
        let response = reqwest::get(&url)
            .await
            .map_err(|e| RepositoryError::NetworkError(e.to_string()))?;

        response
            .json()
            .await
            .map_err(|e| RepositoryError::NetworkError(e.to_string()))
    }

    pub async fn get_app(&self, app_id: &AppId) -> Result<AppListing> {
        tracing::info!("Getting app from marketplace: {}", app_id);

        let url = format!("{}/api/apps/{}", self.base_url, app_id);
        let response = reqwest::get(&url)
            .await
            .map_err(|e| RepositoryError::NetworkError(e.to_string()))?;

        if response.status() == 404 {
            return Err(RepositoryError::NotFound(app_id.to_string()));
        }

        response
            .json()
            .await
            .map_err(|e| RepositoryError::NetworkError(e.to_string()))
    }

    pub async fn fetch(&self, app_id: &AppId, version: &Version) -> Result<Vec<u8>> {
        tracing::info!("Fetching {} v{} from marketplace", app_id, version);

        let url = format!("{}/api/packages/{}/{}", self.base_url, app_id, version);
        let response = reqwest::get(&url)
            .await
            .map_err(|e| RepositoryError::NetworkError(e.to_string()))?;

        response
            .bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| RepositoryError::NetworkError(e.to_string()))
    }

    pub async fn fetch_manifest(&self, app_id: &AppId, version: &Version) -> Result<Manifest> {
        tracing::info!("Fetching manifest for {} v{}", app_id, version);

        let url = format!("{}/api/manifests/{}/{}", self.base_url, app_id, version);
        let response = reqwest::get(&url)
            .await
            .map_err(|e| RepositoryError::NetworkError(e.to_string()))?;

        response
            .json()
            .await
            .map_err(|e| RepositoryError::NetworkError(e.to_string()))
    }

    pub async fn list_versions(&self, app_id: &AppId) -> Result<Vec<Version>> {
        tracing::info!("Listing versions for {}", app_id);

        let url = format!("{}/api/apps/{}/versions", self.base_url, app_id);
        let response = reqwest::get(&url)
            .await
            .map_err(|e| RepositoryError::NetworkError(e.to_string()))?;

        response
            .json()
            .await
            .map_err(|e| RepositoryError::NetworkError(e.to_string()))
    }

    pub async fn get_trending(&self) -> Result<Vec<AppListing>> {
        tracing::info!("Fetching trending apps");

        let url = format!("{}/api/trending", self.base_url);
        let response = reqwest::get(&url)
            .await
            .map_err(|e| RepositoryError::NetworkError(e.to_string()))?;

        response
            .json()
            .await
            .map_err(|e| RepositoryError::NetworkError(e.to_string()))
    }

    pub async fn get_featured(&self) -> Result<Vec<AppListing>> {
        tracing::info!("Fetching featured apps");

        let url = format!("{}/api/featured", self.base_url);
        let response = reqwest::get(&url)
            .await
            .map_err(|e| RepositoryError::NetworkError(e.to_string()))?;

        response
            .json()
            .await
            .map_err(|e| RepositoryError::NetworkError(e.to_string()))
    }

    pub async fn rate_app(&self, app_id: &AppId, rating: f32) -> Result<()> {
        tracing::info!("Rating {} with score {}", app_id, rating);

        if rating < 0.0 || rating > 5.0 {
            return Err(RepositoryError::ValidationFailed("Rating must be 0-5".to_string()));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_marketplace_creation() {
        let marketplace = Marketplace::new("https://test.local".to_string());
        assert_eq!(marketplace.base_url, "https://test.local");
    }

    #[tokio::test]
    async fn test_invalid_rating() {
        let marketplace = Marketplace::new("https://test.local".to_string());
        let app_id = AppId::new("test").unwrap();

        let result = marketplace.rate_app(&app_id, 6.0).await;
        assert!(result.is_err());
    }
}
