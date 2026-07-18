// Session management for stateful conversations

use crate::UserId;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub user_id: UserId,
    pub state: HashMap<String, serde_json::Value>,
    pub created_at: u64,
    pub last_activity: u64,
}

impl Session {
    pub fn new(user_id: UserId) -> Self {
        let now = chrono::Utc::now().timestamp() as u64;
        Self {
            user_id,
            state: HashMap::new(),
            created_at: now,
            last_activity: now,
        }
    }

    pub fn set(&mut self, key: String, value: serde_json::Value) {
        self.state.insert(key, value);
        self.last_activity = chrono::Utc::now().timestamp() as u64;
    }

    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.state.get(key)
    }

    pub fn clear(&mut self) {
        self.state.clear();
    }
}

pub struct SessionManager {
    sessions: DashMap<UserId, Arc<Session>>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: DashMap::new(),
        }
    }

    pub async fn get_or_create(&self, user_id: &UserId) -> Arc<Session> {
        if let Some(session) = self.sessions.get(user_id) {
            session.value().clone()
        } else {
            let session = Arc::new(Session::new(user_id.clone()));
            self.sessions.insert(user_id.clone(), session.clone());
            session
        }
    }

    pub fn get(&self, user_id: &UserId) -> Option<Arc<Session>> {
        self.sessions.get(user_id).map(|r| r.value().clone())
    }

    pub fn clear(&self, user_id: &UserId) {
        self.sessions.remove(user_id);
    }

    pub fn clear_all(&self) {
        self.sessions.clear();
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_creation() {
        let user_id = UserId::telegram("123");
        let mut session = Session::new(user_id);

        session.set("key1".into(), serde_json::json!("value1"));
        assert_eq!(session.get("key1"), Some(&serde_json::json!("value1")));
    }

    #[tokio::test]
    async fn test_session_manager() {
        let manager = SessionManager::new();
        let user_id = UserId::telegram("123");

        let session = manager.get_or_create(&user_id).await;
        assert_eq!(session.user_id, user_id);

        assert!(manager.get(&user_id).is_some());

        manager.clear(&user_id);
        assert!(manager.get(&user_id).is_none());
    }
}
