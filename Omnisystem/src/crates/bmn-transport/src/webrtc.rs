// WebRTC transport for interactive streaming
//
// Real ICE/DTLS negotiation is not implemented -- setup_peer() marks the
// transport connected and returns a placeholder SDP answer rather than
// running a real PeerConnection handshake. What *is* real: byte
// accounting in send_frame/send, and this now implements bmn-common's
// shared `Transport` trait using the real `TransportStats` type (the
// original archived version redefined a lookalike struct with a
// jitter_ms field TransportStats doesn't have and without its
// `connected` field, so it never compiled against the real bmn-common).

use async_trait::async_trait;
use bmn_common::error::{BmnError, BmnResult};
use bmn_common::transport::{Transport, TransportProtocol, TransportStats};

pub struct WebRTCTransport {
    connected: bool,
    stats: TransportStats,
}

impl WebRTCTransport {
    pub fn new() -> Self {
        Self {
            connected: false,
            stats: TransportStats {
                latency_ms: 200.0, // typical WebRTC latency, until measured
                ..TransportStats::default()
            },
        }
    }

    /// WebRTC-specific offer/answer negotiation. Marks the transport
    /// connected and returns the SDP answer.
    pub async fn setup_peer(&mut self, _sdp_offer: &str) -> BmnResult<String> {
        tracing::info!("Setting up WebRTC peer connection");
        // WebRTC PeerConnection setup would go here.
        self.connected = true;
        self.stats.connected = true;
        Ok("sdp_answer".into())
    }

    /// Convenience alias for [`Transport::send`] with WebRTC-flavored naming.
    pub async fn send_frame(&mut self, data: &[u8]) -> BmnResult<()> {
        self.send(data).await
    }
}

impl Default for WebRTCTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Transport for WebRTCTransport {
    fn protocol(&self) -> TransportProtocol {
        TransportProtocol::WebRTC
    }

    async fn connect(&mut self, url: &str) -> BmnResult<()> {
        // For WebRTC, `url` is treated as the remote SDP offer.
        self.setup_peer(url).await.map(|_| ())
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
    async fn test_webrtc_setup_peer() {
        let mut transport = WebRTCTransport::new();
        let answer = transport.setup_peer("sdp_offer_string").await.unwrap();
        assert!(!answer.is_empty());
        assert!(transport.stats().connected);
    }

    #[tokio::test]
    async fn test_webrtc_send_requires_connection() {
        let mut transport = WebRTCTransport::new();
        assert!(matches!(
            transport.send_frame(b"data").await,
            Err(BmnError::TransportNotConnected)
        ));
    }

    #[tokio::test]
    async fn test_webrtc_tracks_real_bytes_sent() {
        let mut transport = WebRTCTransport::new();
        transport.setup_peer("offer").await.unwrap();
        transport.send_frame(&[0u8; 64]).await.unwrap();
        assert_eq!(transport.stats().bytes_sent, 64);
    }
}
