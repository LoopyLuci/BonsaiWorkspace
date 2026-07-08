// Example: Capturing from audio sources

use bmn_sources::AudioSource;
use bmn_common::source::Source;
use bmn_common::frame::AudioFormat;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🎤 Audio Capture Example");

    // Create microphone source
    let mut microphone = AudioSource::microphone(0)
        .with_sample_rate(48000)
        .with_channels(2)
        .with_format(AudioFormat::S16);

    println!("Source: {}", microphone.name());
    println!("Type: {}", microphone.source_type());

    // Start capturing
    microphone.start().await?;
    println!("✓ Microphone capture started");

    // Capture audio frames
    for i in 0..10 {
        if let Some(frame) = microphone.get_audio_frame().await? {
            println!(
                "Frame {}: {}Hz, {} channels, {} bytes",
                i,
                frame.sample_rate,
                frame.channels,
                frame.data.len()
            );
        }

        // 10ms per frame
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    microphone.stop().await?;
    println!("✓ Microphone capture stopped");

    // Also demonstrate system audio capture
    println!("\n🔊 System Audio Capture");

    let mut system_audio = AudioSource::system_audio(0)
        .with_sample_rate(44100)
        .with_channels(1)
        .with_format(AudioFormat::F32);

    println!("Source: {}", system_audio.name());

    system_audio.start().await?;
    println!("✓ System audio capture started");

    for i in 0..5 {
        if let Some(_frame) = system_audio.get_audio_frame().await? {
            println!("System Audio Frame {}", i);
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    }

    system_audio.stop().await?;
    println!("✓ System audio capture stopped");

    Ok(())
}
