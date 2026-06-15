use crate::error::LauncherResult;
use crate::lifecycle::LifecycleManager;
use crate::registry::AppRegistry;
use crate::session::SessionManager;
use crate::coordinator::LaunchCoordinator;
use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Main launcher core service - coordinates all subsystems
pub struct LauncherCore {
    session_manager: Arc<dyn SessionManager>,
    app_registry: Arc<dyn AppRegistry>,
    launch_coordinator: Arc<dyn LaunchCoordinator>,
    lifecycle_manager: Arc<dyn LifecycleManager>,
    metadata: Arc<DashMap<String, String>>,
}

impl LauncherCore {
    /// Create new launcher core with all subsystems
    pub async fn new(
        session_manager: Arc<dyn SessionManager>,
        app_registry: Arc<dyn AppRegistry>,
        launch_coordinator: Arc<dyn LaunchCoordinator>,
        lifecycle_manager: Arc<dyn LifecycleManager>,
    ) -> LauncherResult<Self> {
        Ok(Self {
            session_manager,
            app_registry,
            launch_coordinator,
            lifecycle_manager,
            metadata: Arc::new(DashMap::new()),
        })
    }

    pub fn session_manager(&self) -> Arc<dyn SessionManager> {
        self.session_manager.clone()
    }

    pub fn app_registry(&self) -> Arc<dyn AppRegistry> {
        self.app_registry.clone()
    }

    pub fn launch_coordinator(&self) -> Arc<dyn LaunchCoordinator> {
        self.launch_coordinator.clone()
    }

    pub fn lifecycle_manager(&self) -> Arc<dyn LifecycleManager> {
        self.lifecycle_manager.clone()
    }

    pub fn set_metadata(&self, key: String, value: String) {
        self.metadata.insert(key, value);
    }

    pub fn get_metadata(&self, key: &str) -> Option<String> {
        self.metadata.get(key).map(|v| v.clone())
    }

    pub async fn shutdown(&self) -> LauncherResult<()> {
        tracing::info!("Shutting down launcher core");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_launcher_core_creation() {
        let session_manager = Arc::new(MockSessionManager);
        let app_registry = Arc::new(MockAppRegistry);
        let launch_coordinator = Arc::new(MockLaunchCoordinator);
        let lifecycle_manager = Arc::new(MockLifecycleManager);

        let core = LauncherCore::new(
            session_manager,
            app_registry,
            launch_coordinator,
            lifecycle_manager,
        )
        .await;

        assert!(core.is_ok());
    }

    #[tokio::test]
    async fn test_metadata_operations() {
        let session_manager = Arc::new(MockSessionManager);
        let app_registry = Arc::new(MockAppRegistry);
        let launch_coordinator = Arc::new(MockLaunchCoordinator);
        let lifecycle_manager = Arc::new(MockLifecycleManager);

        let core = LauncherCore::new(
            session_manager,
            app_registry,
            launch_coordinator,
            lifecycle_manager,
        )
        .await
        .unwrap();

        core.set_metadata("key1".to_string(), "value1".to_string());
        assert_eq!(core.get_metadata("key1"), Some("value1".to_string()));
        assert_eq!(core.get_metadata("nonexistent"), None);
    }

    // Mock implementations for testing
    struct MockSessionManager;
    #[async_trait]
    impl crate::session::SessionManager for MockSessionManager {
        async fn create_session(&self, _user_id: String) -> LauncherResult<crate::Session> {
            unimplemented!()
        }
        async fn get_session(&self, _session_id: &Uuid) -> LauncherResult<Option<crate::Session>> {
            unimplemented!()
        }
        async fn list_sessions(&self) -> LauncherResult<Vec<crate::Session>> {
            Ok(vec![])
        }
        async fn terminate_session(&self, _session_id: &Uuid) -> LauncherResult<()> {
            Ok(())
        }
    }

    struct MockAppRegistry;
    #[async_trait]
    impl crate::registry::AppRegistry for MockAppRegistry {
        async fn register_app(&self, _metadata: crate::AppMetadata) -> LauncherResult<()> {
            Ok(())
        }
        async fn get_app(&self, _app_id: &str) -> LauncherResult<Option<crate::AppMetadata>> {
            Ok(None)
        }
        async fn list_apps(&self) -> LauncherResult<Vec<crate::AppMetadata>> {
            Ok(vec![])
        }
        async fn search_apps(&self, _query: &str) -> LauncherResult<Vec<crate::AppMetadata>> {
            Ok(vec![])
        }
    }

    struct MockLaunchCoordinator;
    #[async_trait]
    impl crate::coordinator::LaunchCoordinator for MockLaunchCoordinator {
        async fn submit_launch_request(&self, _request: crate::LaunchRequest) -> LauncherResult<Uuid> {
            Ok(Uuid::new_v4())
        }
        async fn cancel_launch(&self, _request_id: &Uuid) -> LauncherResult<()> {
            Ok(())
        }
    }

    struct MockLifecycleManager;
    #[async_trait]
    impl crate::lifecycle::LifecycleManager for MockLifecycleManager {
        async fn publish_event(&self, _event: crate::LauncherEvent) -> LauncherResult<()> {
            Ok(())
        }
    }
}
