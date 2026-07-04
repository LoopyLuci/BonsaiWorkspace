use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

use tracing::{info, warn};

use crate::module::{LoadedModule, ModuleInfo};
use crate::{KdbError, Result};

/// Hot-swappable KDB retriever. Modules can be loaded/unloaded at runtime.
pub struct KdbRetriever {
    modules: Arc<RwLock<HashMap<String, LoadedModule>>>,
    dim: usize,
    top_k: usize,
}

#[derive(Debug, Clone)]
pub struct RetrievedContext {
    pub module_name: String,
    pub entries: Vec<(String, f32)>, // (value, distance)
}

impl KdbRetriever {
    pub fn new(dim: usize, top_k: usize) -> Self {
        KdbRetriever {
            modules: Arc::new(RwLock::new(HashMap::new())),
            dim,
            top_k,
        }
    }

    pub fn load_module(&self, name: &str, dir: &Path) -> Result<()> {
        let module = LoadedModule::load(dir)?;
        if module.manifest.dim != self.dim {
            return Err(KdbError::DimMismatch {
                expected: self.dim,
                got: module.manifest.dim,
            });
        }
        let mut modules = self.modules.write().unwrap();
        modules.insert(name.to_owned(), module);
        info!("kdb: loaded module '{name}'");
        Ok(())
    }

    pub fn unload_module(&self, name: &str) -> bool {
        let mut modules = self.modules.write().unwrap();
        let removed = modules.remove(name).is_some();
        if removed {
            info!("kdb: unloaded module '{name}'");
        } else {
            warn!("kdb: unload requested for unknown module '{name}'");
        }
        removed
    }

    pub fn is_empty(&self) -> bool {
        self.modules.read().unwrap().is_empty()
    }

    pub fn list_modules(&self) -> Vec<ModuleInfo> {
        let modules = self.modules.read().unwrap();
        modules.values().map(|m| m.info()).collect()
    }

    /// Retrieve top_k nearest entries from all loaded modules for the given query vector.
    pub fn retrieve(&self, query: &[f32]) -> Result<Vec<RetrievedContext>> {
        if query.len() != self.dim {
            return Err(KdbError::DimMismatch {
                expected: self.dim,
                got: query.len(),
            });
        }
        let modules = self.modules.read().unwrap();
        let mut results = Vec::new();

        for (name, module) in modules.iter() {
            match module.index.search(query, self.top_k) {
                Ok(hits) => {
                    let entries = hits
                        .into_iter()
                        .filter_map(|(id, dist)| module.values.get(id).map(|v| (v.clone(), dist)))
                        .collect();
                    results.push(RetrievedContext {
                        module_name: name.clone(),
                        entries,
                    });
                }
                Err(e) => {
                    warn!("kdb: search failed in module '{name}': {e}");
                }
            }
        }

        Ok(results)
    }

    /// Format retrieved context as a system prompt prefix for injection.
    pub fn format_context_prefix(&self, query: &[f32]) -> Result<String> {
        let contexts = self.retrieve(query)?;
        if contexts.is_empty() {
            return Ok(String::new());
        }

        let mut lines = vec!["[Knowledge Context]".to_string()];
        for ctx in &contexts {
            lines.push(format!("# Module: {}", ctx.module_name));
            for (entry, _dist) in &ctx.entries {
                lines.push(format!("- {entry}"));
            }
        }
        lines.push(String::new());
        Ok(lines.join("\n"))
    }
}
