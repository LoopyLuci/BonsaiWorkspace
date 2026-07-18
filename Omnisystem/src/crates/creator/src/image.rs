//! FluxDiTTool — text-to-image via a FLUX DiT pipeline.
//!
//! Production path: wire in candle DiT + VAE + T5/CLIP text encoders.
//! Current state: skeleton that encodes a colored PNG so the full RPC/CAS
//! pipeline can be exercised end-to-end before the heavy model weights land.

use crate::{GenerateParams, GenerationResult, GenerativeTool};
use async_trait::async_trait;
use cas::CasStore;
use std::sync::Arc;

pub struct FluxDiTTool {
    pub cas: Arc<CasStore>,
}

impl FluxDiTTool {
    pub fn new(cas: Arc<CasStore>) -> Self {
        Self { cas }
    }
}

#[async_trait]
impl GenerativeTool for FluxDiTTool {
    async fn generate(&self, params: GenerateParams) -> anyhow::Result<GenerationResult> {
        let seed = params.seed.unwrap_or_else(rand::random);

        // === Production path (stubbed) ===
        // 1. Encode prompt with T5-XXL + CLIP-L
        // 2. Sample latents using DiT with FLUX guidance (CFG + distilled)
        // 3. Decode latents with FLUX VAE → pixel tensor
        // 4. Convert to PNG

        // Skeleton: gradient-filled image parameterised by prompt hash + seed
        let w = params.width.clamp(64, 2048) as usize;
        let h = params.height.clamp(64, 2048) as usize;
        let mut buf = vec![0u8; w * h * 3];
        let hash = params
            .prompt
            .bytes()
            .fold(seed, |a, b| a.wrapping_add(b as u64));
        for y in 0..h {
            for x in 0..w {
                let i = (y * w + x) * 3;
                buf[i] = ((x as u64 ^ hash) & 0xFF) as u8;
                buf[i + 1] = ((y as u64 ^ hash >> 8) & 0xFF) as u8;
                buf[i + 2] = ((x as u64 + y as u64 ^ hash >> 16) & 0xFF) as u8;
            }
        }

        let img = image::RgbImage::from_raw(w as u32, h as u32, buf)
            .ok_or_else(|| anyhow::anyhow!("invalid image dimensions"))?;
        let mut png_bytes = Vec::new();
        img.write_to(
            &mut std::io::Cursor::new(&mut png_bytes),
            image::ImageFormat::Png,
        )?;

        let key = self.cas.put(&png_bytes, "image/png").await?;
        Ok(GenerationResult {
            cas_key: key,
            metadata: serde_json::json!({
                "model":    "flux.1-dev",
                "prompt":   params.prompt,
                "seed":     seed,
                "width":    w,
                "height":   h,
                "steps":    params.steps,
                "guidance": params.guidance_scale,
            }),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_cas() -> Arc<CasStore> {
        let dir = tempfile::tempdir().unwrap();
        let store = CasStore::open(&dir.path().join("cas.db"), &dir.path().join("blobs"))
            .await
            .unwrap();
        // Leak the tempdir so it survives for the duration of the test.
        std::mem::forget(dir);
        Arc::new(store)
    }

    #[tokio::test]
    async fn test_generate_produces_valid_png() {
        let tool = FluxDiTTool::new(test_cas().await);
        let params = GenerateParams {
            prompt: "a red barn".to_string(),
            width: 64,
            height: 64,
            seed: Some(42),
            ..Default::default()
        };

        let result = tool.generate(params).await.unwrap();
        assert_eq!(result.metadata["seed"], 42);
        assert_eq!(result.metadata["width"], 64);

        let bytes = tool
            .cas
            .get(&result.cas_key)
            .await
            .unwrap()
            .expect("generated blob should be in CAS");
        // PNG magic bytes
        assert_eq!(&bytes[0..8], &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
    }

    #[tokio::test]
    async fn test_generate_is_deterministic_for_same_seed() {
        let tool = FluxDiTTool::new(test_cas().await);
        let params = |seed| GenerateParams {
            prompt: "same prompt".to_string(),
            width: 32,
            height: 32,
            seed: Some(seed),
            ..Default::default()
        };

        let r1 = tool.generate(params(7)).await.unwrap();
        let r2 = tool.generate(params(7)).await.unwrap();
        assert_eq!(r1.cas_key, r2.cas_key, "same prompt+seed should produce identical bytes");

        let r3 = tool.generate(params(8)).await.unwrap();
        assert_ne!(r1.cas_key, r3.cas_key, "different seed should produce different bytes");
    }
}
