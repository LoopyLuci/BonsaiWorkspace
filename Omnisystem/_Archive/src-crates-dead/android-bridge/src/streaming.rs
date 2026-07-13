use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Video codec type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum VideoCodec {
    H264,
    H265,
    VP9,
}

/// Screen frame format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenFrame {
    /// Frame timestamp (microseconds since epoch)
    pub timestamp_us: u64,
    /// Frame sequence number (for reordering)
    pub sequence: u64,
    /// Encoded frame data
    pub frame_data: Vec<u8>,
    /// Is key frame
    pub is_keyframe: bool,
    /// Frame width
    pub width: u32,
    /// Frame height
    pub height: u32,
    /// Frames per second
    pub fps: u8,
    /// Codec used
    pub codec: VideoCodec,
    /// Checksum for integrity
    pub checksum: u32,
}

impl ScreenFrame {
    /// Calculate CRC32 checksum
    pub fn calculate_checksum(&self) -> u32 {
        let mut crc: u32 = 0xffffffff;
        for &byte in &self.frame_data {
            crc ^= byte as u32;
            for _ in 0..8 {
                crc = if (crc & 1) != 0 {
                    (crc >> 1) ^ 0xedb88320
                } else {
                    crc >> 1
                };
            }
        }
        crc ^ 0xffffffff
    }

    /// Verify checksum
    pub fn verify_checksum(&self) -> bool {
        self.checksum == self.calculate_checksum()
    }
}

/// Adaptive bitrate configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BitrateConfig {
    /// Target bitrate in kbps
    pub target_kbps: u32,
    /// Minimum bitrate in kbps
    pub min_kbps: u32,
    /// Maximum bitrate in kbps
    pub max_kbps: u32,
    /// Enable adaptive bitrate
    pub adaptive: bool,
    /// Latency target (ms)
    pub latency_target_ms: u32,
}

impl Default for BitrateConfig {
    fn default() -> Self {
        Self {
            target_kbps: 5000,
            min_kbps: 1000,
            max_kbps: 20000,
            adaptive: true,
            latency_target_ms: 50,
        }
    }
}

/// Network condition metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMetrics {
    /// Current bandwidth estimate (kbps)
    pub estimated_bandwidth_kbps: u32,
    /// Packet loss rate (0.0 - 1.0)
    pub packet_loss_rate: f32,
    /// Round-trip latency (ms)
    pub rtt_ms: u32,
    /// Jitter (ms)
    pub jitter_ms: u32,
}

/// Screen stream controller
pub struct ScreenStreamer {
    /// Current configuration
    config: Arc<RwLock<BitrateConfig>>,
    /// Network metrics
    network_metrics: Arc<RwLock<NetworkMetrics>>,
    /// Current codec
    codec: VideoCodec,
    /// Frame queue
    frame_queue: tokio::sync::mpsc::UnboundedSender<ScreenFrame>,
    /// Streaming state
    is_streaming: Arc<RwLock<bool>>,
}

