use dashmap::DashMap;
use module_interfaces::ModuleError;
use std::sync::Arc;
use tracing::{debug, info};
use universal_module_loader::ModuleLoader;
use universal_module_registry::ModuleRegistry;
use usee_search_engine::SearchEngine;

pub struct AgentModuleController {
    registry: Arc<ModuleRegistry>,
    loader: Arc<ModuleLoader>,
    search_engine: Arc<SearchEngine>,
    agent_preferences: Arc<DashMap<String, AgentPreference>>,
}

#[derive(Clone, Debug)]
pub struct AgentPreference {
    pub agent_id: String,
    pub preferred_modules: Vec<String>,
    pub preferred_versions: Vec<String>,
    pub capability_mapping: std::collections::HashMap<String, String>,
}

#[derive(Clone, Debug)]
pub struct ModuleDiscoveryRequest {
    pub agent_id: String,
    pub required_capability: String,
    pub version_constraint: Option<String>,
}

impl AgentModuleController {
    pub fn new(
        registry: Arc<ModuleRegistry>,
        loader: Arc<ModuleLoader>,
        search_engine: Arc<SearchEngine>,
    ) -> Self {
        info!("Creating AgentModuleController");
        Self {
            registry,
            loader,
            search_engine,
            agent_preferences: Arc::new(DashMap::new()),
        }
    }

    pub async fn discover_module_for_capability(
        &self,
        request: &ModuleDiscoveryRequest,
    ) -> Result<String, ModuleError> {
        debug!("Agent {} discovering module for capability: {}", request.agent_id, request.required_capability);

        let results = self.search_engine
            .search_by_capability(&format!("capability:{}", request.required_capability))
            .await
            .map_err(|e| ModuleError::InternalError(e))?;

        if results.is_empty() {
            return Err(ModuleError::DependencyNotFound(request.required_capability.clone()));
        }

        let best_match = results.iter().max_by_key(|r| (r.relevance_score * 100.0) as u32);
        Ok(best_match.unwrap().module_id.clone())
    }

    pub async fn agent_load_module(
        &self,
        agent_id: &str,
        module_id: &str,
    ) -> Result<(), ModuleError> {
        debug!("Agent {} loading module: {}", agent_id, module_id);

        self.loader.load_module(&module_interfaces::ModuleLoadRequest {
            module_id: module_id.to_string(),
            version: None,
            config: None,
        }).await?;

        info!("Agent {} successfully loaded module: {}", agent_id, module_id);
        Ok(())
    }

    pub async fn agent_unload_module(
        &self,
        agent_id: &str,
        module_id: &str,
    ) -> Result<(), ModuleError> {
        debug!("Agent {} unloading module: {}", agent_id, module_id);
        self.loader.unload_module(module_id).await
    }

    pub fn set_agent_preference(
        &self,
        agent_id: String,
        preference: AgentPreference,
    ) -> Result<(), ModuleError> {
        self.agent_preferences.insert(agent_id, preference);
        Ok(())
    }

    pub fn get_agent_preference(&self, agent_id: &str) -> Option<AgentPreference> {
        self.agent_preferences.get(agent_id).map(|entry| entry.value().clone())
    }
}

impl Clone for AgentModuleController {
    fn clone(&self) -> Self {
        Self {
            registry: Arc::clone(&self.registry),
            loader: Arc::clone(&self.loader),
            search_engine: Arc::clone(&self.search_engine),
            agent_preferences: Arc::clone(&self.agent_preferences),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_controller_creation() {
        let registry = Arc::new(ModuleRegistry::new());
        let loader = Arc::new(ModuleLoader::new(registry.clone()));
        let search_engine = Arc::new(SearchEngine::new(registry));
        let controller = AgentModuleController::new(registry, loader, search_engine);
        assert!(controller.get_agent_preference("agent1").is_none());
    }

    #[test]
    fn test_agent_preference() {
        let registry = Arc::new(ModuleRegistry::new());
        let loader = Arc::new(ModuleLoader::new(registry.clone()));
        let search_engine = Arc::new(SearchEngine::new(registry));
        let controller = AgentModuleController::new(registry, loader, search_engine);

        let pref = AgentPreference {
            agent_id: "agent1".to_string(),
            preferred_modules: vec!["module1".to_string()],
            preferred_versions: vec!["1.0.0".to_string()],
            capability_mapping: std::collections::HashMap::new(),
        };

        assert!(controller.set_agent_preference("agent1".to_string(), pref.clone()).is_ok());
        assert!(controller.get_agent_preference("agent1").is_some());
    }
}
