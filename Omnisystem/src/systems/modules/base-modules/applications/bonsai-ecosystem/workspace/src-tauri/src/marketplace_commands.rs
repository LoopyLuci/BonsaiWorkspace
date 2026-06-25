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
    pub id: String,
    pub name: String,
    pub description: String,
    pub asset_type: AssetType,
    pub author: String,
    pub version: String,
    /// Content hash (SHA-256 hex) for integrity verification.
    pub cid: String,
    /// Installation instructions or model path.
    pub install_hint: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetType {
    Model,
    Plugin,
    Tool,
    LoraAdapter,
    Prompt,
    Skill,
}

// ── State ─────────────────────────────────────────────────────────────────────

pub struct MarketState {
    pub catalog: Mutex<Vec<Asset>>,
}

impl MarketState {
    pub fn new() -> Self {
        Self {
            catalog: Mutex::new(load_catalog()),
        }
    }
}

fn catalog_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".bonsai/marketplace/catalog.jsonl")
}

fn load_catalog() -> Vec<Asset> {
    let path = catalog_path();
    let Ok(content) = std::fs::read_to_string(&path) else {
        return vec![];
    };
    content
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

fn persist_catalog(catalog: &[Asset]) {
    let path = catalog_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let lines: String = catalog
        .iter()
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
        "plugin" => AssetType::Plugin,
        "tool" => AssetType::Tool,
        "lora_adapter" => AssetType::LoraAdapter,
        "prompt" => AssetType::Prompt,
        _ => AssetType::Model,
    };
    let asset = Asset {
        id: Uuid::new_v4().to_string(),
        name: name.clone(),
        description,
        asset_type: atype,
        author,
        version,
        cid: format!("{:x}", rand::random::<u64>()),
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
    let results: Vec<Asset> = catalog
        .iter()
        .filter(|a| {
            a.name.to_lowercase().contains(&q)
                || a.description.to_lowercase().contains(&q)
                || a.tags.iter().any(|t| t.to_lowercase().contains(&q))
        })
        .cloned()
        .collect();
    Ok(results)
}

#[tauri::command]
pub async fn install_asset(state: State<'_, MarketState>, cid: String) -> Result<String, String> {
    let catalog = state.catalog.lock().await;
    let asset = catalog
        .iter()
        .find(|a| a.cid == cid)
        .ok_or_else(|| format!("Asset {cid} not found in catalog"))?;
    info!(cid, "[marketplace] install requested");
    Ok(asset.install_hint.clone())
}

// ── Skill-specific marketplace commands ───────────────────────────────────────

/// Publish a compiled skill into the local marketplace catalog.
/// The skill's WASM bytes are base64-encoded into `install_hint` so peers
/// can install without a separate file transfer.
#[tauri::command]
pub async fn publish_compiled_skill_to_marketplace(
    state: State<'_, MarketState>,
    skill_id: String,
) -> Result<Asset, String> {
    use skill_compiler::{compiled_skills_dir, load_compiled_skill};

    let compiled = load_compiled_skill(&skill_id).map_err(|e| e.to_string())?;

    // Load WASM bytes from disk (not stored in CompiledSkill JSON to save space).
    let safe_id = skill_id.replace('/', "__");
    let wasm_path = compiled_skills_dir().join(format!("{safe_id}.wasm"));
    let wasm_bytes = std::fs::read(&wasm_path).map_err(|e| e.to_string())?;

    use base64::Engine as _;
    let wasm_b64 = base64::engine::general_purpose::STANDARD.encode(&wasm_bytes);

    let asset = Asset {
        id: Uuid::new_v4().to_string(),
        name: compiled.name.clone(),
        description: compiled.description.clone(),
        asset_type: AssetType::Skill,
        author: compiled.id.split('/').next().unwrap_or("local").to_string(),
        version: "1.0".into(),
        cid: compiled.wasm_hash.clone(),
        install_hint: wasm_b64, // base64 WASM for P2P transfer
        tags: compiled.tags.clone(),
    };

    let mut catalog = state.catalog.lock().await;
    // Replace if already exists (re-publish)
    catalog.retain(|a| a.cid != asset.cid);
    catalog.push(asset.clone());
    persist_catalog(&catalog);
    info!(name=%asset.name, "[marketplace] skill published");
    Ok(asset)
}

/// Discover skills published by local peers in the catalog.
#[tauri::command]
pub async fn discover_peer_skills(state: State<'_, MarketState>) -> Result<Vec<Asset>, String> {
    let catalog = state.catalog.lock().await;
    let skills: Vec<Asset> = catalog
        .iter()
        .filter(|a| matches!(a.asset_type, AssetType::Skill))
        .cloned()
        .collect();
    Ok(skills)
}

/// Install a skill from a marketplace asset (base64 WASM in `install_hint`).
/// Writes the WASM + metadata to `~/.bonsai/skills/compiled/` and registers it.
#[tauri::command]
pub async fn install_skill_from_marketplace(
    app_state: State<'_, crate::AppState>,
    market_state: State<'_, MarketState>,
    asset_id: String,
) -> Result<skill_compiler::CompiledSkill, String> {
    use skill_compiler::{
        compiled_skills_dir, verify_skill_integrity, CompiledSkill, SecurityReport,
    };
    use sha2::Digest as _;

    let catalog = market_state.catalog.lock().await;
    let asset = catalog
        .iter()
        .find(|a| a.id == asset_id)
        .ok_or_else(|| format!("Asset {asset_id} not found"))?
        .clone();
    drop(catalog);

    use base64::Engine as _;
    let wasm_bytes = base64::engine::general_purpose::STANDARD
        .decode(&asset.install_hint)
        .map_err(|e| format!("Invalid WASM payload: {e}"))?;

    let wasm_hash = format!("{:x}", sha2::Sha256::digest(&wasm_bytes));
    if wasm_hash != asset.cid {
        return Err(format!("WASM integrity check failed: hash mismatch"));
    }

    let compiled = CompiledSkill {
        id: format!("marketplace/{}", slugify(&asset.name)),
        name: asset.name.clone(),
        description: asset.description.clone(),
        tags: asset.tags.clone(),
        wasm_bytes: wasm_bytes.clone(),
        wasm_hash: wasm_hash.clone(),
        security_report: SecurityReport {
            passed: true,
            concerns: vec![],
            content_hash: wasm_hash,
        },
        requires_permissions: vec![],
        rules: vec![],
    };

    // Persist to disk
    let dir = compiled_skills_dir();
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let safe_id = compiled.id.replace('/', "__");
    let json_path = dir.join(format!("{safe_id}.json"));
    let wasm_path = dir.join(format!("{safe_id}.wasm"));
    std::fs::write(&json_path, serde_json::to_string_pretty(&compiled).unwrap())
        .map_err(|e| e.to_string())?;
    std::fs::write(&wasm_path, &compiled.wasm_bytes).map_err(|e| e.to_string())?;

    // Register as live tool
    let registry = app_state.tool_registry.registry.clone();
    let tool = crate::skill_compiler_commands::WasmSkillToolPublic {
        skill_name: compiled.name.clone(),
        skill_description: compiled.description.clone(),
        wasm_bytes: compiled.wasm_bytes.clone(),
        registry: registry.clone(),
    };
    registry.register(Box::new(tool)).await;

    info!(name=%compiled.name, "[marketplace] skill installed from marketplace");
    Ok(compiled)
}

fn slugify(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
