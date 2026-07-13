//! VirtioFaultChannel protocol for live fault injection.
//!
//! Communication protocol between test harness and vault for dynamic fault injection,
//! confirmation of application, and recovery acknowledgment.

use crate::error::{ChaosError, Result};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Fault channel message types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FaultChannelMessage {
    /// Inject a fault (harness → vault).
    InjectFault {
        fault_id: Uuid,
        fault_type: String,
        inject_time: u64,
        duration_secs: u64,
        parameters: serde_json::Value,
    },

    /// Acknowledge fault injection (vault → harness).
    FaultInjectionAck {
        fault_id: Uuid,
        applied_at: u64,
        success: bool,
        message: String,
    },

    /// Confirm recovery (vault → harness).
    RecoveryConfirm {
        fault_id: Uuid,
        recovered_at: u64,
        failure_count: u64,
    },

    /// Heartbeat (bidirectional).
    Heartbeat { timestamp: u64 },

    /// Request status (harness → vault).
    StatusRequest,

    /// Status response (vault → harness).
    StatusResponse {
        active_faults: Vec<Uuid>,
        total_injected: u64,
        total_recovered: u64,
        uptime_secs: u64,
    },

    /// Error message.
    Error {
        fault_id: Option<Uuid>,
        error_code: u32,
        message: String,
    },

    /// Request cancellation of a fault.
    CancelFault { fault_id: Uuid },

    /// Acknowledge cancellation.
    CancellationAck { fault_id: Uuid, cancelled: bool },
}

/// Fault injection statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaultStats {
    /// Total faults injected.
    pub total_injected: u64,
    /// Total faults successfully recovered.
    pub total_recovered: u64,
    /// Currently active faults.
    pub active_faults: u64,
    /// Total failures caused.
    pub total_failures: u64,
    /// Successful recoveries (no data loss).
    pub successful_recoveries: u64,
    /// Failed recoveries (data loss).
    pub failed_recoveries: u64,
    /// Average time to detect fault (millis).
    pub avg_detection_time_ms: u64,
    /// Average time to recover (millis).
    pub avg_recovery_time_ms: u64,
}

impl Default for FaultStats {
    fn default() -> Self {
        Self {
            total_injected: 0,
            total_recovered: 0,
            active_faults: 0,
            total_failures: 0,
            successful_recoveries: 0,
            failed_recoveries: 0,
            avg_detection_time_ms: 0,
            avg_recovery_time_ms: 0,
        }
    }
}

/// VirtioFaultChannel for live fault injection.
pub struct VirtioFaultChannel {
    id: Uuid,
    tx: mpsc::UnboundedSender<FaultChannelMessage>,
    rx: tokio::sync::Mutex<mpsc::UnboundedReceiver<FaultChannelMessage>>,
    stats: Arc<FaultStats>,
    is_healthy: Arc<AtomicBool>,
    last_heartbeat: Arc<AtomicU64>,
}

impl VirtioFaultChannel {
    /// Create a new fault channel.
    pub fn new() -> (Self, VirtioChannelSink) {
        let (tx, rx) = mpsc::unbounded_channel();
        let id = Uuid::new_v4();

        let channel = Self {
            id,
            tx: tx.clone(),
            rx: tokio::sync::Mutex::new(rx),
            stats: Arc::new(FaultStats::default()),
            is_healthy: Arc::new(AtomicBool::new(true)),
            last_heartbeat: Arc::new(AtomicU64::new(0)),
        };

        let sink = VirtioChannelSink {
            id,
            tx,
            stats: channel.stats.clone(),
            is_healthy: channel.is_healthy.clone(),
            last_heartbeat: channel.last_heartbeat.clone(),
        };

        (channel, sink)
    }

    /// Receive next message from channel.
    pub async fn recv(&mut self) -> Option<FaultChannelMessage> {
        self.rx.lock().await.recv().await
    }

    /// Check if channel is healthy.
    pub fn is_healthy(&self) -> bool {
        self.is_healthy.load(Ordering::Relaxed)
    }

    /// Mark channel as unhealthy.
    pub fn mark_unhealthy(&self) {
        self.is_healthy.store(false, Ordering::Relaxed);
        warn!("Fault channel marked unhealthy");
    }

    /// Get channel ID.
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Get fault statistics.
    pub fn stats(&self) -> &FaultStats {
        &self.stats
    }

    /// Get time since last heartbeat (seconds).
    pub fn time_since_heartbeat(&self, now: u64) -> u64 {
        let last = self.last_heartbeat.load(Ordering::Relaxed);
        now.saturating_sub(last)
    }
}

impl Default for VirtioFaultChannel {
    fn default() -> Self {
        Self::new().0
    }
}

/// Sink for sending messages to the fault channel.
pub struct VirtioChannelSink {
    id: Uuid,
    tx: mpsc::UnboundedSender<FaultChannelMessage>,
    stats: Arc<FaultStats>,
    is_healthy: Arc<AtomicBool>,
    last_heartbeat: Arc<AtomicU64>,
}

