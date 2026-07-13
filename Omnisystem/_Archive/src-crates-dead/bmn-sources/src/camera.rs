// Camera/Webcam capture source

use crate::{SourceConfig, SourceHealth, SourceState};
use bmn_common::{
    source::{Source, Capability},
    frame::{VideoFrame, PixelFormat, AudioFrame},
    error::{BmnResult, BmnError},
};
use async_trait::async_trait;
use bytes::Bytes;
use std::sync::Arc;

/// Camera capture configuration
#[derive(Debug, Clone)]
pub struct CameraCaptureConfig {
    pub device_index: u32,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub pixel_format: PixelFormat,
}

impl Default for CameraCaptureConfig {
    fn default() -> Self {
        Self {
            device_index: 0,
            width: 1920,
            height: 1080,
            fps: 30,
            pixel_format: PixelFormat::YUV420,
        }
    }
}

/// Camera source
pub struct CameraSource {
    config: SourceConfig,
    capture_config: CameraCaptureConfig,
    active: bool,
    health: SourceHealth,
    frame_counter: u64,
}

impl CameraSource {
    pub fn new(device_index: u32) -> Self {
        let config = SourceConfig::new(
            format!("Camera {}", device_index),
            "camera",
        );

        Self {
            config,
            capture_config: CameraCaptureConfig {
                device_index,
                ..Default::default()
            },
            active: false,
            health: SourceHealth::default(),
            frame_counter: 0,
        }
    }

    pub fn with_resolution(mut self, width: u32, height: u32) -> Self {
        self.capture_config.width = width;
        self.capture_config.height = height;
        self
    }

    pub fn with_fps(mut self, fps: u32) -> Self {
        self.capture_config.fps = fps;
        self
    }

    pub fn with_format(mut self, format: PixelFormat) -> Self {
        self.capture_config.pixel_format = format;
        self
    }

    pub fn health(&self) -> SourceHealth {
        self.health.clone()
    }

    fn create_test_frame(&mut self) -> VideoFrame {
        // Create a test frame (in production, would be actual camera feed)
        let width = self.capture_config.width as usize;
        let height = self.capture_config.height as usize;

        // YUV420 frame size: Y plane + U/V subsampled planes
        let y_size = width * height;
        let uv_size = (width * height) / 4;
        let frame_size = y_size + 2 * uv_size;

        let data = vec![128u8; frame_size]; // Gray test pattern

        VideoFrame {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            width: self.capture_config.width,
            height: self.capture_config.height,
            format: self.capture_config.pixel_format,
            data: Arc::new(Bytes::from(data)),
        }
    }
}

#[async_trait]
impl Source for CameraSource {
    fn id(&self) -> &str {
        &self.config.id
    }

    fn name(&self) -> &str {
        &self.config.name
    }

    fn source_type(&self) -> &str {
        &self.config.source_type
    }

    fn is_active(&self) -> bool {
        self.active
    }

    async fn start(&mut self) -> BmnResult<()> {
        if self.active {
            return Err(BmnError::source_error("Camera source already started"));
        }

        // Platform-specific camera initialization would happen here
        #[cfg(windows)]
        {
            // Windows Media Foundation or DirectShow setup
        }

        #[cfg(target_os = "linux")]
        {
            // V4L2 or GStreamer setup
        }

        #[cfg(target_os = "macos")]
        {
            // AVFoundation setup
        }

        self.active = true;
        self.health.state = SourceState::Active;
        Ok(())
    }

    async fn stop(&mut self) -> BmnResult<()> {
        self.active = false;
        self.health.state = SourceState::Stopped;
        Ok(())
    }

    async fn get_video_frame(&mut self) -> BmnResult<Option<VideoFrame>> {
        if !self.active {
            return Ok(None);
        }

        self.frame_counter += 1;
        self.health.frames_captured += 1;

        Ok(Some(self.create_test_frame()))
    }

    async fn get_audio_frame(&mut self) -> BmnResult<Option<AudioFrame>> {
        // Some camera devices have built-in mics, but typically we'd use a dedicated audio source
        Ok(None)
    }

    fn requires_capability(&self) -> Option<Capability> {
        Some(Capability::CameraCapture)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_camera_source_creation() {
        let source = CameraSource::new(0);
        assert_eq!(source.name(), "Camera 0");
        assert_eq!(source.source_type(), "camera");
        assert!(!source.is_active());
    }

    #[tokio::test]
    async fn test_camera_source_lifecycle() {
        let mut source = CameraSource::new(0);

        source.start().await.unwrap();
        assert!(source.is_active());

        let frame = source.get_video_frame().await.unwrap();
        assert!(frame.is_some());

        let frame = frame.unwrap();
        assert_eq!(frame.format, PixelFormat::YUV420);
        assert_eq!(frame.width, 1920);
        assert_eq!(frame.height, 1080);

        source.stop().await.unwrap();
        assert!(!source.is_active());
    }

    #[tokio::test]
    async fn test_camera_source_resolution() {
        let mut source = CameraSource::new(0)
            .with_resolution(1280, 720)
            .with_fps(24);

        source.start().await.unwrap();
        let frame = source.get_video_frame().await.unwrap().unwrap();
        assert_eq!(frame.width, 1280);
        assert_eq!(frame.height, 720);
    }

    #[tokio::test]
    async fn test_camera_source_format() {
        let mut source = CameraSource::new(0)
            .with_format(PixelFormat::RGBA);

        source.start().await.unwrap();
        let frame = source.get_video_frame().await.unwrap().unwrap();
        assert_eq!(frame.format, PixelFormat::RGBA);
    }

    #[test]
    fn test_camera_requires_capability() {
        let source = CameraSource::new(0);
        assert_eq!(
            source.requires_capability(),
            Some(Capability::CameraCapture)
        );
    }
}
