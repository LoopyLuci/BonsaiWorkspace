use dashmap::DashMap;
use module_interfaces::ModuleError;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, info};
use universal_module_loader::ModuleLoader;
use universal_module_registry::ModuleRegistry;
use usee_search_engine::SearchEngine;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Application {
    pub id: String,
    pub name: String,
    pub version: String,
    pub category: String,
    pub description: String,
    pub required_modules: Vec<String>,
    pub features: Vec<String>,
    pub status: ApplicationStatus,
    pub rating: f64,
    pub installs: u64,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ApplicationStatus {
    Available,
    Installing,
    Installed,
    Running,
    Paused,
    Updating,
    Error,
}

pub struct AppMarketplace {
    registry: Arc<ModuleRegistry>,
    loader: Arc<ModuleLoader>,
    search_engine: Arc<SearchEngine>,
    applications: Arc<DashMap<String, Application>>,
    installed_apps: Arc<DashMap<String, Application>>,
}

impl AppMarketplace {
    pub fn new(
        registry: Arc<ModuleRegistry>,
        loader: Arc<ModuleLoader>,
        search_engine: Arc<SearchEngine>,
    ) -> Self {
        info!("Creating AppMarketplace");
        Self {
            registry,
            loader,
            search_engine,
            applications: Arc::new(DashMap::new()),
            installed_apps: Arc::new(DashMap::new()),
        }
    }

    pub fn register_application(&self, app: Application) -> Result<(), ModuleError> {
        debug!("Registering application: {}", app.id);

        if self.applications.contains_key(&app.id) {
            return Err(ModuleError::AlreadyLoaded(app.id));
        }

        self.applications.insert(app.id.clone(), app);
        info!("Application registered");
        Ok(())
    }

    pub async fn search_applications(&self, query: &str) -> Result<Vec<Application>, ModuleError> {
        debug!("Searching applications for: {}", query);

        let mut results = Vec::new();
        let query_lower = query.to_lowercase();

        for entry in self.applications.iter() {
            let app = entry.value();
            if app.name.to_lowercase().contains(&query_lower)
                || app.description.to_lowercase().contains(&query_lower)
            {
                results.push(app.clone());
            }
        }

        results.sort_by(|a, b| b.rating.partial_cmp(&a.rating).unwrap());
        Ok(results)
    }

    pub fn get_application(&self, app_id: &str) -> Result<Application, ModuleError> {
        debug!("Getting application: {}", app_id);
        self.applications
            .get(app_id)
            .map(|entry| entry.value().clone())
            .ok_or_else(|| ModuleError::NotFound(app_id.to_string()))
    }

    pub async fn install_application(&self, app_id: &str) -> Result<(), ModuleError> {
        debug!("Installing application: {}", app_id);

        let app = self.get_application(app_id)?;

        for module_id in &app.required_modules {
            self.loader.load_module(&module_interfaces::ModuleLoadRequest {
                module_id: module_id.clone(),
                version: None,
                config: None,
            }).await?;
        }

        let mut installed_app = app.clone();
        installed_app.status = ApplicationStatus::Installed;
        self.installed_apps.insert(app_id.to_string(), installed_app);

        info!("Application installed: {}", app_id);
        Ok(())
    }

    pub async fn uninstall_application(&self, app_id: &str) -> Result<(), ModuleError> {
        debug!("Uninstalling application: {}", app_id);

        let app = self.installed_apps
            .remove(app_id)
            .ok_or_else(|| ModuleError::NotLoaded(app_id.to_string()))?
            .1;

        for module_id in &app.required_modules {
            let _ = self.loader.unload_module(module_id).await;
        }

        info!("Application uninstalled: {}", app_id);
        Ok(())
    }

    pub fn get_installed_applications(&self) -> Vec<Application> {
        self.installed_apps
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn count_applications(&self) -> usize {
        self.applications.len()
    }

    pub fn count_installed(&self) -> usize {
        self.installed_apps.len()
    }
}

impl Clone for AppMarketplace {
    fn clone(&self) -> Self {
        Self {
            registry: Arc::clone(&self.registry),
            loader: Arc::clone(&self.loader),
            search_engine: Arc::clone(&self.search_engine),
            applications: Arc::clone(&self.applications),
            installed_apps: Arc::clone(&self.installed_apps),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_marketplace_creation() {
        let registry = Arc::new(ModuleRegistry::new());
        let loader = Arc::new(ModuleLoader::new(registry.clone()));
        let search_engine = Arc::new(SearchEngine::new(registry));
        let marketplace = AppMarketplace::new(registry, loader, search_engine);
        assert_eq!(marketplace.count_applications(), 0);
    }

    #[test]
    fn test_register_application() {
        let registry = Arc::new(ModuleRegistry::new());
        let loader = Arc::new(ModuleLoader::new(registry.clone()));
        let search_engine = Arc::new(SearchEngine::new(registry));
        let marketplace = AppMarketplace::new(registry, loader, search_engine);

        let app = Application {
            id: "test-app".to_string(),
            name: "Test App".to_string(),
            version: "1.0.0".to_string(),
            category: "productivity".to_string(),
            description: "Test application".to_string(),
            required_modules: vec![],
            features: vec!["test".to_string()],
            status: ApplicationStatus::Available,
            rating: 4.5,
            installs: 1000,
        };

        assert!(marketplace.register_application(app).is_ok());
        assert_eq!(marketplace.count_applications(), 1);
    }

    #[test]
    fn test_application_clone() {
        let app = Application {
            id: "test".to_string(),
            name: "Test".to_string(),
            version: "1.0.0".to_string(),
            category: "test".to_string(),
            description: "Test".to_string(),
            required_modules: vec![],
            features: vec![],
            status: ApplicationStatus::Available,
            rating: 4.5,
            installs: 100,
        };
        let cloned = app.clone();
        assert_eq!(app.id, cloned.id);
    }
}
