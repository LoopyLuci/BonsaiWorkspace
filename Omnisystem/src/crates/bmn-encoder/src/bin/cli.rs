//! bmn-encoder CLI: builds an adaptive bitrate ladder for a 1080p source,
//! spins up a small hardware encoder pool with a software fallback, and
//! prints a summary of each.

use bmn_encoder::{AdaptiveBitrateladder, EncoderBackend, EncoderPool};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ladder = AdaptiveBitrateladder::generate_ladder(1920, 1080, 6000);
    println!("Generated {} bitrate rung(s):", ladder.len());
    for profile in &ladder {
        println!(
            "  {} @ {}x{} - {} kbps (target VMAF {})",
            profile.resolution, profile.width, profile.height, profile.bitrate_kbps, profile.target_vmaf
        );
    }

    let mut pool = EncoderPool::new(EncoderBackend::NVENC, 1920, 1080, 6000, 60, 3);
    println!("\nEncoder pool has {} hardware encoder(s)", pool.encoder_count());
    for _ in 0..pool.encoder_count() {
        let encoder = pool.get_next_encoder().await;
        let guard = encoder.read().await;
        println!(
            "  backend={:?} b_frames={} 10bit={}",
            guard.backend(),
            guard.supports_b_frames(),
            guard.supports_10bit()
        );
    }

    let fallback = pool.software_fallback();
    let guard = fallback.read().await;
    println!("\nSoftware fallback CPU estimate: {:.1}%", guard.cpu_usage_percent());

    Ok(())
}
