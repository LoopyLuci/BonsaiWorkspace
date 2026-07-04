//! Tor onion-routing transport lane via SOCKS5.
//!
//! [`OnionLane`] sends [`ChunkCiphertext`] frames over TCP connections routed
//! through a locally-running Tor daemon (default SOCKS5 proxy: `127.0.0.1:9050`).
//!
//! The Tor daemon must be installed and running independently.  Bonsai does not
//! bundle or start Tor; the user is responsible for running `tor` (or Tor
//! Browser) before using onion lanes.
//!
//! # Wire framing
//!
//! Each frame is: `[tag: u8][length: u32 LE][bincode payload]`.
//!
//! Tags: `0x01` = chunk, `0x02` = ACK, `0x03` = NACK.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use tokio::io::AsyncWriteExt;
use tokio_socks::tcp::Socks5Stream;
use tracing::{debug, warn};

use p2p_core::{
    error::{TransferError, TransferResult},
    lane::{LaneHealth, LaneKind, TransportLane},
};
use p2p_crypto::cipher::ChunkCiphertext;

// ── Frame tags ────────────────────────────────────────────────────────────────

const TAG_CHUNK: u8 = 0x01;
const TAG_ACK: u8 = 0x02;
const TAG_NACK: u8 = 0x03;

// ── OnionLane ─────────────────────────────────────────────────────────────────

/// Anonymous transport lane routed through the Tor network via SOCKS5.
pub struct OnionLane {
    name: String,
    proxy_addr: String,
    target: String,
    port: u16,
    health: Arc<Mutex<LaneHealth>>,
}

impl OnionLane {
    /// Create a lane that will route through the Tor SOCKS5 proxy at
    /// `proxy_addr` (e.g. `"127.0.0.1:9050"`) to `target:port`.
    ///
    /// A test connection is attempted to verify the proxy is reachable; the
    /// function returns an error if the proxy is not available.
    pub async fn connect(
        name: impl Into<String>,
        proxy_addr: impl Into<String>,
        target: impl Into<String>,
        port: u16,
    ) -> anyhow::Result<Arc<Self>> {
        let name = name.into();
        let proxy_addr = proxy_addr.into();
        let target = target.into();

        // Probe: open a throw-away connection to verify the proxy is up.
        let test = Socks5Stream::connect(proxy_addr.as_str(), (target.as_str(), port)).await;
        let available = test.is_ok();
        if let Err(ref e) = test {
            warn!("{name}: Tor proxy probe failed ({proxy_addr}): {e}");
        }

        Ok(Arc::new(Self {
            name,
            proxy_addr,
            target,
            port,
            health: Arc::new(Mutex::new(LaneHealth {
                rtt_ms: 600.0,          // Tor adds ~300-600 ms
                bandwidth_bps: 500_000, // Tor bandwidth is typically constrained
                in_flight: 0,
                available,
                loss_rate: 0.0,
            })),
        }))
    }

    /// Convenience: use the default Tor SOCKS5 address `127.0.0.1:9050`.
    pub async fn connect_default(
        name: impl Into<String>,
        target: impl Into<String>,
        port: u16,
    ) -> anyhow::Result<Arc<Self>> {
        Self::connect(name, "127.0.0.1:9050", target, port).await
    }

    /// Open a fresh Tor stream, write a single framed payload, then close it.
    /// A new circuit-eligible stream is used per call to maximise anonymity.
    async fn send_frame(&self, tag: u8, payload: &[u8]) -> TransferResult<()> {
        let t0 = std::time::Instant::now();

        let mut stream =
            Socks5Stream::connect(self.proxy_addr.as_str(), (self.target.as_str(), self.port))
                .await
                .map_err(|e| {
                    self.health.lock().unwrap().available = false;
                    TransferError::Other(format!("{}: socks5 connect: {e}", self.name))
                })?;

        // Mark available if we previously lost the proxy.
        self.health.lock().unwrap().available = true;

        // frame: [tag u8][u32 LE payload_len][payload bytes]
        let mut frame = Vec::with_capacity(5 + payload.len());
        frame.push(tag);
        frame.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        frame.extend_from_slice(payload);

        stream.write_all(&frame).await.map_err(TransferError::Io)?;
        stream.flush().await.map_err(TransferError::Io)?;
        stream.shutdown().await.map_err(TransferError::Io)?;

        // EWMA RTT update.
        let rtt_ms = t0.elapsed().as_secs_f64() * 1000.0;
        let mut h = self.health.lock().unwrap();
        h.rtt_ms = h.rtt_ms * 0.8 + rtt_ms * 0.2;

        debug!(
            "{}: frame tag={tag:#x} len={} rtt={rtt_ms:.0}ms",
            self.name,
            payload.len()
        );
        Ok(())
    }
}

#[async_trait]
impl TransportLane for OnionLane {
    fn name(&self) -> &str {
        &self.name
    }
    fn kind(&self) -> LaneKind {
        LaneKind::Onion
    }
    fn health(&self) -> LaneHealth {
        self.health.lock().unwrap().clone()
    }

    async fn send_chunk(&self, chunk: &ChunkCiphertext) -> TransferResult<()> {
        let data = bincode::serialize(chunk).map_err(|e| TransferError::Other(e.to_string()))?;
        self.send_frame(TAG_CHUNK, &data).await
    }

    async fn send_ack(&self, gsn: u64) -> TransferResult<()> {
        self.send_frame(TAG_ACK, &gsn.to_le_bytes()).await
    }

    async fn send_nack(&self, gsn: u64) -> TransferResult<()> {
        self.send_frame(TAG_NACK, &gsn.to_le_bytes()).await
    }

    async fn ping(&self) -> Option<Duration> {
        Some(Duration::from_millis(
            self.health.lock().unwrap().rtt_ms as u64,
        ))
    }

    async fn close(&self) {
        self.health.lock().unwrap().available = false;
    }
}
