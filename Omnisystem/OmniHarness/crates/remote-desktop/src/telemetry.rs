//! Telemetry and event logging for remote desktop sessions.
//!
//! This module integrates with audit-log for comprehensive event logging
//! and monitoring of remote desktop operations.

use crate::{PeerId, SessionId};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use chrono::{DateTime, Utc};

/// Errors that can occur during telemetry operations.
#[derive(Debug, Error)]
pub enum TelemetryError {
    #[error("Event queue full")]
    QueueFull,

    #[error("Telemetry service not running")]
    NotRunning,
}

/// Event types for remote desktop operations.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum RemoteDesktopEvent {
    /// Peer discovered.
    PeerDiscovered { peer_id: PeerId, name: String },

    /// Peer lost (went offline).
    PeerLost { peer_id: PeerId },

    /// Session created.
    SessionCreated { session_id: SessionId, peer_id: PeerId },

    /// Session activated (became active).
    SessionActivated { session_id: SessionId },

    /// Session paused.
    SessionPaused { session_id: SessionId },

    /// Session resumed.
    SessionResumed { session_id: SessionId },

    /// Session closed.
    SessionClosed { session_id: SessionId, duration_secs: u64 },

    /// Data transferred.
    DataTransferred {
        session_id: SessionId,
        bytes_sent: u64,
        bytes_received: u64,
    },

    /// Network stats updated.
    NetworkStats {
        session_id: SessionId,
        bitrate_mbps: f64,
        rtt_ms: f64,
        packet_loss_percent: f64,
        fps: f64,
    },

    /// Security event (token verified, etc.).
    SecurityEvent { session_id: SessionId, event_type: String, details: String },
}

impl RemoteDesktopEvent {
    pub fn event_type_name(&self) -> &'static str {
        match self {
            RemoteDesktopEvent::PeerDiscovered { .. } => "PeerDiscovered",
            RemoteDesktopEvent::PeerLost { .. } => "PeerLost",
            RemoteDesktopEvent::SessionCreated { .. } => "SessionCreated",
            RemoteDesktopEvent::SessionActivated { .. } => "SessionActivated",
            RemoteDesktopEvent::SessionPaused { .. } => "SessionPaused",
            RemoteDesktopEvent::SessionResumed { .. } => "SessionResumed",
            RemoteDesktopEvent::SessionClosed { .. } => "SessionClosed",
            RemoteDesktopEvent::DataTransferred { .. } => "DataTransferred",
            RemoteDesktopEvent::NetworkStats { .. } => "NetworkStats",
            RemoteDesktopEvent::SecurityEvent { .. } => "SecurityEvent",
        }
    }
}

/// Telemetry service for remote desktop events.
pub struct RemoteDesktopTelemetry {
    /// Event queue (in production, backed by Universe).
    events: Arc<tokio::sync::RwLock<Vec<(DateTime<Utc>, RemoteDesktopEvent)>>>,

    /// Maximum events to keep in memory.
    max_events: usize,

    /// Service is running.
    running: Arc<std::sync::atomic::AtomicBool>,
}

