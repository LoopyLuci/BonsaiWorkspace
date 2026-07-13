use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Application metadata from registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub icon: Option<String>,
    pub executable: String,
}

/// Application instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInstance {
    pub instance_id: Uuid,
    pub app_id: String,
    pub status: String,
    pub pid: Option<u32>,
    pub started_at: String,
}

/// Launch request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchRequest {
    pub app_id: String,
    pub args: Vec<String>,
    pub priority: String,
}

/// Launch response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchResponse {
    pub instance_id: Uuid,
    pub status: String,
}

/// Client trait for extensible implementations
#[async_trait]
pub trait LauncherClient: Send + Sync {
    async fn list_apps(&self) -> Result<Vec<AppMetadata>>;
    async fn get_app(&self, app_id: &str) -> Result<Option<AppMetadata>>;
    async fn search_apps(&self, query: &str) -> Result<Vec<AppMetadata>>;
    async fn launch_app(&self, request: LaunchRequest) -> Result<LaunchResponse>;
    async fn list_instances(&self) -> Result<Vec<AppInstance>>;
    async fn terminate_app(&self, instance_id: &Uuid) -> Result<()>;
    async fn get_system_status(&self) -> Result<SystemStatus>;
}

/// System status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatus {
    pub healthy: bool,
    pub uptime_ms: u64,
    pub active_instances: usize,
    pub total_apps: usize,
}

/// In-memory mock implementation for testing and UI prototyping
pub struct MockLauncherClient {
    apps: Arc<RwLock<HashMap<String, AppMetadata>>>,
    instances: Arc<RwLock<Vec<AppInstance>>>,
}

impl MockLauncherClient {
    pub fn new() -> Self {
        let mut apps = HashMap::new();
        apps.insert(
            "app1".to_string(),
            AppMetadata {
                id: "app1".to_string(),
                name: "Text Editor".to_string(),
                version: "1.0.0".to_string(),
                description: "Edit text files".to_string(),
                icon: Some("📝".to_string()),
                executable: "/usr/bin/nano".to_string(),
            },
        );
        apps.insert(
            "app2".to_string(),
            AppMetadata {
                id: "app2".to_string(),
                name: "File Manager".to_string(),
                version: "2.1.0".to_string(),
                description: "Browse files".to_string(),
                icon: Some("📁".to_string()),
                executable: "/usr/bin/nautilus".to_string(),
            },
        );

        Self {
            apps: Arc::new(RwLock::new(apps)),
            instances: Arc::new(RwLock::new(vec![])),
        }
    }
}

#[async_trait]
impl LauncherClient for MockLauncherClient {
    async fn list_apps(&self) -> Result<Vec<AppMetadata>> {
        let apps = self.apps.read().await;
        Ok(apps.values().cloned().collect())
    }

    async fn get_app(&self, app_id: &str) -> Result<Option<AppMetadata>> {
        let apps = self.apps.read().await;
        Ok(apps.get(app_id).cloned())
    }

    async fn search_apps(&self, query: &str) -> Result<Vec<AppMetadata>> {
        let apps = self.apps.read().await;
        let q = query.to_lowercase();
        Ok(apps
            .values()
            .filter(|app| {
                app.name.to_lowercase().contains(&q)
                    || app.description.to_lowercase().contains(&q)
            })
            .cloned()
            .collect())
    }

    async fn launch_app(&self, request: LaunchRequest) -> Result<LaunchResponse> {
        let instance_id = Uuid::new_v4();
        let instance = AppInstance {
            instance_id,
            app_id: request.app_id,
            status: "running".to_string(),
            pid: Some(12345),
            started_at: chrono::Utc::now().to_rfc3339(),
        };

        let mut instances = self.instances.write().await;
        instances.push(instance);

        Ok(LaunchResponse {
            instance_id,
            status: "launched".to_string(),
        })
    }

    async fn list_instances(&self) -> Result<Vec<AppInstance>> {
        let instances = self.instances.read().await;
        Ok(instances.clone())
    }

    async fn terminate_app(&self, instance_id: &Uuid) -> Result<()> {
        let mut instances = self.instances.write().await;
        instances.retain(|i| i.instance_id != *instance_id);
        Ok(())
    }

    async fn get_system_status(&self) -> Result<SystemStatus> {
        let instances = self.instances.read().await;
        Ok(SystemStatus {
            healthy: true,
            uptime_ms: 3600000,
            active_instances: instances.len(),
            total_apps: self.apps.read().await.len(),
        })
    }
}

/// Default UI client wrapper
pub struct UI;

impl UI {
    pub async fn render() -> Result<()> {
        let client = MockLauncherClient::new();
        let _apps = client.list_apps().await?;
        Ok(())
    }

    pub fn with_client(client: Arc<dyn LauncherClient>) -> UIClient {
        UIClient { client }
    }
}

/// UI client with injected dependency
pub struct UIClient {
    pub client: Arc<dyn LauncherClient>,
}

impl UIClient {
    pub async fn list_apps(&self) -> Result<Vec<AppMetadata>> {
        self.client.list_apps().await
    }

    pub async fn launch_app(&self, app_id: &str) -> Result<AppInstance> {
        let response = self
            .client
            .launch_app(LaunchRequest {
                app_id: app_id.to_string(),
                args: vec![],
                priority: "normal".to_string(),
            })
            .await?;

        Ok(AppInstance {
            instance_id: response.instance_id,
            app_id: app_id.to_string(),
            status: response.status,
            pid: Some(0),
            started_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    pub async fn search_apps(&self, query: &str) -> Result<Vec<AppMetadata>> {
        self.client.search_apps(query).await
    }

    pub async fn terminate_app(&self, instance_id: &Uuid) -> Result<()> {
        self.client.terminate_app(instance_id).await
    }

    pub async fn get_status(&self) -> Result<SystemStatus> {
        self.client.get_system_status().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_client_list_apps() {
        let client = MockLauncherClient::new();
        let apps = client.list_apps().await.unwrap();
        assert!(apps.len() >= 2);
        let names: Vec<&str> = apps.iter().map(|a| a.name.as_str()).collect();
        assert!(names.contains(&"Text Editor"));
        assert!(names.contains(&"File Manager"));
    }

    #[tokio::test]
    async fn test_mock_client_search() {
        let client = MockLauncherClient::new();
        let results = client.search_apps("file").await.unwrap();
        assert!(results.iter().any(|a| a.name.contains("File")));
    }

    #[tokio::test]
    async fn test_mock_client_launch() {
        let client = MockLauncherClient::new();
        let response = client
            .launch_app(LaunchRequest {
                app_id: "app1".to_string(),
                args: vec![],
                priority: "normal".to_string(),
            })
            .await
            .unwrap();

        assert_eq!(response.status, "launched");
        assert!(response.instance_id != Uuid::nil());
    }

    #[tokio::test]
    async fn test_mock_client_list_instances() {
        let client = MockLauncherClient::new();
        let _resp = client
            .launch_app(LaunchRequest {
                app_id: "app1".to_string(),
                args: vec![],
                priority: "normal".to_string(),
            })
            .await
            .unwrap();

        let instances = client.list_instances().await.unwrap();
        assert_eq!(instances.len(), 1);
    }

    #[tokio::test]
    async fn test_ui_client() {
        let client = Arc::new(MockLauncherClient::new());
        let ui = UIClient { client };
        let apps = ui.list_apps().await.unwrap();
        assert!(apps.len() > 0);
    }

    #[tokio::test]
    async fn test_ui_render() {
        assert!(UI::render().await.is_ok());
    }
}
