//! Service-level fixtures and test helpers

use std::collections::HashMap;

/// Service fixture for setting up test environments
pub struct ServiceFixture {
    pub name: String,
    pub version: String,
    pub services: HashMap<String, ServiceInfo>,
    pub environments: HashMap<String, EnvironmentInfo>,
}

#[derive(Clone, Debug)]
pub struct ServiceInfo {
    pub name: String,
    pub state: String,
    pub dependencies: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct EnvironmentInfo {
    pub id: String,
    pub name: String,
    pub services: Vec<String>,
}

impl ServiceFixture {
    /// Create a new service fixture
    pub fn new() -> Self {
        Self {
            name: "test-fixture".to_string(),
            version: "1.0.0".to_string(),
            services: HashMap::new(),
            environments: HashMap::new(),
        }
    }

    /// Create fixture with standard services
    pub fn with_standard_services() -> Self {
        let mut fixture = Self::new();

        fixture.services.insert(
            "p2p".to_string(),
            ServiceInfo {
                name: "p2p".to_string(),
                state: "stopped".to_string(),
                dependencies: vec![],
            },
        );

        fixture.services.insert(
            "mesh".to_string(),
            ServiceInfo {
                name: "mesh".to_string(),
                state: "stopped".to_string(),
                dependencies: vec!["p2p".to_string()],
            },
        );

        fixture.services.insert(
            "api".to_string(),
            ServiceInfo {
                name: "api".to_string(),
                state: "stopped".to_string(),
                dependencies: vec!["mesh".to_string()],
            },
        );

        fixture
    }

    /// Create fixture with standard environments
    pub fn with_standard_environments() -> Self {
        let mut fixture = Self::new();

        fixture.environments.insert(
            "prod".to_string(),
            EnvironmentInfo {
                id: "prod-1".to_string(),
                name: "prod".to_string(),
                services: vec!["p2p".to_string(), "mesh".to_string(), "api".to_string()],
            },
        );

        fixture.environments.insert(
            "staging".to_string(),
            EnvironmentInfo {
                id: "staging-1".to_string(),
                name: "staging".to_string(),
                services: vec!["mesh".to_string(), "api".to_string()],
            },
        );

        fixture.environments.insert(
            "dev".to_string(),
            EnvironmentInfo {
                id: "dev-1".to_string(),
                name: "dev".to_string(),
                services: vec!["api".to_string()],
            },
        );

        fixture
    }

    /// Add a service to the fixture
    pub fn add_service(mut self, name: String, dependencies: Vec<String>) -> Self {
        self.services.insert(
            name.clone(),
            ServiceInfo {
                name,
                state: "stopped".to_string(),
                dependencies,
            },
        );
        self
    }

    /// Add an environment
    pub fn add_environment(mut self, name: String, services: Vec<String>) -> Self {
        self.environments.insert(
            name.clone(),
            EnvironmentInfo {
                id: format!("{}-1", name),
                name,
                services,
            },
        );
        self
    }

    /// Get service by name
    pub fn get_service(&self, name: &str) -> Option<ServiceInfo> {
        self.services.get(name).cloned()
    }

    /// Get environment by name
    pub fn get_environment(&self, name: &str) -> Option<EnvironmentInfo> {
        self.environments.get(name).cloned()
    }

    /// Get all services
    pub fn list_services(&self) -> Vec<ServiceInfo> {
        self.services.values().cloned().collect()
    }

    /// Get all environments
    pub fn list_environments(&self) -> Vec<EnvironmentInfo> {
        self.environments.values().cloned().collect()
    }

    /// Verify service dependencies are met
    pub fn verify_dependencies(&self) -> bool {
        for (_service_name, service) in &self.services {
            for dep in &service.dependencies {
                if !self.services.contains_key(dep) {
                    return false;
                }
            }
        }
        true
    }

    /// Get service dependency order
    pub fn get_startup_order(&self) -> Vec<String> {
        let mut order = vec![];
        let mut remaining: Vec<_> = self.services.keys().cloned().collect();

        while !remaining.is_empty() {
            for service in &remaining.clone() {
                let service_info = &self.services[service];
                if service_info.dependencies.iter().all(|d| order.contains(d)) {
                    order.push(service.clone());
                }
            }
            remaining.retain(|s| !order.contains(s));
        }

        order
    }

    /// Get shutdown order (reverse of startup)
    pub fn get_shutdown_order(&self) -> Vec<String> {
        let mut order = self.get_startup_order();
        order.reverse();
        order
    }
}

impl Default for ServiceFixture {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixture_creation() {
        let fixture = ServiceFixture::new();
        assert_eq!(fixture.services.len(), 0);
        assert_eq!(fixture.environments.len(), 0);
    }

    #[test]
    fn test_standard_services_fixture() {
        let fixture = ServiceFixture::with_standard_services();
        assert_eq!(fixture.services.len(), 3);
        assert!(fixture.services.contains_key("p2p"));
        assert!(fixture.services.contains_key("mesh"));
        assert!(fixture.services.contains_key("api"));
    }

    #[test]
    fn test_standard_environments_fixture() {
        let fixture = ServiceFixture::with_standard_environments();
        assert_eq!(fixture.environments.len(), 3);
        assert!(fixture.environments.contains_key("prod"));
        assert!(fixture.environments.contains_key("staging"));
        assert!(fixture.environments.contains_key("dev"));
    }

    #[test]
    fn test_add_service() {
        let fixture = ServiceFixture::new()
            .add_service("service-1".to_string(), vec![])
            .add_service("service-2".to_string(), vec!["service-1".to_string()]);

        assert_eq!(fixture.services.len(), 2);
        let service2 = fixture.get_service("service-2").unwrap();
        assert!(service2.dependencies.contains(&"service-1".to_string()));
    }

    #[test]
    fn test_dependency_verification() {
        let fixture = ServiceFixture::with_standard_services();
        assert!(fixture.verify_dependencies());
    }

    #[test]
    fn test_startup_order() {
        let fixture = ServiceFixture::with_standard_services();
        let order = fixture.get_startup_order();
        assert_eq!(order.len(), 3);
        assert_eq!(order[0], "p2p");
        assert_eq!(order[1], "mesh");
        assert_eq!(order[2], "api");
    }

    #[test]
    fn test_shutdown_order() {
        let fixture = ServiceFixture::with_standard_services();
        let order = fixture.get_shutdown_order();
        assert_eq!(order[0], "api");
        assert_eq!(order[1], "mesh");
        assert_eq!(order[2], "p2p");
    }
}
