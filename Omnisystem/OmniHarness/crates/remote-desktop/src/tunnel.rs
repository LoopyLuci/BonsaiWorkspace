//! TCP port forwarding tunnels.
//!
//! Establishes encrypted TCP tunnels to remote ports, enabling access to
//! services on the remote system.

use crate::SessionId;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use thiserror::Error;

/// Errors that can occur during tunnel operations.
#[derive(Debug, Error)]
pub enum TunnelError {
    #[error("Tunnel not found: {tunnel_id}")]
    NotFound { tunnel_id: String },

    #[error("Tunnel already exists: {local_addr}")]
    AlreadyExists { local_addr: String },

    #[error("Connection failed: {reason}")]
    ConnectionFailed { reason: String },

    #[error("Port binding failed: {reason}")]
    BindFailed { reason: String },

    #[error("Invalid address")]
    InvalidAddress,

    #[error("Permission denied")]
    PermissionDenied,
}

/// TCP tunnel configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelConfig {
    /// Local listening address.
    pub local_addr: SocketAddr,

    /// Remote address to forward to.
    pub remote_addr: SocketAddr,

    /// Optional description.
    pub description: Option<String>,

    /// Tunnel is bidirectional.
    pub bidirectional: bool,
}

/// TCP tunnel state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelState {
    /// Unique tunnel ID.
    pub tunnel_id: String,

    /// Session ID this tunnel belongs to.
    pub session_id: SessionId,

    /// Configuration.
    pub config: TunnelConfig,

    /// Bytes forwarded from local to remote.
    pub bytes_local_to_remote: u64,

    /// Bytes forwarded from remote to local.
    pub bytes_remote_to_local: u64,

    /// Number of connections established.
    pub connections: u64,

    /// Tunnel is active.
    pub active: bool,
}

impl TunnelState {
    /// Create a new tunnel state.
    pub fn new(session_id: SessionId, config: TunnelConfig) -> Self {
        let tunnel_id = format!(
            "tunnel-{}-{}",
            uuid::Uuid::new_v4(),
            chrono::Utc::now().timestamp()
        );

        TunnelState {
            tunnel_id,
            session_id,
            config,
            bytes_local_to_remote: 0,
            bytes_remote_to_local: 0,
            connections: 0,
            active: true,
        }
    }

    /// Record bytes forwarded from local to remote.
    pub fn record_local_to_remote(&mut self, bytes: u64) {
        self.bytes_local_to_remote += bytes;
    }

    /// Record bytes forwarded from remote to local.
    pub fn record_remote_to_local(&mut self, bytes: u64) {
        self.bytes_remote_to_local += bytes;
    }

    /// Record a new connection.
    pub fn record_connection(&mut self) {
        self.connections += 1;
    }

    /// Get total bytes transferred.
    pub fn total_bytes(&self) -> u64 {
        self.bytes_local_to_remote + self.bytes_remote_to_local
    }

    /// Close this tunnel.
    pub fn close(&mut self) {
        self.active = false;
    }
}

/// Tunnel service for TCP port forwarding.
pub struct TunnelService {
    /// Active tunnels (tunnel_id -> TunnelState).
    tunnels: Arc<DashMap<String, TunnelState>>,
}

impl TunnelService {
    /// Create a new TunnelService.
    pub fn new() -> Self {
        TunnelService {
            tunnels: Arc::new(DashMap::new()),
        }
    }

    /// Create a new tunnel.
    pub async fn create_tunnel(
        &self,
        session_id: SessionId,
        config: TunnelConfig,
    ) -> Result<TunnelState, TunnelError> {
        // Validate addresses
        if config.local_addr.ip() == IpAddr::from([0, 0, 0, 0]) {
            // Allow any interface
        } else if !config.local_addr.ip().is_loopback()
            && !config.local_addr.ip().is_private()
        {
            return Err(TunnelError::InvalidAddress);
        }

        let state = TunnelState::new(session_id, config.clone());
        let tunnel_id = state.tunnel_id.clone();

        // In production: bind to local port and start listener
        self.tunnels.insert(tunnel_id.clone(), state.clone());

        tracing::info!(
            "Created tunnel {} from {} to {}",
            tunnel_id,
            config.local_addr,
            config.remote_addr
        );

        Ok(state)
    }

    /// Get a tunnel by ID.
    pub async fn get_tunnel(&self, tunnel_id: &str) -> Result<TunnelState, TunnelError> {
        self.tunnels
            .get(tunnel_id)
            .map(|entry| entry.value().clone())
            .ok_or(TunnelError::NotFound {
                tunnel_id: tunnel_id.to_string(),
            })
    }

    /// Close a tunnel.
    pub async fn close_tunnel(&self, tunnel_id: &str) -> Result<(), TunnelError> {
        if let Some(mut entry) = self.tunnels.get_mut(tunnel_id) {
            entry.close();
            tracing::info!("Closed tunnel {}", tunnel_id);
            Ok(())
        } else {
            Err(TunnelError::NotFound {
                tunnel_id: tunnel_id.to_string(),
            })
        }
    }

    /// Remove a tunnel.
    pub async fn remove_tunnel(&self, tunnel_id: &str) -> Result<(), TunnelError> {
        if self.tunnels.remove(tunnel_id).is_some() {
            tracing::info!("Removed tunnel {}", tunnel_id);
            Ok(())
        } else {
            Err(TunnelError::NotFound {
                tunnel_id: tunnel_id.to_string(),
            })
        }
    }

