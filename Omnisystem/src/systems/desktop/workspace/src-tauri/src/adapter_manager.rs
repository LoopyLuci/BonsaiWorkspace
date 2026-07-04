//! LoRA adapter hot-swapping for BonsAI.
//!
//! llama.cpp exposes a `/lora` endpoint (since mid-2024) that lets callers
//! swap adapter weights on a running model without reloading the base model.
//! This enables a single 7B model to behave as code, math, writing, or music
//! specialist — adapter switch takes <100ms.
//!
//! Adapters are stored in `~/.bonsai/adapters/` by default (overridden by
//! `AppConfig::adapters_dir`).  Each adapter is a `.gguf` or `.bin` file
//! accompanied by a sidecar `<name>.json` manifest describing its domain.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

// ── Adapter manifest ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterManifest {
    pub name: String,
    pub domain: String, // "code" | "math" | "music" | "creative" | "general"
    pub description: String,
    pub base_model: Option<String>, // model id/name this was trained on
    pub scale: Option<f32>,         // LoRA scale (0.0–1.0, default 1.0)
}

#[derive(Debug, Clone, Serialize)]
pub struct AdapterInfo {
    pub path: String,
    pub manifest: AdapterManifest,
    pub size_mb: f32,
}

// ── AdapterManager ────────────────────────────────────────────────────────────

pub struct AdapterManager {
    /// Per-slot adapter managers: slot_index → (slot_base_url, current_adapter)
    slots: Arc<Mutex<HashMap<usize, SlotAdapterState>>>,
    adapters_dir: PathBuf,
}

struct SlotAdapterState {
    base_url: String,
    current_adapter: Option<String>,
}

impl AdapterManager {
    pub fn new(adapters_dir: PathBuf) -> Self {
        Self {
            slots: Arc::new(Mutex::new(HashMap::new())),
            adapters_dir,
        }
    }

    /// Register a slot when it becomes ready.
    pub async fn register_slot(&self, slot_index: usize, base_url: String) {
        let mut slots = self.slots.lock().await;
        slots.insert(
            slot_index,
            SlotAdapterState {
                base_url,
                current_adapter: None,
            },
        );
    }

    /// Unregister a slot when it crashes or is evicted.
    pub async fn unregister_slot(&self, slot_index: usize) {
        self.slots.lock().await.remove(&slot_index);
    }

    /// Apply a LoRA adapter to the given slot. No-op if already applied.
    pub async fn apply(&self, slot_index: usize, adapter_path: &str) -> Result<(), String> {
        let mut slots = self.slots.lock().await;
        let slot = slots
            .get_mut(&slot_index)
            .ok_or_else(|| format!("Slot {slot_index} not registered"))?;

        if slot.current_adapter.as_deref() == Some(adapter_path) {
            return Ok(()); // already applied
        }

        let client = reqwest::Client::new();
        let resp = client
            .post(format!("{}/lora", slot.base_url))
            .json(&serde_json::json!({
                "lora": [{
                    "path":  adapter_path,
                    "scale": 1.0
                }]
            }))
            .send()
            .await
            .map_err(|e| format!("LoRA request failed: {e}"))?;

        if resp.status().is_success() {
            slot.current_adapter = Some(adapter_path.to_string());
            info!(slot=slot_index, adapter=%adapter_path, "[adapter] applied");
            Ok(())
        } else {
            let body = resp.text().await.unwrap_or_default();
            Err(format!("LoRA endpoint error: {body}"))
        }
    }

    /// Clear the active adapter on a slot (revert to base model).
    pub async fn clear(&self, slot_index: usize) -> Result<(), String> {
        let mut slots = self.slots.lock().await;
        let slot = slots
            .get_mut(&slot_index)
            .ok_or_else(|| format!("Slot {slot_index} not registered"))?;
        if slot.current_adapter.is_none() {
            return Ok(());
        }

        let client = reqwest::Client::new();
        let _ = client
            .post(format!("{}/lora", slot.base_url))
            .json(&serde_json::json!({ "lora": [] }))
            .send()
            .await;
        slot.current_adapter = None;
        Ok(())
    }

    /// Scan the adapters directory and return discovered adapters.
    pub fn scan(&self) -> Vec<AdapterInfo> {
        scan_adapters(&self.adapters_dir)
    }

    /// Find the best adapter for a given domain string.
    pub fn best_for_domain(&self, domain: &str) -> Option<AdapterInfo> {
        self.scan()
            .into_iter()
            .find(|a| a.manifest.domain == domain)
    }
}

pub fn scan_adapters(dir: &Path) -> Vec<AdapterInfo> {
    if !dir.exists() {
        return vec![];
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return vec![];
    };

    let mut result = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "gguf" && ext != "bin" {
            continue;
        }

        let size_mb = entry
            .metadata()
            .map(|m| m.len() as f32 / 1_048_576.0)
            .unwrap_or(0.0);

        // Load sidecar manifest or synthesize from filename
        let manifest_path = path.with_extension("json");
        let manifest = if manifest_path.exists() {
            std::fs::read_to_string(&manifest_path)
                .ok()
                .and_then(|s| serde_json::from_str::<AdapterManifest>(&s).ok())
        } else {
            None
        };

        let manifest = manifest.unwrap_or_else(|| {
            let name = path
                .file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();
            let domain = infer_domain_from_name(&name);
            AdapterManifest {
                name: name.clone(),
                domain,
                description: format!("Auto-detected adapter: {name}"),
                base_model: None,
                scale: None,
            }
        });

        result.push(AdapterInfo {
            path: path.to_string_lossy().into_owned(),
            manifest,
            size_mb,
        });
    }
    result
}

fn infer_domain_from_name(name: &str) -> String {
    let lower = name.to_lowercase();
    if lower.contains("code") || lower.contains("coder") {
        "code".into()
    } else if lower.contains("math") {
        "math".into()
    } else if lower.contains("music") || lower.contains("audio") {
        "music".into()
    } else if lower.contains("creative") || lower.contains("write") {
        "creative".into()
    } else {
        "general".into()
    }
}

// ── DPO training data export ──────────────────────────────────────────────────
//
// Converts self-play gaps into Direct Preference Optimization (DPO) triples.
// DPO is simpler than PPO/RLHF: no reward model, no RL — just a preference
// loss over (prompt, chosen, rejected) triples.

#[derive(Debug, Serialize)]
pub struct DpoTriple {
    pub prompt: String,
    pub chosen: String,   // better response (corrected)
    pub rejected: String, // worse response (original model output)
    pub source: &'static str,
}

/// Write DPO triples to a JSONL file alongside the alpaca examples.
pub fn write_dpo_triples(path: &Path, triples: &[DpoTriple]) -> std::io::Result<()> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    for triple in triples {
        writeln!(
            file,
            "{}",
            serde_json::to_string(triple).unwrap_or_default()
        )?;
    }
    Ok(())
}
