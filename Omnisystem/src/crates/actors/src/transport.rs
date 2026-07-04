//! TCP/CBOR distributed actor transport + gossip registry.
//!
//! Enables actors on different nodes to send messages across the network.
//!
//! # Architecture
//! ```text
//! Node A                         Node B
//! ┌──────────────────────┐       ┌──────────────────────┐
//! │ ActorSystem          │       │ ActorSystem          │
//! │   ↓                  │ TCP   │   ↑                  │
//! │ TransportLayer ──────┼──────▶│ TransportLayer       │
//! │   ↓                  │       │   ↓                  │
//! │ GossipRegistry       │ CBOR  │ GossipRegistry       │
//! └──────────────────────┘       └──────────────────────┘
//! ```
//!
//! Messages are framed as:
//! `[ 4-byte big-endian length ][ CBOR-encoded RemoteEnvelope ]`

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::ActorId;

// ── Errors ────────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("I/O error: {0}")]
    Io(#[from] tokio::io::Error),
    #[error("CBOR encode error: {0}")]
    CborEncode(String),
    #[error("CBOR decode error: {0}")]
    CborDecode(String),
    #[error("node {0} not reachable")]
    NodeUnreachable(NodeId),
    #[error("message too large: {0} bytes")]
    MessageTooLarge(usize),
    #[error("{0}")]
    Other(String),
}

pub type TransportResult<T> = Result<T, TransportError>;

// ── Node identity ─────────────────────────────────────────────────────────────

pub type NodeId = Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct NodeInfo {
    pub id: NodeId,
    pub addr: String, // "host:port"
    pub actor_count: u32,
    /// Monotonic generation counter — incremented on restart.
    pub generation: u64,
}

// ── Wire format ───────────────────────────────────────────────────────────────

/// Maximum frame body size: 64 MiB
const MAX_FRAME_SIZE: u32 = 64 * 1024 * 1024;

/// A message routed to a remote actor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteEnvelope {
    /// Destination actor on the remote node.
    pub target_actor: ActorId,
    /// Originating actor (or nil for anonymous senders).
    pub sender: Option<ActorId>,
    /// Message type tag (used to dispatch to the right actor mailbox).
    pub message_type: String,
    /// CBOR-encoded payload.
    pub payload: Vec<u8>,
    /// Correlation id for request-reply patterns.
    pub correlation_id: Option<Uuid>,
}

// ── Frame codec ───────────────────────────────────────────────────────────────

async fn write_frame(stream: &mut TcpStream, data: &[u8]) -> TransportResult<()> {
    if data.len() > MAX_FRAME_SIZE as usize {
        return Err(TransportError::MessageTooLarge(data.len()));
    }
    let len = data.len() as u32;
    stream.write_all(&len.to_be_bytes()).await?;
    stream.write_all(data).await?;
    Ok(())
}

async fn read_frame(stream: &mut TcpStream) -> TransportResult<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf);
    if len > MAX_FRAME_SIZE {
        return Err(TransportError::MessageTooLarge(len as usize));
    }
    let mut buf = vec![0u8; len as usize];
    stream.read_exact(&mut buf).await?;
    Ok(buf)
}

fn encode_cbor<T: Serialize>(val: &T) -> TransportResult<Vec<u8>> {
    let mut buf = Vec::new();
    ciborium::into_writer(val, &mut buf).map_err(|e| TransportError::CborEncode(e.to_string()))?;
    Ok(buf)
}

fn decode_cbor<T: for<'de> Deserialize<'de>>(data: &[u8]) -> TransportResult<T> {
    ciborium::from_reader(data).map_err(|e| TransportError::CborDecode(e.to_string()))
}

// ── Gossip Registry ───────────────────────────────────────────────────────────

/// Gossip-based peer registry. Maintains a view of known nodes.
///
/// Each node periodically broadcasts its `NodeInfo`; this registry merges
/// incoming gossip and evicts stale entries.
#[derive(Clone)]
pub struct GossipRegistry {
    inner: Arc<GossipInner>,
}

struct GossipInner {
    local: NodeInfo,
    peers: RwLock<HashMap<NodeId, NodeInfo>>,
}

impl GossipRegistry {
    pub fn new(local: NodeInfo) -> Self {
        Self {
            inner: Arc::new(GossipInner {
                local,
                peers: RwLock::new(HashMap::new()),
            }),
        }
    }

    /// Return this node's identity.
    pub fn local(&self) -> &NodeInfo {
        &self.inner.local
    }

    /// Merge an incoming gossip announcement.
    pub async fn merge(&self, info: NodeInfo) {
        if info.id == self.inner.local.id {
            return;
        }
        let mut peers = self.inner.peers.write().await;
        match peers.get(&info.id) {
            Some(existing) if existing.generation >= info.generation => {}
            _ => {
                peers.insert(info.id, info);
            }
        }
    }

