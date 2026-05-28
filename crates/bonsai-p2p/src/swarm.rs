//! libp2p Kademlia swarm transport lane.
//!
//! The [`SwarmLane`] wraps a libp2p Swarm that uses:
//! - TCP transport with Noise encryption and Yamux multiplexing
//! - Kademlia DHT for peer discovery
//! - `request_response` (CBOR codec) for chunk transfer
//!
//! # Usage
//!
//! ```no_run
//! # async fn example() -> anyhow::Result<()> {
//! use bonsai_p2p::SwarmLane;
//! let lane = SwarmLane::connect("swarm:peer1", "/ip4/1.2.3.4/tcp/7001").await?;
//! # Ok(()) }
//! ```

use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use futures::StreamExt as _;
use libp2p::{
    identify, kad,
    noise,
    request_response::{self, cbor, OutboundRequestId, ProtocolSupport},
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux,
    Multiaddr, PeerId, StreamProtocol, SwarmBuilder,
};
use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};
use tracing::debug;

use bonsai_transfer_core::{
    error::{TransferError, TransferResult},
    lane::{LaneHealth, LaneKind, TransportLane},
};
use bonsai_transfer_crypto::cipher::ChunkCiphertext;

// ── Wire protocol types ───────────────────────────────────────────────────────

/// A chunk sent over the swarm — bincode-encoded ChunkCiphertext.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmChunkReq {
    pub data: Vec<u8>,
}

/// Acknowledgement / NACK sent back.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmChunkResp {
    pub gsn: u64,
    pub ok:  bool,
}

// ── Combined swarm behaviour ──────────────────────────────────────────────────

#[derive(NetworkBehaviour)]
struct BonsaiBehaviour {
    kad:      kad::Behaviour<kad::store::MemoryStore>,
    identify: identify::Behaviour,
    rr:       cbor::Behaviour<SwarmChunkReq, SwarmChunkResp>,
}

// ── Commands sent from lane → swarm task ─────────────────────────────────────

enum SwarmCmd {
    SendChunk {
        peer:  PeerId,
        req:   SwarmChunkReq,
        reply: oneshot::Sender<Result<SwarmChunkResp, String>>,
    },
    Dial(Multiaddr),
    Close,
}

// ── SwarmLane ─────────────────────────────────────────────────────────────────

pub struct SwarmLane {
    name:    String,
    peer_id: PeerId,
    cmd_tx:  mpsc::UnboundedSender<SwarmCmd>,
    health:  Arc<Mutex<LaneHealth>>,
}

impl SwarmLane {
    /// Spin up a swarm and dial `peer_addr` (a `/ip4/.../tcp/.../p2p/...` multiaddr).
    pub async fn connect(
        name: impl Into<String>,
        peer_addr: impl AsRef<str>,
    ) -> anyhow::Result<Arc<Self>> {
        let peer_addr: Multiaddr = peer_addr.as_ref().parse()?;
        let name = name.into();

        let mut swarm = SwarmBuilder::with_new_identity()
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )?
            .with_behaviour(|key| {
                let local_peer_id = PeerId::from_public_key(&key.public());
                let kad_config = kad::Config::new(StreamProtocol::new("/bonsai/kad/1"));
                let store = kad::store::MemoryStore::new(local_peer_id);
                let kad = kad::Behaviour::with_config(local_peer_id, store, kad_config);

                let identify = identify::Behaviour::new(identify::Config::new(
                    "/bonsai/1.0.0".into(),
                    key.public(),
                ));

                let rr = cbor::Behaviour::<SwarmChunkReq, SwarmChunkResp>::new(
                    [(
                        StreamProtocol::new("/bonsai/chunks/1"),
                        ProtocolSupport::Full,
                    )],
                    request_response::Config::default(),
                );

                Ok(BonsaiBehaviour { kad, identify, rr })
            })?
            .build();

        let local_peer_id = *swarm.local_peer_id();

        // Dial the target peer.
        swarm.dial(peer_addr.clone())?;

        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<SwarmCmd>();
        let health = Arc::new(Mutex::new(LaneHealth {
            rtt_ms:        200.0,
            bandwidth_bps: 5_000_000,
            in_flight:     0,
            available:     false,
            loss_rate:     0.0,
        }));
        let health2 = health.clone();

