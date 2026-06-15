//! Codec format definitions and utilities

use serde::{Deserialize, Serialize};
use std::fmt;

/// Supported video codec formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CodecFormat {
    /// H.264 (AVC) video codec
    H264,
    /// H.265 (HEVC) video codec
    H265,
}

impl CodecFormat {
    /// Get the MIME type string for this codec
    pub fn mime_type(&self) -> &'static str {
        match self {
            CodecFormat::H264 => "video/avc",
            CodecFormat::H265 => "video/hevc",
        }
    }

    /// Parse MIME type string to codec format
    pub fn from_mime_type(mime: &str) -> Option<Self> {
        match mime {
            "video/avc" | "video/h264" => Some(CodecFormat::H264),
            "video/hevc" | "video/h265" => Some(CodecFormat::H265),
            _ => None,
        }
    }

    /// Get human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            CodecFormat::H264 => "H.264/AVC",
            CodecFormat::H265 => "H.265/HEVC",
        }
    }
}

impl fmt::Display for CodecFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Media format configuration
#[derive(Debug, Clone)]
pub struct MediaFormat {
    /// Codec format
    pub codec: CodecFormat,
    /// Frame width in pixels
    pub width: u32,
    /// Frame height in pixels
    pub height: u32,
    /// Frames per second (optional)
    pub frame_rate: Option<u32>,
    /// Bitrate in bits per second (optional)
    pub bitrate: Option<u32>,
}

impl MediaFormat {
    /// Create a new media format
    pub fn new(codec: CodecFormat, width: u32, height: u32) -> Self {
        MediaFormat {
            codec,
            width,
            height,
            frame_rate: None,
            bitrate: None,
        }
    }

    /// Set frame rate
    pub fn with_frame_rate(mut self, fps: u32) -> Self {
        self.frame_rate = Some(fps);
        self
    }

    /// Set bitrate
    pub fn with_bitrate(mut self, bitrate: u32) -> Self {
        self.bitrate = Some(bitrate);
        self
    }

    /// Get estimated frame size in bytes (YUV420 planar)
    ///
    /// YUV420 requires 1.5 bytes per pixel:
    /// - Y plane: width × height bytes
    /// - U plane: (width/2) × (height/2) bytes
    /// - V plane: (width/2) × (height/2) bytes
    pub fn estimated_frame_size(&self) -> usize {
        (self.width as usize) * (self.height as usize) * 3 / 2
    }

    /// Validate media format parameters
    pub fn validate(&self) -> crate::Result<()> {
        if self.width == 0 || self.height == 0 {
            return Err(crate::Error::InvalidConfiguration(
                "Invalid frame dimensions".to_string(),
            ));
        }

        // Common resolution check
        if self.width > 8192 || self.height > 8192 {
            return Err(crate::Error::InvalidConfiguration(
                "Frame dimensions exceed maximum (8192x8192)".to_string(),
            ));
        }

        // Check alignment (most decoders require alignment)
        if self.width % 2 != 0 || self.height % 2 != 0 {
            return Err(crate::Error::InvalidConfiguration(
                "Frame dimensions must be even".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codec_mime_types() {
        assert_eq!(CodecFormat::H264.mime_type(), "video/avc");
        assert_eq!(CodecFormat::H265.mime_type(), "video/hevc");
    }

    #[test]
    fn test_parse_mime_type() {
        assert_eq!(CodecFormat::from_mime_type("video/avc"), Some(CodecFormat::H264));
        assert_eq!(CodecFormat::from_mime_type("video/hevc"), Some(CodecFormat::H265));
        assert_eq!(CodecFormat::from_mime_type("invalid"), None);
    }

    #[test]
    fn test_media_format_creation() {
        let fmt = MediaFormat::new(CodecFormat::H264, 1920, 1080);
        assert_eq!(fmt.width, 1920);
        assert_eq!(fmt.height, 1080);
        assert_eq!(fmt.frame_rate, None);
    }

    #[test]
    fn test_frame_size_calculation() {
        let fmt = MediaFormat::new(CodecFormat::H264, 1920, 1080);
        // YUV420: 1920 * 1080 * 1.5 = 3,110,400 bytes
        assert_eq!(fmt.estimated_frame_size(), 3_110_400);
    }

    #[test]
    fn test_media_format_validation() {
        let fmt = MediaFormat::new(CodecFormat::H264, 1920, 1080);
        assert!(fmt.validate().is_ok());

        let invalid = MediaFormat::new(CodecFormat::H264, 1919, 1080);
        assert!(invalid.validate().is_err());

        let zero = MediaFormat::new(CodecFormat::H264, 0, 1080);
        assert!(zero.validate().is_err());
    }

    #[test]
    fn test_media_format_builder() {
        let fmt = MediaFormat::new(CodecFormat::H264, 1920, 1080)
            .with_frame_rate(60)
            .with_bitrate(8_000_000);

        assert_eq!(fmt.frame_rate, Some(60));
        assert_eq!(fmt.bitrate, Some(8_000_000));
    }
}