    /// Get info for a specific peer.
    pub async fn get(&self, id: &NodeId) -> Option<NodeInfo> {
        self.inner.peers.read().await.get(id).cloned()
    }

    /// List all known peers.
    pub async fn peers(&self) -> Vec<NodeInfo> {
        self.inner.peers.read().await.values().cloned().collect()
    }

    /// Remove a peer (e.g. after connection failure).
    pub async fn evict(&self, id: &NodeId) {
        self.inner.peers.write().await.remove(id);
    }
}

// ── Transport Layer ───────────────────────────────────────────────────────────

/// Message received from a remote peer, ready for local dispatch.
#[derive(Debug)]
pub struct InboundMessage {
    pub from_node: NodeId,
    pub envelope: RemoteEnvelope,
}

/// The TCP transport layer — manages listener + outbound connection pool.
pub struct TransportLayer {
    pub node: NodeInfo,
    pub registry: GossipRegistry,
    /// Channel on which inbound messages are delivered to the actor system.
    inbound_tx: mpsc::UnboundedSender<InboundMessage>,
    /// Connection pool: node_id → outbound sender.
    connections: Arc<RwLock<HashMap<NodeId, mpsc::UnboundedSender<Vec<u8>>>>>,
}

impl TransportLayer {
    /// Create the transport and start listening.
    /// Returns `(TransportLayer, inbound_rx)`.
    pub async fn bind(
        node: NodeInfo,
        bind_addr: &str,
    ) -> TransportResult<(Self, mpsc::UnboundedReceiver<InboundMessage>)> {
        let listener = TcpListener::bind(bind_addr).await?;
        let (inbound_tx, inbound_rx) = mpsc::unbounded_channel();
        let registry = GossipRegistry::new(node.clone());
        let connections = Arc::new(RwLock::new(HashMap::new()));

        let transport = Self {
            node: node.clone(),
            registry: registry.clone(),
            inbound_tx: inbound_tx.clone(),
            connections: connections.clone(),
        };

        // Spawn the accept loop
        tokio::spawn(Self::accept_loop(
            listener,
            node.id,
            inbound_tx,
            registry.clone(),
        ));

        info!("TransportLayer listening on {bind_addr} (node {})", node.id);
        Ok((transport, inbound_rx))
    }

    /// Send a remote envelope to `target_node`.
    pub async fn send(&self, target_node: NodeId, envelope: RemoteEnvelope) -> TransportResult<()> {
        let payload = encode_cbor(&envelope)?;

        // Check existing connection
        {
            let conns = self.connections.read().await;
            if let Some(tx) = conns.get(&target_node) {
                let _ = tx.send(payload.clone());
                return Ok(());
            }
        }

        // No existing connection — look up addr in registry and dial
        let peer = self
            .registry
            .get(&target_node)
            .await
            .ok_or(TransportError::NodeUnreachable(target_node))?;

        let (conn_tx, conn_rx) = mpsc::unbounded_channel::<Vec<u8>>();
        {
            self.connections
                .write()
                .await
                .insert(target_node, conn_tx.clone());
        }

        let addr: SocketAddr = peer
            .addr
            .parse()
            .map_err(|_| TransportError::Other(format!("bad addr: {}", peer.addr)))?;

        let conns_clone = self.connections.clone();
        let registry_clone = self.registry.clone();
        tokio::spawn(async move {
            if let Err(e) = outbound_loop(conn_rx, addr, target_node).await {
                warn!("outbound connection to {target_node} failed: {e}");
                conns_clone.write().await.remove(&target_node);
                registry_clone.evict(&target_node).await;
            }
        });

        let _ = conn_tx.send(payload);
        Ok(())
    }

    /// Inject a message directly into the inbound queue without going over the
    /// network (used for local-to-local actor communication on the same node).
    pub fn deliver_local(&self, msg: InboundMessage) -> TransportResult<()> {
        self.inbound_tx
            .send(msg)
            .map_err(|_| TransportError::Other("inbound channel closed".into()))
    }

    /// Broadcast gossip (our NodeInfo) to all known peers.
    pub async fn gossip_broadcast(&self) -> TransportResult<()> {
        let payload = encode_cbor(&GossipMsg::Announce(self.node.clone()))?;
        let peers = self.registry.peers().await;
        for peer in peers {
            let envelope = RemoteEnvelope {
                target_actor: Uuid::nil(),
                sender: None,
                message_type: "__gossip__".into(),
                payload: payload.clone(),
                correlation_id: None,
            };
            let _ = self.send(peer.id, envelope).await;
        }
        Ok(())
    }

