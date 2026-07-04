// Display/Screen capture source

use crate::{SourceConfig, SourceHealth, SourceState};
use bmn_common::{
    source::{Source, Capability},
    frame::{VideoFrame, PixelFormat, AudioFrame},
    error::{BmnResult, BmnError},
};
use async_trait::async_trait;
use bytes::Bytes;
use std::sync::Arc;

/// Display capture configuration
#[derive(Debug, Clone)]
pub struct DisplayCaptureConfig {
    pub monitor_index: u32,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
}

impl Default for DisplayCaptureConfig {
    fn default() -> Self {
        Self {
            monitor_index: 0,
            width: 1920,
            height: 1080,
            fps: 60,
        }
    }
}

/// Display capture source
pub struct DisplaySource {
    config: SourceConfig,
    capture_config: DisplayCaptureConfig,
    active: bool,
    health: SourceHealth,
    frame_counter: u64,
}

impl DisplaySource {
    pub fn new(monitor_index: u32) -> Self {
        let config = SourceConfig::new(
            format!("Display {}", monitor_index + 1),
            "display",
        );

        Self {
            config,
            capture_config: DisplayCaptureConfig {
                monitor_index,
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

    pub fn health(&self) -> SourceHealth {
        self.health.clone()
    }

    fn create_test_frame(&mut self) -> VideoFrame {
        // Create a simple test frame (in production, would be actual screen capture)
        let width = self.capture_config.width as usize;
        let height = self.capture_config.height as usize;

        // BGRA format: 4 bytes per pixel
        let frame_size = width * height * 4;
        let data = vec![0xFF; frame_size]; // Solid white test frame

        VideoFrame {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            width: self.capture_config.width,
            height: self.capture_config.height,
            format: PixelFormat::BGRA,
            data: Arc::new(Bytes::from(data)),
        }
    }
}

#[async_trait]
impl Source for DisplaySource {
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
            return Err(BmnError::source_error("Display source already started"));
        }

        // Platform-specific initialization would happen here
        #[cfg(windows)]
        {
            // DXGI setup would go here
        }

        #[cfg(target_os = "linux")]
        {
            // X11/Wayland setup would go here
        }

        #[cfg(target_os = "macos")]
        {
            // AVFoundation setup would go here
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
        // Display capture typically doesn't provide audio
        Ok(None)
    }

    fn requires_capability(&self) -> Option<Capability> {
        Some(Capability::ScreenCapture)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_display_source_creation() {
        let source = DisplaySource::new(0);
        assert_eq!(source.name(), "Display 1");
        assert_eq!(source.source_type(), "display");
        assert!(!source.is_active());
    }

    #[tokio::test]
    async fn test_display_source_lifecycle() {
        let mut source = DisplaySource::new(0);

        source.start().await.unwrap();
        assert!(source.is_active());

        let frame = source.get_video_frame().await.unwrap();
        assert!(frame.is_some());

        let frame = frame.unwrap();
        assert_eq!(frame.format, PixelFormat::BGRA);
        assert_eq!(frame.width, 1920);
        assert_eq!(frame.height, 1080);

        source.stop().await.unwrap();
        assert!(!source.is_active());
    }

    #[tokio::test]
    async fn test_display_source_resolution() {
        let mut source = DisplaySource::new(0)
            .with_resolution(1280, 720)
            .with_fps(30);

        source.start().await.unwrap();
        let frame = source.get_video_frame().await.unwrap().unwrap();
        assert_eq!(frame.width, 1280);
        assert_eq!(frame.height, 720);
    }

    #[test]
    fn test_display_source_health() {
        let source = DisplaySource::new(0);
        let health = source.health();
        assert_eq!(health.state, SourceState::Idle);
        assert_eq!(health.frames_captured, 0);
    }

    #[tokio::test]
    async fn test_display_requires_capability() {
        let source = DisplaySource::new(0);
        assert_eq!(
            source.requires_capability(),
            Some(Capability::ScreenCapture)
        );
    }
}
