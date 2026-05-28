//! TransportLane — plugin trait for all transport mechanisms.
//!
//! Any transport (TCP, relay, DMI, Wi-Fi, Bluetooth) implements this trait.
//! The ECF-RG scheduler calls `send_chunk` and monitors `health()` to
//! select the optimal lane per chunk.

use std::time::Duration;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use bonsai_transfer_crypto::cipher::ChunkCiphertext;
use crate::error::TransferResult;

// ── Lane kind ─────────────────────────────────────────────────────────────────

/// Which physical/logical transport backs this lane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LaneKind {
    /// Zero-copy DMI ring (Thunderbolt/USB4) — same machine.
    Dmi,
    /// Direct TCP connection.
    Tcp,
    /// Blind relay server.
    Relay,
    /// Wi-Fi Direct (P2P).
    WifiDirect,
    /// Bluetooth Low Energy.
    Ble,
    /// BitTorrent/IPFS swarm.
    Swarm,
    /// MQTT/CoAP for IoT endpoints.
    Mqtt,
    /// In-process channel (for testing and intra-process IPC).
    InProcess,
    /// WebRTC DataChannel (browser-compatible, NAT-traversing).
    WebRtc,
    /// Tor onion-routing (anonymous, censorship-resistant).
    Onion,
}

impl std::fmt::Display for LaneKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Dmi       => write!(f, "DMI"),
            Self::Tcp       => write!(f, "TCP"),
            Self::Relay     => write!(f, "Relay"),
            Self::WifiDirect=> write!(f, "Wi-Fi"),
            Self::Ble       => write!(f, "BLE"),
            Self::Swarm     => write!(f, "Swarm"),
            Self::Mqtt      => write!(f, "MQTT"),
            Self::InProcess => write!(f, "InProcess"),
            Self::WebRtc    => write!(f, "WebRTC"),
            Self::Onion     => write!(f, "Onion"),
        }
    }
}

// ── Lane health ───────────────────────────────────────────────────────────────

/// Runtime health snapshot for a lane — used by ECF-RG scheduler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaneHealth {
    /// Round-trip time estimate (milliseconds).
    pub rtt_ms: f64,
    /// Estimated available bandwidth (bytes/sec).
    pub bandwidth_bps: u64,
    /// Number of chunks currently in-flight on this lane.
    pub in_flight: u32,
    /// Is the lane currently usable?
    pub available: bool,
    /// Packet loss rate [0.0, 1.0].
    pub loss_rate: f32,
}

impl LaneHealth {
    /// Estimated time to complete sending `bytes` more bytes on this lane.
    pub fn estimated_completion_secs(&self, bytes: u64) -> f64 {
        if self.bandwidth_bps == 0 { return f64::MAX; }
        let rtt = self.rtt_ms / 1000.0;
        let transfer_time = bytes as f64 / self.bandwidth_bps as f64;
        rtt + transfer_time
    }

    /// A placeholder "ideal" lane for in-process use.
    pub fn ideal() -> Self {
        Self {
            rtt_ms: 0.1,
            bandwidth_bps: 10_000_000_000, // 10 Gbps
            in_flight: 0,
            available: true,
            loss_rate: 0.0,
        }
    }

    pub fn unavailable() -> Self {
        Self {
            rtt_ms: f64::MAX,
            bandwidth_bps: 0,
            in_flight: 0,
            available: false,
            loss_rate: 1.0,
        }
    }
}

// ── TransportLane trait ───────────────────────────────────────────────────────

/// The core abstraction every transport implements.
#[async_trait]
pub trait TransportLane: Send + Sync + 'static {
    /// Human-readable name for this lane instance (e.g., "tcp:192.168.1.5:7001").
    fn name(&self) -> &str;

    /// The kind of transport.
    fn kind(&self) -> LaneKind;

    /// Current health snapshot (called frequently by the scheduler).
    fn health(&self) -> LaneHealth;

    /// Send an encrypted chunk.  Returns when the chunk is handed off to the
    /// underlying transport (not when it is acknowledged by the receiver).
    async fn send_chunk(&self, chunk: &ChunkCiphertext) -> TransferResult<()>;

    /// Send a raw ACK for `gsn` back to the sender.
    async fn send_ack(&self, gsn: u64) -> TransferResult<()>;

    /// Send a NACK requesting retransmission of `gsn`.
    async fn send_nack(&self, gsn: u64) -> TransferResult<()>;

    /// Attempt to measure current RTT (optional, return None if not supported).
    async fn ping(&self) -> Option<Duration> { None }

    /// Shut down the lane gracefully.
    async fn close(&self) {}
}

// ── In-process lane (for testing and intra-process use) ──────────────────────

use tokio::sync::mpsc;
use std::sync::{Arc, Mutex};

/// A loopback lane that delivers chunks via an in-memory channel.
/// Used for testing and for intra-process Bonsai agent communication.
pub struct InProcessLane {
    name: String,
    tx: mpsc::UnboundedSender<ChunkCiphertext>,
    health: Arc<Mutex<LaneHealth>>,
}

impl InProcessLane {
    pub fn new_pair(name: &str) -> (Self, mpsc::UnboundedReceiver<ChunkCiphertext>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let lane = Self {
            name: name.into(),
            tx,
            health: Arc::new(Mutex::new(LaneHealth::ideal())),
        };
        (lane, rx)
    }
}

#[async_trait]
impl TransportLane for InProcessLane {
    fn name(&self) -> &str { &self.name }
    fn kind(&self) -> LaneKind { LaneKind::InProcess }
    fn health(&self) -> LaneHealth { self.health.lock().unwrap().clone() }

    async fn send_chunk(&self, chunk: &ChunkCiphertext) -> TransferResult<()> {
        self.tx.send(chunk.clone())
            .map_err(|_| crate::error::TransferError::LaneFailed(self.name.clone(), "channel closed".into()))
    }

    async fn send_ack(&self, _gsn: u64) -> TransferResult<()> { Ok(()) }
    async fn send_nack(&self, _gsn: u64) -> TransferResult<()> { Ok(()) }
}
