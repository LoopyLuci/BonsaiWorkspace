/// Bonsai Plugin Marketplace
/// Community plugin distribution and discovery

use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BonsaiPlugin {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub rules: Vec<String>,
    pub languages: Vec<String>,
    pub rating: f32,
    pub rating_count: u32,
    pub downloads: u32,
    pub tags: Vec<String>,
    pub repository: Option<String>,
    pub license: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMetadata {
    pub plugin_id: String,
    pub installed: bool,
    pub installed_version: Option<String>,
    pub update_available: bool,
    pub latest_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishResult {
    pub plugin_id: String,
    pub version: String,
    pub published_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatingRequest {
    pub plugin_id: String,
    pub rating: f32,
}

pub struct PluginMarketplace {
    registry_url: String,
    http_client: Arc<Client>,
    cache: std::sync::Mutex<HashMap<String, BonsaiPlugin>>,
}

impl PluginMarketplace {
    pub async fn new(registry_url: String) -> Result<Self> {
        tracing::info!("Initializing plugin marketplace at: {}", registry_url);

        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        Ok(Self {
            registry_url,
            http_client: Arc::new(http_client),
            cache: std::sync::Mutex::new(HashMap::new()),
        })
    }

    /// Search plugins by query
    pub async fn search_plugins(&self, query: &str) -> Result<Vec<BonsaiPlugin>> {
        tracing::debug!("Searching plugins: {}", query);

        let cache = self.cache.lock().unwrap();
        if let Some(cached) = cache.get(&format!("search:{}", query)) {
            tracing::debug!("Returning cached search result for: {}", query);
            return Ok(vec![cached.clone()]);
        }
        drop(cache);

        let url = format!(
            "{}/search?q={}",
            self.registry_url,
            urlencoding::encode(query)
        );

        let response = self.http_client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to search plugins: HTTP {}",
                response.status()
            ));
        }

        let plugins: Vec<BonsaiPlugin> = response.json().await?;

        for plugin in &plugins {
            let mut cache = self.cache.lock().unwrap();
            cache.insert(plugin.id.clone(), plugin.clone());
        }

        tracing::info!("Found {} plugins matching query: {}", plugins.len(), query);
        Ok(plugins)
    }

    /// Get plugins by language
    pub async fn get_plugins_for_language(&self, language: &str) -> Result<Vec<BonsaiPlugin>> {
        tracing::debug!("Fetching plugins for language: {}", language);

        let cache = self.cache.lock().unwrap();
        if let Some(cached) = cache.get(&format!("lang:{}", language)) {
            return Ok(vec![cached.clone()]);
        }
        drop(cache);

        let url = format!("{}/language/{}", self.registry_url, language);
        let response = self.http_client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to fetch plugins for language: HTTP {}",
                response.status()
            ));
        }

        let plugins: Vec<BonsaiPlugin> = response.json().await?;

        for plugin in &plugins {
            let mut cache = self.cache.lock().unwrap();
            cache.insert(plugin.id.clone(), plugin.clone());
        }

        tracing::info!(
            "Found {} plugins for language: {}",
            plugins.len(),
            language
        );
        Ok(plugins)
    }

    /// Get top-rated plugins
    pub async fn get_top_plugins(&self, limit: usize) -> Result<Vec<BonsaiPlugin>> {
        tracing::debug!("Fetching top {} plugins", limit);

        let url = format!("{}/top?limit={}", self.registry_url, limit);
        let response = self.http_client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to fetch top plugins: HTTP {}",
                response.status()
            ));
        }

        let plugins: Vec<BonsaiPlugin> = response.json().await?;

        for plugin in &plugins {
            let mut cache = self.cache.lock().unwrap();
            cache.insert(plugin.id.clone(), plugin.clone());
        }

        tracing::info!("Fetched {} top-rated plugins", plugins.len());
        Ok(plugins)
    }

    /// Install a plugin
    pub async fn install_plugin(
        &mut self,
        plugin_id: &str,
        version: Option<&str>,
    ) -> Result<BonsaiPlugin> {
        tracing::info!("Installing plugin: {}", plugin_id);

        let plugin = self.fetch_plugin(plugin_id, version).await?;

        self.extract_and_register(&plugin).await?;

        tracing::info!("Plugin {} v{} installed", plugin.name, plugin.version);
        Ok(plugin)
    }

    /// Uninstall a plugin
    pub async fn uninstall_plugin(&mut self, plugin_id: &str) -> Result<()> {
        tracing::info!("Uninstalling plugin: {}", plugin_id);

        self.remove_plugin_files(plugin_id).await?;

        let mut cache = self.cache.lock().unwrap();
        cache.remove(plugin_id);

        tracing::info!("Plugin {} uninstalled", plugin_id);
        Ok(())
    }

    /// Publish a plugin to marketplace
    pub async fn publish_plugin(&self, plugin: BonsaiPlugin) -> Result<PublishResult> {
        tracing::info!("Publishing plugin: {} v{}", plugin.name, plugin.version);

        let url = format!("{}/publish", self.registry_url);

        let response = self.http_client
            .post(&url)
            .json(&plugin)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to publish plugin: HTTP {}",
                response.status()
            ));
        }

        let result: PublishResult = response.json().await?;

        tracing::info!(
            "Plugin {} v{} published at {}",
            plugin.name,
            plugin.version,
            result.published_at
        );

        Ok(result)
    }

    /// Rate a plugin
    pub async fn rate_plugin(&self, plugin_id: &str, rating: f32) -> Result<()> {
        let rating = rating.clamp(1.0, 5.0);
        tracing::info!("Rating plugin {}: {:.1} stars", plugin_id, rating);

        let url = format!("{}/rate", self.registry_url);

        let request = RatingRequest {
            plugin_id: plugin_id.to_string(),
            rating,
        };

        let response = self.http_client
            .post(&url)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Failed to rate plugin: HTTP {}",
                response.status()
            ));
        }

        tracing::info!("Successfully rated plugin {} with {:.1} stars", plugin_id, rating);
        Ok(())
    }

    /// Check for plugin updates
    pub async fn check_updates(
        &self,
        installed_plugins: Vec<PluginMetadata>,
    ) -> Result<Vec<PluginMetadata>> {
        let mut updates = Vec::new();

        for plugin in installed_plugins {
            match self.fetch_latest_version(&plugin.plugin_id).await {
                Ok(latest_version) => {
                    let installed_version = plugin.installed_version.as_deref().unwrap_or("0.0.0");
                    if self.is_newer_version(&latest_version, installed_version) {
                        let mut updated = plugin.clone();
                        updated.update_available = true;
                        updated.latest_version = latest_version;
                        updates.push(updated);
                        tracing::info!(
                            "Update available for plugin {}: {} -> {}",
                            plugin.plugin_id,
                            installed_version,
                            latest_version
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to check updates for {}: {}", plugin.plugin_id, e);
                }
            }
        }

        tracing::info!("Found {} plugins with available updates", updates.len());
        Ok(updates)
    }

    async fn fetch_plugin(&self, plugin_id: &str, version: Option<&str>) -> Result<BonsaiPlugin> {
        tracing::debug!("Fetching plugin metadata: {}", plugin_id);

        let cache = self.cache.lock().unwrap();
        if let Some(cached) = cache.get(plugin_id) {
            return Ok(cached.clone());
        }
        drop(cache);

        let url = if let Some(v) = version {
            format!("{}/plugins/{}?version={}", self.registry_url, plugin_id, v)
        } else {
            format!("{}/plugins/{}", self.registry_url, plugin_id)
        };

        let response = self.http_client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "Plugin not found: {} (HTTP {})",
                plugin_id,
                response.status()
            ));
        }

        let plugin: BonsaiPlugin = response.json().await?;

        let mut cache = self.cache.lock().unwrap();
        cache.insert(plugin.id.clone(), plugin.clone());

        Ok(plugin)
    }

    async fn fetch_latest_version(&self, plugin_id: &str) -> Result<String> {
        let url = format!("{}/plugins/{}/latest", self.registry_url, plugin_id);
        let response = self.http_client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!("Failed to fetch latest version"));
        }

        let data: serde_json::Value = response.json().await?;
        let version = data
            .get("version")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Invalid response format"))?;

        Ok(version.to_string())
    }

    async fn extract_and_register(&self, plugin: &BonsaiPlugin) -> Result<()> {
        tracing::debug!("Extracting and registering plugin: {}", plugin.id);

        let plugin_dir = format!("./plugins/{}", plugin.id);
        tokio::fs::create_dir_all(&plugin_dir).await?;

        let metadata_file = format!("{}/plugin.json", plugin_dir);
        let metadata = serde_json::to_string_pretty(plugin)?;
        tokio::fs::write(&metadata_file, metadata).await?;

        tracing::info!("Plugin {} registered at {}", plugin.id, plugin_dir);
        Ok(())
    }

    async fn remove_plugin_files(&self, plugin_id: &str) -> Result<()> {
        tracing::debug!("Removing plugin files: {}", plugin_id);

        let plugin_dir = format!("./plugins/{}", plugin_id);

        if tokio::fs::try_exists(&plugin_dir).await? {
            tokio::fs::remove_dir_all(&plugin_dir).await?;
        }

        tracing::info!("Plugin files removed: {}", plugin_id);
        Ok(())
    }

    fn is_newer_version(&self, new_version: &str, old_version: &str) -> bool {
        let new_parts: Vec<&str> = new_version.split('.').collect();
        let old_parts: Vec<&str> = old_version.split('.').collect();

        for i in 0..3.min(new_parts.len()).min(old_parts.len()) {
            if let (Ok(new), Ok(old)) = (
                new_parts[i].parse::<u32>(),
                old_parts[i].parse::<u32>(),
            ) {
                if new > old {
                    return true;
                } else if new < old {
                    return false;
                }
            }
        }

        new_parts.len() > old_parts.len()
    }
}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_marketplace_creation() {
        let marketplace = PluginMarketplace::new("https://plugins.bonsai.sh".to_string())
            .await
            .unwrap();
        assert_eq!(marketplace.registry_url, "https://plugins.bonsai.sh");
    }

    #[tokio::test]
    async fn test_search_plugins() {
        let marketplace = PluginMarketplace::new("https://plugins.bonsai.sh".to_string())
            .await
            .unwrap();
        let plugins = marketplace.search_plugins("performance").await.unwrap();
        assert!(plugins.is_empty()); // No mock data
    }

    #[tokio::test]
    async fn test_install_plugin() {
        let marketplace = PluginMarketplace::new("https://plugins.bonsai.sh".to_string())
            .await
            .unwrap();
        let plugin = marketplace.install_plugin("test-plugin", None).await.unwrap();
        assert_eq!(plugin.id, "test-plugin");
    }

    #[tokio::test]
    async fn test_publish_plugin() {
        let marketplace = PluginMarketplace::new("https://plugins.bonsai.sh".to_string())
            .await
            .unwrap();
        let plugin = BonsaiPlugin {
            id: "my-plugin".to_string(),
            name: "My Plugin".to_string(),
            version: "1.0.0".to_string(),
            author: "me".to_string(),
            description: "My plugin".to_string(),
            rules: vec!["rule1".to_string()],
            languages: vec!["rust".to_string()],
            rating: 0.0,
            rating_count: 0,
            downloads: 0,
            tags: vec![],
            repository: None,
            license: "MIT".to_string(),
        };
        let result = marketplace.publish_plugin(plugin).await.unwrap();
        assert_eq!(result.plugin_id, "my-plugin");
    }
}
