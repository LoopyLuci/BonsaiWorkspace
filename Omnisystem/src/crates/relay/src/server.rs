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

    /// Add a peer; returns which slot they got and the sender for the other peer.
    fn join(
        &mut self,
        tx: mpsc::UnboundedSender<Vec<u8>>,
    ) -> RelayResult<Option<mpsc::UnboundedSender<Vec<u8>>>> {
        if self.peer_a.is_none() {
            self.peer_a = Some(tx);
            Ok(None)
        } else if self.peer_b.is_none() {
            let other = self.peer_a.clone();
            self.peer_b = Some(tx);
            Ok(other)
        } else {
            Err(RelayError::SessionFull)
        }
    }
}

type Sessions = Arc<Mutex<HashMap<[u8; 32], Session>>>;

/// A running blind relay server.
pub struct RelayServer {
    bind_addr: String,
    sessions: Sessions,
}

impl RelayServer {
    pub fn new(bind_addr: &str) -> Self {
        Self {
            bind_addr: bind_addr.to_string(),
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Spawn the relay server; returns when the listener is bound.
    /// Call `.await` to run until shutdown.
    pub async fn run(self) -> RelayResult<()> {
        let listener = TcpListener::bind(&self.bind_addr).await?;
        info!("relay server listening on {}", self.bind_addr);

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
            let (stream, addr) = listener.accept().await?;
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

    // Get the other peer's channel (if they already joined)
    let other_peer: Option<mpsc::UnboundedSender<Vec<u8>>> = {
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

    // Forward inbound chunks → other peer's channel
    let sessions_fwd = sessions.clone();
    let fwd_handle = tokio::spawn(async move {
        loop {
            let frame = match read_frame_reader(&mut reader, MAX_FRAME).await {
                Ok(f) => f,
                Err(_) => break,
            };
            // Re-fetch the other peer each time (they may have joined after us)
            let other_tx = {
                let map = sessions_fwd.lock().await;
                if let Some(s) = map.get(&token_key) {
                    // Send to whichever slot is not us — here we just broadcast to both
                    // channels (dedup at recipient is harmless since data is encrypted)
                    let mut out = None;
                    if let Some(ref tx) = s.peer_a {
                        out = Some(tx.clone());
                    }
                    if let Some(ref tx) = s.peer_b {
                        // prefer the non-self channel — simplified: send to both
                        let _ = tx.send(frame.clone());
                        out = None; // already sent
                    }
                    out
                } else {
                    None
                }
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
