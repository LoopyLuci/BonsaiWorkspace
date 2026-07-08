//! WebRTC DataChannel transport lane.
//!
//! # Usage
//!
//! Signaling (SDP/ICE) is performed outside the lane — call
//! [`create_peer_connection`] + [`create_offer`] on the offering side,
//! exchange SDP with the remote, then [`accept_answer`] to complete ICE.
//! Once the data channel fires `on_open`, the lane is ready.
//!
//! ```no_run
//! # async fn example() -> anyhow::Result<()> {
//! use p2p::WebRtcLane;
//! let stun = vec!["stun:stun.l.google.com:19302".into()];
//! let (lane, offer_sdp) = WebRtcLane::new_offer("webrtc:peer1", stun).await?;
//! // send offer_sdp to remote via signaling channel
//! // receive answer_sdp from remote
//! // WebRtcLane::accept_answer(&lane, &answer_sdp).await?;
//! # Ok(()) }
//! ```

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use async_trait::async_trait;
use bytes::Bytes;
use tokio::sync::{mpsc, oneshot};
use tracing::warn;

use webrtc::{
    api::APIBuilder,
    data_channel::{data_channel_message::DataChannelMessage, RTCDataChannel},
    ice_transport::ice_server::RTCIceServer,
    peer_connection::{
        configuration::RTCConfiguration, peer_connection_state::RTCPeerConnectionState,
        sdp::session_description::RTCSessionDescription, RTCPeerConnection,
    },
};

use p2p_core::{
    error::{TransferError, TransferResult},
    lane::{LaneHealth, LaneKind, TransportLane},
};
use p2p_crypto::cipher::ChunkCiphertext;

// ── WebRtcLane ────────────────────────────────────────────────────────────────

pub struct WebRtcLane {
    name: String,
    /// Kept alive so ICE/DTLS stays up.
    _pc: Arc<RTCPeerConnection>,
    dc: Arc<RTCDataChannel>,
    /// Inbound chunks fed from `on_message` callback.
    rx: Mutex<Option<mpsc::UnboundedReceiver<ChunkCiphertext>>>,
    health: Arc<Mutex<LaneHealth>>,
    closed: Arc<AtomicBool>,
    /// Fires when the data channel opens (ICE + DTLS handshake done).
    open_notify: Arc<tokio::sync::Notify>,
    ping_rtt: Arc<Mutex<Option<Duration>>>,
}

impl WebRtcLane {
    // ── Signaling helpers ─────────────────────────────────────────────────────

    /// Build a new `RTCPeerConnection` with the given STUN servers.
    async fn make_pc(stun_urls: Vec<String>) -> anyhow::Result<Arc<RTCPeerConnection>> {
        let api = APIBuilder::new().build();
        let config = RTCConfiguration {
            ice_servers: vec![RTCIceServer {
                urls: stun_urls,
                ..Default::default()
            }],
            ..Default::default()
        };
        Ok(Arc::new(api.new_peer_connection(config).await?))
    }

    /// Create the offering side.  Returns the lane (not yet connected) and the
    /// local SDP offer string to be sent to the remote via a signaling channel.
    pub async fn new_offer(
        name: impl Into<String>,
        stun_urls: Vec<String>,
    ) -> anyhow::Result<(Arc<Self>, String)> {
        let name = name.into();
        let pc = Self::make_pc(stun_urls).await?;
        let dc = pc.create_data_channel("bonsai", None).await?;
        let offer = pc.create_offer(None).await?;
        pc.set_local_description(offer.clone()).await?;

        // Wait for ICE gathering to complete.
        let mut gather_rx = pc.gathering_complete_promise().await;
        gather_rx.recv().await;

        let local_sdp = pc
            .local_description()
            .await
            .ok_or_else(|| anyhow::anyhow!("no local SDP after ICE gathering"))?
            .sdp;

        let lane = Self::attach(name, pc, dc).await;
        Ok((lane, local_sdp))
    }

    /// Create the answering side.  `remote_offer_sdp` is the SDP received from
    /// the offerer via the signaling channel.
    pub async fn new_answer(
        name: impl Into<String>,
        stun_urls: Vec<String>,
        remote_offer_sdp: &str,
    ) -> anyhow::Result<(Arc<Self>, String)> {
        let name = name.into();
        let pc = Self::make_pc(stun_urls).await?;

        let offer = RTCSessionDescription::offer(remote_offer_sdp.to_owned())?;
        pc.set_remote_description(offer).await?;
        let answer = pc.create_answer(None).await?;
        pc.set_local_description(answer).await?;

        let mut gather_rx = pc.gathering_complete_promise().await;
        gather_rx.recv().await;

        let local_sdp = pc
            .local_description()
            .await
            .ok_or_else(|| anyhow::anyhow!("no local SDP after ICE gathering"))?
            .sdp;

        // Answerer receives the data channel via `on_data_channel`.
        let (dc_tx, dc_rx) = oneshot::channel::<Arc<RTCDataChannel>>();
        let dc_tx = Mutex::new(Some(dc_tx));
        pc.on_data_channel(Box::new(move |dc: Arc<RTCDataChannel>| {
            if let Some(tx) = dc_tx.lock().unwrap().take() {
                let _ = tx.send(dc);
            }
            Box::pin(async {})
        }));

        let dc = tokio::time::timeout(Duration::from_secs(30), async move {
            dc_rx
                .await
                .map_err(|_| anyhow::anyhow!("data channel never arrived"))
        })
        .await??;

        let lane = Self::attach(name, pc, dc).await;
        Ok((lane, local_sdp))
    }

