//! Adaptive bitrate streaming with network-aware feedback.
//!
//! Uses PID controller to dynamically adjust bitrate based on packet loss,
//! latency, and bandwidth estimates.

use crate::SessionId;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use chrono::Utc;

/// Errors that can occur during streaming operations.
#[derive(Debug, Error)]
pub enum StreamError {
    #[error("Stream not found: {session_id}")]
    NotFound { session_id: String },

    #[error("Stream not active")]
    NotActive,

    #[error("PID controller error")]
    ControllerError,

    #[error("Network error: {reason}")]
    NetworkError { reason: String },
}

/// PID controller for adaptive bitrate.
#[derive(Clone, Serialize, Deserialize)]
pub struct PidController {
    /// Proportional gain.
    pub kp: f64,
    /// Integral gain.
    pub ki: f64,
    /// Derivative gain.
    pub kd: f64,
    /// Accumulated error for integral term.
    pub integral: f64,
    /// Previous error for derivative term.
    pub prev_error: f64,
    /// Minimum output (Mbps).
    pub min_output: f64,
    /// Maximum output (Mbps).
    pub max_output: f64,
}

impl PidController {
    /// Create a new PID controller for bitrate adjustment.
    pub fn new() -> Self {
        PidController {
            kp: 0.5,  // Proportional
            ki: 0.1,  // Integral
            kd: 0.2,  // Derivative
            integral: 0.0,
            prev_error: 0.0,
            min_output: 0.5,  // Min 0.5 Mbps
            max_output: 50.0, // Max 50 Mbps
        }
    }

    /// Compute next bitrate adjustment based on error (packet loss, latency).
    pub fn update(&mut self, error: f64) -> f64 {
        self.integral += error;
        let derivative = error - self.prev_error;
        self.prev_error = error;

        let output = (self.kp * error) + (self.ki * self.integral) + (self.kd * derivative);

        output.max(self.min_output).min(self.max_output)
    }

    /// Reset the controller.
    pub fn reset(&mut self) {
        self.integral = 0.0;
        self.prev_error = 0.0;
    }
}

impl Default for PidController {
    fn default() -> Self {
        Self::new()
    }
}

/// Stream state for a session.
#[derive(Clone, Serialize, Deserialize)]
pub struct StreamState {
    /// Session ID.
    pub session_id: SessionId,

    /// Current bitrate (Mbps).
    pub bitrate_mbps: f64,

    /// Round-trip time (ms).
    pub rtt_ms: f64,

    /// Packet loss percentage (0-100).
    pub packet_loss_percent: f64,

    /// Frames per second.
    pub fps: f64,

    /// Bytes sent.
    pub bytes_sent: u64,

    /// Bytes received.
    pub bytes_received: u64,

    /// PID controller state.
    pub controller: PidController,

    /// Stream is active.
    pub active: bool,

    /// Last update timestamp.
    pub last_update: chrono::DateTime<chrono::Utc>,
}

impl StreamState {
    /// Create a new stream state.
    pub fn new(session_id: SessionId) -> Self {
        StreamState {
            session_id,
            bitrate_mbps: 5.0,
            rtt_ms: 10.0,
            packet_loss_percent: 0.0,
            fps: 60.0,
            bytes_sent: 0,
            bytes_received: 0,
            controller: PidController::new(),
            active: true,
            last_update: Utc::now(),
        }
    }

    /// Update network metrics and adjust bitrate.
    pub fn update_metrics(&mut self, rtt_ms: f64, packet_loss_percent: f64) {
        self.rtt_ms = rtt_ms;
        self.packet_loss_percent = packet_loss_percent;
        self.last_update = Utc::now();

        // Calculate error as combination of packet loss and latency
        // High packet loss or latency should reduce bitrate
        let error = (packet_loss_percent / 10.0) + (rtt_ms / 100.0);

        // Apply PID controller
        let adjustment = self.controller.update(error);
        self.bitrate_mbps = (self.bitrate_mbps + adjustment).max(0.5);

        // Cap at reasonable maximum
        if self.bitrate_mbps > 50.0 {
            self.bitrate_mbps = 50.0;
        }
    }

    /// Record bytes sent.
    pub fn record_sent(&mut self, bytes: u64) {
        self.bytes_sent += bytes;
    }

    /// Record bytes received.
    pub fn record_received(&mut self, bytes: u64) {
        self.bytes_received += bytes;
    }

    /// Update FPS.
    pub fn set_fps(&mut self, fps: f64) {
        self.fps = fps;
    }
}

/// Streaming service with adaptive bitrate control.
pub struct StreamService {
    /// Active streams (SessionId -> StreamState).
    streams: Arc<DashMap<SessionId, StreamState>>,
}

impl StreamService {
    /// Create a new StreamService.
    pub fn new() -> Self {
        StreamService {
            streams: Arc::new(DashMap::new()),
        }
    }

