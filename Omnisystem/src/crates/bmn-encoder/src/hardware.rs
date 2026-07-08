// Hardware encoder abstraction

use crate::EncoderBackend;
use bmn_common::error::BmnResult;

pub struct HardwareEncoder {
    backend: EncoderBackend,
    width: u32,
    height: u32,
    bitrate_kbps: u32,
    fps: u32,
}

impl HardwareEncoder {
    pub fn new(backend: EncoderBackend, width: u32, height: u32, bitrate_kbps: u32, fps: u32) -> Self {
        Self {
            backend,
            width,
            height,
            bitrate_kbps,
            fps,
        }
    }

    pub fn backend(&self) -> EncoderBackend {
        self.backend
    }

    pub async fn initialize(&mut self) -> BmnResult<()> {
        match self.backend {
            EncoderBackend::NVENC => {
                tracing::info!("Initializing NVENC encoder");
                // NVENC init via FFmpeg or native SDK
            }
            EncoderBackend::AMF => {
                tracing::info!("Initializing AMF encoder");
                // AMD AMF init
            }
            EncoderBackend::QSV => {
                tracing::info!("Initializing QSV encoder");
                // Intel QSV init
            }
            EncoderBackend::VideoToolbox => {
                tracing::info!("Initializing VideoToolbox encoder");
                // Apple VideoToolbox init
            }
            EncoderBackend::VAAPI => {
                tracing::info!("Initializing VAAPI encoder");
                // Linux VAAPI init
            }
            _ => {}
        }
        Ok(())
    }

    pub fn supports_b_frames(&self) -> bool {
        matches!(self.backend, EncoderBackend::NVENC | EncoderBackend::X265)
    }

    pub fn supports_10bit(&self) -> bool {
        matches!(self.backend, EncoderBackend::NVENC | EncoderBackend::X265 | EncoderBackend::VAAPI)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hardware_encoder_creation() {
        let encoder = HardwareEncoder::new(EncoderBackend::NVENC, 1920, 1080, 5000, 60);
        assert_eq!(encoder.width, 1920);
        assert_eq!(encoder.height, 1080);
        assert!(encoder.supports_b_frames());
    }
}
