use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Deserialize, Serialize)]
pub struct BonsaiPlugin {
    pub plugin: PluginMeta,
    pub agent: Option<AgentDef>,
    pub security: SecurityPolicy,
}

#[derive(Deserialize, Serialize)]
pub struct PluginMeta {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub license: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct AgentDef {
    pub trait_name: String,
    pub capabilities: Vec<String>,
}

#[derive(Deserialize, Serialize)]
pub struct SecurityPolicy {
    pub sandbox: String,
    pub permissions: Vec<String>,
}

pub fn load_plugin(path: &Path) -> Result<BonsaiPlugin, String> {
    let manifest = path.join("bonsai-plugin.toml");
    if !manifest.exists() {
        return Err("bonsai-plugin.toml not found".into());
    }
    let content = std::fs::read_to_string(&manifest).map_err(|e| e.to_string())?;
    toml::from_str(&content).map_err(|e| format!("TOML parse: {e}"))
}

pub fn list_plugins(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().join("bonsai-plugin.toml").exists())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default()
}

#[tauri::command]
pub fn list_plugins_cmd() -> Vec<String> {
    list_plugins(
        &dirs::data_local_dir()
            .unwrap_or_default()
            .join("com.bonsai.workspace")
            .join("plugins"),
    )
}

#[tauri::command]
pub fn load_plugin_cmd(path: String) -> Result<String, String> {
    load_plugin(Path::new(&path)).map(|p| serde_json::to_string_pretty(&p).unwrap_or_default())
}
