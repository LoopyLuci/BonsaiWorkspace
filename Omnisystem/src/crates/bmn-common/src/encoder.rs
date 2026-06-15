// Encoder traits and configurations

use crate::{VideoFrame, error::BmnError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EncoderType {
    NVENC,        // Nvidia hardware encoder
    AMF,          // AMD hardware encoder
    QSV,          // Intel Quick Sync Video
    VideoToolbox, // Apple
    VAAPI,        // Linux
    X264,         // Software fallback
    X265,         // Software H.265
    VP9,          // Software VP9
    AV1,          // Software AV1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncodedPacket {
    pub timestamp: u64,
    pub data: Vec<u8>,
    pub is_keyframe: bool,
    pub duration: u32, // microseconds
}

/// Trait for all encoders
#[async_trait]
pub trait Encoder: Send + Sync {
    /// Get encoder type
    fn encoder_type(&self) -> EncoderType;

    /// Initialize encoder
    async fn init(&mut self) -> Result<(), BmnError>;

    /// Encode a frame
    async fn encode_frame(&mut self, frame: &VideoFrame) -> Result<Option<Vec<EncodedPacket>>, BmnError>;

    /// Flush any pending frames
    async fn flush(&mut self) -> Result<Option<Vec<EncodedPacket>>, BmnError>;

    /// Get encoder statistics
    fn stats(&self) -> EncoderStats;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncoderStats {
    pub frames_encoded: u64,
    pub frames_dropped: u64,
    pub total_bytes: u64,
    pub avg_bitrate: f32,
    pub avg_latency_ms: f32,
}

impl Default for EncoderStats {
    fn default() -> Self {
        Self {
            frames_encoded: 0,
            frames_dropped: 0,
            total_bytes: 0,
            avg_bitrate: 0.0,
            avg_latency_ms: 0.0,
        }
    }
}

/// Hardware encoder pool for parallel encoding
pub struct EncoderPool {
    encoders: Vec<Box<dyn Encoder>>,
    active_index: usize,
}

impl EncoderPool {
    pub fn new() -> Self {
        Self {
            encoders: Vec::new(),
            active_index: 0,
        }
    }

    pub fn add_encoder(&mut self, encoder: Box<dyn Encoder>) {
        self.encoders.push(encoder);
    }

    pub fn count(&self) -> usize {
        self.encoders.len()
    }

    pub fn get_active(&mut self) -> Option<&mut dyn Encoder> {
        if self.encoders.is_empty() {
            return None;
        }

        let encoder = &mut self.encoders[self.active_index];
        Some(encoder.as_mut())
    }

    pub fn round_robin(&mut self) {
        if !self.encoders.is_empty() {
            self.active_index = (self.active_index + 1) % self.encoders.len();
        }
    }
}

impl Default for EncoderPool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encoder_pool_round_robin() {
        let mut pool = EncoderPool::new();
        assert_eq!(pool.count(), 0);

        // Add dummy encoders (in real code, these would be actual encoder instances)
        // For now, just test the pooling logic
        assert_eq!(pool.active_index, 0);

        pool.round_robin();
        // Should stay at 0 since pool is empty
        assert_eq!(pool.active_index, 0);
    }
}
