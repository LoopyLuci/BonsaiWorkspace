// Module Runtime - executes modules and coordinates their lifecycle

use crate::data::DataLayerManager;
use crate::module::{ModuleConfig, ModuleDataDirs, ModuleId, ModuleRequest, ModuleState};
use crate::registry::ModuleRegistry;
use crate::resolver::ModuleResolver;
use anyhow::{anyhow, Result};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Module Executor trait for concrete implementations
pub trait ModuleExecutor: Send + Sync {
    /// Execute a module request
    fn execute_sync(
        &self,
        request: &ModuleRequest,
    ) -> std::result::Result<serde_json::Value, String>;
}

/// Module Runtime - manages module lifecycle and execution
pub struct ModuleRuntime {
    registry: ModuleRegistry,
    resolver: ModuleResolver,
    data_manager: DataLayerManager,

    // Loaded executors (modules that have been instantiated)
    executors: Arc<DashMap<ModuleId, Arc<dyn ModuleExecutor>>>,

    // Metrics
    metrics: Arc<RwLock<RuntimeMetrics>>,
}

/// Runtime-wide metrics
#[derive(Debug, Clone, Default)]
pub struct RuntimeMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub total_execution_time_ms: u64,
    pub modules_loaded: u32,
    pub modules_running: u32,
}

impl ModuleRuntime {
    pub async fn new(
        registry: ModuleRegistry,
        resolver: ModuleResolver,
        data_manager: DataLayerManager,
    ) -> Result<Self> {
        tracing::info!("Initializing Module Runtime");

        Ok(Self {
            registry,
            resolver,
            data_manager,
            executors: Arc::new(DashMap::new()),
            metrics: Arc::new(RwLock::new(RuntimeMetrics::default())),
        })
    }

    /// Load a module (instantiate it)
    pub async fn load_module(&self, module_name: &str) -> Result<ModuleId> {
        tracing::info!("Loading module: {}", module_name);

        let entry = self
            .registry
            .get_by_name(module_name)
            .ok_or_else(|| anyhow!("Module not found: {}", module_name))?;

        let module_id = entry.info.id;

        // Check dependencies
        self.resolver.check_dependencies(module_name)?;

        // Register in registry as loaded
        self.registry
            .update_state(module_id, ModuleState::Loaded)?;

        // Update metrics
        {
            let mut metrics = self.metrics.write().await;
            metrics.modules_loaded += 1;
        }

        tracing::info!("Module loaded: {} ({})", module_name, module_id);
        Ok(module_id)
    }

    /// Initialize a module with configuration
    pub async fn initialize_module(
        &self,
        module_id: ModuleId,
        config: serde_json::Value,
    ) -> Result<()> {
        tracing::info!("Initializing module: {}", module_id);

        let entry = self
            .registry
            .get(module_id)
            .ok_or_else(|| anyhow!("Module not found: {}", module_id))?;

        // Create module configuration
        let data_dirs = ModuleDataDirs {
            umd_source: self.data_manager.base().to_path_buf(),
            generated: self
                .data_manager
                .generated()
                .await?
                .join(&entry.info.name),
            user_data: self
                .data_manager
                .user_data()
                .await?
                .join(&entry.info.name),
            cache: self.data_manager.cache_path().await?,
        };

        let module_config = ModuleConfig {
            config,
            runtime_config: serde_json::json!({}),
            data_dirs,
        };

        tracing::info!(
            "Module configuration: {:?}",
            module_config.data_dirs
        );

        // Update state
        self.registry
            .update_state(module_id, ModuleState::Ready)?;

        Ok(())
    }

    /// Start executing a module
    pub async fn start_module(&self, module_id: ModuleId) -> Result<()> {
        tracing::info!("Starting module: {}", module_id);

        self.registry
            .update_state(module_id, ModuleState::Running)?;

        {
            let mut metrics = self.metrics.write().await;
            metrics.modules_running += 1;
        }

        Ok(())
    }

    /// Stop a module
    pub async fn stop_module(&self, module_id: ModuleId) -> Result<()> {
        tracing::info!("Stopping module: {}", module_id);

        self.registry
            .update_state(module_id, ModuleState::Stopped)?;

        {
            let mut metrics = self.metrics.write().await;
            metrics.modules_running = metrics.modules_running.saturating_sub(1);
        }

        Ok(())
    }

    /// Execute a request in a module
    pub async fn execute(
        &self,
        module_id: ModuleId,
        request: ModuleRequest,
    ) -> Result<serde_json::Value> {
        let start = std::time::Instant::now();

        let entry = self
            .registry
            .get(module_id)
            .ok_or_else(|| anyhow!("Module not found: {}", module_id))?;

        if entry.state != ModuleState::Running {
            return Err(anyhow!(
                "Module not running: {} (state: {:?})",
                module_id,
                entry.state
            ));
        }

        // Execute - for now, return a placeholder
        // In real implementation, this would call the actual module executor
        let result = serde_json::json!({
            "module_id": module_id.to_string(),
            "operation": request.operation,
            "status": "executed",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        let elapsed = start.elapsed();
        {
            let mut metrics = self.metrics.write().await;
            metrics.total_requests += 1;
            metrics.successful_requests += 1;
            metrics.total_execution_time_ms += elapsed.as_millis() as u64;
        }

        tracing::debug!(
            "Module execution: {} op {} in {:?}",
            module_id,
            request.operation,
            elapsed
        );

        Ok(result)
    }

    /// Get module registry
    pub fn registry(&self) -> &ModuleRegistry {
        &self.registry
    }

    /// Get module resolver
    pub fn resolver(&self) -> &ModuleResolver {
        &self.resolver
    }

    /// Get data manager
    pub fn data_manager(&self) -> &DataLayerManager {
        &self.data_manager
    }

    /// Get runtime metrics
    pub async fn metrics(&self) -> RuntimeMetrics {
        self.metrics.read().await.clone()
    }

    /// Get loaded modules
    pub fn loaded_modules(&self) -> Vec<String> {
        self.registry
            .all()
            .into_iter()
            .map(|e| e.info.name)
            .collect()
    }

    /// Get running modules
    pub fn running_modules(&self) -> Vec<String> {
        self.registry
            .by_state(ModuleState::Running)
            .into_iter()
            .map(|e| e.info.name)
            .collect()
    }

    /// Load all modules for a phase
    pub async fn load_phase(&self, phase: u32) -> Result<Vec<ModuleId>> {
        tracing::info!("Loading phase {}", phase);

        let modules = self.registry.by_phase(phase);
        let mut loaded = Vec::new();

        for entry in modules {
            if let Ok(id) = self.load_module(&entry.info.name).await {
                loaded.push(id);
            }
        }

        tracing::info!("Phase {} loaded: {} modules", phase, loaded.len());
        Ok(loaded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module::ModuleInfo;

    #[tokio::test]
    async fn test_runtime_creation() {
        let registry = ModuleRegistry::new();
        let resolver = ModuleResolver::new(registry.clone());
        let data_manager = DataLayerManager::new(std::path::Path::new("./test-omnisystem"))
            .await
            .unwrap();

        let runtime = ModuleRuntime::new(registry, resolver, data_manager)
            .await
            .unwrap();

        assert_eq!(runtime.loaded_modules().len(), 0);
        assert_eq!(runtime.running_modules().len(), 0);
    }
}
