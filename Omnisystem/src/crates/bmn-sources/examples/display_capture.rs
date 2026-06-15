// Example: Capturing from display sources

use bmn_sources::DisplaySource;
use bmn_common::source::Source;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎬 Display Capture Example");

    // Create a display source
    let mut display = DisplaySource::new(0)
        .with_resolution(1280, 720)
        .with_fps(30);

    println!("Display: {}", display.name());
    println!("Type: {}", display.source_type());

    // Start capturing
    display.start().await?;
    println!("✓ Display capture started");

    // Capture a few frames
    for i in 0..5 {
        if let Some(frame) = display.get_video_frame().await? {
            println!(
                "Frame {}: {}x{} ({}ms)",
                i, frame.width, frame.height, frame.timestamp
            );
        }
    }

    // Stop capturing
    display.stop().await?;
    println!("✓ Display capture stopped");

    Ok(())
}
