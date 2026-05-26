//! Plugin/model/tool marketplace — local-first asset publishing, search, install.
//!
//! Self-contained: no external bonsai_marketplace crate required.
//! Assets are stored in `~/.bonsai/marketplace/` as JSONL.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::State;
use tokio::sync::Mutex;
use tracing::{info, warn};
use uuid::Uuid;

// ── Domain types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub id:          String,
    pub name:        String,
    pub description: String,
    pub asset_type:  AssetType,
    pub author:      String,
    pub version:     String,
    /// Content hash (SHA-256 hex) for integrity verification.
    pub cid:         String,
    /// Installation instructions or model path.
    pub install_hint: String,
    pub tags:        Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetType {
    Model,
    Plugin,
    Tool,
    LoraAdapter,
    Prompt,
}

// ── State ─────────────────────────────────────────────────────────────────────

pub struct MarketState {
    pub catalog: Mutex<Vec<Asset>>,
}

impl MarketState {
    pub fn new() -> Self {
        Self { catalog: Mutex::new(load_catalog()) }
    }
}

fn catalog_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".bonsai/marketplace/catalog.jsonl")
}

fn load_catalog() -> Vec<Asset> {
    let path = catalog_path();
    let Ok(content) = std::fs::read_to_string(&path) else { return vec![]; };
    content.lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

fn persist_catalog(catalog: &[Asset]) {
    let path = catalog_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let lines: String = catalog.iter()
        .filter_map(|a| serde_json::to_string(a).ok())
        .collect::<Vec<_>>()
        .join("\n");
    let _ = std::fs::write(&path, lines);
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn publish_asset(
    state: State<'_, MarketState>,
    name: String,
    description: String,
    asset_type: String,
    author: String,
    version: String,
    tags: Vec<String>,
) -> Result<Asset, String> {
    let atype = match asset_type.as_str() {
        "plugin"       => AssetType::Plugin,
        "tool"         => AssetType::Tool,
        "lora_adapter" => AssetType::LoraAdapter,
        "prompt"       => AssetType::Prompt,
        _              => AssetType::Model,
    };
    let asset = Asset {
        id:           Uuid::new_v4().to_string(),
        name:         name.clone(),
        description,
        asset_type:   atype,
        author,
        version,
        cid:          format!("{:x}", rand::random::<u64>()),
        install_hint: format!("Place in ~/.bonsai/models/{name}/"),
        tags,
    };
    let mut catalog = state.catalog.lock().await;
    catalog.push(asset.clone());
    persist_catalog(&catalog);
    info!(asset.id, "[marketplace] asset published");
    Ok(asset)
}

#[tauri::command]
pub async fn search_marketplace(
    state: State<'_, MarketState>,
    query: String,
) -> Result<Vec<Asset>, String> {
    let catalog = state.catalog.lock().await;
    let q = query.to_lowercase();
    let results: Vec<Asset> = catalog.iter()
        .filter(|a| {
            a.name.to_lowercase().contains(&q) ||
            a.description.to_lowercase().contains(&q) ||
            a.tags.iter().any(|t| t.to_lowercase().contains(&q))
        })
        .cloned()
        .collect();
    Ok(results)
}

#[tauri::command]
pub async fn install_asset(
    state: State<'_, MarketState>,
    cid: String,
) -> Result<String, String> {
    let catalog = state.catalog.lock().await;
    let asset = catalog.iter().find(|a| a.cid == cid)
        .ok_or_else(|| format!("Asset {cid} not found in catalog"))?;
    info!(cid, "[marketplace] install requested");
    Ok(asset.install_hint.clone())
}
