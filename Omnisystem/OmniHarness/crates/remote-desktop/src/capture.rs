//! Screen, audio, and camera capture.
//!
//! Platform-specific capture implementations for screen, audio, and camera.
//! This module provides trait-based abstractions that are implemented differently
//! on Windows (DXGI), macOS (CoreGraphics), and Linux (X11/Wayland).

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

/// Errors that can occur during capture operations.
#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("Capture device not found")]
    DeviceNotFound,

    #[error("Capture failed: {reason}")]
    CaptureFailed { reason: String },

    #[error("Buffer allocation failed")]
    BufferError,

    #[error("Unsupported format: {format}")]
    UnsupportedFormat { format: String },

    #[error("Permission denied")]
    PermissionDenied,
}

/// Screen resolution.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

impl Resolution {
    pub fn new(width: u32, height: u32) -> Self {
        Resolution { width, height }
    }

    pub fn pixels(&self) -> u64 {
        (self.width as u64) * (self.height as u64)
    }
}

/// Screen capture frame.
#[derive(Clone, Serialize, Deserialize)]
pub struct ScreenFrame {
    /// Frame data (raw pixel data or compressed).
    pub data: Vec<u8>,

    /// Resolution.
    pub resolution: Resolution,

    /// Bytes per pixel (3 for RGB, 4 for RGBA).
    pub bytes_per_pixel: u8,

    /// Frame number (for deduplication).
    pub frame_number: u64,

    /// Capture timestamp in milliseconds.
    pub timestamp_ms: u64,
}

impl ScreenFrame {
    /// Create a new screen frame.
    pub fn new(
        data: Vec<u8>,
        resolution: Resolution,
        bytes_per_pixel: u8,
        frame_number: u64,
    ) -> Self {
        ScreenFrame {
            data,
            resolution,
            bytes_per_pixel,
            frame_number,
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        }
    }
}

/// Audio sample.
#[derive(Clone, Serialize, Deserialize)]
pub struct AudioFrame {
    /// Audio data (PCM samples).
    pub data: Vec<u8>,

    /// Sample rate (Hz).
    pub sample_rate: u32,

    /// Channels (1=mono, 2=stereo).
    pub channels: u8,

    /// Bits per sample (16, 24, 32).
    pub bits_per_sample: u8,

    /// Frame number.
    pub frame_number: u64,

    /// Capture timestamp in milliseconds.
    pub timestamp_ms: u64,
}

impl AudioFrame {
    /// Create a new audio frame.
    pub fn new(
        data: Vec<u8>,
        sample_rate: u32,
        channels: u8,
        bits_per_sample: u8,
        frame_number: u64,
    ) -> Self {
        AudioFrame {
            data,
            sample_rate,
            channels,
            bits_per_sample,
            frame_number,
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        }
    }
}

/// Camera/Webcam capture frame.
#[derive(Clone, Serialize, Deserialize)]
pub struct CameraFrame {
    /// Frame data (video codec compressed or raw).
    pub data: Vec<u8>,

    /// Resolution.
    pub resolution: Resolution,

    /// Frame number.
    pub frame_number: u64,

    /// Capture timestamp in milliseconds.
    pub timestamp_ms: u64,
}

impl CameraFrame {
    /// Create a new camera frame.
    pub fn new(data: Vec<u8>, resolution: Resolution, frame_number: u64) -> Self {
        CameraFrame {
            data,
            resolution,
            frame_number,
            timestamp_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        }
    }
}

/// Screen capture service.
pub struct CaptureService {
    /// Capture is active.
    active: Arc<std::sync::atomic::AtomicBool>,

    /// Default resolution for screen capture.
    resolution: Arc<tokio::sync::RwLock<Resolution>>,

    /// Frame counter.
    frame_count: Arc<std::sync::atomic::AtomicU64>,
}

