// Example: Capturing from camera sources

use bmn_sources::CameraSource;
use bmn_common::source::Source;
use bmn_common::frame::PixelFormat;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("📷 Camera Capture Example");

    // Create a camera source
    let mut camera = CameraSource::new(0)
        .with_resolution(1920, 1080)
        .with_fps(60)
        .with_format(PixelFormat::RGBA);

    println!("Camera: {}", camera.name());
    println!("Type: {}", camera.source_type());

    // Start capturing
    camera.start().await?;
    println!("✓ Camera capture started");

    // Capture a few frames
    for i in 0..10 {
        if let Some(frame) = camera.get_video_frame().await? {
            println!(
                "Frame {}: {}x{} ({} format, {} bytes)",
                i,
                frame.width,
                frame.height,
                match frame.format {
                    PixelFormat::RGBA => "RGBA",
                    PixelFormat::BGRA => "BGRA",
                    PixelFormat::YUV420 => "YUV420",
                    PixelFormat::NV12 => "NV12",
                    PixelFormat::P010 => "P010",
                },
                frame.data.len()
            );
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(33)).await;
    }

    // Stop capturing
    camera.stop().await?;
    println!("✓ Camera capture stopped");

    Ok(())
}