impl RemoteDesktopTelemetry {
    /// Create a new RemoteDesktopTelemetry service.
    pub fn new() -> Self {
        RemoteDesktopTelemetry {
            events: Arc::new(tokio::sync::RwLock::new(vec![])),
            max_events: 10000,
            running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Start the telemetry service.
    pub async fn start(&self) -> Result<(), TelemetryError> {
        if self.running.swap(true, std::sync::atomic::Ordering::SeqCst) {
            return Ok(()); // Already running
        }

        tracing::info!("RemoteDesktopTelemetry started");

        // In production, this would:
        // 1. Connect to Universe emitter
        // 2. Start background event processor
        // 3. Initialize event sinks

        Ok(())
    }

    /// Log an event.
    pub async fn log_event(&self, event: RemoteDesktopEvent) -> Result<(), TelemetryError> {
        if !self.running.load(std::sync::atomic::Ordering::Acquire) {
            return Err(TelemetryError::NotRunning);
        }

        let mut events = self.events.write().await;

        if events.len() >= self.max_events {
            events.remove(0); // Remove oldest
        }

        events.push((Utc::now(), event));
        Ok(())
    }

    /// Log peer discovered.
    pub fn log_peer_discovered(&self, peer_id: PeerId, name: String) {
        let event = RemoteDesktopEvent::PeerDiscovered { peer_id, name };
        let _ = tokio::spawn({
            let self_clone = Arc::new(self.clone());
            async move {
                let _ = self_clone.log_event(event).await;
            }
        });
    }

    /// Log peer lost.
    pub fn log_peer_lost(&self, peer_id: PeerId) {
        let event = RemoteDesktopEvent::PeerLost { peer_id };
        let _ = tokio::spawn({
            let self_clone = Arc::new(self.clone());
            async move {
                let _ = self_clone.log_event(event).await;
            }
        });
    }

    /// Log session created.
    pub fn log_session_created(&self, session_id: &SessionId, peer_id: &PeerId) {
        let event = RemoteDesktopEvent::SessionCreated {
            session_id: *session_id,
            peer_id: *peer_id,
        };
        let _ = tokio::spawn({
            let self_clone = Arc::new(self.clone());
            async move {
                let _ = self_clone.log_event(event).await;
            }
        });
    }

    /// Log session activated.
    pub fn log_session_activated(&self, session_id: SessionId) {
        let event = RemoteDesktopEvent::SessionActivated { session_id };
        let _ = tokio::spawn({
            let self_clone = Arc::new(self.clone());
            async move {
                let _ = self_clone.log_event(event).await;
            }
        });
    }

    /// Log session paused.
    pub fn log_session_paused(&self, session_id: SessionId) {
        let event = RemoteDesktopEvent::SessionPaused { session_id };
        let _ = tokio::spawn({
            let self_clone = Arc::new(self.clone());
            async move {
                let _ = self_clone.log_event(event).await;
            }
        });
    }

    /// Log session resumed.
    pub fn log_session_resumed(&self, session_id: SessionId) {
        let event = RemoteDesktopEvent::SessionResumed { session_id };
        let _ = tokio::spawn({
            let self_clone = Arc::new(self.clone());
            async move {
                let _ = self_clone.log_event(event).await;
            }
        });
    }

    /// Log session closed.
    pub fn log_session_closed(&self, session_id: &SessionId, duration_secs: u64) {
        let event = RemoteDesktopEvent::SessionClosed {
            session_id: *session_id,
            duration_secs,
        };
        let _ = tokio::spawn({
            let self_clone = Arc::new(self.clone());
            async move {
                let _ = self_clone.log_event(event).await;
            }
        });
    }

    /// Log data transferred.
    pub fn log_data_transferred(&self, session_id: SessionId, bytes_sent: u64, bytes_received: u64) {
        let event = RemoteDesktopEvent::DataTransferred {
            session_id,
            bytes_sent,
            bytes_received,
        };
        let _ = tokio::spawn({
            let self_clone = Arc::new(self.clone());
            async move {
                let _ = self_clone.log_event(event).await;
            }
        });
    }

    /// Log network stats update.
    pub fn log_network_stats(
        &self,
        session_id: SessionId,
        bitrate_mbps: f64,
        rtt_ms: f64,
        packet_loss_percent: f64,
        fps: f64,
    ) {
        let event = RemoteDesktopEvent::NetworkStats {
            session_id,
            bitrate_mbps,
            rtt_ms,
            packet_loss_percent,
            fps,
        };
        let _ = tokio::spawn({
            let self_clone = Arc::new(self.clone());
            async move {
                let _ = self_clone.log_event(event).await;
            }
        });
    }

    /// Log security event.
    pub fn log_security_event(
        &self,
        session_id: SessionId,
        event_type: String,
        details: String,
    ) {
        let event = RemoteDesktopEvent::SecurityEvent {
            session_id,
            event_type,
            details,
        };
        let _ = tokio::spawn({
            let self_clone = Arc::new(self.clone());
            async move {
                let _ = self_clone.log_event(event).await;
            }
        });
    }

    /// Get recent events.
    pub async fn get_recent_events(&self, count: usize) -> Vec<(DateTime<Utc>, RemoteDesktopEvent)> {
        let events = self.events.read().await;
        let start = if events.len() > count {
            events.len() - count
        } else {
            0
        };
        events[start..].to_vec()
    }

    /// Get all events of a specific type.
    pub async fn get_events_by_type(
        &self,
        event_type: &str,
    ) -> Vec<(DateTime<Utc>, RemoteDesktopEvent)> {
        let events = self.events.read().await;
        events
            .iter()
            .filter(|(_, event)| event.event_type_name() == event_type)
            .cloned()
            .collect()
    }

    /// Clear all events.
    pub async fn clear_events(&self) {
        self.events.write().await.clear();
    }

    /// Get event count.
    pub async fn event_count(&self) -> usize {
        self.events.read().await.len()
    }
}

impl Clone for RemoteDesktopTelemetry {
    fn clone(&self) -> Self {
        RemoteDesktopTelemetry {
            events: Arc::clone(&self.events),
            max_events: self.max_events,
            running: Arc::clone(&self.running),
        }
    }
}

impl Default for RemoteDesktopTelemetry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_log_event() {
        let telemetry = RemoteDesktopTelemetry::new();
        telemetry.start().await.unwrap();

        let peer_id = PeerId::from_bytes(&[1u8; 32]);
        telemetry.log_peer_discovered(peer_id, "test-peer".to_string());

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let events = telemetry.get_recent_events(10).await;
        assert!(!events.is_empty());
    }

    #[tokio::test]
    async fn test_log_session_created() {
        let telemetry = RemoteDesktopTelemetry::new();
        telemetry.start().await.unwrap();

        let session_id = SessionId::new();
        let peer_id = PeerId::from_bytes(&[1u8; 32]);

        telemetry.log_session_created(&session_id, &peer_id);

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let events = telemetry.get_recent_events(10).await;
        assert!(!events.is_empty());
    }

    #[tokio::test]
    async fn test_get_events_by_type() {
        let telemetry = RemoteDesktopTelemetry::new();
        telemetry.start().await.unwrap();

        let peer_id = PeerId::from_bytes(&[1u8; 32]);
        telemetry.log_peer_discovered(peer_id, "peer1".to_string());
        telemetry.log_peer_discovered(peer_id, "peer2".to_string());

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let discovered = telemetry.get_events_by_type("PeerDiscovered").await;
        assert!(discovered.len() >= 2);
    }

    #[tokio::test]
    async fn test_clear_events() {
        let telemetry = RemoteDesktopTelemetry::new();
        telemetry.start().await.unwrap();

        let peer_id = PeerId::from_bytes(&[1u8; 32]);
        telemetry.log_peer_discovered(peer_id, "test-peer".to_string());

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        assert!(telemetry.event_count().await > 0);
        telemetry.clear_events().await;
        assert_eq!(telemetry.event_count().await, 0);
    }
}
