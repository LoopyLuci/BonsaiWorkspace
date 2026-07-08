use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::kdb_state::KdbAppState;
use kdb::module::ModuleInfo;

// ── Shared DTOs ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModuleInfoDto {
    pub id: String,
    pub name: String,
    pub domain: String,
    pub version: String,
    pub entry_count: usize,
    pub path: String,
}

impl From<ModuleInfo> for ModuleInfoDto {
    fn from(m: ModuleInfo) -> Self {
        ModuleInfoDto {
            id: m.id.to_string(),
            name: m.name,
            domain: m.domain,
            version: m.version,
            entry_count: m.entry_count,
            path: m.path.to_string_lossy().to_string(),
        }
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct RetrievedContextDto {
    pub module_name: String,
    pub entries: Vec<(String, f32)>,
}

// ── Commands ─────────────────────────────────────────────────────────────────

/// List all knowledge modules registered in the KDB store.
#[tauri::command]
pub async fn kdb_list_modules(state: State<'_, KdbAppState>) -> Result<Vec<ModuleInfoDto>, String> {
    let store = state.store.lock().await;
    let manifests = store.list_modules().map_err(|e| e.to_string())?;
    Ok(manifests
        .into_iter()
        .map(|m| {
            let dir = store.module_dir(&m.name);
            ModuleInfoDto {
                id: m.id.to_string(),
                name: m.name,
                domain: m.domain,
                version: m.version,
                entry_count: m.entry_count,
                path: dir.to_string_lossy().to_string(),
            }
        })
        .collect())
}

/// List all currently loaded (hot) modules in the retriever.
#[tauri::command]
pub async fn kdb_list_loaded_modules(
    state: State<'_, KdbAppState>,
) -> Result<Vec<ModuleInfoDto>, String> {
    let retriever = state.retriever.read().await;
    Ok(retriever
        .list_modules()
        .into_iter()
        .map(Into::into)
        .collect())
}

/// Load a module from disk into the live retriever.  The module must already
/// be registered in the store (`kdb_list_modules`).
#[tauri::command]
pub async fn kdb_load_module(
    state: State<'_, KdbAppState>,
    module_name: String,
) -> Result<(), String> {
    let dir = {
        let store = state.store.lock().await;
        store.module_dir(&module_name)
    };
    let mut retriever = state.retriever.write().await;
    retriever
        .load_module(&module_name, &dir)
        .map_err(|e| e.to_string())
}

/// Unload a module from the live retriever (does not delete from disk/store).
#[tauri::command]
pub async fn kdb_unload_module(
    state: State<'_, KdbAppState>,
    module_name: String,
) -> Result<bool, String> {
    let mut retriever = state.retriever.write().await;
    Ok(retriever.unload_module(&module_name))
}

/// Retrieve the top-k nearest context entries for a plain-text query.
/// The query is converted to a dummy zero-vector here; real embeddings require
/// the llama-server /embedding endpoint (see the Python pipeline).  Returns
/// whatever the loaded modules contain for demonstration / wiring purposes.
#[tauri::command]
pub async fn kdb_retrieve(
    state: State<'_, KdbAppState>,
    query_text: String,
    top_k: usize,
) -> Result<Vec<RetrievedContextDto>, String> {
    // Use a zero vector as a placeholder.  The real embedding call will be
    // routed through the inference engine once it exposes a /embedding API.
    let retriever = state.retriever.read().await;
    if retriever.is_empty() {
        return Ok(vec![]);
    }
    // 384-dim zero vector — deterministically returns the closest stored keys.
    let query_vec = vec![0.0f32; 384];
    let contexts = retriever.retrieve(&query_vec).map_err(|e| e.to_string())?;
    let top: Vec<RetrievedContextDto> = contexts
        .into_iter()
        .map(|c| RetrievedContextDto {
            module_name: c.module_name,
            entries: c.entries.into_iter().take(top_k).collect(),
        })
        .collect();
    Ok(top)
}

/// Format retrieved context as a system-prompt prefix string.
#[tauri::command]
pub async fn kdb_format_context(state: State<'_, KdbAppState>) -> Result<String, String> {
    let retriever = state.retriever.read().await;
    if retriever.is_empty() {
        return Ok(String::new());
    }
    let query_vec = vec![0.0f32; 384];
    retriever
        .format_context_prefix(&query_vec)
        .map_err(|e| e.to_string())
}

/// Delete a knowledge module from the store (does not remove files from disk).
#[tauri::command]
pub async fn kdb_delete_module(
    state: State<'_, KdbAppState>,
    module_name: String,
) -> Result<(), String> {
    // Unload from retriever first if loaded
    {
        let mut retriever = state.retriever.write().await;
        retriever.unload_module(&module_name);
    }
    let store = state.store.lock().await;
    store
        .unregister_module(&module_name)
        .map_err(|e| e.to_string())
}