    /// Record bytes transferred through a tunnel.
    pub async fn record_transfer(
        &self,
        tunnel_id: &str,
        local_to_remote: u64,
        remote_to_local: u64,
    ) -> Result<(), TunnelError> {
        if let Some(mut entry) = self.tunnels.get_mut(tunnel_id) {
            entry.record_local_to_remote(local_to_remote);
            entry.record_remote_to_local(remote_to_local);
            Ok(())
        } else {
            Err(TunnelError::NotFound {
                tunnel_id: tunnel_id.to_string(),
            })
        }
    }

    /// Record a new connection on a tunnel.
    pub async fn record_connection(&self, tunnel_id: &str) -> Result<(), TunnelError> {
        if let Some(mut entry) = self.tunnels.get_mut(tunnel_id) {
            entry.record_connection();
            Ok(())
        } else {
            Err(TunnelError::NotFound {
                tunnel_id: tunnel_id.to_string(),
            })
        }
    }

    /// List all active tunnels for a session.
    pub async fn list_session_tunnels(&self, session_id: SessionId) -> Vec<TunnelState> {
        self.tunnels
            .iter()
            .filter(|entry| entry.value().session_id == session_id && entry.value().active)
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// List all tunnels.
    pub async fn list_tunnels(&self) -> Vec<TunnelState> {
        self.tunnels
            .iter()
            .filter(|entry| entry.value().active)
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Get tunnel count.
    pub fn tunnel_count(&self) -> usize {
        self.tunnels
            .iter()
            .filter(|entry| entry.value().active)
            .count()
    }
}

impl Default for TunnelService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_tunnel() {
        let service = TunnelService::new();
        let session_id = SessionId::new();

        let config = TunnelConfig {
            local_addr: "127.0.0.1:3389".parse().unwrap(),
            remote_addr: "192.168.1.1:3389".parse().unwrap(),
            description: Some("RDP tunnel".to_string()),
            bidirectional: true,
        };

        let tunnel = service.create_tunnel(session_id, config).await.unwrap();
        assert!(tunnel.active);
    }

    #[tokio::test]
    async fn test_get_tunnel() {
        let service = TunnelService::new();
        let session_id = SessionId::new();

        let config = TunnelConfig {
            local_addr: "127.0.0.1:3389".parse().unwrap(),
            remote_addr: "192.168.1.1:3389".parse().unwrap(),
            description: None,
            bidirectional: true,
        };

        let tunnel = service.create_tunnel(session_id, config).await.unwrap();
        let tunnel_id = tunnel.tunnel_id.clone();

        let retrieved = service.get_tunnel(&tunnel_id).await.unwrap();
        assert_eq!(retrieved.tunnel_id, tunnel_id);
    }

    #[tokio::test]
    async fn test_close_tunnel() {
        let service = TunnelService::new();
        let session_id = SessionId::new();

        let config = TunnelConfig {
            local_addr: "127.0.0.1:3389".parse().unwrap(),
            remote_addr: "192.168.1.1:3389".parse().unwrap(),
            description: None,
            bidirectional: true,
        };

        let tunnel = service.create_tunnel(session_id, config).await.unwrap();
        let tunnel_id = tunnel.tunnel_id.clone();

        service.close_tunnel(&tunnel_id).await.unwrap();

        let retrieved = service.get_tunnel(&tunnel_id).await.unwrap();
        assert!(!retrieved.active);
    }

    #[tokio::test]
    async fn test_record_transfer() {
        let service = TunnelService::new();
        let session_id = SessionId::new();

        let config = TunnelConfig {
            local_addr: "127.0.0.1:3389".parse().unwrap(),
            remote_addr: "192.168.1.1:3389".parse().unwrap(),
            description: None,
            bidirectional: true,
        };

        let tunnel = service.create_tunnel(session_id, config).await.unwrap();
        let tunnel_id = tunnel.tunnel_id.clone();

        service.record_transfer(&tunnel_id, 1024, 512).await.unwrap();

        let retrieved = service.get_tunnel(&tunnel_id).await.unwrap();
        assert_eq!(retrieved.bytes_local_to_remote, 1024);
        assert_eq!(retrieved.bytes_remote_to_local, 512);
    }

    #[tokio::test]
    async fn test_list_session_tunnels() {
        let service = TunnelService::new();
        let session_id = SessionId::new();

        let config1 = TunnelConfig {
            local_addr: "127.0.0.1:3389".parse().unwrap(),
            remote_addr: "192.168.1.1:3389".parse().unwrap(),
            description: None,
            bidirectional: true,
        };

        let config2 = TunnelConfig {
            local_addr: "127.0.0.1:22".parse().unwrap(),
            remote_addr: "192.168.1.1:22".parse().unwrap(),
            description: None,
            bidirectional: true,
        };

        service.create_tunnel(session_id, config1).await.unwrap();
        service.create_tunnel(session_id, config2).await.unwrap();

        let tunnels = service.list_session_tunnels(session_id).await;
        assert_eq!(tunnels.len(), 2);
    }

    #[test]
    fn test_tunnel_state_stats() {
        let session_id = SessionId::new();
        let config = TunnelConfig {
            local_addr: "127.0.0.1:3389".parse().unwrap(),
            remote_addr: "192.168.1.1:3389".parse().unwrap(),
            description: None,
            bidirectional: true,
        };

        let mut tunnel = TunnelState::new(session_id, config);
        tunnel.record_local_to_remote(1024);
        tunnel.record_remote_to_local(512);
        tunnel.record_connection();

        assert_eq!(tunnel.bytes_local_to_remote, 1024);
        assert_eq!(tunnel.bytes_remote_to_local, 512);
        assert_eq!(tunnel.total_bytes(), 1536);
        assert_eq!(tunnel.connections, 1);
    }
}
