//! Session management and lifecycle.
//!
//! The SessionManager handles creation, tracking, and termination of remote
//! desktop sessions, including permission management and state tracking.

use crate::{PeerId, SessionId, RemoteDesktopToken};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use chrono::{DateTime, Utc};

/// Errors that can occur during session operations.
#[derive(Debug, Error)]
pub enum SessionError {
    #[error("Session not found: {session_id}")]
    NotFound { session_id: String },

    #[error("Session already exists: {session_id}")]
    AlreadyExists { session_id: String },

    #[error("Invalid peer ID")]
    InvalidPeer,

    #[error("Invalid token: {0}")]
    InvalidToken(String),

    #[error("Permission denied: {reason}")]
    PermissionDenied { reason: String },

    #[error("Session closed")]
    SessionClosed,

    #[error("Session limit exceeded")]
    LimitExceeded,
}

/// State of a remote desktop session.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum SessionStateStatus {
    /// Session is connecting.
    Connecting,
    /// Session is active.
    Active,
    /// Session is paused.
    Paused,
    /// Session is closing.
    Closing,
    /// Session is closed.
    Closed,
}

/// Information about a remote desktop session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionState {
    /// Session ID.
    pub session_id: SessionId,

    /// Remote peer ID.
    pub remote_peer: PeerId,

    /// Current status.
    pub status: SessionStateStatus,

    /// Time when session was created.
    pub created_at: DateTime<Utc>,

    /// Time when session started (became active).
    pub started_at: Option<DateTime<Utc>>,

    /// Time when session ended.
    pub ended_at: Option<DateTime<Utc>>,

    /// Granted capabilities for this session.
    pub capabilities: Vec<String>,

    /// Is this session read-only?
    pub read_only: bool,

    /// Allowed remote addresses (CIDR notation).
    pub allowed_addresses: Vec<String>,

    /// Session duration limit (seconds), None for unlimited.
    pub duration_limit_secs: Option<u64>,

    /// Capability token (if authenticated).
    pub token: Option<RemoteDesktopToken>,
}

impl SessionState {
    /// Create a new session state.
    pub fn new(session_id: SessionId, remote_peer: PeerId) -> Self {
        SessionState {
            session_id,
            remote_peer,
            status: SessionStateStatus::Connecting,
            created_at: Utc::now(),
            started_at: None,
            ended_at: None,
            capabilities: vec![],
            read_only: false,
            allowed_addresses: vec!["0.0.0.0/0".to_string()], // Allow all by default
            duration_limit_secs: None,
            token: None,
        }
    }

    /// Mark session as active.
    pub fn activate(&mut self) {
        if self.status == SessionStateStatus::Connecting {
            self.status = SessionStateStatus::Active;
            self.started_at = Some(Utc::now());
        }
    }

    /// Pause the session.
    pub fn pause(&mut self) {
        if self.status == SessionStateStatus::Active {
            self.status = SessionStateStatus::Paused;
        }
    }

    /// Resume the session.
    pub fn resume(&mut self) {
        if self.status == SessionStateStatus::Paused {
            self.status = SessionStateStatus::Active;
        }
    }

    /// Close the session.
    pub fn close(&mut self) {
        if self.status != SessionStateStatus::Closed {
            self.status = SessionStateStatus::Closed;
            self.ended_at = Some(Utc::now());
        }
    }

    /// Check if session is active.
    pub fn is_active(&self) -> bool {
        self.status == SessionStateStatus::Active
    }

    /// Check if session is closed.
    pub fn is_closed(&self) -> bool {
        self.status == SessionStateStatus::Closed
    }

    /// Check if a capability is granted.
    pub fn has_capability(&self, capability: &str) -> bool {
        self.capabilities.contains(&capability.to_string())
    }

    /// Add a capability to this session.
    pub fn grant_capability(&mut self, capability: String) {
        if !self.capabilities.contains(&capability) {
            self.capabilities.push(capability);
        }
    }

    /// Get session duration (if session has ended).
    pub fn duration_secs(&self) -> Option<u64> {
        match (self.started_at, self.ended_at) {
            (Some(start), Some(end)) => Some((end - start).num_seconds() as u64),
            (Some(start), None) => {
                // Calculate duration up to now
                Some((Utc::now() - start).num_seconds() as u64)
            }
            _ => None,
        }
    }
}

/// Session manager for creating and tracking sessions.
pub struct SessionManager {
    /// Active sessions (SessionId -> SessionState).
    sessions: Arc<DashMap<SessionId, SessionState>>,

    /// Maximum concurrent sessions.
    max_sessions: usize,
}

impl SessionManager {
    /// Create a new SessionManager.
    pub fn new() -> Self {
        SessionManager {
            sessions: Arc::new(DashMap::new()),
            max_sessions: 100, // Default limit
        }
    }

    /// Create a new session.
    pub async fn create_session(
        &self,
        session_id: SessionId,
        remote_peer: PeerId,
        token: Option<RemoteDesktopToken>,
    ) -> Result<SessionState, SessionError> {
        // Check session limit
        if self.sessions.len() >= self.max_sessions {
            return Err(SessionError::LimitExceeded);
        }

        // Check if session already exists
        if self.sessions.contains_key(&session_id) {
            return Err(SessionError::AlreadyExists {
                session_id: session_id.to_string(),
            });
        }

        let mut state = SessionState::new(session_id, remote_peer);

        // Apply token capabilities if provided
        if let Some(t) = &token {
            if let Err(e) = t.verify() {
                return Err(SessionError::InvalidToken(e.to_string()));
            }
            state.token = Some(t.clone());
        }

        self.sessions.insert(session_id, state.clone());
        Ok(state)
    }

