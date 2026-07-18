//! Blind relay server — forwards encrypted chunks between two peers.
//!
//! Sessions are keyed by `RelayToken`. Each session has two slots (A and B).
//! When both slots are occupied the server acts as a byte-level forwarder,
//! never inspecting payload content.

use crate::error::{RelayError, RelayResult};
use crate::token::RegisterRequest;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex};
use tracing::{info, warn};

const SESSION_TTL: Duration = Duration::from_secs(300); // 5 minutes
const MAX_FRAME: usize = 18 * 1024 * 1024; // 18 MiB

/// Which of the two slots in a [`Session`] a connection occupies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Slot {
    A,
    B,
}

struct Session {
    /// Channel to the first peer that registered.
    peer_a: Option<mpsc::UnboundedSender<Vec<u8>>>,
    /// Channel to the second peer.
    peer_b: Option<mpsc::UnboundedSender<Vec<u8>>>,
    created_at: Instant,
}

impl Session {
    fn new() -> Self {
        Self {
            peer_a: None,
            peer_b: None,
            created_at: Instant::now(),
        }
    }

    fn is_expired(&self) -> bool {
        self.created_at.elapsed() > SESSION_TTL
    }

    fn is_full(&self) -> bool {
        self.peer_a.is_some() && self.peer_b.is_some()
    }

    /// Channel belonging to the slot opposite `slot`.
    fn other(&self, slot: Slot) -> Option<mpsc::UnboundedSender<Vec<u8>>> {
        match slot {
            Slot::A => self.peer_b.clone(),
            Slot::B => self.peer_a.clone(),
        }
    }

    /// Add a peer; returns which slot they got and the sender for the other peer.
    fn join(
        &mut self,
        tx: mpsc::UnboundedSender<Vec<u8>>,
    ) -> RelayResult<(Slot, Option<mpsc::UnboundedSender<Vec<u8>>>)> {
        if self.peer_a.is_none() {
            self.peer_a = Some(tx);
            Ok((Slot::A, self.peer_b.clone()))
        } else if self.peer_b.is_none() {
            let other = self.peer_a.clone();
            self.peer_b = Some(tx);
            Ok((Slot::B, other))
        } else {
            Err(RelayError::SessionFull)
        }
    }
}

type Sessions = Arc<Mutex<HashMap<[u8; 32], Session>>>;

/// A running blind relay server.
pub struct RelayServer {
    listener: TcpListener,
    sessions: Sessions,
}

impl RelayServer {
    /// Bind the listening socket. Binding is split from [`RelayServer::run`]
    /// so callers (and tests) can observe the actual bound address --
    /// useful when binding to port 0 for an OS-assigned port.
    pub async fn bind(bind_addr: &str) -> RelayResult<Self> {
        let listener = TcpListener::bind(bind_addr).await?;
        Ok(Self {
            listener,
            sessions: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// The address this server is actually listening on.
    pub fn local_addr(&self) -> std::io::Result<std::net::SocketAddr> {
        self.listener.local_addr()
    }

    /// Run until shutdown, accepting and relaying connections.
    pub async fn run(self) -> RelayResult<()> {
        info!("relay server listening on {}", self.listener.local_addr()?);

        // Spawn a background task to evict expired sessions
        {
            let sessions = self.sessions.clone();
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(Duration::from_secs(60)).await;
                    let mut s = sessions.lock().await;
                    s.retain(|_, session| !session.is_expired());
                }
            });
        }

        loop {
            let (stream, addr) = self.listener.accept().await?;
            info!("relay: new connection from {addr}");
            let sessions = self.sessions.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_connection(stream, sessions).await {
                    warn!("relay connection error from {addr}: {e}");
                }
            });
        }
    }
}

async fn handle_connection(mut stream: TcpStream, sessions: Sessions) -> RelayResult<()> {
    // First frame: RegisterRequest JSON
    let reg_frame = read_frame(&mut stream).await?;
    let reg: RegisterRequest = serde_json::from_slice(&reg_frame)?;

    if !reg.verify() {
        return Err(RelayError::PowFailed);
    }

    let token_key = reg.token.0;

    // Outbound channel for this peer
    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<Vec<u8>>();

    // Get our slot and the other peer's channel (if they already joined).
    let (self_slot, other_peer): (Slot, Option<mpsc::UnboundedSender<Vec<u8>>>) = {
        let mut map = sessions.lock().await;
        let session = map.entry(token_key).or_insert_with(Session::new);
        if session.is_expired() {
            *session = Session::new();
        }
        // Reject a third peer — sessions only allow two participants.
        if session.is_full() {
            return Err(RelayError::SessionFull);
        }
        session.join(out_tx.clone())?
    };

    // ACK the connection
    write_frame(&mut stream, b"OK").await?;

    // Notify the other peer that we're ready, if they were waiting
    if let Some(ref other_tx) = other_peer {
        let _ = other_tx.send(b"PEER_READY".to_vec());
    }

    let (mut reader, mut writer) = stream.into_split();

    // Forward inbound chunks → the *other* slot's channel only (never our own).
    let sessions_fwd = sessions.clone();
    let fwd_handle = tokio::spawn(async move {
        loop {
            let frame = match read_frame_reader(&mut reader, MAX_FRAME).await {
                Ok(f) => f,
                Err(_) => break,
            };
            // Re-fetch the other peer each time (they may have joined after us).
            let other_tx = {
                let map = sessions_fwd.lock().await;
                map.get(&token_key).and_then(|s| s.other(self_slot))
            };
            if let Some(tx) = other_tx {
                let _ = tx.send(frame);
            }
        }
    });

    // Forward outbound queue → TCP writer
    while let Some(data) = out_rx.recv().await {
        if write_frame_writer(&mut writer, &data).await.is_err() {
            break;
        }
    }

    fwd_handle.abort();
    Ok(())
}

