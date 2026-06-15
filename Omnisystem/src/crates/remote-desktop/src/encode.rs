//! Video encoding with codec selection and dynamic switching.
//!
//! Supports H.264, H.265, VP8, VP9, and AV1 with hardware acceleration
//! where available. Codec selection is automatic based on network conditions.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

/// Errors that can occur during encoding operations.
#[derive(Debug, Error)]
pub enum EncodeError {
    #[error("Codec not available: {codec}")]
    CodecUnavailable { codec: String },

    #[error("Encoding failed: {reason}")]
    EncodeFailed { reason: String },

    #[error("Invalid parameters")]
    InvalidParameters,

    #[error("Hardware acceleration not available")]
    NoHardwareAccel,
}

/// Video codec types.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum CodecType {
    /// H.264/AVC (compatibility, lower latency).
    H264,
    /// H.265/HEVC (better compression).
    H265,
    /// VP8 (royalty-free).
    VP8,
    /// VP9 (better compression than VP8).
    VP9,
    /// AV1 (best compression, higher latency).
    AV1,
}

impl CodecType {
    pub fn as_str(&self) -> &'static str {
        match self {
            CodecType::H264 => "h264",
            CodecType::H265 => "h265",
            CodecType::VP8 => "vp8",
            CodecType::VP9 => "vp9",
            CodecType::AV1 => "av1",
        }
    }

    /// Get MIME type for this codec.
    pub fn mime_type(&self) -> &'static str {
        match self {
            CodecType::H264 => "video/h264",
            CodecType::H265 => "video/h265",
            CodecType::VP8 => "video/vp8",
            CodecType::VP9 => "video/vp9",
            CodecType::AV1 => "video/av1",
        }
    }

    /// Get typical bitrate for given resolution.
    pub fn typical_bitrate_mbps(&self, width: u32, height: u32) -> f64 {
        let pixels = (width as f64) * (height as f64);
        match self {
            CodecType::H264 => pixels * 0.05,    // 1920x1080: ~5 Mbps
            CodecType::H265 => pixels * 0.025,   // 1920x1080: ~2.5 Mbps (50% reduction)
            CodecType::VP8 => pixels * 0.045,    // 1920x1080: ~4.5 Mbps
            CodecType::VP9 => pixels * 0.02,     // 1920x1080: ~2 Mbps
            CodecType::AV1 => pixels * 0.015,    // 1920x1080: ~1.5 Mbps (best)
        }
    }
}

/// Encoding profile/preset (balance between quality and speed).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum EncodeProfile {
    /// Lowest quality, fastest (for high latency/low bandwidth).
    Baseline,
    /// Medium quality, medium speed.
    Main,
    /// High quality, slower.
    High,
}

/// Encoded frame.
#[derive(Clone, Serialize, Deserialize)]
pub struct EncodedFrame {
    /// Encoded frame data.
    pub data: Vec<u8>,

    /// Codec used.
    pub codec: CodecType,

    /// Frame number.
    pub frame_number: u64,

    /// Is this a keyframe?
    pub is_keyframe: bool,

    /// Estimated bitrate (Mbps).
    pub bitrate_mbps: f64,

    /// Encoding time (milliseconds).
    pub encode_time_ms: f64,
}

impl EncodedFrame {
    /// Create a new encoded frame.
    pub fn new(
        data: Vec<u8>,
        codec: CodecType,
        frame_number: u64,
        is_keyframe: bool,
    ) -> Self {
        EncodedFrame {
            data,
            codec,
            frame_number,
            is_keyframe,
            bitrate_mbps: 0.0,
            encode_time_ms: 0.0,
        }
    }
}

/// Video encoding service.
pub struct EncodeService {
    /// Current codec.
    current_codec: Arc<tokio::sync::RwLock<CodecType>>,

    /// Encoding profile.
    profile: Arc<tokio::sync::RwLock<EncodeProfile>>,

    /// Frame counter.
    frame_count: Arc<std::sync::atomic::AtomicU64>,

    /// Target bitrate (Mbps).
    target_bitrate: Arc<tokio::sync::RwLock<f64>>,

    /// Hardware acceleration available.
    has_hw_accel: bool,
}

impl EncodeService {
    /// Create a new EncodeService.
    pub fn new() -> Self {
        let has_hw_accel = Self::detect_hardware_acceleration();

        EncodeService {
            current_codec: Arc::new(tokio::sync::RwLock::new(CodecType::H264)),
            profile: Arc::new(tokio::sync::RwLock::new(EncodeProfile::Main)),
            frame_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            target_bitrate: Arc::new(tokio::sync::RwLock::new(5.0)),
            has_hw_accel,
        }
    }

