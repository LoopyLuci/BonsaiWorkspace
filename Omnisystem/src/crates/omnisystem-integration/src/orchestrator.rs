use crate::Result;
use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ModuleInstance {
    pub name: String,
    pub status: ModuleStatus,
    pub initialized: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleStatus {
    Stopped,
    Starting,
    Running,
    Stopping,
}

pub struct ModuleOrchestrator {
    modules: Arc<DashMap<String, ModuleInstance>>,
}

impl ModuleOrchestrator {
    pub fn new() -> Self {
        Self {
            modules: Arc::new(DashMap::new()),
        }
    }

    pub async fn register_module(&self, name: String) -> Result<()> {
        let module = ModuleInstance {
            name: name.clone(),
            status: ModuleStatus::Stopped,
            initialized: false,
        };
        self.modules.insert(name, module);
        tracing::info!("Module registered");
        Ok(())
    }

    pub async fn start_module(&self, name: &str) -> Result<()> {
        if let Some(mut module) = self.modules.get_mut(name) {
            module.status = ModuleStatus::Running;
            module.initialized = true;
        }
        Ok(())
    }

    pub fn module_count(&self) -> usize {
        self.modules.len()
    }

    pub fn get_module(&self, name: &str) -> Option<ModuleInstance> {
        self.modules.get(name).map(|m| m.clone())
    }
}

impl Default for ModuleOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_register_module() {
        let orch = ModuleOrchestrator::new();
        orch.register_module("m1".to_string()).await.unwrap();
        assert_eq!(orch.module_count(), 1);
    }
}
