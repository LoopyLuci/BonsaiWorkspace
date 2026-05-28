//! Relay client — connects to a blind relay server and wraps it as a `TransportLane`.

use std::sync::{Arc, Mutex};
use std::time::Duration;
use async_trait::async_trait;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use bonsai_transfer_crypto::cipher::ChunkCiphertext;
use bonsai_transfer_core::lane::{LaneHealth, LaneKind, TransportLane};
use bonsai_transfer_core::error::TransferResult;
use crate::error::{RelayError, RelayResult};
use crate::token::{RegisterRequest, RelayToken};

const MAX_FRAME: usize = 18 * 1024 * 1024;

/// A `TransportLane` backed by a blind relay server.
pub struct RelayClient {
    name: String,
    token: RelayToken,
    relay_addr: String,
    health: Arc<Mutex<LaneHealth>>,
    /// Sender to the write loop; `None` until `connect()` is called.
    tx: Arc<Mutex<Option<mpsc::UnboundedSender<Vec<u8>>>>>,
}

impl RelayClient {
    /// Create a client that will connect to `relay_addr` using `token`.
    pub fn new(name: &str, relay_addr: &str, token: RelayToken) -> Self {
        Self {
            name: name.to_string(),
            token,
            relay_addr: relay_addr.to_string(),
            health: Arc::new(Mutex::new(LaneHealth {
                rtt_ms: 50.0,
                bandwidth_bps: 10_000_000, // 10 Mbps initial estimate
                in_flight: 0,
                available: false,
                loss_rate: 0.0,
            })),
            tx: Arc::new(Mutex::new(None)),
        }
    }

    /// Perform PoW mining + TCP registration. Spawns read/write loops.
    /// Returns a receiver for inbound chunks.
    pub async fn connect(&self) -> RelayResult<mpsc::UnboundedReceiver<Vec<u8>>> {
        let req = RegisterRequest::mine(self.token.clone());
        let req_bytes = serde_json::to_vec(&req).map_err(RelayError::Serde)?;

        let mut stream = TcpStream::connect(&self.relay_addr).await?;

        // Send registration
        stream.write_u32(req_bytes.len() as u32).await?;
        stream.write_all(&req_bytes).await?;

        // Wait for ACK
        let ack_len = stream.read_u32().await? as usize;
        let mut ack = vec![0u8; ack_len];
        stream.read_exact(&mut ack).await?;
        if &ack != b"OK" { return Err(RelayError::InvalidToken); }

        let (out_tx, mut out_rx) = mpsc::unbounded_channel::<Vec<u8>>();
        let (in_tx, in_rx) = mpsc::unbounded_channel::<Vec<u8>>();

        *self.tx.lock().unwrap() = Some(out_tx);
        self.health.lock().unwrap().available = true;

        let (mut reader, mut writer) = stream.into_split();

        // Write loop
        tokio::spawn(async move {
            while let Some(data) = out_rx.recv().await {
                if writer.write_u32(data.len() as u32).await.is_err() { break; }
                if writer.write_all(&data).await.is_err() { break; }
            }
        });

        // Read loop
        tokio::spawn(async move {
            loop {
                let len = match reader.read_u32().await {
                    Ok(n) => n as usize,
                    Err(_) => break,
                };
                if len > MAX_FRAME { break; }
                let mut buf = vec![0u8; len];
                if reader.read_exact(&mut buf).await.is_err() { break; }
                if in_tx.send(buf).is_err() { break; }
            }
        });

        Ok(in_rx)
    }

    fn send_raw(&self, data: Vec<u8>) -> TransferResult<()> {
        let guard = self.tx.lock().unwrap();
        if let Some(ref tx) = *guard {
            tx.send(data)
                .map_err(|_| bonsai_transfer_core::error::TransferError::LaneFailed(
                    self.name.clone(), "relay channel closed".into()
                ))?;
            Ok(())
        } else {
            Err(bonsai_transfer_core::error::TransferError::LaneFailed(
                self.name.clone(), "not connected".into()
            ))
        }
    }
}

#[async_trait]
impl TransportLane for RelayClient {
    fn name(&self) -> &str { &self.name }
    fn kind(&self) -> LaneKind { LaneKind::Relay }
    fn health(&self) -> LaneHealth { self.health.lock().unwrap().clone() }

    async fn send_chunk(&self, chunk: &ChunkCiphertext) -> TransferResult<()> {
        let bytes = serde_json::to_vec(chunk)
            .map_err(|e| bonsai_transfer_core::error::TransferError::Other(e.to_string()))?;
        self.send_raw(bytes)
    }

    async fn send_ack(&self, gsn: u64) -> TransferResult<()> {
        let msg = format!("ACK:{gsn}");
        self.send_raw(msg.into_bytes())
    }

    async fn send_nack(&self, gsn: u64) -> TransferResult<()> {
        let msg = format!("NACK:{gsn}");
        self.send_raw(msg.into_bytes())
    }

    async fn ping(&self) -> Option<Duration> {
        let rtt = self.health.lock().unwrap().rtt_ms;
        Some(Duration::from_micros((rtt * 1000.0) as u64))
    }

    async fn close(&self) {
        *self.tx.lock().unwrap() = None;
        self.health.lock().unwrap().available = false;
    }
}
