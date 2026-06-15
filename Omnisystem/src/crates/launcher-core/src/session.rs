use crate::error::LauncherResult;
use crate::types::AppInstance;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub session_id: Uuid,
    pub user_id: String,
    pub created_at: DateTime<Utc>,
    pub environment: HashMap<String, String>,
    pub status: SessionStatus,
    pub apps: Vec<AppInstance>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SessionStatus {
    Initializing,
    Ready,
    Running,
    Pausing,
    Paused,
    Stopping,
    Stopped,
}

#[async_trait]
pub trait SessionManager: Send + Sync {
    async fn create_session(&self, user_id: String) -> LauncherResult<Session>;
    async fn get_session(&self, session_id: &Uuid) -> LauncherResult<Option<Session>>;
    async fn list_sessions(&self) -> LauncherResult<Vec<Session>>;
    async fn update_session_status(&self, session_id: &Uuid, status: SessionStatus) -> LauncherResult<()>;
    async fn terminate_session(&self, session_id: &Uuid) -> LauncherResult<()>;
}

pub struct DefaultSessionManager {
    sessions: Arc<DashMap<Uuid, Session>>,
}

impl DefaultSessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
        }
    }
}

impl Default for DefaultSessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SessionManager for DefaultSessionManager {
    async fn create_session(&self, user_id: String) -> LauncherResult<Session> {
        let session_id = Uuid::new_v4();
        let session = Session {
            session_id,
            user_id,
            created_at: Utc::now(),
            environment: HashMap::new(),
            status: SessionStatus::Initializing,
            apps: vec![],
        };
        self.sessions.insert(session_id, session.clone());
        Ok(session)
    }

    async fn get_session(&self, session_id: &Uuid) -> LauncherResult<Option<Session>> {
        Ok(self.sessions.get(session_id).map(|s| s.clone()))
    }

    async fn list_sessions(&self) -> LauncherResult<Vec<Session>> {
        Ok(self
            .sessions
            .iter()
            .map(|entry| entry.value().clone())
            .collect())
    }

    async fn update_session_status(&self, session_id: &Uuid, status: SessionStatus) -> LauncherResult<()> {
        if let Some(mut session) = self.sessions.get_mut(session_id) {
            session.status = status;
            Ok(())
        } else {
            Err(crate::error::LauncherError::SessionNotFound(*session_id))
        }
    }

    async fn terminate_session(&self, session_id: &Uuid) -> LauncherResult<()> {
        self.sessions.remove(session_id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_session() {
        let manager = DefaultSessionManager::new();
        let session = manager.create_session("user1".to_string()).await.unwrap();
        assert_eq!(session.user_id, "user1");
        assert_eq!(session.status, SessionStatus::Initializing);
    }

    #[tokio::test]
    async fn test_get_session() {
        let manager = DefaultSessionManager::new();
        let created = manager.create_session("user1".to_string()).await.unwrap();
        let retrieved = manager.get_session(&created.session_id).await.unwrap();
        assert_eq!(retrieved, Some(created));
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let manager = DefaultSessionManager::new();
        manager.create_session("user1".to_string()).await.unwrap();
        manager.create_session("user2".to_string()).await.unwrap();
        let sessions = manager.list_sessions().await.unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[tokio::test]
    async fn test_update_session_status() {
        let manager = DefaultSessionManager::new();
        let session = manager.create_session("user1".to_string()).await.unwrap();
        manager
            .update_session_status(&session.session_id, SessionStatus::Ready)
            .await
            .unwrap();
        let updated = manager.get_session(&session.session_id).await.unwrap();
        assert_eq!(updated.unwrap().status, SessionStatus::Ready);
    }
}