// ── Framing helpers ───────────────────────────────────────────────────────────

async fn read_frame(stream: &mut TcpStream) -> RelayResult<Vec<u8>> {
    let len = stream.read_u32().await? as usize;
    if len > MAX_FRAME {
        return Err(RelayError::FrameTooLarge(len));
    }
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).await?;
    Ok(buf)
}

async fn read_frame_reader<R: AsyncReadExt + Unpin>(
    reader: &mut R,
    max: usize,
) -> RelayResult<Vec<u8>> {
    let len = reader.read_u32().await? as usize;
    if len > max {
        return Err(RelayError::FrameTooLarge(len));
    }
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf).await?;
    Ok(buf)
}

async fn write_frame(stream: &mut TcpStream, data: &[u8]) -> RelayResult<()> {
    stream.write_u32(data.len() as u32).await?;
    stream.write_all(data).await?;
    Ok(())
}

async fn write_frame_writer<W: AsyncWriteExt + Unpin>(
    writer: &mut W,
    data: &[u8],
) -> RelayResult<()> {
    writer.write_u32(data.len() as u32).await?;
    writer.write_all(data).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token::RelayToken;

    #[test]
    fn first_joiner_gets_slot_a_with_no_peer_yet() {
        let mut session = Session::new();
        let (tx, _rx) = mpsc::unbounded_channel();
        let (slot, other) = session.join(tx).unwrap();
        assert_eq!(slot, Slot::A);
        assert!(other.is_none());
    }

    #[test]
    fn second_joiner_gets_slot_b_and_the_first_peers_channel() {
        let mut session = Session::new();
        let (tx_a, _rx_a) = mpsc::unbounded_channel();
        let (tx_b, _rx_b) = mpsc::unbounded_channel();
        session.join(tx_a).unwrap();
        let (slot, other) = session.join(tx_b).unwrap();
        assert_eq!(slot, Slot::B);
        assert!(other.is_some());
    }

    #[test]
    fn third_joiner_is_rejected() {
        let mut session = Session::new();
        let (tx_a, _rx_a) = mpsc::unbounded_channel();
        let (tx_b, _rx_b) = mpsc::unbounded_channel();
        let (tx_c, _rx_c) = mpsc::unbounded_channel();
        session.join(tx_a).unwrap();
        session.join(tx_b).unwrap();
        assert!(matches!(session.join(tx_c), Err(RelayError::SessionFull)));
    }

    #[test]
    fn other_never_returns_the_callers_own_channel() {
        // This is the regression test for the forwarding bug where a lone
        // peer (slot A, no slot B yet) had frames echoed back to itself.
        let mut session = Session::new();
        let (tx_a, _rx_a) = mpsc::unbounded_channel();
        session.join(tx_a).unwrap();

        // Only slot A is occupied; slot B's peer shouldn't exist yet, so
        // asking for "the other side of A" must be None, not A's own channel.
        assert!(session.other(Slot::A).is_none());
    }

    #[test]
    fn other_resolves_to_the_opposite_slot_once_both_joined() {
        let mut session = Session::new();
        let (tx_a, mut rx_a) = mpsc::unbounded_channel();
        let (tx_b, mut rx_b) = mpsc::unbounded_channel();
        session.join(tx_a).unwrap();
        session.join(tx_b).unwrap();

        session.other(Slot::A).unwrap().send(b"to-b".to_vec()).unwrap();
        session.other(Slot::B).unwrap().send(b"to-a".to_vec()).unwrap();

        assert_eq!(rx_b.try_recv().unwrap(), b"to-b");
        assert_eq!(rx_a.try_recv().unwrap(), b"to-a");
    }

    /// End-to-end: two TCP clients register with the same token against a
    /// real bound RelayServer; a frame sent by peer A must arrive at peer B
    /// (and vice versa) and must never be echoed back to its own sender.
    #[tokio::test]
    async fn relays_frames_between_exactly_two_peers() {
        let server = RelayServer::bind("127.0.0.1:0").await.unwrap();
        let addr = server.local_addr().unwrap();
        tokio::spawn(server.run());

        let token = RelayToken::random();

        let mut client_a = TcpStream::connect(addr).await.unwrap();
        register(&mut client_a, token.clone()).await;

        let mut client_b = TcpStream::connect(addr).await.unwrap();
        register(&mut client_b, token.clone()).await;

        // Peer A (the first to register) gets a PEER_READY notification
        // once peer B joins; drain it before expecting real data.
        let ready = read_frame(&mut client_a).await.unwrap();
        assert_eq!(ready, b"PEER_READY");

        // A -> B: must arrive at B, and B must be the only recipient.
        write_frame(&mut client_a, b"hello from A").await.unwrap();
        let received = read_frame(&mut client_b).await.unwrap();
        assert_eq!(received, b"hello from A");

        // B -> A: must arrive at A, and A must be the only recipient
        // (this is the regression case for the self-echo/no-op-forwarding bug).
        write_frame(&mut client_b, b"hello from B").await.unwrap();
        let received = read_frame(&mut client_a).await.unwrap();
        assert_eq!(received, b"hello from B");
    }

    async fn register(stream: &mut TcpStream, token: RelayToken) {
        let req = RegisterRequest::mine(token);
        let bytes = serde_json::to_vec(&req).unwrap();
        write_frame(stream, &bytes).await.unwrap();
        let ack = read_frame(stream).await.unwrap();
        assert_eq!(ack, b"OK");
    }
}
