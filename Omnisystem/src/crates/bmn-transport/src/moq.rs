// Media over QUIC (MoQ) — native QUIC streaming

use crate::TransportStats;
use bmn_common::error::BmnResult;

pub struct MoqTransport {
    connected: bool,
    stats: TransportStats,
}

impl MoqTransport {
    pub fn new() -> Self {
        Self {
            connected: false,
            stats: TransportStats {
                bytes_sent: 0,
                bytes_received: 0,
                packets_lost: 0,
                latency_ms: 0.0,
                jitter_ms: 0.0,
                bandwidth_mbps: 0.0,
            },
        }
    }

    pub async fn connect(&mut self, url: &str) -> BmnResult<()> {
        tracing::info!("Connecting to MoQ endpoint: {}", url);
        // Quinn QUIC setup would go here
        self.connected = true;
        Ok(())
    }

    pub async fn send_frame(&mut self, data: &[u8]) -> BmnResult<()> {
        if !self.connected {
            return Err(bmn_common::error::BmnError::internal("MoQ not connected"));
        }
        self.stats.bytes_sent += data.len() as u64;
        Ok(())
    }

    pub fn stats(&self) -> TransportStats {
        self.stats.clone()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_moq_transport() {
        let mut transport = MoqTransport::new();
        assert!(!transport.is_connected());

        transport.connect("moq://stream.example.com").await.unwrap();
        assert!(transport.is_connected());
    }
}
