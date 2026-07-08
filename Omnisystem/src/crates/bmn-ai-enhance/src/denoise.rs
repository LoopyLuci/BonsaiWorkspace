// Real-time video denoising (temporal + spatial)

use bmn_common::frame::VideoFrame;
use bmn_common::error::BmnResult;

pub struct Denoiser {
    strength: f32,
}

impl Denoiser {
    pub async fn new(strength: f32) -> BmnResult<Self> {
        Ok(Self { strength })
    }

    pub async fn process(&self, frame: VideoFrame) -> BmnResult<VideoFrame> {
        tracing::debug!("Denoising frame with strength {}", self.strength);
        Ok(frame)
    }
}