    /// Apply the remote's answer SDP to complete ICE.
    pub async fn accept_answer(lane: &Arc<Self>, answer_sdp: &str) -> anyhow::Result<()> {
        let answer = RTCSessionDescription::answer(answer_sdp.to_owned())?;
        lane._pc.set_remote_description(answer).await?;
        Ok(())
    }

    // ── Internal construction ─────────────────────────────────────────────────

    async fn attach(
        name: String,
        pc: Arc<RTCPeerConnection>,
        dc: Arc<RTCDataChannel>,
    ) -> Arc<Self> {
        let (tx, rx) = mpsc::unbounded_channel::<ChunkCiphertext>();
        let health = Arc::new(Mutex::new(LaneHealth {
            rtt_ms: 150.0,
            bandwidth_bps: 10_000_000, // 10 Mbps initial estimate
            in_flight: 0,
            available: false, // becomes true on_open
            loss_rate: 0.0,
        }));
        let closed = Arc::new(AtomicBool::new(false));
        let open_notify = Arc::new(tokio::sync::Notify::new());
        let ping_rtt = Arc::new(Mutex::new(None::<Duration>));

        // on_open — mark lane available
        {
            let h = health.clone();
            let n = open_notify.clone();
            dc.on_open(Box::new(move || {
                h.lock().unwrap().available = true;
                n.notify_waiters();
                Box::pin(async {})
            }));
        }

        // on_message — deserialize ChunkCiphertext and push to channel
        {
            let tx2 = tx.clone();
            let name2 = name.clone();
            dc.on_message(Box::new(move |msg: DataChannelMessage| {
                let tx3 = tx2.clone();
                let n2 = name2.clone();
                Box::pin(async move {
                    match bincode::deserialize::<ChunkCiphertext>(&msg.data) {
                        Ok(chunk) => {
                            let _ = tx3.send(chunk);
                        }
                        Err(e) => warn!("{n2}: deserialize error: {e}"),
                    }
                })
            }));
        }

        // on_connection_state_change — mark unavailable on failure
        {
            let h = health.clone();
            let c = closed.clone();
            pc.on_peer_connection_state_change(Box::new(move |s: RTCPeerConnectionState| {
                if matches!(
                    s,
                    RTCPeerConnectionState::Failed
                        | RTCPeerConnectionState::Closed
                        | RTCPeerConnectionState::Disconnected
                ) {
                    h.lock().unwrap().available = false;
                    c.store(true, Ordering::Relaxed);
                }
                Box::pin(async {})
            }));
        }

        Arc::new(Self {
            name,
            _pc: pc,
            dc,
            rx: Mutex::new(Some(rx)),
            health,
            closed,
            open_notify,
            ping_rtt,
        })
    }

    /// Wait until the data channel is open (up to `timeout`).
    pub async fn wait_open(&self, timeout: Duration) -> bool {
        tokio::time::timeout(timeout, self.open_notify.notified())
            .await
            .is_ok()
    }

    /// Non-blocking receive of the next inbound chunk.
    pub fn try_recv(&self) -> Option<ChunkCiphertext> {
        self.rx.lock().unwrap().as_mut()?.try_recv().ok()
    }
}

impl WebRtcLane {
    /// Return the last measured ping RTT, if `ping()` has been called at least once.
    pub fn last_ping_rtt(&self) -> Option<Duration> {
        *self.ping_rtt.lock().unwrap()
    }
}

#[async_trait]
impl TransportLane for WebRtcLane {
    fn name(&self) -> &str {
        &self.name
    }
    fn kind(&self) -> LaneKind {
        LaneKind::WebRtc
    }
    fn health(&self) -> LaneHealth {
        self.health.lock().unwrap().clone()
    }

    async fn send_chunk(&self, chunk: &ChunkCiphertext) -> TransferResult<()> {
        if self.closed.load(Ordering::Relaxed) {
            return Err(TransferError::Other(format!(
                "{}: data channel closed",
                self.name
            )));
        }
        let data =
            bincode::serialize(chunk).map_err(|e| TransferError::Other(format!("bincode: {e}")))?;
        self.dc
            .send(&Bytes::from(data))
            .await
            .map_err(|e| TransferError::Other(format!("{}: send: {e}", self.name)))?;
        Ok(())
    }

    async fn send_ack(&self, gsn: u64) -> TransferResult<()> {
        let payload =
            bincode::serialize(&("ack", gsn)).map_err(|e| TransferError::Other(e.to_string()))?;
        self.dc
            .send(&Bytes::from(payload))
            .await
            .map_err(|e| TransferError::Other(e.to_string()))?;
        Ok(())
    }

    async fn send_nack(&self, gsn: u64) -> TransferResult<()> {
        let payload =
            bincode::serialize(&("nack", gsn)).map_err(|e| TransferError::Other(e.to_string()))?;
        self.dc
            .send(&Bytes::from(payload))
            .await
            .map_err(|e| TransferError::Other(e.to_string()))?;
        Ok(())
    }

    async fn ping(&self) -> Option<Duration> {
        // Send a small ping frame and measure RTT.
        let t0 = Instant::now();
        let payload = bincode::serialize(&("ping", t0.elapsed().as_nanos() as u64)).ok()?;
        self.dc.send(&Bytes::from(payload)).await.ok()?;
        // Measure the one-way latency as a RTT approximation and store it.
        let rtt = t0.elapsed();
        *self.ping_rtt.lock().unwrap() = Some(rtt);
        // Also update the health rtt_ms so the scheduler reflects the new measurement.
        self.health.lock().unwrap().rtt_ms = rtt.as_millis() as f64;
        Some(rtt)
    }

    async fn close(&self) {
        self.closed.store(true, Ordering::Relaxed);
        let _ = self.dc.close().await;
        let _ = self._pc.close().await;
        self.health.lock().unwrap().available = false;
    }
}
