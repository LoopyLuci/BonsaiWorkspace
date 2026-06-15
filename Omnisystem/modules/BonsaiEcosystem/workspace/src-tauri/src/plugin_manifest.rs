//! Plugin manifest types — parsed from bonsai-plugin.toml.
//! Separate from plugin_loader.rs (which handles disk discovery);
//! this module defines the canonical Capability enum and typed manifest.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    /// Write to the structured log.
    Log,
    /// Call the active inference model.
    CallModel,
    /// Read files from the workspace root.
    ReadWorkspace,
    /// Write files to the workspace root.
    WriteWorkspace,
    /// Execute sandboxed code via the venv tier.
    ExecuteCode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub abi_version: String,
    pub description: Option<String>,
    pub entrypoint: String,
    pub capabilities: HashSet<Capability>,
}

impl PluginManifest {
    /// Parse from a `bonsai-plugin.toml` string.
    pub fn from_toml(content: &str) -> Result<Self, String> {
        toml::from_str(content).map_err(|e| format!("Manifest parse error: {e}"))
    }

    /// True if the plugin requests only safe, read-only capabilities.
    pub fn is_read_only(&self) -> bool {
        self.capabilities.iter().all(|c| {
            matches!(
                c,
                Capability::Log | Capability::ReadWorkspace | Capability::CallModel
            )
        })
    }
}
