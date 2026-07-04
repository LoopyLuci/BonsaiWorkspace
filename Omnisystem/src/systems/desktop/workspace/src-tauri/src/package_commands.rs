use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::kdb_state::KdbAppState;
use package::{
    manifest::{BaseModelInfo, PackageManifest},
    PackageReader, PackageWriter,
};

// ── DTOs ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PackageSummary {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub base_model_name: String,
    pub base_model_arch: String,
    pub adapter_count: usize,
    pub module_count: usize,
    pub created_at: String,
}

impl From<&PackageManifest> for PackageSummary {
    fn from(m: &PackageManifest) -> Self {
        PackageSummary {
            id: m.id.to_string(),
            name: m.name.clone(),
            version: m.version.clone(),
            description: m.description.clone(),
            base_model_name: m.base_model.name.clone(),
            base_model_arch: m.base_model.arch.clone(),
            adapter_count: m.adapters.len(),
            module_count: m.knowledge_modules.len(),
            created_at: m.created_at.to_rfc3339(),
        }
    }
}

// ── Commands ─────────────────────────────────────────────────────────────────

/// Inspect a .bkp file and return its manifest summary without importing it.
#[tauri::command]
pub async fn package_inspect(path: String) -> Result<PackageSummary, String> {
    let mut reader = PackageReader::open(Path::new(&path)).map_err(|e| e.to_string())?;
    Ok(PackageSummary::from(&reader.manifest))
}

/// Extract a .bkp package into `~/.bonsai/`.
/// - Base model → `~/.bonsai/models/<base_model_name>/`
/// - Knowledge modules → `~/.bonsai/kdb/modules/<module_name>/`
/// - Adapters → `~/.bonsai/adapters/<adapter_name>/`
///
/// After extraction the modules are registered in the KDB store.
#[tauri::command]
pub async fn package_import(
    state: State<'_, KdbAppState>,
    path: String,
) -> Result<PackageSummary, String> {
    let bonsai_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".bonsai");

    let mut reader = PackageReader::open(Path::new(&path)).map_err(|e| e.to_string())?;
    let summary = PackageSummary::from(&reader.manifest);

    // Clone paths/names before calling mutable extract methods to avoid borrow conflict.
    let base_name = reader.manifest.base_model.name.clone();
    let base_pkg_path = reader.manifest.base_model.path_in_package.clone();

    let model_dest = bonsai_dir.join("models").join(&base_name);
    reader
        .extract_prefix(&base_pkg_path, &model_dest)
        .map_err(|e| e.to_string())?;

    // Extract adapters
    let adapters = reader.manifest.adapters.clone();
    for adapter in &adapters {
        let dest = bonsai_dir.join("adapters").join(&adapter.name);
        reader
            .extract_prefix(&adapter.path_in_package, &dest)
            .map_err(|e| e.to_string())?;
    }

    // Extract knowledge modules and register in KDB store
    let modules = reader.manifest.knowledge_modules.clone();
    for kmod in &modules {
        let dest = bonsai_dir.join("kdb").join("modules").join(&kmod.name);
        reader
            .extract_prefix(&kmod.path_in_package, &dest)
            .map_err(|e| e.to_string())?;

        // Register in store (load manifest.json from the extracted dir)
        let manifest_path = dest.join("manifest.json");
        if manifest_path.exists() {
            if let Ok(raw) = std::fs::read_to_string(&manifest_path) {
                if let Ok(kmod_manifest) =
                    serde_json::from_str::<kdb::module::ModuleManifest>(&raw)
                {
                    let store = state.store.lock().await;
                    let _ = store.register_module(&kmod_manifest, &dest);
                }
            }
        }
    }

    Ok(summary)
}

/// Export the currently active model configuration as a .bkp package.
/// `output_path` must end in `.bkp`.
#[tauri::command]
pub async fn package_export(
    base_model_path: String,
    base_model_name: String,
    base_model_arch: String,
    package_name: String,
    package_version: String,
    description: String,
    output_path: String,
) -> Result<String, String> {
    let base_info = BaseModelInfo {
        name: base_model_name.clone(),
        arch: base_model_arch,
        quant: "unknown".into(),
        blake3: String::new(),
        size_bytes: std::fs::metadata(&base_model_path)
            .map(|m| m.len())
            .unwrap_or(0),
        path_in_package: format!("base_model/{}", base_model_name),
    };

    let mut manifest = PackageManifest::new(&package_name, &package_version, base_info);
    manifest.description = description;

    let out = Path::new(&output_path);
    let mut writer = PackageWriter::create(out, manifest).map_err(|e| e.to_string())?;

    // Add the base model file/directory
    let src = Path::new(&base_model_path);
    if src.is_file() {
        writer
            .add_file(src, &format!("base_model/{}", base_model_name))
            .map_err(|e| e.to_string())?;
    } else if src.is_dir() {
        writer
            .add_dir(src, &format!("base_model/{}", base_model_name))
            .map_err(|e| e.to_string())?;
    }

    writer.finish().map_err(|e| e.to_string())?;
    Ok(output_path)
}

/// List the modules inside an already-loaded package (without importing).
#[tauri::command]
pub async fn package_list_entries(path: String) -> Result<Vec<String>, String> {
    let mut reader = PackageReader::open(Path::new(&path)).map_err(|e| e.to_string())?;
    reader.list_entries().map_err(|e| e.to_string())
}

/// Verify the BLAKE3 integrity of the base model file inside a .bkp package.
#[tauri::command]
pub async fn package_verify(path: String) -> Result<bool, String> {
    let mut reader = PackageReader::open(Path::new(&path)).map_err(|e| e.to_string())?;
    let base_path = reader.manifest.base_model.path_in_package.clone();
    let expected = reader.manifest.base_model.blake3.clone();
    if expected.is_empty() {
        return Ok(true); // no hash to verify
    }
    reader
        .verify_entry(&base_path, &expected)
        .map_err(|e| e.to_string())
}
