//! SvdVideoTool — image-to-video via Stable Video Diffusion (SVD-XT).
//!
//! Production path: encode input image → latent, run temporal diffusion
//! (14-25 frames), decode frames, assemble MP4 via FFmpeg.

use crate::{GenerateParams, GenerationResult, GenerativeTool};
use async_trait::async_trait;
use cas::CasStore;
use std::sync::Arc;

pub struct SvdVideoTool {
    pub cas: Arc<CasStore>,
}

impl SvdVideoTool {
    pub fn new(cas: Arc<CasStore>) -> Self {
        Self { cas }
    }
}

#[async_trait]
impl GenerativeTool for SvdVideoTool {
    async fn generate(&self, params: GenerateParams) -> anyhow::Result<GenerationResult> {
        let input_key_hex = params.extra["input_image_key"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing input_image_key"))?;

        let input_key = cas::CasKey::from_hex(input_key_hex)
            .map_err(|e| anyhow::anyhow!("invalid CAS key: {e}"))?;

        let _input_image = self
            .cas
            .get(&input_key)
            .await?
            .ok_or_else(|| anyhow::anyhow!("input image not found in CAS: {input_key_hex}"))?;

        let num_frames = params.extra["num_frames"].as_u64().unwrap_or(25) as usize;

        // === Production path (stubbed) ===
        // 1. Encode image to SVD latent space
        // 2. Run SVD-XT temporal UNet (num_frames frames)
        // 3. Decode each frame latent → RGB
        // 4. Encode frames to MP4 via FFmpeg subprocess or mp4 crate

        // Placeholder: minimal valid MP4-like bytes
        let placeholder = format!(
            "SVD-XT placeholder video | frames={num_frames} | prompt={}",
            params.prompt
        );
        let key = self.cas.put(placeholder.as_bytes(), "video/mp4").await?;

        Ok(GenerationResult {
            cas_key: key,
            metadata: serde_json::json!({
                "model":       "svd-xt",
                "num_frames":  num_frames,
                "fps":         8,
                "input_key":   input_key_hex,
                "prompt":      params.prompt,
            }),
        })
    }
}
