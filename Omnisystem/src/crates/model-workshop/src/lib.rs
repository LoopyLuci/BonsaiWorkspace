//! Model Workshop: a small axum backend for managing LLM/module "knowledge modules",
//! training datasets, model design configs, training jobs, and format conversion.
//!
//! The crate exposes an [`AppState`] shared across the HTTP handlers in
//! [`library`], [`datasets`], [`designer`], [`builder`], [`editor`],
//! [`converter`], and [`monitor`].

pub mod builder;
pub mod converter;
pub mod datasets;
pub mod designer;
pub mod editor;
pub mod error;
pub mod library;
pub mod monitor;

pub use error::{Error, Result};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// A registered knowledge module (a named bundle of text chunks used to
/// condition or fine-tune a model).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub num_chunks: usize,
    pub domains: Vec<String>,
    pub created_at: String,
}

/// A registered training dataset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetInfo {
    pub id: String,
    pub name: String,
    pub num_examples: usize,
    pub domains: Vec<String>,
    pub created_at: String,
}

/// A built/converted model artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub base_model: String,
    pub quantization: String,
    pub size_gb: f32,
    pub created_at: String,
}

/// A queued/running/completed training job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingJob {
    pub id: String,
    pub config: String,
    pub status: String,
    pub progress: f32,
    pub current_stage: u32,
    pub started_at: String,
    pub estimated_completion: String,
    pub logs: Vec<String>,
}

/// Shared application state for the Model Workshop HTTP API.
#[derive(Clone)]
pub struct AppState {
    pub modules: Arc<RwLock<HashMap<String, ModuleInfo>>>,
    pub datasets: Arc<RwLock<HashMap<String, DatasetInfo>>>,
    pub training_jobs: Arc<RwLock<Vec<TrainingJob>>>,
    pub models: Arc<RwLock<HashMap<String, ModelInfo>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            modules: Arc::new(RwLock::new(HashMap::new())),
            datasets: Arc::new(RwLock::new(HashMap::new())),
            training_jobs: Arc::new(RwLock::new(Vec::new())),
            models: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn app_state_starts_empty() {
        let state = AppState::new();
        assert!(state.modules.read().await.is_empty());
        assert!(state.datasets.read().await.is_empty());
        assert!(state.training_jobs.read().await.is_empty());
        assert!(state.models.read().await.is_empty());
    }

    #[tokio::test]
    async fn app_state_modules_roundtrip() {
        let state = AppState::new();
        let module = ModuleInfo {
            id: "mod-1".into(),
            name: "Test".into(),
            version: "1.0.0".into(),
            description: "desc".into(),
            num_chunks: 3,
            domains: vec!["general".into()],
            created_at: "now".into(),
        };
        state.modules.write().await.insert(module.id.clone(), module.clone());
        let read = state.modules.read().await;
        assert_eq!(read.get("mod-1").unwrap().num_chunks, 3);
    }
}
