// Media over QUIC (MoQ) -- native QUIC streaming
//
// Real connection setup (an actual Quinn QUIC handshake) is not
// implemented here -- connect() marks the transport connected without
// opening a real socket. What *is* real: byte accounting in
// send_frame/send, and this now implements bmn-common's shared
// `Transport` trait using the real `TransportStats` type (the original
// archived version here redefined a lookalike struct with a jitter_ms
// field TransportStats doesn't actually have and without its
// `connected` field, so it never compiled against the real bmn-common)
// so a MoqTransport is interchangeable with RtmpTransport/EchoTransport
// wherever `dyn Transport` is expected.

use async_trait::async_trait;
use bmn_common::error::{BmnError, BmnResult};
use bmn_common::transport::{Transport, TransportProtocol, TransportStats};

pub struct MoqTransport {
    connected: bool,
    stats: TransportStats,
}

impl MoqTransport {
    pub fn new() -> Self {
        Self {
            connected: false,
            stats: TransportStats::default(),
        }
    }

    /// Convenience alias for [`Transport::send`] with MoQ-flavored naming.
    pub async fn send_frame(&mut self, data: &[u8]) -> BmnResult<()> {
        self.send(data).await
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }
}

impl Default for MoqTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Transport for MoqTransport {
    fn protocol(&self) -> TransportProtocol {
        TransportProtocol::MoQ
    }

    async fn connect(&mut self, url: &str) -> BmnResult<()> {
        tracing::info!("Connecting to MoQ endpoint: {}", url);
        // Quinn QUIC setup would go here.
        self.connected = true;
        self.stats.connected = true;
        Ok(())
    }

    async fn send(&mut self, data: &[u8]) -> BmnResult<()> {
        if !self.connected {
            return Err(BmnError::TransportNotConnected);
        }
        self.stats.bytes_sent += data.len() as u64;
        Ok(())
    }

    async fn disconnect(&mut self) -> BmnResult<()> {
        self.connected = false;
        self.stats.connected = false;
        Ok(())
    }

    fn stats(&self) -> TransportStats {
        self.stats.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_moq_transport_connect() {
        let mut transport = MoqTransport::new();
        assert!(!transport.is_connected());

        transport
            .connect("moq://stream.example.com")
            .await
            .unwrap();
        assert!(transport.is_connected());
        assert!(transport.stats().connected);
    }

    #[tokio::test]
    async fn test_moq_transport_send_requires_connection() {
        let mut transport = MoqTransport::new();
        assert!(matches!(
            transport.send_frame(b"data").await,
            Err(BmnError::TransportNotConnected)
        ));
    }

    #[tokio::test]
    async fn test_moq_transport_tracks_real_bytes_sent() {
        let mut transport = MoqTransport::new();
        transport
            .connect("moq://stream.example.com")
            .await
            .unwrap();

        transport.send_frame(&[0u8; 128]).await.unwrap();
        transport.send_frame(&[0u8; 32]).await.unwrap();

        assert_eq!(transport.stats().bytes_sent, 160);
    }
}
