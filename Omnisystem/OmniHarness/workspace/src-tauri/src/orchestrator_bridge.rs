//! Optional bridge to the Python OmniHarness orchestrator's cloud model
//! catalog (FastAPI, default http://127.0.0.1:8080 — the same service the
//! VS Code extension's OmniHarnessClient talks to). Workspace has its own,
//! entirely separate hardware-aware local model routing (`smart_router.rs`);
//! this doesn't touch that — it only gives the user visibility into
//! provider-backed cloud models (anthropic/openai/etc.) the orchestrator has
//! configured, without workspace needing to reimplement those provider
//! adapters or duplicate API key management. Degrades to an empty list when
//! the orchestrator isn't running — same graceful-degradation contract as
//! kernel_bridge.rs.

use serde::{Deserialize, Serialize};

const DEFAULT_ORCHESTRATOR_URL: &str = "http://127.0.0.1:8080";

// Mirrors orchestrator/omniharness/models/base.py's ModelInfo exactly —
// note this is a DIFFERENT schema than the kernel's proto ModelInfo
// (kernel_bridge.rs), which has display_name/available fields this one
// doesn't; the two "ModelInfo"s are unrelated despite the shared name.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OrchestratorModel {
    pub id: String,
    pub provider: String,
    #[serde(default)]
    pub context_window: i64,
    #[serde(default)]
    pub supports_tools: bool,
    #[serde(default)]
    pub supports_vision: bool,
    #[serde(default)]
    pub description: String,
}

#[derive(Deserialize)]
struct ModelsResponse {
    models: Vec<OrchestratorModel>,
}

/// Fetches the orchestrator's known model list. Returns an empty vec (never
/// an error the caller has to handle specially) when it's not reachable —
/// this is expected/normal, not a fault condition.
#[tauri::command]
pub async fn list_orchestrator_models() -> Vec<OrchestratorModel> {
    let base = std::env::var("OMNIHARNESS_ORCHESTRATOR_URL")
        .unwrap_or_else(|_| DEFAULT_ORCHESTRATOR_URL.to_string());
    let url = format!("{base}/api/models");

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(1500))
        .build()
    {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => resp
            .json::<ModelsResponse>()
            .await
            .map(|r| r.models)
            .unwrap_or_default(),
        _ => vec![],
    }
}
