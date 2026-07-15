//! BMN Common CLI - exercises the frame types, source trait, and metrics collector

use bmn_common::source::{CameraSource, Source};
use bmn_common::{AudioFormat, AudioFrame, MetricsCollector, PixelFormat, VideoFrame};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let video = VideoFrame::new(0, 1920, 1080, PixelFormat::BGRA, vec![0u8; 1920 * 1080 * 4], 1920 * 4, true);
    println!("video frame: {:?}", video);

    let audio = AudioFrame::new(0, 48000, 2, AudioFormat::S16, vec![0u8; 48000 * 2 * 2]);
    println!("audio frame: {} samples", audio.sample_count());

    let mut source = CameraSource {
        id: "camera-0".to_string(),
        name: "Webcam".to_string(),
        device_path: "/dev/video0".to_string(),
        active: false,
    };
    source.start().await?;
    println!("source active: {}", source.is_active());

    let metrics = MetricsCollector::new();
    metrics.record_frame_captured().await;
    metrics.record_frame_encoded().await;
    println!("encode rate: {:.1}%", metrics.encode_rate().await);

    Ok(())
}