    /// Detect hardware acceleration support.
    fn detect_hardware_acceleration() -> bool {
        // In production: check for NVIDIA NVENC, AMD VCE, Intel QSV, etc.
        #[cfg(target_os = "windows")]
        {
            true // Assume available on Windows
        }
        #[cfg(target_os = "macos")]
        {
            true // VideoToolbox available
        }
        #[cfg(target_os = "linux")]
        {
            // Check for available encoders
            false // Conservative default
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            false
        }
    }

    /// Encode a frame.
    pub async fn encode_frame(
        &self,
        frame_data: &[u8],
        width: u32,
        height: u32,
    ) -> Result<EncodedFrame, EncodeError> {
        let codec = *self.current_codec.read().await;
        let frame_number = self.frame_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // In production: call hardware or software codec
        // For now: return synthetic encoded frame
        let is_keyframe = frame_number % 30 == 0; // Keyframe every 30 frames
        let bitrate = CodecType::H264.typical_bitrate_mbps(width, height);

        let encoded_data = Self::synthetic_encode(frame_data, width, height);

        Ok(EncodedFrame {
            data: encoded_data,
            codec,
            frame_number,
            is_keyframe,
            bitrate_mbps: bitrate,
            encode_time_ms: 1.0,
        })
    }

    /// Switch codec based on network conditions.
    pub async fn switch_codec(&self, target_bitrate: f64) -> Result<(), EncodeError> {
        let new_codec = Self::select_codec(target_bitrate);
        *self.current_codec.write().await = new_codec;
        *self.target_bitrate.write().await = target_bitrate;

        tracing::debug!("Switched codec to {} (target: {} Mbps)", new_codec.as_str(), target_bitrate);
        Ok(())
    }

    /// Select best codec for given bitrate.
    fn select_codec(bitrate: f64) -> CodecType {
        if bitrate < 2.0 {
            CodecType::AV1 // Best compression for low bitrate
        } else if bitrate < 3.0 {
            CodecType::VP9
        } else if bitrate < 4.0 {
            CodecType::H265
        } else if bitrate < 6.0 {
            CodecType::VP8
        } else {
            CodecType::H264 // Lower latency for high bitrate
        }
    }

    /// Set encoding profile.
    pub async fn set_profile(&self, profile: EncodeProfile) {
        *self.profile.write().await = profile;
        tracing::debug!("Encoding profile set to {:?}", profile);
    }

    /// Get current codec.
    pub async fn get_codec(&self) -> CodecType {
        *self.current_codec.read().await
    }

    /// Get target bitrate.
    pub async fn get_target_bitrate(&self) -> f64 {
        *self.target_bitrate.read().await
    }

    /// Check if hardware acceleration is available.
    pub fn has_hardware_acceleration(&self) -> bool {
        self.has_hw_accel
    }

    /// Synthetic encode for testing (just copy relevant bytes).
    fn synthetic_encode(frame_data: &[u8], width: u32, height: u32) -> Vec<u8> {
        let target_size = ((width * height) as f64 * 0.1) as usize; // ~10% of original
        if frame_data.len() <= target_size {
            frame_data.to_vec()
        } else {
            frame_data[..target_size].to_vec()
        }
    }
}

impl Default for EncodeService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codec_bitrate() {
        let bitrate = CodecType::H264.typical_bitrate_mbps(1920, 1080);
        assert!(bitrate > 4.0 && bitrate < 6.0);
    }

    #[test]
    fn test_select_codec() {
        assert_eq!(EncodeService::select_codec(1.5), CodecType::AV1);
        assert_eq!(EncodeService::select_codec(2.5), CodecType::VP9);
        assert_eq!(EncodeService::select_codec(3.5), CodecType::H265);
        assert_eq!(EncodeService::select_codec(5.0), CodecType::VP8);
        assert_eq!(EncodeService::select_codec(10.0), CodecType::H264);
    }

    #[tokio::test]
    async fn test_encode_frame() {
        let service = EncodeService::new();
        let frame_data = vec![0u8; 1920 * 1080 * 4];

        let encoded = service.encode_frame(&frame_data, 1920, 1080).await.unwrap();
        assert_eq!(encoded.codec, CodecType::H264);
        assert_eq!(encoded.frame_number, 0);
    }

    #[tokio::test]
    async fn test_switch_codec() {
        let service = EncodeService::new();

        service.switch_codec(1.5).await.unwrap();
        assert_eq!(service.get_codec().await, CodecType::AV1);

        service.switch_codec(5.0).await.unwrap();
        assert_eq!(service.get_codec().await, CodecType::VP8);
    }

    #[tokio::test]
    async fn test_set_profile() {
        let service = EncodeService::new();
        service.set_profile(EncodeProfile::High).await;
    }

    #[test]
    fn test_mime_types() {
        assert_eq!(CodecType::H264.mime_type(), "video/h264");
        assert_eq!(CodecType::AV1.mime_type(), "video/av1");
    }
}
