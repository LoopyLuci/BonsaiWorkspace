use crate::error::LauncherResult;
use async_trait::async_trait;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppMetadata {
    pub app_id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub executable: PathBuf,
    pub args: Vec<String>,
    pub dependencies: Vec<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum AppStatus {
    Registered,
    Running,
    Failed,
}

#[async_trait]
pub trait AppRegistry: Send + Sync {
    async fn register_app(&self, metadata: AppMetadata) -> LauncherResult<()>;
    async fn get_app(&self, app_id: &str) -> LauncherResult<Option<AppMetadata>>;
    async fn list_apps(&self) -> LauncherResult<Vec<AppMetadata>>;
    async fn search_apps(&self, query: &str) -> LauncherResult<Vec<AppMetadata>>;
    async fn unregister_app(&self, app_id: &str) -> LauncherResult<()>;
}

pub struct DefaultAppRegistry {
    apps: Arc<DashMap<String, AppMetadata>>,
}

impl DefaultAppRegistry {
    pub fn new() -> Self {
        Self {
            apps: Arc::new(DashMap::new()),
        }
    }
}

impl Default for DefaultAppRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AppRegistry for DefaultAppRegistry {
    async fn register_app(&self, metadata: AppMetadata) -> LauncherResult<()> {
        self.apps.insert(metadata.app_id.clone(), metadata);
        Ok(())
    }

    async fn get_app(&self, app_id: &str) -> LauncherResult<Option<AppMetadata>> {
        Ok(self.apps.get(app_id).map(|a| a.clone()))
    }

    async fn list_apps(&self) -> LauncherResult<Vec<AppMetadata>> {
        Ok(self.apps.iter().map(|entry| entry.value().clone()).collect())
    }

    async fn search_apps(&self, query: &str) -> LauncherResult<Vec<AppMetadata>> {
        let query_lower = query.to_lowercase();
        Ok(self
            .apps
            .iter()
            .filter(|entry| {
                let app = entry.value();
                app.app_id.to_lowercase().contains(&query_lower)
                    || app.name.to_lowercase().contains(&query_lower)
                    || app.description.to_lowercase().contains(&query_lower)
            })
            .map(|entry| entry.value().clone())
            .collect())
    }

    async fn unregister_app(&self, app_id: &str) -> LauncherResult<()> {
        self.apps.remove(app_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_and_get_app() {
        let registry = DefaultAppRegistry::new();
        let app = AppMetadata {
            app_id: "test-app".to_string(),
            name: "Test App".to_string(),
            version: "1.0.0".to_string(),
            description: "A test application".to_string(),
            executable: PathBuf::from("/usr/bin/test"),
            args: vec![],
            dependencies: vec![],
            tags: vec!["test".to_string()],
        };
        
        registry.register_app(app.clone()).await.unwrap();
        let retrieved = registry.get_app("test-app").await.unwrap();
        assert_eq!(retrieved, Some(app));
    }

    #[tokio::test]
    async fn test_search_apps() {
        let registry = DefaultAppRegistry::new();
        registry
            .register_app(AppMetadata {
                app_id: "app1".to_string(),
                name: "Firefox".to_string(),
                version: "1.0.0".to_string(),
                description: "Browser".to_string(),
                executable: PathBuf::from("/usr/bin/firefox"),
                args: vec![],
                dependencies: vec![],
                tags: vec![],
            })
            .await
            .unwrap();

        let results = registry.search_apps("fire").await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_list_apps() {
        let registry = DefaultAppRegistry::new();
        for i in 0..3 {
            registry
                .register_app(AppMetadata {
                    app_id: format!("app{}", i),
                    name: format!("App {}", i),
                    version: "1.0.0".to_string(),
                    description: "Test".to_string(),
                    executable: PathBuf::from("/usr/bin/test"),
                    args: vec![],
                    dependencies: vec![],
                    tags: vec![],
                })
                .await
                .unwrap();
        }
        let apps = registry.list_apps().await.unwrap();
        assert_eq!(apps.len(), 3);
    }
}
