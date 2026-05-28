//! Trellis3DTool — image/text-to-3D via the TRELLIS pipeline.
//!
//! Production path: call the TRELLIS Python sidecar (or Rust bindings once
//! available) which runs SLAT-based 3D generation, exporting a GLB mesh or
//! PLY Gaussian splat.  For now a tagged placeholder is stored in CAS.

use crate::{GenerateParams, GenerationResult, GenerativeTool};
use async_trait::async_trait;
use bonsai_cas::CasStore;
use std::sync::Arc;

pub struct Trellis3DTool {
    pub cas: Arc<CasStore>,
}

impl Trellis3DTool {
    pub fn new(cas: Arc<CasStore>) -> Self { Self { cas } }
}

#[async_trait]
impl GenerativeTool for Trellis3DTool {
    async fn generate(&self, params: GenerateParams) -> anyhow::Result<GenerationResult> {
        let input_key_hex = params.extra["input_image_key"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing input_image_key for TRELLIS"))?;

        let input_key = bonsai_cas::CasKey::from_hex(input_key_hex)
            .map_err(|e| anyhow::anyhow!("invalid CAS key: {e}"))?;
        if !self.cas.exists(&input_key).await? {
            return Err(anyhow::anyhow!("input image not found in CAS"));
        }

        let format = params.extra["output_format"].as_str().unwrap_or("glb");

        // === Production path (stubbed) ===
        // 1. Load image from CAS
        // 2. Call TRELLIS sidecar: `trellis_infer --image <path> --output <path>`
        // 3. Read result GLB/PLY bytes
        // 4. Store in CAS

        let placeholder = format!(
            "TRELLIS-2 placeholder 3D asset | format={format} | prompt={}",
            params.prompt
        );
        let mime = if format == "ply" { "model/ply" } else { "model/gltf-binary" };
        let key = self.cas.put(placeholder.as_bytes(), mime).await?;

        Ok(GenerationResult {
            cas_key: key,
            metadata: serde_json::json!({
                "model":      "trellis-2",
                "format":     format,
                "input_key":  input_key_hex,
                "prompt":     params.prompt,
            }),
        })
    }
}
