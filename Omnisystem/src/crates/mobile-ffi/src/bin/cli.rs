//! CLI demo for mobile-ffi: exercises the (host-side simulated) H.264
//! decode pipeline end-to-end and prints real collected metrics.

use mobile_ffi::{Decoder, DecoderConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = DecoderConfig::new("video/avc", 1920, 1080)?;
    let mut decoder = Decoder::new(config)?;

    println!("Decoder initialized: {}", decoder.is_initialized());

    let dummy_nal = vec![0u8; 4096];
    let mut decoded = 0;
    for i in 0..30 {
        let timestamp_us = (i as i64) * 33_333;
        decoder.decode_frame(&dummy_nal, timestamp_us)?;

        // Drain each frame as it's produced, like a real player would,
        // so the bounded output queue never fills up.
        if let Some(frame) = decoder.get_output_frame()? {
            decoded += 1;
            if decoded == 1 {
                println!(
                    "First frame: {}x{}, timestamp {}us",
                    frame.width, frame.height, frame.timestamp_us
                );
            }
        }
    }

    let metrics = decoder.metrics();
    println!(
        "Decoded {} frames, {} dropped, avg latency {}us, fps ~{:.1}, throughput ~{:.2} Mbps",
        metrics.frames_decoded,
        metrics.frames_dropped,
        metrics.avg_decode_latency_us,
        metrics.fps(),
        metrics.throughput_mbps()
    );

    Ok(())
}
