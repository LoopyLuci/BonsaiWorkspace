//! Creator CLI - exercises the generative-tool orchestrator end to end

use cas::CasStore;
use creator::{CreatorOrchestrator, FluxDiTTool, GenerateParams, Guardian};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let tmp_dir = std::env::temp_dir().join("creator-cli-demo");
    let db_path = tmp_dir.join("cas.db");
    let blob_dir = tmp_dir.join("blobs");
    let cas = Arc::new(CasStore::open(&db_path, &blob_dir).await?);

    let orchestrator = CreatorOrchestrator::new();
    orchestrator.register("image", Arc::new(FluxDiTTool::new(cas.clone())));

    let guardian = Guardian::default();
    let prompt = "a scenic mountain landscape at sunset";
    guardian
        .check_prompt(prompt)
        .map_err(|e| anyhow::anyhow!(e))?;

    let params = GenerateParams {
        prompt: prompt.to_string(),
        modality: "image".to_string(),
        width: 256,
        height: 256,
        ..Default::default()
    };

    let result = orchestrator.generate(params).await?;
    println!("generated asset: {}", result.cas_key.hex());
    println!("metadata: {}", result.metadata);

    Ok(())
}