    /// Get a session by ID.
    pub async fn get_session(&self, session_id: SessionId) -> Result<SessionState, SessionError> {
        self.sessions
            .get(&session_id)
            .map(|entry| entry.value().clone())
            .ok_or(SessionError::NotFound {
                session_id: session_id.to_string(),
            })
    }

    /// Update a session's state.
    pub async fn update_session<F>(
        &self,
        session_id: SessionId,
        updater: F,
    ) -> Result<SessionState, SessionError>
    where
        F: FnOnce(&mut SessionState),
    {
        if let Some(mut entry) = self.sessions.get_mut(&session_id) {
            updater(entry.value_mut());
            Ok(entry.value().clone())
        } else {
            Err(SessionError::NotFound {
                session_id: session_id.to_string(),
            })
        }
    }

    /// End a session.
    pub async fn end_session(&self, session_id: SessionId) -> Result<(), SessionError> {
        if let Some(mut entry) = self.sessions.get_mut(&session_id) {
            entry.close();
            Ok(())
        } else {
            Err(SessionError::NotFound {
                session_id: session_id.to_string(),
            })
        }
    }

    /// List all active sessions.
    pub async fn list_sessions(&self) -> Result<Vec<SessionId>, SessionError> {
        Ok(self
            .sessions
            .iter()
            .filter(|entry| !entry.value().is_closed())
            .map(|entry| entry.key().clone())
            .collect())
    }

    /// List all sessions for a specific peer.
    pub async fn list_peer_sessions(
        &self,
        peer_id: PeerId,
    ) -> Result<Vec<SessionId>, SessionError> {
        Ok(self
            .sessions
            .iter()
            .filter(|entry| {
                entry.value().remote_peer == peer_id && !entry.value().is_closed()
            })
            .map(|entry| entry.key().clone())
            .collect())
    }

    /// Get the number of active sessions.
    pub fn active_session_count(&self) -> usize {
        self.sessions
            .iter()
            .filter(|entry| !entry.value().is_closed())
            .count()
    }

    /// Set maximum concurrent sessions.
    pub fn set_max_sessions(&mut self, max: usize) {
        self.max_sessions = max;
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
    async fn test_create_session() {
        let manager = SessionManager::new();
        let session_id = SessionId::new();
        let peer_id = PeerId::from_bytes(&[1u8; 32]);

        let state = manager
            .create_session(session_id, peer_id, None)
            .await
            .unwrap();

        assert_eq!(state.session_id, session_id);
        assert_eq!(state.remote_peer, peer_id);
        assert_eq!(state.status, SessionStateStatus::Connecting);
    }

    #[tokio::test]
    async fn test_get_session() {
        let manager = SessionManager::new();
        let session_id = SessionId::new();
        let peer_id = PeerId::from_bytes(&[1u8; 32]);

        manager
            .create_session(session_id, peer_id, None)
            .await
            .unwrap();

        let state = manager.get_session(session_id).await.unwrap();
        assert_eq!(state.session_id, session_id);
    }

    #[tokio::test]
    async fn test_session_not_found() {
        let manager = SessionManager::new();
        let unknown_id = SessionId::new();

        let result = manager.get_session(unknown_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_end_session() {
        let manager = SessionManager::new();
        let session_id = SessionId::new();
        let peer_id = PeerId::from_bytes(&[1u8; 32]);

        manager
            .create_session(session_id, peer_id, None)
            .await
            .unwrap();

        manager.end_session(session_id).await.unwrap();

        let state = manager.get_session(session_id).await.unwrap();
        assert!(state.is_closed());
    }

    #[tokio::test]
    async fn test_session_activation() {
        let manager = SessionManager::new();
        let session_id = SessionId::new();
        let peer_id = PeerId::from_bytes(&[1u8; 32]);

        manager
            .create_session(session_id, peer_id, None)
            .await
            .unwrap();

        manager
            .update_session(session_id, |s| s.activate())
            .await
            .unwrap();

        let state = manager.get_session(session_id).await.unwrap();
        assert_eq!(state.status, SessionStateStatus::Active);
        assert!(state.started_at.is_some());
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let manager = SessionManager::new();
        let session_id1 = SessionId::new();
        let session_id2 = SessionId::new();
        let peer_id = PeerId::from_bytes(&[1u8; 32]);

        manager
            .create_session(session_id1, peer_id, None)
            .await
            .unwrap();
        manager
            .create_session(session_id2, peer_id, None)
            .await
            .unwrap();

        let sessions = manager.list_sessions().await.unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[tokio::test]
    async fn test_session_limit() {
        let mut manager = SessionManager::new();
        manager.set_max_sessions(1);

        let session_id1 = SessionId::new();
        let session_id2 = SessionId::new();
        let peer_id = PeerId::from_bytes(&[1u8; 32]);

        manager
            .create_session(session_id1, peer_id, None)
            .await
            .unwrap();

        let result = manager
            .create_session(session_id2, peer_id, None)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_grant_capability() {
        let manager = SessionManager::new();
        let session_id = SessionId::new();
        let peer_id = PeerId::from_bytes(&[1u8; 32]);

        manager
            .create_session(session_id, peer_id, None)
            .await
            .unwrap();

        manager
            .update_session(session_id, |s| {
                s.grant_capability("capture".to_string())
            })
            .await
            .unwrap();

        let state = manager.get_session(session_id).await.unwrap();
        assert!(state.has_capability("capture"));
    }

    #[test]
    fn test_session_duration() {
        let mut state = SessionState::new(SessionId::new(), PeerId::from_bytes(&[1u8; 32]));
        state.activate();

        let duration = state.duration_secs();
        assert!(duration.is_some());
        assert!(duration.unwrap() >= 0);
    }
}