impl CaptureService {
    /// Create a new CaptureService.
    pub fn new() -> Self {
        CaptureService {
            active: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            resolution: Arc::new(tokio::sync::RwLock::new(Resolution::new(1920, 1080))),
            frame_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Start screen capture.
    pub async fn start_screen_capture(&self) -> Result<(), CaptureError> {
        if self.active.swap(true, std::sync::atomic::Ordering::SeqCst) {
            return Ok(()); // Already capturing
        }

        // In production: initialize platform-specific screen capture
        // - Windows: DXGI/Direct3D11
        // - macOS: CoreGraphics/IOSurface
        // - Linux: X11/Wayland
        tracing::info!("Screen capture started");
        Ok(())
    }

    /// Stop screen capture.
    pub async fn stop_screen_capture(&self) -> Result<(), CaptureError> {
        self.active.store(false, std::sync::atomic::Ordering::Release);
        tracing::info!("Screen capture stopped");
        Ok(())
    }

    /// Capture a screen frame (stub - returns synthetic frame).
    pub async fn capture_frame(&self) -> Result<ScreenFrame, CaptureError> {
        if !self.active.load(std::sync::atomic::Ordering::Acquire) {
            return Err(CaptureError::CaptureFailed {
                reason: "Capture not started".to_string(),
            });
        }

        let resolution = *self.resolution.read().await;
        let frame_number = self.frame_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // Stub: create synthetic frame for testing
        let pixels = (resolution.width as usize) * (resolution.height as usize);
        let data = vec![0u8; pixels * 4]; // RGBA

        Ok(ScreenFrame::new(data, resolution, 4, frame_number))
    }

    /// Start audio capture.
    pub async fn start_audio_capture(&self) -> Result<(), CaptureError> {
        // In production: initialize platform-specific audio capture
        // - Windows: WASAPI
        // - macOS: AVFoundation
        // - Linux: PulseAudio/ALSA
        tracing::info!("Audio capture started");
        Ok(())
    }

    /// Capture audio frame (stub).
    pub async fn capture_audio(&self) -> Result<AudioFrame, CaptureError> {
        let frame_number = self.frame_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let data = vec![0u8; 4096]; // 2048 samples, 16-bit stereo

        Ok(AudioFrame::new(data, 48000, 2, 16, frame_number))
    }

    /// Start camera capture.
    pub async fn start_camera_capture(&self) -> Result<(), CaptureError> {
        // In production: enumerate and initialize camera devices
        tracing::info!("Camera capture started");
        Ok(())
    }

    /// Capture camera frame (stub).
    pub async fn capture_camera(&self) -> Result<CameraFrame, CaptureError> {
        let resolution = Resolution::new(1280, 720);
        let frame_number = self.frame_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let data = vec![0u8; 1280 * 720 * 3]; // YUV420

        Ok(CameraFrame::new(data, resolution, frame_number))
    }

    /// Get current screen resolution.
    pub async fn get_resolution(&self) -> Resolution {
        *self.resolution.read().await
    }

    /// Set screen resolution.
    pub async fn set_resolution(&self, resolution: Resolution) {
        *self.resolution.write().await = resolution;
    }

    /// Check if capture is active.
    pub fn is_active(&self) -> bool {
        self.active.load(std::sync::atomic::Ordering::Acquire)
    }
}

impl Default for CaptureService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_capture_frame() {
        let service = CaptureService::new();
        service.start_screen_capture().await.unwrap();

        let frame = service.capture_frame().await.unwrap();
        assert_eq!(frame.resolution.width, 1920);
        assert_eq!(frame.resolution.height, 1080);
        assert_eq!(frame.frame_number, 0);
    }

    #[tokio::test]
    async fn test_capture_audio() {
        let service = CaptureService::new();
        service.start_audio_capture().await.unwrap();

        let frame = service.capture_audio().await.unwrap();
        assert_eq!(frame.sample_rate, 48000);
        assert_eq!(frame.channels, 2);
    }

    #[tokio::test]
    async fn test_set_resolution() {
        let service = CaptureService::new();
        service
            .set_resolution(Resolution::new(1280, 720))
            .await;

        let res = service.get_resolution().await;
        assert_eq!(res.width, 1280);
        assert_eq!(res.height, 720);
    }

    #[tokio::test]
    async fn test_capture_without_start() {
        let service = CaptureService::new();
        let result = service.capture_frame().await;
        assert!(result.is_err());
    }
}
