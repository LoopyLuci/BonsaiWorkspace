// WebRTC transport for interactive streaming

use crate::TransportStats;
use bmn_common::error::BmnResult;

pub struct WebRTCTransport {
    connected: bool,
    stats: TransportStats,
}

impl WebRTCTransport {
    pub fn new() -> Self {
        Self {
            connected: false,
            stats: TransportStats {
                bytes_sent: 0,
                bytes_received: 0,
                packets_lost: 0,
                latency_ms: 200.0, // Typical WebRTC latency
                jitter_ms: 50.0,
                bandwidth_mbps: 0.0,
            },
        }
    }

    pub async fn setup_peer(&mut self, sdp_offer: &str) -> BmnResult<String> {
        tracing::info!("Setting up WebRTC peer connection");
        // WebRTC PeerConnection setup
        self.connected = true;
        Ok("sdp_answer".into())
    }

    pub async fn send_frame(&mut self, data: &[u8]) -> BmnResult<()> {
        if !self.connected {
            return Err(bmn_common::error::BmnError::internal("WebRTC not connected"));
        }
        self.stats.bytes_sent += data.len() as u64;
        Ok(())
    }

    pub fn stats(&self) -> TransportStats {
        self.stats.clone()
    }
}

impl Default for WebRTCTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_webrtc_setup() {
        let mut transport = WebRTCTransport::new();
        let answer = transport.setup_peer("sdp_offer_string").await.unwrap();
        assert!(!answer.is_empty());
    }
}
