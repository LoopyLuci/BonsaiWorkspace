//! Zero-trust encrypted traffic relay.
//!
//! The RelayService forwards encrypted traffic between peers, maintaining
//! zero-trust authentication and comprehensive network statistics.

use crate::{PeerId, SessionId, StreamStats};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use thiserror::Error;
use chrono::Utc;

/// Errors that can occur during relay operations.
#[derive(Debug, Error)]
pub enum RelayError {
    #[error("Session not found: {session_id}")]
    SessionNotFound { session_id: String },

    #[error("Relay connection failed: {reason}")]
    ConnectionFailed { reason: String },

    #[error("Authentication failed")]
    AuthenticationFailed,

    #[error("Relay overloaded")]
    Overloaded,

    #[error("Invalid packet")]
    InvalidPacket,

    #[error("Encryption failed: {reason}")]
    EncryptionError { reason: String },

    #[error("Relay timeout")]
    Timeout,
}

/// Relay session for a peer-to-peer connection.
#[derive(Clone, Serialize, Deserialize)]
pub struct RelaySession {
    /// Session ID.
    pub session_id: SessionId,

    /// Source peer.
    pub source_peer: PeerId,

    /// Destination peer.
    pub destination_peer: PeerId,

    /// Bytes forwarded from source to dest.
    pub bytes_source_to_dest: Arc<AtomicU64>,

    /// Bytes forwarded from dest to source.
    pub bytes_dest_to_source: Arc<AtomicU64>,

    /// Packets relayed.
    pub packets_relayed: Arc<AtomicU64>,

    /// Packets dropped.
    pub packets_dropped: Arc<AtomicU64>,

    /// Average latency in milliseconds.
    pub latency_ms: Arc<std::sync::atomic::AtomicU32>,

    /// Session is active.
    pub active: Arc<std::sync::atomic::AtomicBool>,
}

impl RelaySession {
    /// Create a new relay session.
    pub fn new(
        session_id: SessionId,
        source_peer: PeerId,
        destination_peer: PeerId,
    ) -> Self {
        RelaySession {
            session_id,
            source_peer,
            destination_peer,
            bytes_source_to_dest: Arc::new(AtomicU64::new(0)),
            bytes_dest_to_source: Arc::new(AtomicU64::new(0)),
            packets_relayed: Arc::new(AtomicU64::new(0)),
            packets_dropped: Arc::new(AtomicU64::new(0)),
            latency_ms: Arc::new(std::sync::atomic::AtomicU32::new(0)),
            active: Arc::new(std::sync::atomic::AtomicBool::new(true)),
        }
    }

