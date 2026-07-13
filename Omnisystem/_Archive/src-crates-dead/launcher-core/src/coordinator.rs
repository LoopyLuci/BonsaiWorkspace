use crate::error::LauncherResult;
use async_trait::async_trait;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchRequest {
    pub request_id: Uuid,
    pub app_id: String,
    pub session_id: Uuid,
    pub args: Vec<String>,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum LaunchPhase {
    Validation,
    DependencyResolution,
    EnvironmentSetup,
    ProcessCreation,
    Monitoring,
    Complete,
}

#[derive(Debug, Clone)]
pub struct LaunchContext {
    pub request: LaunchRequest,
    pub phase: LaunchPhase,
    pub resolved_dependencies: Vec<String>,
}

#[async_trait]
pub trait LaunchCoordinator: Send + Sync {
    async fn submit_launch_request(&self, request: LaunchRequest) -> LauncherResult<Uuid>;
    async fn get_launch_status(&self, request_id: &Uuid) -> LauncherResult<Option<LaunchContext>>;
    async fn cancel_launch(&self, request_id: &Uuid) -> LauncherResult<()>;
}

pub struct DefaultLaunchCoordinator {
    requests: Arc<DashMap<Uuid, LaunchContext>>,
}

impl DefaultLaunchCoordinator {
    pub fn new() -> Self {
        Self {
            requests: Arc::new(DashMap::new()),
        }
    }
}

impl Default for DefaultLaunchCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LaunchCoordinator for DefaultLaunchCoordinator {
    async fn submit_launch_request(&self, request: LaunchRequest) -> LauncherResult<Uuid> {
        let request_id = request.request_id;
        let context = LaunchContext {
            request,
            phase: LaunchPhase::Validation,
            resolved_dependencies: vec![],
        };
        self.requests.insert(request_id, context);
        Ok(request_id)
    }

    async fn get_launch_status(&self, request_id: &Uuid) -> LauncherResult<Option<LaunchContext>> {
        Ok(self.requests.get(request_id).map(|c| c.clone()))
    }

    async fn cancel_launch(&self, request_id: &Uuid) -> LauncherResult<()> {
        self.requests.remove(request_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_submit_launch_request() {
        let coordinator = DefaultLaunchCoordinator::new();
        let request = LaunchRequest {
            request_id: Uuid::new_v4(),
            app_id: "test-app".to_string(),
            session_id: Uuid::new_v4(),
            args: vec![],
            dependencies: vec![],
        };
        let request_id = request.request_id;
        let result = coordinator.submit_launch_request(request).await.unwrap();
        assert_eq!(result, request_id);
    }

    #[tokio::test]
    async fn test_get_launch_status() {
        let coordinator = DefaultLaunchCoordinator::new();
        let request = LaunchRequest {
            request_id: Uuid::new_v4(),
            app_id: "test-app".to_string(),
            session_id: Uuid::new_v4(),
            args: vec![],
            dependencies: vec![],
        };
        let request_id = request.request_id;
        coordinator.submit_launch_request(request).await.unwrap();
        let status = coordinator.get_launch_status(&request_id).await.unwrap();
        assert!(status.is_some());
        assert_eq!(status.unwrap().phase, LaunchPhase::Validation);
    }

    #[tokio::test]
    async fn test_cancel_launch() {
        let coordinator = DefaultLaunchCoordinator::new();
        let request = LaunchRequest {
            request_id: Uuid::new_v4(),
            app_id: "test-app".to_string(),
            session_id: Uuid::new_v4(),
            args: vec![],
            dependencies: vec![],
        };
        let request_id = request.request_id;
        coordinator.submit_launch_request(request).await.unwrap();
        coordinator.cancel_launch(&request_id).await.unwrap();
        let status = coordinator.get_launch_status(&request_id).await.unwrap();
        assert!(status.is_none());
    }
}
