use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Session state for a mobile-to-desktop remote connection
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SessionState {
    pub session_id: String,
    pub peer_id: String,
    pub started_at: DateTime<Utc>,
    pub status: SessionStatus,
    pub connection_type: ConnectionType,
    pub encryption_enabled: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum SessionStatus {
    #[serde(rename = "connecting")]
    Connecting,
    #[serde(rename = "connected")]
    Connected,
    #[serde(rename = "streaming")]
    Streaming,
    #[serde(rename = "paused")]
    Paused,
    #[serde(rename = "disconnected")]
    Disconnected,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ConnectionType {
    #[serde(rename = "local")]
    Local,      // LAN/WiFi
    #[serde(rename = "remote")]
    Remote,     // Internet via BRDF tunnel
    #[serde(rename = "p2p")]
    P2P,        // Direct P2P (NAT traversal)
}

/// Real-time session statistics
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SessionStats {
    pub fps: f32,
    pub bitrate_mbps: f32,
    pub latency_ms: f32,
    pub bandwidth_usage_mb: f64,
    pub frames_decoded: u64,
    pub frames_dropped: u64,
    pub connection_uptime_secs: u64,
    pub battery_drain_percent_per_hour: f32,
}

/// Information about an available peer
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeerInfo {
    pub peer_id: String,
    pub device_name: String,
    pub device_model: String,
    pub last_seen: DateTime<Utc>,
    pub status: PeerStatus,
    pub local_ip: Option<String>,
    pub is_trusted: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum PeerStatus {
    #[serde(rename = "online")]
    Online,
    #[serde(rename = "offline")]
    Offline,
    #[serde(rename = "pairing")]
    Pairing,
}

/// In-memory session registry
pub struct SessionRegistry {
    sessions: Arc<RwLock<HashMap<String, RemoteSession>>>,
    peers: Arc<RwLock<HashMap<String, PeerInfo>>>,
}

struct RemoteSession {
    state: SessionState,
    stats: SessionStats,
    created_at: DateTime<Utc>,
}

impl SessionRegistry {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            peers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new remote session
    pub async fn create_session(
        &self,
        peer_id: String,
        encryption_enabled: bool,
    ) -> SessionState {
        let session_id = Uuid::new_v4().to_string();
        let state = SessionState {
            session_id: session_id.clone(),
            peer_id,
            started_at: Utc::now(),
            status: SessionStatus::Connecting,
            connection_type: ConnectionType::Local,
            encryption_enabled,
        };

        let session = RemoteSession {
            state: state.clone(),
            stats: SessionStats::default(),
            created_at: Utc::now(),
        };

        let mut sessions = self.sessions.write().await;
        sessions.insert(session_id, session);

        state
    }

    /// Update session status
    pub async fn update_status(&self, session_id: &str, status: SessionStatus) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.state.status = status;
        }
    }

    /// Update session statistics
    pub async fn update_stats(&self, session_id: &str, stats: SessionStats) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            session.stats = stats;
        }
    }

    /// Get current session stats
    pub async fn get_stats(&self, session_id: &str) -> Option<SessionStats> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).map(|s| s.stats.clone())
    }

    /// Get session state
    pub async fn get_session(&self, session_id: &str) -> Option<SessionState> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).map(|s| s.state.clone())
    }

    /// Close a session
    pub async fn close_session(&self, session_id: &str) -> bool {
        let mut sessions = self.sessions.write().await;
        sessions.remove(session_id).is_some()
    }

    /// Register a peer
    pub async fn register_peer(&self, peer: PeerInfo) {
        let mut peers = self.peers.write().await;
        peers.insert(peer.peer_id.clone(), peer);
    }

    /// Get all available peers
    pub async fn list_peers(&self, filter_status: Option<PeerStatus>) -> Vec<PeerInfo> {
        let peers = self.peers.read().await;
        peers
            .values()
            .filter(|p| {
                filter_status
                    .as_ref()
                    .map(|status| &p.status == status)
                    .unwrap_or(true)
            })
            .cloned()
            .collect()
    }

    /// Get peer by ID
    pub async fn get_peer(&self, peer_id: &str) -> Option<PeerInfo> {
        let peers = self.peers.read().await;
        peers.get(peer_id).cloned()
    }

    /// List all active sessions
    pub async fn list_sessions(&self) -> Vec<SessionState> {
        let sessions = self.sessions.read().await;
        sessions.values().map(|s| s.state.clone()).collect()
    }

    /// Check if a session is active
    pub async fn is_session_active(&self, session_id: &str) -> bool {
        let sessions = self.sessions.read().await;
        sessions
            .get(session_id)
            .map(|s| s.state.status != SessionStatus::Disconnected)
            .unwrap_or(false)
    }
}

impl Default for SessionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_session() {
        let registry = SessionRegistry::new();
        let state = registry.create_session("peer-123".to_string(), true).await;

        assert_eq!(state.peer_id, "peer-123");
        assert_eq!(state.status, SessionStatus::Connecting);
        assert!(state.encryption_enabled);
        assert!(!state.session_id.is_empty());
    }

    #[tokio::test]
    async fn test_update_session_status() {
        let registry = SessionRegistry::new();
        let state = registry.create_session("peer-123".to_string(), true).await;
        let session_id = state.session_id.clone();

        registry
            .update_status(&session_id, SessionStatus::Connected)
            .await;

        let updated = registry.get_session(&session_id).await.unwrap();
        assert_eq!(updated.status, SessionStatus::Connected);
    }

    #[tokio::test]
    async fn test_close_session() {
        let registry = SessionRegistry::new();
        let state = registry.create_session("peer-123".to_string(), true).await;
        let session_id = state.session_id.clone();

        assert!(registry.close_session(&session_id).await);
        assert!(registry.get_session(&session_id).await.is_none());
    }

    #[tokio::test]
    async fn test_register_and_list_peers() {
        let registry = SessionRegistry::new();

        let peer = PeerInfo {
            peer_id: "peer-123".to_string(),
            device_name: "Desktop".to_string(),
            device_model: "Windows 10".to_string(),
            last_seen: Utc::now(),
            status: PeerStatus::Online,
            local_ip: Some("192.168.1.100".to_string()),
            is_trusted: true,
        };

        registry.register_peer(peer).await;

        let peers = registry.list_peers(None).await;
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].peer_id, "peer-123");
    }

    #[tokio::test]
    async fn test_filter_peers_by_status() {
        let registry = SessionRegistry::new();

        registry
            .register_peer(PeerInfo {
                peer_id: "peer-1".to_string(),
                device_name: "Desktop 1".to_string(),
                device_model: "Windows 10".to_string(),
                last_seen: Utc::now(),
                status: PeerStatus::Online,
                local_ip: None,
                is_trusted: true,
            })
            .await;

        registry
            .register_peer(PeerInfo {
                peer_id: "peer-2".to_string(),
                device_name: "Desktop 2".to_string(),
                device_model: "Windows 10".to_string(),
                last_seen: Utc::now(),
                status: PeerStatus::Offline,
                local_ip: None,
                is_trusted: true,
            })
            .await;

        let online_peers = registry.list_peers(Some(PeerStatus::Online)).await;
        assert_eq!(online_peers.len(), 1);
        assert_eq!(online_peers[0].peer_id, "peer-1");
    }
}
