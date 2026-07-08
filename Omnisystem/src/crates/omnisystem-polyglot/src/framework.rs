/// Polyglot Module Framework
/// Core trait system for all 750 language modules
/// Every language implements PolyglotModule for seamless integration

use async_trait::async_trait;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Core trait that every language module must implement
#[async_trait]
pub trait PolyglotModule: Send + Sync {
    /// Language identifier (e.g., "assembly", "fortran", "c", "rust", etc.)
    fn language_id(&self) -> &str;

    /// Language name (human readable)
    fn language_name(&self) -> &str;

    /// Batch this language belongs to (1-5)
    fn batch(&self) -> u8;

    /// What language does this module receive input from?
    fn previous_language(&self) -> Option<&str>;

    /// What language will receive output from this module?
    fn next_language(&self) -> Option<&str>;

    /// Initialize the module
    async fn initialize(&self) -> anyhow::Result<()>;

    /// Process input from previous language
    async fn process(&self, input: Vec<u8>) -> anyhow::Result<Vec<u8>>;

    /// Execute module-specific functionality
    async fn execute(&self) -> anyhow::Result<()>;

    /// Get module metadata
    fn metadata(&self) -> ModuleMetadata;

    /// Run module tests
    async fn run_tests(&self) -> anyhow::Result<()> {
        Ok(())
    }

    /// Get module version
    fn version(&self) -> &str {
        "1.0.0"
    }

    /// Health check
    async fn health_check(&self) -> anyhow::Result<bool> {
        Ok(true)
    }
}

/// Module metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleMetadata {
    pub language_id: String,
    pub language_name: String,
    pub batch: u8,
    pub version: String,
    pub loc_count: usize,
    pub test_count: usize,
    pub status: ModuleStatus,
}

/// Module status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModuleStatus {
    Registered,
    Initialized,
    Ready,
    Running,
    Paused,
    Failed,
    Completed,
}

/// Module Registry - manages all 750 language modules
pub struct ModuleRegistry {
    modules: Arc<DashMap<String, Arc<dyn PolyglotModule>>>,
    #[allow(dead_code)]
    execution_order: Arc<parking_lot::Mutex<Vec<String>>>,
    stats: Arc<DashMap<String, ModuleStats>>,
}

/// Module execution statistics
#[derive(Debug, Clone)]
pub struct ModuleStats {
    pub language_id: String,
    pub executions: u64,
    pub successes: u64,
    pub failures: u64,
    pub total_time_ms: u64,
    pub avg_time_ms: f64,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        ModuleRegistry {
            modules: Arc::new(DashMap::new()),
            execution_order: Arc::new(parking_lot::Mutex::new(Vec::new())),
            stats: Arc::new(DashMap::new()),
        }
    }

    pub fn register(&self, module: Arc<dyn PolyglotModule>) {
        let lang_id = module.language_id().to_string();
        self.modules.insert(lang_id, module);
    }

    pub fn get(&self, language_id: &str) -> Option<Arc<dyn PolyglotModule>> {
        self.modules.get(language_id).map(|m| Arc::clone(&m))
    }

    pub fn list_modules(&self) -> Vec<String> {
        self.modules.iter().map(|m| m.key().clone()).collect()
    }

    pub fn module_count(&self) -> usize {
        self.modules.len()
    }

    pub fn get_stats(&self, language_id: &str) -> Option<ModuleStats> {
        self.stats.get(language_id).map(|s| s.clone())
    }

    pub fn update_stats(&self, language_id: &str, success: bool, time_ms: u64) {
        let mut entry = self.stats
            .entry(language_id.to_string())
            .or_insert_with(|| ModuleStats {
                language_id: language_id.to_string(),
                executions: 0,
                successes: 0,
                failures: 0,
                total_time_ms: 0,
                avg_time_ms: 0.0,
            });

        entry.executions += 1;
        if success {
            entry.successes += 1;
        } else {
            entry.failures += 1;
        }
        entry.total_time_ms += time_ms;
        entry.avg_time_ms = entry.total_time_ms as f64 / entry.executions as f64;
    }
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Polyglot Runtime - orchestrates execution across all modules
pub struct PolyglotRuntime {
    registry: Arc<ModuleRegistry>,
    message_bus: Arc<crate::messaging::MessageBus>,
}

impl PolyglotRuntime {
    pub fn new(registry: Arc<ModuleRegistry>) -> Self {
        PolyglotRuntime {
            registry,
            message_bus: Arc::new(crate::messaging::MessageBus::new()),
        }
    }

    pub async fn execute_chain(&self) -> anyhow::Result<()> {
        let modules = self.registry.list_modules();

        for lang_id in modules {
            if let Some(module) = self.registry.get(&lang_id) {
                let start = std::time::Instant::now();
                match module.execute().await {
                    Ok(_) => {
                        let elapsed = start.elapsed().as_millis() as u64;
                        self.registry.update_stats(&lang_id, true, elapsed);
                    }
                    Err(e) => {
                        tracing::error!("Module {} execution failed: {}", lang_id, e);
                        self.registry.update_stats(&lang_id, false, 0);
                    }
                }
            }
        }

        Ok(())
    }

    pub fn registry(&self) -> &ModuleRegistry {
        &self.registry
    }

    pub fn message_bus(&self) -> &crate::messaging::MessageBus {
        &self.message_bus
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry() {
        let registry = ModuleRegistry::new();
        assert_eq!(registry.module_count(), 0);
    }
}
