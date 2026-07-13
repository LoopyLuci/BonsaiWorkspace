use crate::{FtpError, FtpResult, FtpSession, Protocol, SessionId, SessionManager, UserId};
use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;

pub struct DefaultSessionManager {
    sessions: Arc<DashMap<String, FtpSession>>,
    user_sessions: Arc<DashMap<String, Vec<SessionId>>>,
}

impl DefaultSessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
            user_sessions: Arc::new(DashMap::new()),
        }
    }

    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }
}

impl Default for DefaultSessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SessionManager for DefaultSessionManager {
    async fn create_session(
        &self,
        user_id: &UserId,
        protocol: Protocol,
        remote_addr: String,
    ) -> FtpResult<SessionId> {
        let session = FtpSession::new(user_id.clone(), protocol, remote_addr);
        let session_id = session.id.clone();

        self.sessions
            .insert(session_id.0.to_string(), session);

        self.user_sessions
            .entry(user_id.0.clone())
            .or_insert_with(Vec::new)
            .push(session_id.clone());

        Ok(session_id)
    }

    async fn get_session(&self, session_id: &SessionId) -> FtpResult<FtpSession> {
        self.sessions
            .get(&session_id.0.to_string())
            .map(|entry| entry.clone())
            .ok_or_else(|| FtpError::SessionNotFound(session_id.0.to_string()))
    }

    async fn update_session(&self, session_id: &SessionId, session: FtpSession) -> FtpResult<()> {
        if self.sessions.contains_key(&session_id.0.to_string()) {
            self.sessions.insert(session_id.0.to_string(), session);
            Ok(())
        } else {
            Err(FtpError::SessionNotFound(session_id.0.to_string()))
        }
    }

    async fn close_session(&self, session_id: &SessionId) -> FtpResult<()> {
        self.sessions.remove(&session_id.0.to_string());
        Ok(())
    }

    async fn list_sessions(&self, user_id: &UserId) -> FtpResult<Vec<FtpSession>> {
        let session_ids = self
            .user_sessions
            .get(&user_id.0)
            .map(|entry| entry.clone())
            .unwrap_or_default();

        let sessions: Vec<FtpSession> = session_ids
            .iter()
            .filter_map(|id| self.sessions.get(&id.0.to_string()).map(|entry| entry.clone()))
            .collect();

        Ok(sessions)
    }

    async fn get_active_session_count(&self, user_id: &UserId) -> FtpResult<usize> {
        let sessions = self.list_sessions(user_id).await?;
        Ok(sessions
            .iter()
            .filter(|s| s.status != crate::SessionStatus::Closed)
            .count())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_session() {
        let manager = DefaultSessionManager::new();
        let user_id = UserId("user1".to_string());

        let session_id = manager
            .create_session(&user_id, Protocol::Ftp, "127.0.0.1:21".to_string())
            .await
            .unwrap();

        assert_eq!(manager.session_count(), 1);
    }

    #[tokio::test]
    async fn test_get_session() {
        let manager = DefaultSessionManager::new();
        let user_id = UserId("user1".to_string());

        let _session_id = manager
            .create_session(&user_id, Protocol::Sftp, "127.0.0.1:22".to_string())
            .await
            .unwrap();

        let session = manager.get_session(&_session_id).await.unwrap();
        assert_eq!(session.user_id, user_id);
        assert_eq!(session.protocol, Protocol::Sftp);
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let manager = DefaultSessionManager::new();
        let user_id = UserId("user1".to_string());

        let _ = manager
            .create_session(&user_id, Protocol::Ftp, "127.0.0.1:21".to_string())
            .await
            .unwrap();

        let _ = manager
            .create_session(&user_id, Protocol::Sftp, "127.0.0.1:22".to_string())
            .await
            .unwrap();

        let sessions = manager.list_sessions(&user_id).await.unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[tokio::test]
    async fn test_close_session() {
        let manager = DefaultSessionManager::new();
        let user_id = UserId("user1".to_string());

        let session_id = manager
            .create_session(&user_id, Protocol::Ftp, "127.0.0.1:21".to_string())
            .await
            .unwrap();

        assert_eq!(manager.session_count(), 1);

        manager.close_session(&session_id).await.unwrap();
        assert_eq!(manager.session_count(), 0);
    }

    #[tokio::test]
    async fn test_get_active_session_count() {
        let manager = DefaultSessionManager::new();
        let user_id = UserId("user1".to_string());

        manager
            .create_session(&user_id, Protocol::Ftp, "127.0.0.1:21".to_string())
            .await
            .unwrap();

        manager
            .create_session(&user_id, Protocol::Sftp, "127.0.0.1:22".to_string())
            .await
            .unwrap();

        let count = manager.get_active_session_count(&user_id).await.unwrap();
        assert_eq!(count, 2);
    }
}
