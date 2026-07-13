//! SessionManager Actor - Manages per-user sessions and state
//!
//! Responsibilities:
//! - Create and manage user sessions
//! - Track session state and metadata
//! - Create state snapshots for persistence
//! - Implement session expiration
//! - Track user activity

use crate::actor::{Actor, ActorId, Snapshot};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use dashmap::DashMap;
use omni_bot_core::SessionId;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Session state container
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    pub session_id: SessionId,
    pub user_id: String,
    pub created_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub context: std::collections::HashMap<String, serde_json::Value>,
    pub is_active: bool,
}

impl SessionState {
    pub fn new(user_id: String, ttl_seconds: i64) -> Self {
        let now = Utc::now();
        Self {
            session_id: SessionId::new(),
            user_id,
            created_at: now,
            last_activity: now,
            expires_at: now + Duration::seconds(ttl_seconds),
            context: std::collections::HashMap::new(),
            is_active: true,
        }
    }

    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    pub fn touch(&mut self) {
        self.last_activity = Utc::now();
    }

    pub fn set_context(&mut self, key: String, value: serde_json::Value) {
        self.context.insert(key, value);
    }

    pub fn get_context(&self, key: &str) -> Option<serde_json::Value> {
        self.context.get(key).cloned()
    }
}

/// Messages for SessionManager
#[derive(Debug, Clone)]
pub enum SessionManagerMessage {
    /// Create a new session
    CreateSession { user_id: String },
    /// Get existing session
    GetSession { session_id: SessionId },
    /// Update session context
    UpdateContext {
        session_id: SessionId,
        key: String,
        value: serde_json::Value,
    },
    /// Expire a session
    ExpireSession { session_id: SessionId },
    /// Clean up expired sessions
    CleanupExpired,
    /// Get all active sessions
    GetActiveSessions,
    /// Get metrics
    GetMetrics,
    /// Stop the actor
    Stop,
}

/// Session metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionMetrics {
    pub total_sessions_created: u64,
    pub active_sessions: u64,
    pub expired_sessions: u64,
    pub avg_session_duration_seconds: f64,
}

