// Source trait and implementations

use crate::{AudioFrame, VideoFrame, error::BmnError, CapabilityToken, Capability};
use async_trait::async_trait;
use std::sync::Arc;

/// Trait that all sources must implement
#[async_trait]
pub trait Source: Send + Sync {
    /// Source identifier
    fn id(&self) -> &str;

    /// Source name (human readable)
    fn name(&self) -> &str;

    /// Source type (e.g., "display", "camera", "browser")
    fn source_type(&self) -> &str;

    /// Check if source is active
    fn is_active(&self) -> bool;

    /// Start the source
    async fn start(&mut self) -> Result<(), BmnError>;

    /// Stop the source
    async fn stop(&mut self) -> Result<(), BmnError>;

    /// Get next video frame (if this is a video source)
    async fn get_video_frame(&mut self) -> Result<Option<VideoFrame>, BmnError>;

    /// Get next audio frame (if this is an audio source)
    async fn get_audio_frame(&mut self) -> Result<Option<AudioFrame>, BmnError>;

    /// Check capabilities
    fn requires_capability(&self) -> Option<Capability>;
}

/// Display/Screen capture source
pub struct DisplaySource {
    pub id: String,
    pub name: String,
    pub display_index: usize,
    pub active: bool,
}

#[async_trait]
impl Source for DisplaySource {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn source_type(&self) -> &str {
        "display"
    }

    fn is_active(&self) -> bool {
        self.active
    }

    async fn start(&mut self) -> Result<(), BmnError> {
        self.active = true;
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), BmnError> {
        self.active = false;
        Ok(())
    }

    async fn get_video_frame(&mut self) -> Result<Option<VideoFrame>, BmnError> {
        if !self.active {
            return Ok(None);
        }

        // In a real implementation, this would capture the display
        // For now, return None
        Ok(None)
    }

    async fn get_audio_frame(&mut self) -> Result<Option<AudioFrame>, BmnError> {
        Ok(None) // Display sources don't have audio
    }

    fn requires_capability(&self) -> Option<Capability> {
        Some(Capability::SourceCapture)
    }
}

/// Camera/Webcam source
pub struct CameraSource {
    pub id: String,
    pub name: String,
    pub device_path: String,
    pub active: bool,
}

#[async_trait]
impl Source for CameraSource {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn source_type(&self) -> &str {
        "camera"
    }

    fn is_active(&self) -> bool {
        self.active
    }

    async fn start(&mut self) -> Result<(), BmnError> {
        self.active = true;
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), BmnError> {
        self.active = false;
        Ok(())
    }

    async fn get_video_frame(&mut self) -> Result<Option<VideoFrame>, BmnError> {
        if !self.active {
            return Ok(None);
        }

        // In a real implementation, this would capture from the camera
        // For now, return None
        Ok(None)
    }

    async fn get_audio_frame(&mut self) -> Result<Option<AudioFrame>, BmnError> {
        Ok(None)
    }

    fn requires_capability(&self) -> Option<Capability> {
        Some(Capability::SourceCapture)
    }
}

/// Audio input source
pub struct AudioSource {
    pub id: String,
    pub name: String,
    pub device: String,
    pub active: bool,
}

#[async_trait]
impl Source for AudioSource {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn source_type(&self) -> &str {
        "audio"
    }

    fn is_active(&self) -> bool {
        self.active
    }

    async fn start(&mut self) -> Result<(), BmnError> {
        self.active = true;
        Ok(())
    }

    async fn stop(&mut self) -> Result<(), BmnError> {
        self.active = false;
        Ok(())
    }

    async fn get_video_frame(&mut self) -> Result<Option<VideoFrame>, BmnError> {
        Ok(None) // Audio sources don't have video
    }

    async fn get_audio_frame(&mut self) -> Result<Option<AudioFrame>, BmnError> {
        if !self.active {
            return Ok(None);
        }

        // In a real implementation, this would capture audio
        // For now, return None
        Ok(None)
    }

    fn requires_capability(&self) -> Option<Capability> {
        Some(Capability::SourceCapture)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_display_source_lifecycle() {
        let mut source = DisplaySource {
            id: "display-0".to_string(),
            name: "Primary Monitor".to_string(),
            display_index: 0,
            active: false,
        };

        assert!(!source.is_active());

        source.start().await.unwrap();
        assert!(source.is_active());

        source.stop().await.unwrap();
        assert!(!source.is_active());
    }

    #[tokio::test]
    async fn test_camera_source() {
        let source = CameraSource {
            id: "camera-0".to_string(),
            name: "Webcam".to_string(),
            device_path: "/dev/video0".to_string(),
            active: false,
        };

        assert_eq!(source.source_type(), "camera");
        assert_eq!(source.requires_capability(), Some(Capability::SourceCapture));
    }
}
