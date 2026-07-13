/// Plugin system for extending the linter with custom rules and grammars.
/// Plugins are distributed as .bkp (Bonsai Knowledge Package) files.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plugin {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub capabilities: Vec<PluginCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginCapability {
    Grammar { language: String },
    RuleSet { category: String },
    LspServer { language: String },
}

/// Plugin manifest (plugin.yaml inside .bkp file).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub plugin: Plugin,
    pub rules_dir: Option<String>,
    pub grammar_file: Option<String>,
    pub lsp_command: Option<String>,
}

impl PluginManifest {
    pub fn from_file(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        serde_yaml::from_str(&content).map_err(|e| anyhow::anyhow!("Failed to parse plugin manifest: {}", e))
    }
}

/// Plugin registry for managing loaded plugins.
pub struct PluginRegistry {
    plugins: Vec<Plugin>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self { plugins: Vec::new() }
    }

    pub fn load_from_directory(dir: &PathBuf) -> Result<Self> {
        let mut registry = Self::new();

        if dir.exists() {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().map(|e| e == "bkp").unwrap_or(false) {
                    match Self::load_plugin(&path) {
                        Ok(plugin) => registry.plugins.push(plugin),
                        Err(e) => tracing::warn!("Failed to load plugin {:?}: {:?}", path, e),
                    }
                }
            }
        }

        Ok(registry)
    }

    fn load_plugin(path: &PathBuf) -> Result<Plugin> {
        // In production, this would unzip the .bkp file and load the plugin.yaml
        // For now, return a placeholder
        Ok(Plugin {
            id: uuid::Uuid::new_v4().to_string(),
            name: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
            version: "1.0.0".to_string(),
            description: None,
            author: None,
            capabilities: Vec::new(),
        })
    }

    pub fn plugins(&self) -> &[Plugin] {
        &self.plugins
    }

    pub fn grammars(&self) -> Vec<(String, String)> {
        self.plugins
            .iter()
            .flat_map(|p| {
                p.capabilities.iter().filter_map(|cap| {
                    if let PluginCapability::Grammar { language } = cap {
                        Some((language.clone(), p.id.clone()))
                    } else {
                        None
                    }
                })
            })
            .collect()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_registry_creation() {
        let registry = PluginRegistry::new();
        assert_eq!(registry.plugins().len(), 0);
    }

    #[test]
    fn test_plugin_manifest_serialization() -> Result<()> {
        let manifest = PluginManifest {
            plugin: Plugin {
                id: "test-plugin".to_string(),
                name: "Test Plugin".to_string(),
                version: "1.0.0".to_string(),
                description: Some("A test plugin".to_string()),
                author: Some("Test Author".to_string()),
                capabilities: vec![PluginCapability::Grammar {
                    language: "rust".to_string(),
                }],
            },
            rules_dir: Some("rules".to_string()),
            grammar_file: Some("grammar.so".to_string()),
            lsp_command: None,
        };

        let yaml = serde_yaml::to_string(&manifest)?;
        assert!(yaml.contains("test-plugin"));
        Ok(())
    }
}