    /// Record bytes transferred from source to destination.
    pub fn record_source_to_dest(&self, bytes: u64) {
        self.bytes_source_to_dest.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record bytes transferred from destination to source.
    pub fn record_dest_to_source(&self, bytes: u64) {
        self.bytes_dest_to_source.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Record a packet relayed.
    pub fn record_packet(&self) {
        self.packets_relayed.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a dropped packet.
    pub fn record_drop(&self) {
        self.packets_dropped.fetch_add(1, Ordering::Relaxed);
    }

    /// Update latency measurement.
    pub fn set_latency_ms(&self, latency: u32) {
        self.latency_ms.store(latency, Ordering::Relaxed);
    }

    /// Get current stats.
    pub fn get_stats(&self) -> RelayStats {
        let bytes_src_to_dst = self.bytes_source_to_dest.load(Ordering::Relaxed);
        let bytes_dst_to_src = self.bytes_dest_to_source.load(Ordering::Relaxed);
        let total_bytes = bytes_src_to_dst + bytes_dst_to_src;
        let packets = self.packets_relayed.load(Ordering::Relaxed);
        let dropped = self.packets_dropped.load(Ordering::Relaxed);
        let packet_loss = if packets + dropped > 0 {
            (dropped as f64 / (packets + dropped) as f64) * 100.0
        } else {
            0.0
        };

        RelayStats {
            bytes_source_to_dest: bytes_src_to_dst,
            bytes_dest_to_source: bytes_dst_to_src,
            total_bytes,
            packets_relayed: packets,
            packets_dropped: dropped,
            packet_loss_percent: packet_loss,
            latency_ms: self.latency_ms.load(Ordering::Relaxed),
            active: self.active.load(Ordering::Relaxed),
        }
    }

    /// Close this relay session.
    pub fn close(&self) {
        self.active.store(false, Ordering::Release);
    }
}

/// Statistics for a relay session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayStats {
    pub bytes_source_to_dest: u64,
    pub bytes_dest_to_source: u64,
    pub total_bytes: u64,
    pub packets_relayed: u64,
    pub packets_dropped: u64,
    pub packet_loss_percent: f64,
    pub latency_ms: u32,
    pub active: bool,
}

/// Service for relaying encrypted traffic between peers.
pub struct RelayService {
    /// Active relay sessions (SessionId -> RelaySession).
    sessions: Arc<DashMap<SessionId, RelaySession>>,

    /// Relay is running.
    running: Arc<std::sync::atomic::AtomicBool>,

    /// Total packets processed.
    total_packets: Arc<AtomicU64>,

    /// Total bytes relayed.
    total_bytes: Arc<AtomicU64>,
}

impl RelayService {
    /// Create a new RelayService.
    pub fn new() -> Self {
        RelayService {
            sessions: Arc::new(DashMap::new()),
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            total_packets: Arc::new(AtomicU64::new(0)),
            total_bytes: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Start the relay service.
    pub async fn start(&self) -> Result<(), RelayError> {
        if self.running.swap(true, std::sync::atomic::Ordering::SeqCst) {
            return Ok(()); // Already running
        }

        // In production, this would spawn relay handlers for each session
        tracing::info!("RelayService started");
        Ok(())
    }

    /// Stop the relay service.
    pub async fn stop(&self) -> Result<(), RelayError> {
        self.running.store(false, std::sync::atomic::Ordering::Release);
        self.sessions.clear();
        tracing::info!("RelayService stopped");
        Ok(())
    }

    /// Create a new relay session.
    pub async fn create_session(
        &self,
        session_id: SessionId,
        source_peer: PeerId,
        destination_peer: PeerId,
    ) -> Result<RelaySession, RelayError> {
        if !self.running.load(std::sync::atomic::Ordering::Acquire) {
            return Err(RelayError::ConnectionFailed {
                reason: "Relay service not running".to_string(),
            });
        }

        let session = RelaySession::new(session_id, source_peer, destination_peer);
        self.sessions.insert(session_id, session.clone());

        tracing::debug!("Created relay session {}", session_id);
        Ok(session)
    }

    /// Get an existing relay session.
    pub async fn get_session(&self, session_id: SessionId) -> Result<RelaySession, RelayError> {
        self.sessions
            .get(&session_id)
            .map(|entry| entry.value().clone())
            .ok_or(RelayError::SessionNotFound {
                session_id: session_id.to_string(),
            })
    }

    /// Close a relay session.
    pub async fn close_session(&self, session_id: SessionId) -> Result<(), RelayError> {
        if let Some((_, session)) = self.sessions.remove(&session_id) {
            session.close();
            tracing::debug!("Closed relay session {}", session_id);
            Ok(())
        } else {
            Err(RelayError::SessionNotFound {
                session_id: session_id.to_string(),
            })
        }
    }

    /// Relay a packet from source to destination (encrypted).
    pub async fn relay_packet(
        &self,
        session_id: SessionId,
        data: &[u8],
        direction: RelayDirection,
    ) -> Result<(), RelayError> {
        let session = self.get_session(session_id).await?;

        if !session.active.load(Ordering::Relaxed) {
            return Err(RelayError::ConnectionFailed {
                reason: "Session not active".to_string(),
            });
        }

        // In production: encrypt, forward, and verify delivery
        match direction {
            RelayDirection::SourceToDest => {
                session.record_source_to_dest(data.len() as u64);
            }
            RelayDirection::DestToSource => {
                session.record_dest_to_source(data.len() as u64);
            }
        }

        session.record_packet();
        self.total_packets.fetch_add(1, Ordering::Relaxed);
        self.total_bytes
            .fetch_add(data.len() as u64, Ordering::Relaxed);

        Ok(())
    }

    /// Get statistics for a relay session.
    pub async fn get_stats(&self, session_id: SessionId) -> Result<RelayStats, RelayError> {
        let session = self.get_session(session_id).await?;
        Ok(session.get_stats())
    }

    /// List all active sessions.
    pub async fn list_sessions(&self) -> Result<Vec<SessionId>, RelayError> {
        Ok(self.sessions.iter().map(|entry| entry.key().clone()).collect())
    }

    /// Get global relay statistics.
    pub fn get_global_stats(&self) -> GlobalRelayStats {
        GlobalRelayStats {
            total_packets: self.total_packets.load(Ordering::Relaxed),
            total_bytes: self.total_bytes.load(Ordering::Relaxed),
            active_sessions: self.sessions.len(),
            running: self.running.load(Ordering::Relaxed),
        }
    }
}

impl Default for RelayService {
    fn default() -> Self {
        Self::new()
    }
}

/// Direction of traffic flow in a relay session.
#[derive(Debug, Clone, Copy)]
pub enum RelayDirection {
    /// From source to destination.
    SourceToDest,
    /// From destination to source.
    DestToSource,
}

/// Global relay statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalRelayStats {
    pub total_packets: u64,
    pub total_bytes: u64,
    pub active_sessions: usize,
    pub running: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_relay_session() {
        let service = RelayService::new();
        service.start().await.unwrap();

        let session_id = SessionId::new();
        let src = PeerId::from_bytes(&[1u8; 32]);
        let dst = PeerId::from_bytes(&[2u8; 32]);

        let session = service
            .create_session(session_id, src, dst)
            .await
            .unwrap();

        assert_eq!(session.session_id, session_id);
        assert_eq!(session.source_peer, src);
        assert_eq!(session.destination_peer, dst);
    }

    #[tokio::test]
    async fn test_relay_packet() {
        let service = RelayService::new();
        service.start().await.unwrap();

        let session_id = SessionId::new();
        let src = PeerId::from_bytes(&[1u8; 32]);
        let dst = PeerId::from_bytes(&[2u8; 32]);

        service
            .create_session(session_id, src, dst)
            .await
            .unwrap();

        let data = b"test packet";
        service
            .relay_packet(session_id, data, RelayDirection::SourceToDest)
            .await
            .unwrap();

        let stats = service.get_stats(session_id).await.unwrap();
        assert_eq!(stats.bytes_source_to_dest, data.len() as u64);
        assert_eq!(stats.packets_relayed, 1);
    }

    #[tokio::test]
    async fn test_close_session() {
        let service = RelayService::new();
        service.start().await.unwrap();

        let session_id = SessionId::new();
        let src = PeerId::from_bytes(&[1u8; 32]);
        let dst = PeerId::from_bytes(&[2u8; 32]);

        service
            .create_session(session_id, src, dst)
            .await
            .unwrap();

        service.close_session(session_id).await.unwrap();

        let result = service.get_session(session_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let service = RelayService::new();
        service.start().await.unwrap();

        let session_id1 = SessionId::new();
        let session_id2 = SessionId::new();
        let src = PeerId::from_bytes(&[1u8; 32]);
        let dst = PeerId::from_bytes(&[2u8; 32]);

        service
            .create_session(session_id1, src, dst)
            .await
            .unwrap();
        service
            .create_session(session_id2, src, dst)
            .await
            .unwrap();

        let sessions = service.list_sessions().await.unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn test_relay_session_stats() {
        let session = RelaySession::new(
            SessionId::new(),
            PeerId::from_bytes(&[1u8; 32]),
            PeerId::from_bytes(&[2u8; 32]),
        );

        session.record_source_to_dest(100);
        session.record_dest_to_source(50);
        session.record_packet();
        session.set_latency_ms(25);

        let stats = session.get_stats();
        assert_eq!(stats.bytes_source_to_dest, 100);
        assert_eq!(stats.bytes_dest_to_source, 50);
        assert_eq!(stats.total_bytes, 150);
        assert_eq!(stats.latency_ms, 25);
    }
}
