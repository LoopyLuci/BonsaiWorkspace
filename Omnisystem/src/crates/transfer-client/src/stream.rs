use crate::error::TransferClientError;
use crate::framing::FrameCodec;

/// A bidirectional, length‑delimited stream to a remote peer.
///
/// Backed by whatever transport was negotiated by the client
/// (relay raw‑frame channel, HTTP fallback polling, or a native P2P lane).
pub struct PeerStream {
    pub name: String,
    pub peer_id: String,
    codec: FrameCodec,
    inner: StreamInner,
}

enum StreamInner {
    /// Relay raw‑frame channel (read/write halves)
    Relay {
        read_half: Box<dyn StreamReadHalf>,
        write_half: Box<dyn StreamWriteHalf>,
    },
    /// HTTP fallback (request/response exchange via bridge)
    HttpFallback {
        base_url: String,
        client: reqwest::Client,
    },
}

// ---- Trait aliases for relay channel halves ----
#[allow(private_interfaces)]
pub trait StreamReadHalf: tokio::io::AsyncRead + Unpin + Send {}
impl<T: tokio::io::AsyncRead + Unpin + Send> StreamReadHalf for T {}

#[allow(private_interfaces)]
pub trait StreamWriteHalf: tokio::io::AsyncWrite + Unpin + Send {}
impl<T: tokio::io::AsyncWrite + Unpin + Send> StreamWriteHalf for T {}

impl PeerStream {
    /// Create a stream backed by a relay raw‑frame channel.
    #[allow(private_interfaces)]
    pub fn new_relay(
        name: &str,
        peer_id: &str,
        read_half: Box<dyn StreamReadHalf>,
        write_half: Box<dyn StreamWriteHalf>,
    ) -> Self {
        Self {
            name: name.to_string(),
            peer_id: peer_id.to_string(),
            codec: FrameCodec::new(),
            inner: StreamInner::Relay { read_half, write_half },
        }
    }

    /// Create a stream backed by HTTP fallback (request/response model).
    pub fn new_http_fallback(name: &str, peer_id: &str, base_url: &str) -> Self {
        Self {
            name: name.to_string(),
            peer_id: peer_id.to_string(),
            codec: FrameCodec::new(),
            inner: StreamInner::HttpFallback {
                base_url: base_url.to_string(),
                client: reqwest::Client::new(),
            },
        }
    }

    /// Send raw bytes over the stream.
    pub async fn send(&mut self, data: &[u8]) -> Result<(), TransferClientError> {
        match &mut self.inner {
            StreamInner::Relay { write_half, .. } => {
                self.codec
                    .write_frame(write_half, data)
                    .await
                    .map_err(|e| TransferClientError::SendError {
                        name: self.name.clone(),
                        reason: e.to_string(),
                    })?;
                Ok(())
            }
            StreamInner::HttpFallback { base_url, client } => {
                let url = format!("{}/td/{}/{}", base_url, self.peer_id, self.name);
                client
                    .post(&url)
                    .body(data.to_vec())
                    .send()
                    .await
                    .map_err(|e| TransferClientError::SendError {
                        name: self.name.clone(),
                        reason: e.to_string(),
                    })?;
                Ok(())
            }
        }
    }

    /// Receive raw bytes from the stream.
    pub async fn recv(&mut self) -> Result<Vec<u8>, TransferClientError> {
        match &mut self.inner {
            StreamInner::Relay { read_half, .. } => {
                self.codec
                    .read_frame(read_half)
                    .await
                    .map_err(|e| TransferClientError::ReceiveError {
                        name: self.name.clone(),
                        reason: e.to_string(),
                    })
            }
            StreamInner::HttpFallback { base_url, client } => {
                let url = format!("{}/td/{}/{}/poll", base_url, self.peer_id, self.name);
                let resp = client
                    .get(&url)
                    .send()
                    .await
                    .map_err(|e| TransferClientError::ReceiveError {
                        name: self.name.clone(),
                        reason: e.to_string(),
                    })?;
                let bytes = resp
                    .bytes()
                    .await
                    .map_err(|e| TransferClientError::ReceiveError {
                        name: self.name.clone(),
                        reason: e.to_string(),
                    })?;
                Ok(bytes.to_vec())
            }
        }
    }

    /// Full request‑response exchange (send then recv).
    pub async fn exchange(&mut self, request: &[u8]) -> Result<Vec<u8>, TransferClientError> {
        self.send(request).await?;
        self.recv().await
    }
}
