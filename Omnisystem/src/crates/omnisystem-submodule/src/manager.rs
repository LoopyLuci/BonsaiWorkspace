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

    pub fn get_state(&self, name: &str) -> Option<ModuleState> {
        self.modules.get(name).map(|m| {
            futures::executor::block_on(async {
                m.lock().await.state()
            })
        })
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
}
