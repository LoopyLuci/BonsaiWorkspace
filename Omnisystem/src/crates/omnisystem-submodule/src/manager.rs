use crate::{ModuleState, SubModule, SubModuleError, Result};
use dashmap::DashMap;
use std::sync::Arc;

pub struct SubModuleManager {
    modules: Arc<DashMap<String, Arc<tokio::sync::Mutex<Box<dyn SubModule>>>>>,
    dependency_graph: Arc<DashMap<String, Vec<String>>>,
}

impl SubModuleManager {
    pub fn new() -> Self {
        Self {
            modules: Arc::new(DashMap::new()),
            dependency_graph: Arc::new(DashMap::new()),
        }
    }

    pub async fn load_module(
        &self,
        name: String,
        module: Box<dyn SubModule>,
    ) -> Result<()> {
        if self.modules.contains_key(&name) {
            return Err(SubModuleError::AlreadyLoaded(name));
        }

        let mut m = module;
        m.initialize().await?;

        let metadata = m.metadata().clone();
        self.modules.insert(
            name.clone(),
            Arc::new(tokio::sync::Mutex::new(m)),
        );

        let deps: Vec<String> = metadata
            .dependencies
            .iter()
            .map(|d| d.name.clone())
            .collect();
        self.dependency_graph.insert(name, deps);

        tracing::info!("Loaded module: {}", metadata.name);
        Ok(())
    }

    pub async fn start_module(&self, name: &str) -> Result<()> {
        let module = self.modules
            .get(name)
            .ok_or_else(|| SubModuleError::NotFound(name.to_string()))?;

        let mut m = module.lock().await;
        m.start().await?;
        tracing::info!("Started module: {}", name);
        Ok(())
    }

    pub async fn stop_module(&self, name: &str) -> Result<()> {
        let module = self.modules
            .get(name)
            .ok_or_else(|| SubModuleError::NotFound(name.to_string()))?;

        let mut m = module.lock().await;
        m.stop().await?;
        tracing::info!("Stopped module: {}", name);
        Ok(())
    }

    pub fn module_count(&self) -> usize {
        self.modules.len()
    }

    pub fn list_modules(&self) -> Vec<String> {
        self.modules.iter().map(|ref_| ref_.key().clone()).collect()
    }

    /// Read a loaded module's current lifecycle state. This is `async`
    /// (rather than blocking on the module's mutex via
    /// `futures::executor::block_on`, which the original implementation
    /// did) because blocking synchronously on an async mutex from inside a
    /// Tokio worker thread risks deadlocking a current-thread runtime and
    /// starves a worker thread even on a multi-thread runtime.
    pub async fn get_state(&self, name: &str) -> Option<ModuleState> {
        let module = self.modules.get(name)?.clone();
        let guard = module.lock().await;
        Some(guard.state())
    }

    /// The dependency names declared by a loaded module, as recorded when
    /// it was loaded.
    pub fn get_dependencies(&self, name: &str) -> Option<Vec<String>> {
        self.dependency_graph.get(name).map(|d| d.clone())
    }
}

impl Default for SubModuleManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ModuleDependency, ModuleMetadata, ModuleVersion};
    use async_trait::async_trait;

    struct TestModule {
        state: ModuleState,
        metadata: ModuleMetadata,
    }

    impl TestModule {
        fn new(name: &str, dependencies: Vec<ModuleDependency>) -> Self {
            Self {
                state: ModuleState::Unloaded,
                metadata: ModuleMetadata {
                    name: name.to_string(),
                    version: ModuleVersion::new(1, 0, 0),
                    author: "test".to_string(),
                    description: "test module".to_string(),
                    dependencies,
                    capabilities: vec![],
                },
            }
        }
    }

    #[async_trait]
    impl SubModule for TestModule {
        fn metadata(&self) -> &ModuleMetadata {
            &self.metadata
        }

        fn state(&self) -> ModuleState {
            self.state
        }

        async fn initialize(&mut self) -> Result<()> {
            self.state = ModuleState::Initialized;
            Ok(())
        }

        async fn start(&mut self) -> Result<()> {
            self.state = ModuleState::Running;
            Ok(())
        }

        async fn stop(&mut self) -> Result<()> {
            self.state = ModuleState::Stopped;
            Ok(())
        }

        async fn unload(&mut self) -> Result<()> {
            self.state = ModuleState::Unloaded;
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_manager_new() {
        let manager = SubModuleManager::new();
        assert_eq!(manager.module_count(), 0);
    }

    #[tokio::test]
    async fn test_module_count() {
        let manager = SubModuleManager::new();
        assert_eq!(manager.list_modules().len(), 0);
    }

    #[tokio::test]
    async fn test_load_start_stop_lifecycle() {
        let manager = SubModuleManager::new();
        let module = Box::new(TestModule::new("mod-a", vec![]));

        manager.load_module("mod-a".to_string(), module).await.unwrap();
        assert_eq!(manager.module_count(), 1);
        // initialize() runs during load_module, so state should already
        // have advanced past Unloaded.
        assert_eq!(manager.get_state("mod-a").await, Some(ModuleState::Initialized));

        manager.start_module("mod-a").await.unwrap();
        assert_eq!(manager.get_state("mod-a").await, Some(ModuleState::Running));

        manager.stop_module("mod-a").await.unwrap();
        assert_eq!(manager.get_state("mod-a").await, Some(ModuleState::Stopped));
    }

    #[tokio::test]
    async fn test_load_module_twice_fails() {
        let manager = SubModuleManager::new();
        manager
            .load_module("dup".to_string(), Box::new(TestModule::new("dup", vec![])))
            .await
            .unwrap();

        let result = manager.load_module("dup".to_string(), Box::new(TestModule::new("dup", vec![]))).await;
        assert!(matches!(result, Err(SubModuleError::AlreadyLoaded(name)) if name == "dup"));
    }

    #[tokio::test]
    async fn test_start_unknown_module_fails() {
        let manager = SubModuleManager::new();
        let result = manager.start_module("nope").await;
        assert!(matches!(result, Err(SubModuleError::NotFound(name)) if name == "nope"));
    }

    #[tokio::test]
    async fn test_dependency_graph_recorded_on_load() {
        let manager = SubModuleManager::new();
        let dep = ModuleDependency {
            name: "base".to_string(),
            version: ModuleVersion::new(1, 0, 0),
            mode: crate::DependencyMode::Required,
        };
        manager
            .load_module("consumer".to_string(), Box::new(TestModule::new("consumer", vec![dep])))
            .await
            .unwrap();

        let deps = manager.get_dependencies("consumer").unwrap();
        assert_eq!(deps, vec!["base".to_string()]);
    }

    #[tokio::test]
    async fn test_get_state_unknown_module_is_none() {
        let manager = SubModuleManager::new();
        assert_eq!(manager.get_state("nope").await, None);
    }
}
