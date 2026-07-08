//! Deterministic lockfile with content hashes

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use tokio::fs;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lockfile {
    pub packages: BTreeMap<String, LockedPackage>,
    pub runtimes: BTreeMap<String, LockedRuntime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedPackage {
    pub name: String,
    pub version: String,
    pub hash: String,
    pub language: String,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedRuntime {
    pub language: String,
    pub version: String,
    pub hash: String,
}

impl Lockfile {
    pub fn new() -> Self {
        Self {
            packages: BTreeMap::new(),
            runtimes: BTreeMap::new(),
        }
    }

    pub async fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path).await?;
        Ok(serde_json::from_str(&content)?)
    }

    pub async fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content).await?;
        Ok(())
    }

    pub fn add_package(&mut self, pkg: LockedPackage) {
        self.packages.insert(pkg.name.clone(), pkg);
    }

    pub fn add_runtime(&mut self, lang: String, runtime: LockedRuntime) {
        self.runtimes.insert(lang, runtime);
    }
}

impl Default for Lockfile {
    fn default() -> Self {
        Self::new()
    }
}
