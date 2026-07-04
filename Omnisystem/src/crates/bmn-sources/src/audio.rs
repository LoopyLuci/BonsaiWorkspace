// Audio capture source

use crate::{SourceConfig, SourceHealth, SourceState};
use bmn_common::{
    source::{Source, Capability},
    frame::{AudioFrame, AudioFormat, VideoFrame},
    error::{BmnResult, BmnError},
};
use async_trait::async_trait;
use bytes::Bytes;
use std::sync::Arc;

/// Audio capture configuration
#[derive(Debug, Clone)]
pub struct AudioCaptureConfig {
    pub device_index: u32,
    pub sample_rate: u32,
    pub channels: u32,
    pub format: AudioFormat,
}

impl Default for AudioCaptureConfig {
    fn default() -> Self {
        Self {
            device_index: 0,
            sample_rate: 48000,
            channels: 2,
            format: AudioFormat::S16,
        }
    }
}

/// Audio source
pub struct AudioSource {
    config: SourceConfig,
    capture_config: AudioCaptureConfig,
    active: bool,
    health: SourceHealth,
    frame_counter: u64,
}

impl AudioSource {
    pub fn new(device_index: u32, source_type: &str) -> Self {
        let name = match source_type {
            "microphone" => format!("Microphone {}", device_index),
            "loopback" => format!("System Audio {}", device_index),
            _ => format!("Audio {}", device_index),
        };

        let config = SourceConfig::new(name, source_type);

        Self {
            config,
            capture_config: AudioCaptureConfig {
                device_index,
                ..Default::default()
            },
            active: false,
            health: SourceHealth::default(),
            frame_counter: 0,
        }
    }

    pub fn microphone(device_index: u32) -> Self {
        Self::new(device_index, "microphone")
    }

    pub fn system_audio(device_index: u32) -> Self {
        Self::new(device_index, "loopback")
    }

    pub fn with_sample_rate(mut self, sample_rate: u32) -> Self {
        self.capture_config.sample_rate = sample_rate;
        self
    }

    pub fn with_channels(mut self, channels: u32) -> Self {
        self.capture_config.channels = channels;
        self
    }

    pub fn with_format(mut self, format: AudioFormat) -> Self {
        self.capture_config.format = format;
        self
    }

    pub fn health(&self) -> SourceHealth {
        self.health.clone()
    }

    fn bytes_per_sample(&self) -> usize {
        match self.capture_config.format {
            AudioFormat::S16 => 2,
            AudioFormat::S32 => 4,
            AudioFormat::F32 => 4,
        }
    }

    fn create_test_frame(&mut self) -> AudioFrame {
        // Create a test audio frame (in production, would be actual audio)
        let sample_count = (self.capture_config.sample_rate / 100) as usize; // 10ms buffer
        let frame_size = sample_count * self.capture_config.channels as usize * self.bytes_per_sample();

        // Generate silence
        let data = vec![0u8; frame_size];

        AudioFrame {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            sample_rate: self.capture_config.sample_rate,
            channels: self.capture_config.channels,
            format: self.capture_config.format,
            data: Arc::new(Bytes::from(data)),
        }
    }
}

#[async_trait]
impl Source for AudioSource {
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
            return Err(BmnError::source_error("Audio source already started"));
        }

        // Platform-specific audio initialization would happen here
        #[cfg(windows)]
        {
            // WASAPI setup
        }

        #[cfg(target_os = "linux")]
        {
            // PulseAudio or JACK setup
        }

        #[cfg(target_os = "macos")]
        {
            // AVAudioEngine setup
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
        // Audio source doesn't provide video
        Ok(None)
    }

    async fn get_audio_frame(&mut self) -> BmnResult<Option<AudioFrame>> {
        if !self.active {
            return Ok(None);
        }

        self.frame_counter += 1;
        self.health.frames_captured += 1;

        Ok(Some(self.create_test_frame()))
    }

    fn requires_capability(&self) -> Option<Capability> {
        match self.source_type() {
            "microphone" => Some(Capability::AudioCapture),
            "loopback" => Some(Capability::SystemAudioCapture),
            _ => Some(Capability::AudioCapture),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_audio_source_creation() {
        let source = AudioSource::microphone(0);
        assert_eq!(source.name(), "Microphone 0");
        assert_eq!(source.source_type(), "microphone");
        assert!(!source.is_active());
    }

    #[tokio::test]
    async fn test_system_audio_source() {
        let source = AudioSource::system_audio(0);
        assert_eq!(source.name(), "System Audio 0");
        assert_eq!(source.source_type(), "loopback");
    }

    #[tokio::test]
    async fn test_audio_source_lifecycle() {
        let mut source = AudioSource::microphone(0);

        source.start().await.unwrap();
        assert!(source.is_active());

        let frame = source.get_audio_frame().await.unwrap();
        assert!(frame.is_some());

        let frame = frame.unwrap();
        assert_eq!(frame.sample_rate, 48000);
        assert_eq!(frame.channels, 2);
        assert_eq!(frame.format, AudioFormat::S16);

        source.stop().await.unwrap();
        assert!(!source.is_active());
    }

    #[tokio::test]
    async fn test_audio_source_configuration() {
        let mut source = AudioSource::microphone(0)
            .with_sample_rate(44100)
            .with_channels(1)
            .with_format(AudioFormat::F32);

        source.start().await.unwrap();
        let frame = source.get_audio_frame().await.unwrap().unwrap();
        assert_eq!(frame.sample_rate, 44100);
        assert_eq!(frame.channels, 1);
        assert_eq!(frame.format, AudioFormat::F32);
    }

    #[tokio::test]
    async fn test_audio_source_no_video() {
        let mut source = AudioSource::microphone(0);
        source.start().await.unwrap();

        let video = source.get_video_frame().await.unwrap();
        assert!(video.is_none());
    }

    #[test]
    fn test_audio_microphone_capability() {
        let source = AudioSource::microphone(0);
        assert_eq!(
            source.requires_capability(),
            Some(Capability::AudioCapture)
        );
    }

    #[test]
    fn test_audio_loopback_capability() {
        let source = AudioSource::system_audio(0);
        assert_eq!(
            source.requires_capability(),
            Some(Capability::SystemAudioCapture)
        );
    }
}
