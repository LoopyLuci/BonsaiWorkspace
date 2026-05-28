//! SylvaComposer — multi-step generative pipelines expressed as Sylva scripts.
//!
//! A Sylva script can call `generate_image(prompt)`, `generate_video(key)`,
//! `generate_audio(prompt)` etc.; each call goes through the [`CreatorOrchestrator`]
//! and the outputs are linked through CAS keys so pipelines compose naturally.

use crate::{CreatorOrchestrator, GenerateParams, GenerationResult};
use std::sync::Arc;

pub struct SylvaComposer {
    pub orchestrator: Arc<CreatorOrchestrator>,
    pub tool_registry: Arc<bonsai_tool_registry::ToolRegistry>,
}

impl SylvaComposer {
    pub fn new(
        orchestrator: Arc<CreatorOrchestrator>,
        tool_registry: Arc<bonsai_tool_registry::ToolRegistry>,
    ) -> Self {
        Self { orchestrator, tool_registry }
    }

    /// Execute a Sylva script whose built-in functions are wired to the creator
    /// orchestrator.  The final expression value is interpreted as a CAS key.
    pub async fn execute(&self, script: String) -> anyhow::Result<GenerationResult> {
        let orchestrator = self.orchestrator.clone();

        // Build a Sylva VM with creator tool-function bindings.
        let tool_fn: bonsai_sylva::vm::ToolFn = Arc::new(move |method: String, args: serde_json::Value| {
            let orch = orchestrator.clone();
            // Parse modality from method name, e.g. "generate_image" → "image"
            let modality = method.trim_start_matches("generate_").to_string();
            let prompt = args["prompt"].as_str().unwrap_or("").to_string();
            let result = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async move {
                    let tool = orch.get(&modality).await
                        .ok_or_else(|| anyhow::anyhow!("no tool for modality: {modality}"))?;
                    let params = GenerateParams {
                        prompt,
                        negative_prompt: None,
                        width: 512,
                        height: 512,
                        steps: 20,
                        guidance_scale: 7.5,
                        seed: None,
                        modality: modality.clone(),
                        extra: args.clone(),
                    };
                    tool.generate(params).await
                })
            });
            result
                .map(|r| serde_json::json!({ "cas_key": r.cas_key.hex(), "metadata": r.metadata }))
                .map_err(|e| bonsai_sylva::VmError::ToolCallFailed(e.to_string()))
        });

        let mut vm = bonsai_sylva::SylvaVm::with_tool_fn(tool_fn);
        bonsai_sylva::stdlib::register_stdlib(&mut vm);

        let result_val = vm.eval_str(&script).map_err(|e| anyhow::anyhow!("{e}"))?;
        let json = result_val.to_json();

        // Expect the script to return something with a cas_key field, or wrap raw output.
        let cas_key_hex = json["cas_key"].as_str()
            .ok_or_else(|| anyhow::anyhow!("script must return {{cas_key, metadata}}"))?;
        let cas_key = bonsai_cas::CasKey::from_hex(cas_key_hex)
            .map_err(|e| anyhow::anyhow!("invalid cas_key in script result: {e}"))?;

        Ok(GenerationResult {
            cas_key,
            metadata: serde_json::json!({
                "type":          "sylva_composed",
                "script_length": script.len(),
                "result":        json,
            }),
        })
    }
}
