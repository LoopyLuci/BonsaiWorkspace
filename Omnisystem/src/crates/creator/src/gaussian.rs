//! GaussianSplattingTool — 3D Gaussian Splatting reconstruction and rendering.
//!
//! Takes an image or video from CAS, runs 3DGS optimisation (or a
//! feed-forward model sidecar), and stores the resulting PLY / glTF-GS asset.
//!
//! See [`crate::gaussian_pipeline`] for the WebGPU rasterizer that consumes
//! the stored splat data for real-time rendering.

use crate::{GenerateParams, GenerationResult, GenerativeTool};
use async_trait::async_trait;
use cas::CasStore;
use std::sync::Arc;

pub struct GaussianSplattingTool {
    pub cas: Arc<CasStore>,
}

impl GaussianSplattingTool {
    pub fn new(cas: Arc<CasStore>) -> Self {
        Self { cas }
    }
}

#[async_trait]
impl GenerativeTool for GaussianSplattingTool {
    async fn generate(&self, params: GenerateParams) -> anyhow::Result<GenerationResult> {
        let input_key_hex = params.extra["input_image_key"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing input_image_key for Gaussian splatting"))?;

        let input_key = cas::CasKey::from_hex(input_key_hex)
            .map_err(|e| anyhow::anyhow!("invalid CAS key: {e}"))?;
        if !self.cas.exists(&input_key).await? {
            return Err(anyhow::anyhow!(
                "input asset not found in CAS: {input_key_hex}"
            ));
        }

        let num_points = params.extra["num_points"].as_u64().unwrap_or(500_000) as u32;
        let output_format = params.extra["output_format"].as_str().unwrap_or("ply");

        // === Production path (stubbed) ===
        // 1. Load image/video frames from CAS
        // 2. Run colmap SfM (or monocular depth) to get camera poses
        // 3. Initialize 3DGS splats from point cloud
        // 4. Optimise splat parameters for N iterations
        // 5. Export as PLY or glTF with KHR_gaussian_splatting extension

        let placeholder = format!(
            "3DGS asset | format={output_format} points={num_points} input={input_key_hex}"
        );
        let mime = if output_format == "ply" {
            "model/ply"
        } else {
            "model/gltf-binary"
        };
        let key = self.cas.put(placeholder.as_bytes(), mime).await?;

        Ok(GenerationResult {
            cas_key: key,
            metadata: serde_json::json!({
                "model":         "3dgs",
                "num_points":    num_points,
                "output_format": output_format,
                "input_key":     input_key_hex,
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
        std::mem::forget(dir);
        Arc::new(store)
    }

    #[tokio::test]
    async fn test_requires_existing_input_asset() {
        let tool = GaussianSplattingTool::new(test_cas().await);
        let mut params = GenerateParams::default();
        params.extra = serde_json::json!({ "input_image_key": "cd".repeat(32) });

        assert!(tool.generate(params).await.is_err());
    }

    #[tokio::test]
    async fn test_default_point_count_and_format() {
        let cas = test_cas().await;
        let input_key = cas.put(b"scan frames", "video/mp4").await.unwrap();
        let tool = GaussianSplattingTool::new(cas.clone());

        let mut params = GenerateParams::default();
        params.extra = serde_json::json!({ "input_image_key": input_key.hex() });

        let result = tool.generate(params).await.unwrap();
        assert_eq!(result.metadata["num_points"], 500_000);
        assert_eq!(result.metadata["output_format"], "ply");
        assert!(cas.exists(&result.cas_key).await.unwrap());
    }
}