/// SessionManager actor
pub struct SessionManager {
    id: ActorId,
    sessions: Arc<DashMap<SessionId, SessionState>>,
    metrics: SessionMetrics,
    session_ttl_seconds: i64,
    #[allow(dead_code)]
    cleanup_interval_seconds: i64,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            id: ActorId::new(),
            sessions: Arc::new(DashMap::new()),
            metrics: SessionMetrics::default(),
            session_ttl_seconds: 3600, // 1 hour
            cleanup_interval_seconds: 300, // 5 minutes
        }
    }

    pub fn with_ttl(mut self, seconds: i64) -> Self {
        self.session_ttl_seconds = seconds;
        self
    }

    fn create_session(&mut self, user_id: String) -> SessionState {
        let session = SessionState::new(user_id.clone(), self.session_ttl_seconds);
        self.sessions.insert(session.session_id, session.clone());
        self.metrics.total_sessions_created += 1;
        self.metrics.active_sessions += 1;

        log::info!(
            "[SessionManager] Created session {} for user {}",
            session.session_id.0,
            user_id
        );

        session
    }

    fn get_session(&self, session_id: SessionId) -> Option<SessionState> {
        self.sessions
            .get(&session_id)
            .map(|s| s.value().clone())
    }

    fn update_session_context(
        &self,
        session_id: SessionId,
        key: String,
        value: serde_json::Value,
    ) -> Result<(), String> {
        if let Some(mut session) = self.sessions.get_mut(&session_id) {
            session.set_context(key.clone(), value);
            session.touch();
            log::info!(
                "[SessionManager] Updated context for session {}: {}",
                session_id.0,
                key
            );
            Ok(())
        } else {
            Err(format!("Session {} not found", session_id.0))
        }
    }

    fn expire_session(&mut self, session_id: SessionId) -> Result<(), String> {
        if let Some(mut session) = self.sessions.get_mut(&session_id) {
            session.is_active = false;
            self.metrics.active_sessions = self.metrics.active_sessions.saturating_sub(1);
            self.metrics.expired_sessions += 1;

            let duration = session
                .last_activity
                .signed_duration_since(session.created_at);
            self.metrics.avg_session_duration_seconds =
                (self.metrics.avg_session_duration_seconds + duration.num_seconds() as f64) / 2.0;

            log::info!("[SessionManager] Expired session {}", session_id.0);
            Ok(())
        } else {
            Err(format!("Session {} not found", session_id.0))
        }
    }

    fn cleanup_expired_sessions(&mut self) -> usize {
        let expired_ids: Vec<_> = self
            .sessions
            .iter()
            .filter(|s| s.value().is_expired())
            .map(|s| s.key().clone())
            .collect();

        for session_id in expired_ids.iter() {
            let _ = self.expire_session(*session_id);
        }

        let count = expired_ids.len();
        log::info!("[SessionManager] Cleaned up {} expired sessions", count);
        count
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Actor for SessionManager {
    type Message = SessionManagerMessage;

    fn id(&self) -> ActorId {
        self.id
    }

    async fn handle(&mut self, msg: Self::Message) -> Result<bool, String> {
        match msg {
            SessionManagerMessage::CreateSession { user_id } => {
                let _session = self.create_session(user_id);
                Ok(true)
            }
            SessionManagerMessage::GetSession { session_id } => {
                if let Some(session) = self.get_session(session_id) {
                    log::info!(
                        "[SessionManager] Retrieved session {} for user {}",
                        session_id.0,
                        session.user_id
                    );
                } else {
                    log::warn!("[SessionManager] Session {} not found", session_id.0);
                }
                Ok(true)
            }
            SessionManagerMessage::UpdateContext {
                session_id,
                key,
                value,
            } => {
                self.update_session_context(session_id, key, value)
                    .map(|_| true)
                    .map_err(|e| {
                        log::error!("[SessionManager] {}", e);
                        e
                    })
            }
            SessionManagerMessage::ExpireSession { session_id } => {
                self.expire_session(session_id)
                    .map(|_| true)
                    .map_err(|e| {
                        log::error!("[SessionManager] {}", e);
                        e
                    })
            }
            SessionManagerMessage::CleanupExpired => {
                let count = self.cleanup_expired_sessions();
                log::info!("[SessionManager] Cleanup complete: {} sessions removed", count);
                Ok(true)
            }
            SessionManagerMessage::GetActiveSessions => {
                let count = self.sessions.len();
                log::info!("[SessionManager] Active sessions: {}", count);
                self.metrics.active_sessions = count as u64;
                Ok(true)
            }
            SessionManagerMessage::GetMetrics => {
                log::info!("[SessionManager] Metrics: {:?}", self.metrics);
                Ok(true)
            }
            SessionManagerMessage::Stop => {
                log::info!("[SessionManager] Stop signal received");
                Ok(false)
            }
        }
    }

    async fn snapshot(&self) -> Result<Snapshot, String> {
        let sessions_snapshot: Vec<_> = self
            .sessions
            .iter()
            .map(|entry| entry.value().clone())
            .collect();

        let state = serde_json::json!({
            "metrics": self.metrics,
            "sessions": sessions_snapshot,
            "session_count": self.sessions.len(),
        });

        Ok(Snapshot::new(
            self.id,
            "SessionManager".to_string(),
            state,
        ))
    }

    async fn restore(&mut self, snapshot: Snapshot) -> Result<(), String> {
        if let Some(sessions_array) = snapshot.state.get("sessions").and_then(|v| v.as_array())
        {
            for session_val in sessions_array {
                if let Ok(session) =
                    serde_json::from_value::<SessionState>(session_val.clone())
                {
                    self.sessions.insert(session.session_id, session);
                }
            }
        }

        log::info!(
            "[SessionManager] Restored from snapshot: {} sessions",
            self.sessions.len()
        );
        Ok(())
    }

    fn actor_type(&self) -> &'static str {
        "SessionManager"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = SessionState::new("alice".to_string(), 3600);
        assert_eq!(session.user_id, "alice");
        assert!(!session.is_expired());
    }

    #[test]
    fn test_session_expiration() {
        let mut session = SessionState::new("alice".to_string(), 1); // 1 second TTL
        assert!(!session.is_expired());

        // Manually set expiry to past
        session.expires_at = Utc::now() - Duration::seconds(1);
        assert!(session.is_expired());
    }

    #[test]
    fn test_session_context() {
        let mut session = SessionState::new("alice".to_string(), 3600);
        session.set_context("key1".to_string(), serde_json::json!("value1"));

        let value = session.get_context("key1");
        assert_eq!(value, Some(serde_json::json!("value1")));
    }

    #[test]
    fn test_session_touch() {
        let mut session = SessionState::new("alice".to_string(), 3600);
        let initial_activity = session.last_activity;

        std::thread::sleep(std::time::Duration::from_millis(10));
        session.touch();

        assert!(session.last_activity > initial_activity);
    }
}
