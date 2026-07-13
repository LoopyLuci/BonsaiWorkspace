// Software encoder implementations (x264, x265)

use crate::EncoderBackend;
use bmn_common::error::BmnResult;

pub struct SoftwareEncoder {
    backend: EncoderBackend,
    width: u32,
    height: u32,
    bitrate_kbps: u32,
    fps: u32,
    preset: String, // ultrafast, superfast, veryfast, faster, fast, medium, slow, slower, veryslow
}

impl SoftwareEncoder {
    pub fn new(backend: EncoderBackend, width: u32, height: u32, bitrate_kbps: u32, fps: u32) -> Self {
        Self {
            backend,
            width,
            height,
            bitrate_kbps,
            fps,
            preset: "medium".into(),
        }
    }

    pub fn with_preset(mut self, preset: String) -> Self {
        self.preset = preset;
        self
    }

    pub async fn initialize(&mut self) -> BmnResult<()> {
        match self.backend {
            EncoderBackend::X264 => {
                tracing::info!("Initializing x264 software encoder (preset: {})", self.preset);
            }
            EncoderBackend::X265 => {
                tracing::info!("Initializing x265 software encoder (preset: {})", self.preset);
            }
            _ => {}
        }
        Ok(())
    }

    pub fn cpu_usage_percent(&self) -> f32 {
        // Rough estimate based on preset
        match self.preset.as_str() {
            "ultrafast" => 5.0,
            "superfast" => 10.0,
            "veryfast" => 20.0,
            "faster" => 30.0,
            "fast" => 40.0,
            "medium" => 60.0,
            "slow" => 80.0,
            _ => 50.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_software_encoder() {
        let encoder = SoftwareEncoder::new(EncoderBackend::X264, 1920, 1080, 5000, 60);
        assert_eq!(encoder.preset, "medium");
    }

    #[test]
    fn test_preset_cpu_usage() {
        let enc_fast = SoftwareEncoder::new(EncoderBackend::X264, 1920, 1080, 5000, 60)
            .with_preset("fast".into());
        assert_eq!(enc_fast.cpu_usage_percent(), 40.0);
    }
}
