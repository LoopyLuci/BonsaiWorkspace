//! Bonsai Creator — multi-modal generative AI orchestration.
//!
//! Each modality is a [`GenerativeTool`] that accepts [`GenerateParams`] and
//! stores its output in the CAS, returning a [`GenerationResult`] with the key
//! and structured metadata.
//!
//! The [`CreatorOrchestrator`] maintains a registry of named tools.  The daemon
//! registers all tools at startup and dispatches `creator.generate` RPC calls
//! to the correct tool by matching `params.modality`.

pub mod image;
pub mod video;
pub mod three_d;
pub mod audio;
pub mod composer;
pub mod fine_tuning;
pub mod gaussian;
pub mod gaussian_pipeline;
pub mod model_fetch;
pub mod guardian;
pub mod progress;

use async_trait::async_trait;
use bonsai_cas::CasKey;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// ── Unified generation parameters ────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct GenerateParams {
    pub prompt: String,
    pub negative_prompt: Option<String>,
    #[serde(default = "default_dim")]
    pub width: u32,
    #[serde(default = "default_dim")]
    pub height: u32,
    #[serde(default = "default_steps")]
    pub steps: u32,
    #[serde(default = "default_guidance")]
    pub guidance_scale: f64,
    pub seed: Option<u64>,
    /// Which tool to invoke: "image", "video", "3d", "audio", "gaussian"
    pub modality: String,
    /// Modality-specific fields (e.g., `input_image_key`, `duration_sec`).
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

fn default_dim() -> u32 { 512 }
fn default_steps() -> u32 { 20 }
fn default_guidance() -> f64 { 7.5 }

// ── Generation result ─────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Debug)]
pub struct GenerationResult {
    pub cas_key: CasKey,
    pub metadata: serde_json::Value,
}

// ── GenerativeTool trait ──────────────────────────────────────────────────────

#[async_trait]
pub trait GenerativeTool: Send + Sync {
    async fn generate(&self, params: GenerateParams) -> anyhow::Result<GenerationResult>;
}

// ── CreatorOrchestrator ───────────────────────────────────────────────────────

pub struct CreatorOrchestrator {
    tools: tokio::sync::Mutex<Vec<(String, Arc<dyn GenerativeTool>)>>,
    pub cas: Arc<bonsai_cas::CasStore>,
}

impl CreatorOrchestrator {
    pub fn new(cas: Arc<bonsai_cas::CasStore>) -> Self {
        Self {
            tools: tokio::sync::Mutex::new(Vec::new()),
            cas,
        }
    }

    pub async fn register(&self, name: &str, tool: Arc<dyn GenerativeTool>) {
        self.tools.lock().await.push((name.to_string(), tool));
    }

    pub async fn get(&self, name: &str) -> Option<Arc<dyn GenerativeTool>> {
        self.tools.lock().await
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, t)| t.clone())
    }

    /// List registered tool names.
    pub async fn list_tools(&self) -> Vec<String> {
        self.tools.lock().await.iter().map(|(n, _)| n.clone()).collect()
    }
}