impl ScreenStreamer {
    /// Create new screen streamer
    pub fn new(
        config: BitrateConfig,
        frame_queue: tokio::sync::mpsc::UnboundedSender<ScreenFrame>,
    ) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            network_metrics: Arc::new(RwLock::new(NetworkMetrics {
                estimated_bandwidth_kbps: 5000,
                packet_loss_rate: 0.0,
                rtt_ms: 10,
                jitter_ms: 2,
            })),
            codec: VideoCodec::H265,
            frame_queue,
            is_streaming: Arc::new(RwLock::new(false)),
        }
    }

    /// Start streaming
    pub async fn start(&self) -> Result<()> {
        let mut streaming = self.is_streaming.write().await;
        *streaming = true;
        Ok(())
    }

    /// Stop streaming
    pub async fn stop(&self) -> Result<()> {
        let mut streaming = self.is_streaming.write().await;
        *streaming = false;
        Ok(())
    }

    /// Submit frame for streaming
    pub async fn submit_frame(&self, mut frame: ScreenFrame) -> Result<()> {
        if !*self.is_streaming.read().await {
            return Err(crate::error::Error::StreamingError(
                "Streaming not active".to_string(),
            ));
        }

        // Calculate checksum
        frame.checksum = frame.calculate_checksum();

        self.frame_queue.send(frame).map_err(|e| {
            crate::error::Error::StreamingError(e.to_string())
        })
    }

    /// Update network metrics
    pub async fn update_network_metrics(&self, metrics: NetworkMetrics) -> Result<()> {
        let mut network = self.network_metrics.write().await;
        *network = metrics;

        // Adjust bitrate based on network conditions
        if network.estimated_bandwidth_kbps < 2000 {
            // Poor network - reduce bitrate
            let mut config = self.config.write().await;
            config.target_kbps = (config.target_kbps * 8) / 10;
            config.target_kbps = config.target_kbps.max(config.min_kbps);
        } else if network.estimated_bandwidth_kbps > 10000 {
            // Good network - increase bitrate
            let mut config = self.config.write().await;
            config.target_kbps = (config.target_kbps * 12) / 10;
            config.target_kbps = config.target_kbps.min(config.max_kbps);
        }

        Ok(())
    }

    /// Get current bitrate config
    pub async fn get_config(&self) -> BitrateConfig {
        self.config.read().await.clone()
    }

    /// Get network metrics
    pub async fn get_metrics(&self) -> NetworkMetrics {
        self.network_metrics.read().await.clone()
    }

    /// Set target resolution
    pub async fn set_resolution(&self, width: u32, height: u32) -> Result<()> {
        // Validate resolution is reasonable
        if width == 0 || height == 0 || width > 7680 || height > 4320 {
            return Err(crate::error::Error::StreamingError(
                "Invalid resolution".to_string(),
            ));
        }
        Ok(())
    }

    /// Change codec
    pub fn set_codec(&mut self, codec: VideoCodec) {
        self.codec = codec;
    }

    /// Get current streaming status
    pub async fn is_streaming(&self) -> bool {
        *self.is_streaming.read().await
    }
}

/// WebRTC stream manager (for peer-to-peer streaming)
pub struct WebRTCStream {
    /// Peer connection ID
    pub peer_id: String,
    /// Stream state
    state: Arc<RwLock<StreamState>>,
}

#[derive(Debug, Clone, Copy)]
enum StreamState {
    Idle,
    Negotiating,
    Connected,
    Streaming,
    Failed,
}

impl WebRTCStream {
    /// Create new WebRTC stream
    pub fn new(peer_id: String) -> Self {
        Self {
            peer_id,
            state: Arc::new(RwLock::new(StreamState::Idle)),
        }
    }

    /// Initiate stream negotiation
    pub async fn initiate_offer(&self) -> Result<String> {
        let mut state = self.state.write().await;
        *state = StreamState::Negotiating;

        // In production, would generate actual WebRTC SDP offer
        Ok(format!(
            r#"{{ "type": "offer", "sdp": "v=0\r\no=- ... (truncated)" }}"#
        ))
    }

    /// Handle remote answer
    pub async fn handle_answer(&self, _answer_sdp: &str) -> Result<()> {
        let mut state = self.state.write().await;
        *state = StreamState::Connected;
        Ok(())
    }

    /// Start streaming
    pub async fn start_streaming(&self) -> Result<()> {
        let mut state = self.state.write().await;
        if matches!(*state, StreamState::Connected) {
            *state = StreamState::Streaming;
            Ok(())
        } else {
            Err(crate::error::Error::StreamingError(
                "Not connected".to_string(),
            ))
        }
    }

    /// Get stream state
    pub async fn get_state(&self) -> String {
        let state = self.state.read().await;
        match *state {
            StreamState::Idle => "idle",
            StreamState::Negotiating => "negotiating",
            StreamState::Connected => "connected",
            StreamState::Streaming => "streaming",
            StreamState::Failed => "failed",
        }
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screen_frame_checksum() {
        let frame = ScreenFrame {
            timestamp_us: 1000,
            sequence: 1,
            frame_data: b"test data".to_vec(),
            is_keyframe: true,
            width: 1080,
            height: 2400,
            fps: 60,
            codec: VideoCodec::H265,
            checksum: 0,
        };

        let checksum = frame.calculate_checksum();
        assert_ne!(checksum, 0);
    }

    #[tokio::test]
    async fn test_bitrate_config() {
        let config = BitrateConfig::default();
        assert_eq!(config.target_kbps, 5000);
        assert!(config.adaptive);
    }
}
