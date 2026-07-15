//! Core generative-tool types: the parameter/result shapes every tool in
//! this crate speaks, the [`GenerativeTool`] trait each tool implements,
//! and [`CreatorOrchestrator`], a registry that dispatches a generation
//! request to the tool registered for its modality.

use async_trait::async_trait;
use cas::CasKey;
use dashmap::DashMap;
use std::sync::Arc;

/// Parameters for a single generation request
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GenerateParams {
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub width: u32,
    pub height: u32,
    pub steps: u32,
    pub guidance_scale: f32,
    pub seed: Option<u64>,
    /// Which tool this request is routed to (e.g. "image", "audio", "video")
    pub modality: String,
    /// Modality-specific parameters (e.g. `duration_sec`, `input_image_key`)
    pub extra: serde_json::Value,
}

impl Default for GenerateParams {
    fn default() -> Self {
        Self {
            prompt: String::new(),
            negative_prompt: None,
            width: 512,
            height: 512,
            steps: 20,
            guidance_scale: 7.5,
            seed: None,
            modality: String::new(),
            extra: serde_json::Value::Null,
        }
    }
}

/// Result of a generation request: the CAS key of the generated asset plus
/// arbitrary metadata describing how it was produced
#[derive(Debug, Clone)]
pub struct GenerationResult {
    pub cas_key: CasKey,
    pub metadata: serde_json::Value,
}

/// Trait implemented by every generative tool (image, audio, video, 3D, ...)
#[async_trait]
pub trait GenerativeTool: Send + Sync {
    async fn generate(&self, params: GenerateParams) -> anyhow::Result<GenerationResult>;
}

/// Registry that dispatches a [`GenerateParams::modality`] to its
/// registered [`GenerativeTool`]
#[derive(Default)]
pub struct CreatorOrchestrator {
    tools: DashMap<String, Arc<dyn GenerativeTool>>,
}

impl CreatorOrchestrator {
    pub fn new() -> Self {
        Self {
            tools: DashMap::new(),
        }
    }

    /// Register a tool under a modality name (e.g. "image", "audio")
    pub fn register(&self, modality: impl Into<String>, tool: Arc<dyn GenerativeTool>) {
        self.tools.insert(modality.into(), tool);
    }

    /// Look up the tool registered for a modality, if any
    pub async fn get(&self, modality: &str) -> Option<Arc<dyn GenerativeTool>> {
        self.tools.get(modality).map(|entry| entry.value().clone())
    }

    /// Dispatch a request to the tool registered for `params.modality`
    pub async fn generate(&self, params: GenerateParams) -> anyhow::Result<GenerationResult> {
        let tool = self.get(&params.modality).await.ok_or_else(|| {
            anyhow::anyhow!("no generative tool registered for modality: {}", params.modality)
        })?;
        tool.generate(params).await
    }

    /// Number of registered tools
    pub fn tool_count(&self) -> usize {
        self.tools.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EchoTool;

    #[async_trait]
    impl GenerativeTool for EchoTool {
        async fn generate(&self, params: GenerateParams) -> anyhow::Result<GenerationResult> {
            Ok(GenerationResult {
                cas_key: CasKey::from_hex(&"ab".repeat(32)).unwrap(),
                metadata: serde_json::json!({ "prompt": params.prompt }),
            })
        }
    }

    #[tokio::test]
    async fn test_register_and_get() {
        let orchestrator = CreatorOrchestrator::new();
        orchestrator.register("echo", Arc::new(EchoTool));

        assert_eq!(orchestrator.tool_count(), 1);
        assert!(orchestrator.get("echo").await.is_some());
        assert!(orchestrator.get("missing").await.is_none());
    }

    #[tokio::test]
    async fn test_generate_dispatches_to_registered_tool() {
        let orchestrator = CreatorOrchestrator::new();
        orchestrator.register("echo", Arc::new(EchoTool));

        let params = GenerateParams {
            prompt: "hello".to_string(),
            modality: "echo".to_string(),
            ..Default::default()
        };

        let result = orchestrator.generate(params).await.unwrap();
        assert_eq!(result.metadata["prompt"], "hello");
    }

    #[tokio::test]
    async fn test_generate_unknown_modality_errors() {
        let orchestrator = CreatorOrchestrator::new();
        let params = GenerateParams {
            modality: "nonexistent".to_string(),
            ..Default::default()
        };

        let result = orchestrator.generate(params).await;
        assert!(result.is_err());
    }
}
