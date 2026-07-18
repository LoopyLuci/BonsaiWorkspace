// Frame definitions for video and audio data

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use bytes::Bytes;

/// Video frame formats supported by BMN
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PixelFormat {
    RGBA,      // 32-bit RGBA
    BGRA,      // 32-bit BGRA
    YUV420,    // 8-bit YUV 4:2:0 (planar)
    YUV422,    // 8-bit YUV 4:2:2
    NV12,      // 12-bit NV12 (semi-planar)
    P010,      // 10-bit P010 (semi-planar)
}

#[derive(Clone)]
pub struct VideoFrame {
    pub timestamp: u64,           // microseconds since epoch
    pub width: u32,
    pub height: u32,
    pub format: PixelFormat,
    pub data: Arc<Bytes>,         // Immutable frame data
    pub stride: u32,              // Bytes per row
    pub is_keyframe: bool,
}

impl VideoFrame {
    pub fn new(
        timestamp: u64,
        width: u32,
        height: u32,
        format: PixelFormat,
        data: Vec<u8>,
        stride: u32,
        is_keyframe: bool,
    ) -> Self {
        Self {
            timestamp,
            width,
            height,
            format,
            data: Arc::new(Bytes::from(data)),
            stride,
            is_keyframe,
        }
    }

    pub fn size_bytes(&self) -> usize {
        self.data.len()
    }
}

impl std::fmt::Debug for VideoFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VideoFrame")
            .field("timestamp", &self.timestamp)
            .field("width", &self.width)
            .field("height", &self.height)
            .field("format", &self.format)
            .field("size_bytes", &self.size_bytes())
            .field("stride", &self.stride)
            .field("is_keyframe", &self.is_keyframe)
            .finish()
    }
}

/// Audio sample formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioFormat {
    S16,   // 16-bit signed PCM
    S32,   // 32-bit signed PCM
    F32,   // 32-bit floating point
}

/// Audio frame (sample batch)
#[derive(Clone)]
pub struct AudioFrame {
    pub timestamp: u64,     // microseconds since epoch
    pub sample_rate: u32,   // Hz (e.g., 48000)
    pub channels: u32,      // mono=1, stereo=2, etc.
    pub format: AudioFormat,
    pub data: Arc<Bytes>,   // Immutable sample data
}

impl AudioFrame {
    pub fn new(
        timestamp: u64,
        sample_rate: u32,
        channels: u32,
        format: AudioFormat,
        data: Vec<u8>,
    ) -> Self {
        Self {
            timestamp,
            sample_rate,
            channels,
            format,
            data: Arc::new(Bytes::from(data)),
        }
    }

    pub fn sample_count(&self) -> usize {
        let bytes_per_sample = match self.format {
            AudioFormat::S16 => 2,
            AudioFormat::S32 => 4,
            AudioFormat::F32 => 4,
        };
        self.data.len() / (bytes_per_sample * self.channels as usize)
    }
}

impl std::fmt::Debug for AudioFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioFrame")
            .field("timestamp", &self.timestamp)
            .field("sample_rate", &self.sample_rate)
            .field("channels", &self.channels)
            .field("format", &self.format)
            .field("sample_count", &self.sample_count())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_video_frame_creation() {
        let data = vec![0u8; 1920 * 1080 * 4];
        let frame = VideoFrame::new(
            1000,
            1920,
            1080,
            PixelFormat::RGBA,
            data.clone(),
            1920 * 4,
            true,
        );

        assert_eq!(frame.width, 1920);
        assert_eq!(frame.height, 1080);
        assert_eq!(frame.size_bytes(), 1920 * 1080 * 4);
        assert!(frame.is_keyframe);
    }

    #[test]
    fn test_audio_frame_creation() {
        let data = vec![0u8; 48000 * 2 * 2]; // 1 second of stereo 16-bit audio at 48kHz
        let frame = AudioFrame::new(1000, 48000, 2, AudioFormat::S16, data);

        assert_eq!(frame.sample_rate, 48000);
        assert_eq!(frame.channels, 2);
        assert_eq!(frame.sample_count(), 48000);
    }
}
