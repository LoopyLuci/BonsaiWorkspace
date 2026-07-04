//! Runtime manifest format and verification

use serde::{Deserialize, Serialize};
use anyhow::{anyhow, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeManifest {
    pub runtimes: Vec<RuntimeEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeEntry {
    pub name: String,
    pub version: String,
    pub platform: String,
    pub url: String,
    pub hash: String, // "blake3:hex..."
    #[serde(default)]
    pub signature: String,
    #[serde(default)]
    pub compressed: bool,
    #[serde(default)]
    pub build_script: Option<String>,
}

impl RuntimeEntry {
    pub fn full_id(&self) -> String {
        format!("{}@{}", self.name, self.version)
    }

    pub fn hash_without_algo(&self) -> Result<String> {
        let parts: Vec<&str> = self.hash.split(':').collect();
        if parts.len() != 2 {
            return Err(anyhow!("Invalid hash format: {}", self.hash));
        }
        Ok(parts[1].to_string())
    }

    pub fn verify_hash(&self, data: &[u8]) -> Result<bool> {
        let expected = self.hash_without_algo()?;
        let actual = blake3::hash(data).to_hex().to_string();
        Ok(actual == expected)
    }
}

impl RuntimeManifest {
    pub fn from_toml(content: &str) -> Result<Self> {
        Ok(toml::from_str(content)?)
    }

    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::from_toml(&content)
    }

    pub fn find(&self, name: &str, version: &str) -> Option<RuntimeEntry> {
        self.runtimes
            .iter()
            .find(|r| r.name == name && r.version == version)
            .cloned()
    }

    pub fn all_for_language(&self, name: &str) -> Vec<RuntimeEntry> {
        self.runtimes
            .iter()
            .filter(|r| r.name == name)
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_runtime_manifest() {
        let toml = r#"
[[runtimes]]
name = "python"
version = "3.11.9"
platform = "x86_64-unknown-linux-gnu"
url = "https://example.com/python-3.11.9.tar.xz"
hash = "blake3:0a1b2c3d4e5f"
compressed = true
"#;
        let manifest = RuntimeManifest::from_toml(toml).unwrap();
        assert_eq!(manifest.runtimes.len(), 1);
        assert_eq!(manifest.runtimes[0].name, "python");
        assert_eq!(manifest.runtimes[0].version, "3.11.9");
    }

    #[test]
    fn test_find_runtime() {
        let toml = r#"
[[runtimes]]
name = "python"
version = "3.11.9"
platform = "x86_64-unknown-linux-gnu"
url = "https://example.com/python-3.11.9.tar.xz"
hash = "blake3:0a1b2c3d4e5f"

[[runtimes]]
name = "python"
version = "3.12.0"
platform = "x86_64-unknown-linux-gnu"
url = "https://example.com/python-3.12.0.tar.xz"
hash = "blake3:1a2b3c4d5e6f"
"#;
        let manifest = RuntimeManifest::from_toml(toml).unwrap();
        let py311 = manifest.find("python", "3.11.9").unwrap();
        assert_eq!(py311.full_id(), "python@3.11.9");

        let py_all = manifest.all_for_language("python");
        assert_eq!(py_all.len(), 2);
    }
}
