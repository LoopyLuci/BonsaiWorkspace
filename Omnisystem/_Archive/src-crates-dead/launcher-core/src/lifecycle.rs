use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::error::LauncherResult;
use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LauncherEvent {
    SessionCreated(String),
    SessionTerminated(Uuid),
    AppLaunching(String),
    AppStarted(String),
    AppStopped { app_id: String, exit_code: i32 },
    AppFailed { app_id: String, reason: String },
}

#[async_trait]
pub trait LifecycleManager: Send + Sync {
    async fn publish_event(&self, event: LauncherEvent) -> LauncherResult<()>;
    async fn get_event_history(&self, limit: usize) -> LauncherResult<Vec<LauncherEvent>>;
}

pub struct DefaultLifecycleManager {
    event_history: Arc<DashMap<usize, LauncherEvent>>,
    counter: Arc<std::sync::atomic::AtomicUsize>,
}

impl DefaultLifecycleManager {
    pub fn new() -> Self {
        Self {
            event_history: Arc::new(DashMap::new()),
            counter: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }
}

impl Default for DefaultLifecycleManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LifecycleManager for DefaultLifecycleManager {
    async fn publish_event(&self, event: LauncherEvent) -> LauncherResult<()> {
        let index = self.counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.event_history.insert(index, event);
        Ok(())
    }

    async fn get_event_history(&self, limit: usize) -> LauncherResult<Vec<LauncherEvent>> {
        let mut events: Vec<_> = self.event_history.iter().map(|e| e.value().clone()).collect();
        events.sort_by_key(|_| std::cmp::Reverse(0));
        Ok(events.into_iter().take(limit).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_publish_event() {
        let manager = DefaultLifecycleManager::new();
        let event = LauncherEvent::SessionCreated("user1".to_string());
        let result = manager.publish_event(event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_event_history() {
        let manager = DefaultLifecycleManager::new();
        manager
            .publish_event(LauncherEvent::SessionCreated("user1".to_string()))
            .await
            .unwrap();
        manager
            .publish_event(LauncherEvent::AppLaunching("app1".to_string()))
            .await
            .unwrap();

        let history = manager.get_event_history(10).await.unwrap();
        assert_eq!(history.len(), 2);
    }
}