impl VirtioChannelSink {
    /// Send a message through the channel.
    pub fn send(&self, msg: FaultChannelMessage) -> Result<()> {
        if !self.is_healthy.load(Ordering::Relaxed) {
            return Err(ChaosError::ChannelError("Channel is unhealthy".to_string()));
        }

        self.tx.send(msg).map_err(|e| {
            ChaosError::ChannelError(format!("Failed to send message: {}", e))
        })?;

        Ok(())
    }

    /// Send heartbeat.
    pub fn send_heartbeat(&self, timestamp: u64) -> Result<()> {
        self.last_heartbeat.store(timestamp, Ordering::Relaxed);
        self.send(FaultChannelMessage::Heartbeat { timestamp })
    }

    /// Acknowledge fault injection.
    pub fn ack_injection(
        &self,
        fault_id: Uuid,
        applied_at: u64,
        success: bool,
        message: String,
    ) -> Result<()> {
        debug!("Acknowledging fault injection: {} (success={})", fault_id, success);
        self.send(FaultChannelMessage::FaultInjectionAck {
            fault_id,
            applied_at,
            success,
            message,
        })
    }

    /// Confirm recovery.
    pub fn confirm_recovery(
        &self,
        fault_id: Uuid,
        recovered_at: u64,
        failure_count: u64,
    ) -> Result<()> {
        debug!("Confirming recovery: {} (failures={})", fault_id, failure_count);
        self.send(FaultChannelMessage::RecoveryConfirm {
            fault_id,
            recovered_at,
            failure_count,
        })
    }

    /// Send error.
    pub fn send_error(
        &self,
        fault_id: Option<Uuid>,
        error_code: u32,
        message: String,
    ) -> Result<()> {
        warn!("Sending error on channel: code={}, message={}", error_code, message);
        self.send(FaultChannelMessage::Error {
            fault_id,
            error_code,
            message,
        })
    }

    /// Send status response.
    pub fn send_status(
        &self,
        active_faults: Vec<Uuid>,
        total_injected: u64,
        total_recovered: u64,
        uptime_secs: u64,
    ) -> Result<()> {
        self.send(FaultChannelMessage::StatusResponse {
            active_faults,
            total_injected,
            total_recovered,
            uptime_secs,
        })
    }
}

/// Fault channel protocol handler.
pub struct FaultChannelProtocol {
    channel: VirtioFaultChannel,
    sink: VirtioChannelSink,
}

impl FaultChannelProtocol {
    /// Create new protocol handler.
    pub fn new() -> Self {
        let (channel, sink) = VirtioFaultChannel::new();
        Self { channel, sink }
    }

    /// Get reference to channel.
    pub fn channel(&self) -> &VirtioFaultChannel {
        &self.channel
    }

    /// Get reference to sink.
    pub fn sink(&self) -> &VirtioChannelSink {
        &self.sink
    }

    /// Run channel processing loop.
    pub async fn run(&mut self) -> Result<()> {
        info!("Starting fault channel protocol handler");

        while let Some(msg) = self.channel.recv().await {
            match msg {
                FaultChannelMessage::Heartbeat { timestamp } => {
                    debug!("Received heartbeat at {}", timestamp);
                }
                FaultChannelMessage::StatusRequest => {
                    debug!("Status request received");
                }
                FaultChannelMessage::Error {
                    fault_id,
                    error_code,
                    message,
                } => {
                    warn!("Error message: code={}, fault={:?}, msg={}", error_code, fault_id, message);
                }
                FaultChannelMessage::CancelFault { fault_id } => {
                    debug!("Fault cancellation requested: {}", fault_id);
                }
                msg => {
                    debug!("Processing message: {:?}", msg);
                }
            }
        }

        Ok(())
    }
}

impl Default for FaultChannelProtocol {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_creation() {
        let (channel, _sink) = VirtioFaultChannel::new();
        assert!(channel.is_healthy());
    }

    #[test]
    fn test_channel_health() {
        let (channel, _sink) = VirtioFaultChannel::new();
        assert!(channel.is_healthy());
        channel.mark_unhealthy();
        assert!(!channel.is_healthy());
    }

    #[tokio::test]
    async fn test_message_send_recv() {
        let (mut channel, sink) = VirtioFaultChannel::new();

        let fault_id = Uuid::new_v4();
        sink.ack_injection(fault_id, 100, true, "OK".to_string())
            .unwrap();

        let msg = channel.recv().await.unwrap();
        match msg {
            FaultChannelMessage::FaultInjectionAck {
                fault_id: id,
                applied_at,
                success,
                ..
            } => {
                assert_eq!(id, fault_id);
                assert_eq!(applied_at, 100);
                assert!(success);
            }
            _ => panic!("Unexpected message type"),
        }
    }

    #[test]
    fn test_heartbeat_tracking() {
        let (channel, sink) = VirtioFaultChannel::new();
        sink.send_heartbeat(1000).unwrap();
        let elapsed = channel.time_since_heartbeat(1050);
        assert_eq!(elapsed, 50);
    }
}
