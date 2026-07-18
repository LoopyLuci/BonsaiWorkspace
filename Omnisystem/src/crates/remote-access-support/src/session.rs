use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct RemoteSession {
    pub session_id: String,
    pub user_id: String,
    pub device_id: String,
    pub connected: bool,
    pub timestamp: u64,
}

pub struct SessionManager {
    sessions: Arc<DashMap<String, RemoteSession>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(DashMap::new()),
        }
    }

    pub fn create_session(&self, user_id: String, device_id: String) -> String {
        let session_id = uuid::Uuid::new_v4().to_string();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let session = RemoteSession {
            session_id: session_id.clone(),
            user_id,
            device_id,
            connected: true,
            timestamp,
        };
        self.sessions.insert(session_id.clone(), session);
        session_id
    }

    pub fn get_session(&self, session_id: &str) -> Option<RemoteSession> {
        self.sessions.get(session_id).map(|s| s.clone())
    }

    pub fn disconnect_session(&self, session_id: &str) -> bool {
        if let Some(mut session) = self.sessions.get_mut(session_id) {
            session.connected = false;
            true
        } else {
            false
        }
    }

    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let sm = SessionManager::new();
        let session_id = sm.create_session("user1".to_string(), "device1".to_string());
        assert!(!session_id.is_empty());
        assert_eq!(sm.session_count(), 1);
    }

    #[test]
    fn test_session_disconnect() {
        let sm = SessionManager::new();
        let session_id = sm.create_session("user1".to_string(), "device1".to_string());
        assert!(sm.disconnect_session(&session_id));
        let session = sm.get_session(&session_id).unwrap();
        assert!(!session.connected);
    }

    #[test]
    fn test_session_timestamp_is_real_creation_time() {
        // Regression test: timestamp used to be hardcoded to 0 regardless
        // of when the session was actually created.
        let before = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let sm = SessionManager::new();
        let session_id = sm.create_session("user1".to_string(), "device1".to_string());
        let session = sm.get_session(&session_id).unwrap();

        assert!(session.timestamp >= before);
    }
}