    /// Create a new stream.
    pub async fn create_stream(&self, session_id: SessionId) -> Result<(), StreamError> {
        let state = StreamState::new(session_id);
        self.streams.insert(session_id, state);
        tracing::debug!("Created stream for session {}", session_id);
        Ok(())
    }

    /// Get stream stats.
    pub async fn get_stats(&self, session_id: SessionId) -> Result<crate::StreamStats, StreamError> {
        let entry = self
            .streams
            .get(&session_id)
            .ok_or(StreamError::NotFound {
                session_id: session_id.to_string(),
            })?;

        let state = entry.value();
        Ok(crate::StreamStats {
            bitrate_mbps: state.bitrate_mbps,
            rtt_ms: state.rtt_ms,
            packet_loss_percent: state.packet_loss_percent,
            fps: state.fps,
            bytes_received: state.bytes_received,
            bytes_sent: state.bytes_sent,
            last_update: state.last_update,
        })
    }

    /// Update network metrics for a stream.
    pub async fn update_network_metrics(
        &self,
        session_id: SessionId,
        rtt_ms: f64,
        packet_loss_percent: f64,
    ) -> Result<(), StreamError> {
        if let Some(mut entry) = self.streams.get_mut(&session_id) {
            entry.update_metrics(rtt_ms, packet_loss_percent);
            Ok(())
        } else {
            Err(StreamError::NotFound {
                session_id: session_id.to_string(),
            })
        }
    }

    /// Record bytes transferred.
    pub async fn record_transfer(
        &self,
        session_id: SessionId,
        bytes_sent: u64,
        bytes_received: u64,
    ) -> Result<(), StreamError> {
        if let Some(mut entry) = self.streams.get_mut(&session_id) {
            entry.record_sent(bytes_sent);
            entry.record_received(bytes_received);
            Ok(())
        } else {
            Err(StreamError::NotFound {
                session_id: session_id.to_string(),
            })
        }
    }

    /// Update FPS for a stream.
    pub async fn set_fps(&self, session_id: SessionId, fps: f64) -> Result<(), StreamError> {
        if let Some(mut entry) = self.streams.get_mut(&session_id) {
            entry.set_fps(fps);
            Ok(())
        } else {
            Err(StreamError::NotFound {
                session_id: session_id.to_string(),
            })
        }
    }

    /// Close a stream.
    pub async fn close_stream(&self, session_id: SessionId) -> Result<(), StreamError> {
        if let Some((_, mut state)) = self.streams.remove(&session_id) {
            state.active = false;
            tracing::debug!("Closed stream for session {}", session_id);
            Ok(())
        } else {
            Err(StreamError::NotFound {
                session_id: session_id.to_string(),
            })
        }
    }

    /// List all active streams.
    pub async fn list_streams(&self) -> Vec<SessionId> {
        self.streams
            .iter()
            .filter(|entry| entry.value().active)
            .map(|entry| entry.key().clone())
            .collect()
    }
}

impl Default for StreamService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pid_controller() {
        let mut controller = PidController::new();

        let output1 = controller.update(0.5);
        assert!(output1 > 0.0);

        let output2 = controller.update(0.3);
        assert!(output2 > 0.0);
    }

    #[test]
    fn test_stream_state_metrics() {
        let mut state = StreamState::new(SessionId::new());
        state.update_metrics(50.0, 5.0);

        assert_eq!(state.rtt_ms, 50.0);
        assert_eq!(state.packet_loss_percent, 5.0);
    }

    #[tokio::test]
    async fn test_create_stream() {
        let service = StreamService::new();
        let session_id = SessionId::new();

        service.create_stream(session_id).await.unwrap();

        let stats = service.get_stats(session_id).await.unwrap();
        assert_eq!(stats.bitrate_mbps, 5.0);
    }

    #[tokio::test]
    async fn test_update_metrics() {
        let service = StreamService::new();
        let session_id = SessionId::new();

        service.create_stream(session_id).await.unwrap();
        service
            .update_network_metrics(session_id, 30.0, 2.0)
            .await
            .unwrap();

        let stats = service.get_stats(session_id).await.unwrap();
        assert_eq!(stats.rtt_ms, 30.0);
        assert_eq!(stats.packet_loss_percent, 2.0);
    }

    #[tokio::test]
    async fn test_record_transfer() {
        let service = StreamService::new();
        let session_id = SessionId::new();

        service.create_stream(session_id).await.unwrap();
        service
            .record_transfer(session_id, 1024, 512)
            .await
            .unwrap();

        let stats = service.get_stats(session_id).await.unwrap();
        assert_eq!(stats.bytes_sent, 1024);
        assert_eq!(stats.bytes_received, 512);
    }

    #[tokio::test]
    async fn test_close_stream() {
        let service = StreamService::new();
        let session_id = SessionId::new();

        service.create_stream(session_id).await.unwrap();
        service.close_stream(session_id).await.unwrap();

        let result = service.get_stats(session_id).await;
        assert!(result.is_err());
    }
}