        // Track pending request → oneshot map.
        let mut pending: std::collections::HashMap<
            OutboundRequestId,
            oneshot::Sender<Result<SwarmChunkResp, String>>,
        > = Default::default();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    event = swarm.select_next_some() => {
                        match event {
                            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                                debug!("swarm: connected to {peer_id}");
                                health2.lock().unwrap().available = true;
                            }
                            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                                debug!("swarm: disconnected from {peer_id}");
                                health2.lock().unwrap().available = false;
                            }
                            SwarmEvent::Behaviour(BonsaiBehaviourEvent::Rr(
                                request_response::Event::Message { peer, message }
                            )) => {
                                match message {
                                    request_response::Message::Request { request, channel, .. } => {
                                        // We received a chunk — deserialize and push to consumer.
                                        // For now we just ACK.
                                        let gsn = bincode::deserialize::<ChunkCiphertext>(&request.data)
                                            .map(|c| c.gsn)
                                            .unwrap_or(0);
                                        let _ = swarm.behaviour_mut().rr.send_response(
                                            channel,
                                            SwarmChunkResp { gsn, ok: true },
                                        );
                                    }
                                    request_response::Message::Response { request_id, response } => {
                                        if let Some(tx) = pending.remove(&request_id) {
                                            let _ = tx.send(Ok(response));
                                        }
                                    }
                                }
                            }
                            SwarmEvent::Behaviour(BonsaiBehaviourEvent::Rr(
                                request_response::Event::OutboundFailure { request_id, error, .. }
                            )) => {
                                if let Some(tx) = pending.remove(&request_id) {
                                    let _ = tx.send(Err(error.to_string()));
                                }
                            }
                            _ => {}
                        }
                    }
                    cmd = cmd_rx.recv() => {
                        match cmd {
                            Some(SwarmCmd::SendChunk { peer, req, reply }) => {
                                let id = swarm.behaviour_mut().rr.send_request(&peer, req);
                                pending.insert(id, reply);
                            }
                            Some(SwarmCmd::Dial(addr)) => {
                                let _ = swarm.dial(addr);
                            }
                            Some(SwarmCmd::Close) | None => break,
                        }
                    }
                }
            }
        });

        // Extract PeerId from the Multiaddr (p2p component) or use a placeholder.
        let peer_id = extract_peer_id(&peer_addr).unwrap_or(local_peer_id);

        Ok(Arc::new(Self {
            name,
            peer_id,
            cmd_tx,
            health,
        }))
    }

    /// Dial an additional peer after construction.
    pub fn dial(&self, addr: Multiaddr) {
        let _ = self.cmd_tx.send(SwarmCmd::Dial(addr));
    }
}

fn extract_peer_id(addr: &Multiaddr) -> Option<PeerId> {
    use libp2p::multiaddr::Protocol;
    addr.iter().find_map(|p| {
        if let Protocol::P2p(h) = p { Some(h) } else { None }
    })
}

#[async_trait]
impl TransportLane for SwarmLane {
    fn name(&self) -> &str { &self.name }
    fn kind(&self) -> LaneKind { LaneKind::Swarm }
    fn health(&self) -> LaneHealth { self.health.lock().unwrap().clone() }

    async fn send_chunk(&self, chunk: &ChunkCiphertext) -> TransferResult<()> {
        let data = bincode::serialize(chunk)
            .map_err(|e| TransferError::Other(e.to_string()))?;
        let req = SwarmChunkReq { data };
        let (reply_tx, reply_rx) = oneshot::channel();
        self.cmd_tx.send(SwarmCmd::SendChunk {
            peer:  self.peer_id,
            req,
            reply: reply_tx,
        }).map_err(|_| TransferError::Other(format!("{}: swarm task closed", self.name)))?;

        let resp = reply_rx.await
            .map_err(|_| TransferError::Other(format!("{}: swarm reply dropped", self.name)))?
            .map_err(|e| TransferError::LaneFailed(self.name.clone(), e))?;

        if !resp.ok {
            return Err(TransferError::LaneFailed(self.name.clone(), "remote rejected chunk".into()));
        }
        Ok(())
    }

    async fn send_ack(&self, _gsn: u64) -> TransferResult<()> {
        // ACKs are embedded in SwarmChunkResp; sending a bare ACK is a no-op here.
        Ok(())
    }

    async fn send_nack(&self, _gsn: u64) -> TransferResult<()> {
        Ok(())
    }

    async fn ping(&self) -> Option<Duration> {
        Some(Duration::from_millis(self.health.lock().unwrap().rtt_ms as u64))
    }

    async fn close(&self) {
        let _ = self.cmd_tx.send(SwarmCmd::Close);
        self.health.lock().unwrap().available = false;
    }
}
