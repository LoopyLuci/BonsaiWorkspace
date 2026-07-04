// Transport protocols and abstractions

use crate::error::BmnError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransportProtocol {
    RTMP,   // Legacy RTMP for Twitch/YouTube
    SRT,    // Secure Reliable Transport (low latency)
    RIST,   // Reliable Internet Stream Transport
    WebRTC, // WebRTC for P2P/low latency interactive
    MoQ,    // Media over QUIC (next-gen)
    Echo,   // Bonsai Echo mesh (P2P)
}

/// Trait for transport implementations
#[async_trait]
pub trait Transport: Send + Sync {
    /// Get transport protocol
    fn protocol(&self) -> TransportProtocol;

    /// Connect to remote endpoint
    async fn connect(&mut self, url: &str) -> Result<(), BmnError>;

    /// Send data chunk
    async fn send(&mut self, data: &[u8]) -> Result<(), BmnError>;

    /// Disconnect
    async fn disconnect(&mut self) -> Result<(), BmnError>;

    /// Get transport statistics
    fn stats(&self) -> TransportStats;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportStats {
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packets_lost: u64,
    pub latency_ms: f32,
    pub bandwidth_mbps: f32,
    pub connected: bool,
}

impl Default for TransportStats {
    fn default() -> Self {
        Self {
            bytes_sent: 0,
            bytes_received: 0,
            packets_lost: 0,
            latency_ms: 0.0,
            bandwidth_mbps: 0.0,
            connected: false,
        }
    }
}

/// RTMP Transport
pub struct RtmpTransport {
    url: Option<String>,
    connected: bool,
    stats: TransportStats,
}

impl RtmpTransport {
    pub fn new() -> Self {
        Self {
            url: None,
            connected: false,
            stats: TransportStats::default(),
        }
    }
}

#[async_trait]
impl Transport for RtmpTransport {
    fn protocol(&self) -> TransportProtocol {
        TransportProtocol::RTMP
    }

    async fn connect(&mut self, url: &str) -> Result<(), BmnError> {
        self.url = Some(url.to_string());
        self.connected = true;
        self.stats.connected = true;
        Ok(())
    }

    async fn send(&mut self, data: &[u8]) -> Result<(), BmnError> {
        if !self.connected {
            return Err(BmnError::TransportNotConnected);
        }
        self.stats.bytes_sent += data.len() as u64;
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), BmnError> {
        self.connected = false;
        self.stats.connected = false;
        Ok(())
    }

    fn stats(&self) -> TransportStats {
        self.stats.clone()
    }
}

impl Default for RtmpTransport {
    fn default() -> Self {
        Self::new()
    }
}

/// Echo P2P Transport (Bonsai native)
pub struct EchoTransport {
    connected: bool,
    stats: TransportStats,
}

impl EchoTransport {
    pub fn new() -> Self {
        Self {
            connected: false,
            stats: TransportStats::default(),
        }
    }
}

#[async_trait]
impl Transport for EchoTransport {
    fn protocol(&self) -> TransportProtocol {
        TransportProtocol::Echo
    }

    async fn connect(&mut self, url: &str) -> Result<(), BmnError> {
        // In real implementation, this would connect to Echo fabric
        self.connected = true;
        self.stats.connected = true;
        Ok(())
    }

    async fn send(&mut self, data: &[u8]) -> Result<(), BmnError> {
        if !self.connected {
            return Err(BmnError::TransportNotConnected);
        }
        self.stats.bytes_sent += data.len() as u64;
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<(), BmnError> {
        self.connected = false;
        self.stats.connected = false;
        Ok(())
    }

    fn stats(&self) -> TransportStats {
        self.stats.clone()
    }
}

impl Default for EchoTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rtmp_transport() {
        let mut transport = RtmpTransport::new();
        assert!(!transport.stats().connected);

        transport.connect("rtmp://twitch.tv/live/key").await.unwrap();
        assert!(transport.stats().connected);

        let data = b"test data";
        transport.send(data).await.unwrap();
        assert_eq!(transport.stats().bytes_sent, 9);

        transport.disconnect().await.unwrap();
        assert!(!transport.stats().connected);
    }

    #[tokio::test]
    async fn test_echo_transport() {
        let mut transport = EchoTransport::new();
        transport.connect("echo://bonsai").await.unwrap();
        assert!(transport.stats().connected);

        assert_eq!(transport.protocol(), TransportProtocol::Echo);
    }
}
