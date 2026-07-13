// Encoder pool for parallel multi-bitrate encoding

use crate::{HardwareEncoder, SoftwareEncoder, EncoderBackend};
use bmn_common::error::BmnResult;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct EncoderPool {
    encoders: Vec<Arc<RwLock<HardwareEncoder>>>,
    software_fallback: Arc<RwLock<SoftwareEncoder>>,
    next_index: usize,
}

impl EncoderPool {
    pub fn new(
        backend: EncoderBackend,
        width: u32,
        height: u32,
        max_bitrate: u32,
        fps: u32,
        count: usize,
    ) -> Self {
        let mut encoders = Vec::new();
        for _ in 0..count {
            let encoder = HardwareEncoder::new(backend, width, height, max_bitrate, fps);
            encoders.push(Arc::new(RwLock::new(encoder)));
        }

        let fallback = SoftwareEncoder::new(backend, width, height, max_bitrate, fps);

        Self {
            encoders,
            software_fallback: Arc::new(RwLock::new(fallback)),
            next_index: 0,
        }
    }

    pub fn encoder_count(&self) -> usize {
        self.encoders.len()
    }

    pub async fn get_next_encoder(&mut self) -> Arc<RwLock<HardwareEncoder>> {
        let encoder = self.encoders[self.next_index].clone();
        self.next_index = (self.next_index + 1) % self.encoders.len();
        encoder
    }

    pub fn software_fallback(&self) -> Arc<RwLock<SoftwareEncoder>> {
        self.software_fallback.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encoder_pool_creation() {
        let pool = EncoderPool::new(EncoderBackend::NVENC, 1920, 1080, 5000, 60, 2);
        assert_eq!(pool.encoder_count(), 2);
    }

    #[tokio::test]
    async fn test_round_robin() {
        let mut pool = EncoderPool::new(EncoderBackend::X264, 1920, 1080, 5000, 60, 3);

        let enc1 = pool.get_next_encoder().await;
        let enc2 = pool.get_next_encoder().await;
        let enc3 = pool.get_next_encoder().await;
        let enc1_again = pool.get_next_encoder().await;

        // Should cycle through all encoders
        assert_ne!(enc1.as_ptr(), enc2.as_ptr());
        assert_ne!(enc2.as_ptr(), enc3.as_ptr());
        assert_eq!(enc1.as_ptr(), enc1_again.as_ptr());
    }
}