    // ── Accept loop ───────────────────────────────────────────────────────────

    async fn accept_loop(
        listener: TcpListener,
        local_node: NodeId,
        inbound_tx: mpsc::UnboundedSender<InboundMessage>,
        registry: GossipRegistry,
    ) {
        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    debug!("accepted connection from {peer_addr}");
                    let tx = inbound_tx.clone();
                    let reg = registry.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_inbound(stream, local_node, tx, reg).await {
                            debug!("inbound connection error: {e}");
                        }
                    });
                }
                Err(e) => {
                    error!("accept error: {e}");
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
            }
        }
    }
}

// ── Connection handlers ───────────────────────────────────────────────────────

async fn handle_inbound(
    mut stream: TcpStream,
    _local_node: NodeId,
    inbound_tx: mpsc::UnboundedSender<InboundMessage>,
    registry: GossipRegistry,
) -> TransportResult<()> {
    // First frame: NodeInfo handshake
    let handshake_data = read_frame(&mut stream).await?;
    let sender_node: NodeInfo = decode_cbor(&handshake_data)?;
    let from_node = sender_node.id;
    registry.merge(sender_node).await;

    // Respond with our NodeInfo
    let our_info = registry.local().clone();
    let resp = encode_cbor(&our_info)?;
    write_frame(&mut stream, &resp).await?;

    loop {
        let frame = match read_frame(&mut stream).await {
            Ok(f) => f,
            Err(_) => break,
        };
        let envelope: RemoteEnvelope = match decode_cbor(&frame) {
            Ok(e) => e,
            Err(e) => {
                warn!("decode error: {e}");
                continue;
            }
        };

        // Handle gossip internally
        if envelope.message_type == "__gossip__" {
            if let Ok(gossip) = decode_cbor::<GossipMsg>(&envelope.payload) {
                match gossip {
                    GossipMsg::Announce(info) => {
                        registry.merge(info).await;
                    }
                }
            }
            continue;
        }

        let _ = inbound_tx.send(InboundMessage {
            from_node,
            envelope,
        });
    }
    Ok(())
}

async fn outbound_loop(
    mut rx: mpsc::UnboundedReceiver<Vec<u8>>,
    addr: SocketAddr,
    _target_node: NodeId,
) -> TransportResult<()> {
    let mut stream = TcpStream::connect(addr).await?;
    debug!("connected to {addr}");

    while let Some(payload) = rx.recv().await {
        write_frame(&mut stream, &payload).await?;
    }
    Ok(())
}

// ── Gossip message ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
enum GossipMsg {
    Announce(NodeInfo),
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cbor_roundtrip() {
        let env = RemoteEnvelope {
            target_actor: Uuid::new_v4(),
            sender: None,
            message_type: "test".into(),
            payload: b"hello".to_vec(),
            correlation_id: None,
        };
        let bytes = encode_cbor(&env).unwrap();
        let decoded: RemoteEnvelope = decode_cbor(&bytes).unwrap();
        assert_eq!(decoded.message_type, "test");
        assert_eq!(decoded.payload, b"hello");
    }

    #[test]
    fn gossip_registry_merge() {
        let local = NodeInfo {
            id: Uuid::new_v4(),
            addr: "127.0.0.1:7001".into(),
            actor_count: 0,
            generation: 1,
        };
        let peer = NodeInfo {
            id: Uuid::new_v4(),
            addr: "127.0.0.1:7002".into(),
            actor_count: 2,
            generation: 1,
        };
        let reg = GossipRegistry::new(local);
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            reg.merge(peer.clone()).await;
            let peers = reg.peers().await;
            assert_eq!(peers.len(), 1);
            assert_eq!(peers[0].addr, peer.addr);
        });
    }

    #[test]
    fn gossip_ignores_stale() {
        let local = NodeInfo {
            id: Uuid::new_v4(),
            addr: "127.0.0.1:7001".into(),
            actor_count: 0,
            generation: 1,
        };
        let peer_id = Uuid::new_v4();
        let peer_v2 = NodeInfo {
            id: peer_id,
            addr: "127.0.0.1:7002".into(),
            actor_count: 5,
            generation: 2,
        };
        let peer_v1 = NodeInfo {
            id: peer_id,
            addr: "127.0.0.1:7002".into(),
            actor_count: 1,
            generation: 1,
        };
        let reg = GossipRegistry::new(local);
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            reg.merge(peer_v2.clone()).await;
            reg.merge(peer_v1).await; // stale — should not overwrite
            let peers = reg.peers().await;
            assert_eq!(peers[0].actor_count, 5); // v2 preserved
        });
    }
}
