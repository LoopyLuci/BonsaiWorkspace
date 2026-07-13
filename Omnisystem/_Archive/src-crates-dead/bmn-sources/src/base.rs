// Base source implementation

use bmn_common::{
    source::{Source, Capability},
    frame::{VideoFrame, AudioFrame},
    error::{BmnResult, BmnError},
};
use async_trait::async_trait;

/// Base implementation for sources that don't generate content
pub struct NoopSource {
    pub id: String,
    pub name: String,
    pub source_type: String,
    pub active: bool,
}

impl NoopSource {
    pub fn new(id: String, name: String, source_type: String) -> Self {
        Self {
            id,
            name,
            source_type,
            active: false,
        }
    }
}

#[async_trait]
impl Source for NoopSource {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn source_type(&self) -> &str {
        &self.source_type
    }

    fn is_active(&self) -> bool {
        self.active
    }

    async fn start(&mut self) -> BmnResult<()> {
        self.active = true;
        Ok(())
    }

    async fn stop(&mut self) -> BmnResult<()> {
        self.active = false;
        Ok(())
    }

    async fn get_video_frame(&mut self) -> BmnResult<Option<VideoFrame>> {
        Ok(None)
    }

    async fn get_audio_frame(&mut self) -> BmnResult<Option<AudioFrame>> {
        Ok(None)
    }

    fn requires_capability(&self) -> Option<Capability> {
        None
    }
}

/// Source state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceState {
    Idle,
    Initializing,
    Active,
    Paused,
    Error,
    Stopped,
}

/// Health status for a source
#[derive(Debug, Clone)]
pub struct SourceHealth {
    pub state: SourceState,
    pub frames_captured: u64,
    pub errors: u64,
    pub last_error: Option<String>,
}

impl Default for SourceHealth {
    fn default() -> Self {
        Self {
            state: SourceState::Idle,
            frames_captured: 0,
            errors: 0,
            last_error: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_noop_source() {
        let mut source = NoopSource::new(
            "test-1".into(),
            "Test Source".into(),
            "noop".into(),
        );

        assert_eq!(source.id(), "test-1");
        assert_eq!(source.name(), "Test Source");
        assert_eq!(source.source_type(), "noop");
        assert!(!source.is_active());

        source.start().await.unwrap();
        assert!(source.is_active());

        let video = source.get_video_frame().await.unwrap();
        assert!(video.is_none());

        source.stop().await.unwrap();
        assert!(!source.is_active());
    }

    #[test]
    fn test_source_health() {
        let health = SourceHealth::default();
        assert_eq!(health.state, SourceState::Idle);
        assert_eq!(health.frames_captured, 0);
        assert_eq!(health.errors, 0);
        assert!(health.last_error.is_none());
    }
}
