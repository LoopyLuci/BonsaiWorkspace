//! BMN Sources CLI - exercises display/camera/audio capture sources

use bmn_common::Source;
use bmn_sources::{AudioSource, CameraSource, DisplaySource};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut display = DisplaySource::new(0);
    display.start().await?;
    let frame = display.get_video_frame().await?;
    println!(
        "display frame: {:?}",
        frame.map(|f| (f.width, f.height, f.format))
    );

    let mut camera = CameraSource::new(0);
    camera.start().await?;
    let frame = camera.get_video_frame().await?;
    println!(
        "camera frame: {:?}",
        frame.map(|f| (f.width, f.height, f.format))
    );

    let mut mic = AudioSource::microphone(0);
    mic.start().await?;
    let frame = mic.get_audio_frame().await?;
    println!(
        "audio frame: {:?}",
        frame.map(|f| (f.sample_rate, f.channels, f.format))
    );

    Ok(())
}
