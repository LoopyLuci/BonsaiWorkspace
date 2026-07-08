/// System Coordinator
/// Cross-system coordination and health management

use crate::message_transport::MessageTransport;
use crate::event_system::EventSystem;
use crate::resource_manager::ResourceManager;
use dashmap::DashMap;
use std::sync::Arc;

/// System Health Status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemHealth {
    Healthy,
    Degraded,
    Critical,
    Offline,
}

/// System Status
#[derive(Debug, Clone)]
pub struct SystemStatus {
    pub system_id: String,
    pub health: SystemHealth,
    pub last_heartbeat: u64,
    pub message_queue_depth: usize,
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
}

/// System Coordinator
pub struct SystemCoordinator {
    transport: Arc<MessageTransport>,
    event_system: Arc<EventSystem>,
    resource_manager: Arc<ResourceManager>,
    system_status: Arc<DashMap<String, SystemStatus>>,
    dependencies: Arc<DashMap<String, Vec<String>>>,
}

impl SystemCoordinator {
    pub fn new(
        transport: Arc<MessageTransport>,
        event_system: Arc<EventSystem>,
        resource_manager: Arc<ResourceManager>,
    ) -> Self {
        SystemCoordinator {
            transport,
            event_system,
            resource_manager,
            system_status: Arc::new(DashMap::new()),
            dependencies: Arc::new(DashMap::new()),
        }
    }

    pub fn register_system(&self, system_id: String) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let status = SystemStatus {
            system_id: system_id.clone(),
            health: SystemHealth::Healthy,
            last_heartbeat: now,
            message_queue_depth: 0,
            cpu_usage_percent: 0.0,
            memory_usage_percent: 0.0,
        };

        self.system_status.insert(system_id.clone(), status);
        self.transport.register_system(system_id);
    }

    pub fn set_dependencies(&self, system_id: String, depends_on: Vec<String>) {
        self.dependencies.insert(system_id, depends_on);
    }

    pub fn register_dependency(&self, dependent: String, dependency: String) {
        self.dependencies
            .entry(dependent)
            .or_insert_with(Vec::new)
            .push(dependency);
    }

    pub async fn coordinate(&self) -> anyhow::Result<()> {
        // Perform system coordination
        for entry in self.system_status.iter() {
            let status = entry.value().clone();

            // Check health
            if status.health == SystemHealth::Offline {
                tracing::warn!("System offline: {}", status.system_id);

                // Emit event
                let event = crate::event_system::Event::new(
                    "system_offline".to_string(),
                    "coordinator".to_string(),
                    serde_json::json!({"system": status.system_id}),
                );
                self.event_system.emit(event).await?;
            }

            // Check for degradation
            if status.cpu_usage_percent > 80.0 || status.memory_usage_percent > 80.0 {
                let event = crate::event_system::Event::new(
                    "system_degraded".to_string(),
                    "coordinator".to_string(),
                    serde_json::json!({
                        "system": status.system_id,
                        "cpu": status.cpu_usage_percent,
                        "memory": status.memory_usage_percent
                    }),
                );
                self.event_system.emit(event).await?;
            }
        }

        Ok(())
    }

    pub fn update_system_status(&self, system_id: String, status: SystemStatus) {
        self.system_status.insert(system_id, status);
    }

    pub fn get_system_status(&self, system_id: &str) -> Option<SystemStatus> {
        self.system_status.get(system_id).map(|s| s.clone())
    }

    pub fn list_all_systems(&self) -> Vec<SystemStatus> {
        self.system_status
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn get_dependencies(&self, system_id: &str) -> Vec<String> {
        self.dependencies
            .get(system_id)
            .map(|deps| deps.clone())
            .unwrap_or_default()
    }

    pub fn check_dependency_health(&self, system_id: &str) -> bool {
        let deps = self.get_dependencies(system_id);

        for dep_id in deps {
            if let Some(status) = self.get_system_status(&dep_id) {
                if status.health == SystemHealth::Offline {
                    return false;
                }
            }
        }

        true
    }

    pub fn overall_health(&self) -> SystemHealth {
        let statuses: Vec<_> = self.list_all_systems();

        let offline_count = statuses.iter().filter(|s| s.health == SystemHealth::Offline).count();
        let critical_count = statuses.iter().filter(|s| s.health == SystemHealth::Critical).count();
        let degraded_count = statuses.iter().filter(|s| s.health == SystemHealth::Degraded).count();

        if offline_count > 0 {
            SystemHealth::Critical
        } else if critical_count > 0 {
            SystemHealth::Critical
        } else if degraded_count > statuses.len() / 2 {
            SystemHealth::Degraded
        } else {
            SystemHealth::Healthy
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_coordinator_creation() {
        let transport = Arc::new(MessageTransport::new());
        let event_system = Arc::new(EventSystem::new());
        let resource_manager = Arc::new(ResourceManager::new());

        let coordinator = SystemCoordinator::new(transport, event_system, resource_manager);

        coordinator.register_system("sys_1".to_string());
        assert!(coordinator.get_system_status("sys_1").is_some());
    }

    #[tokio::test]
    async fn test_dependency_management() {
        let transport = Arc::new(MessageTransport::new());
        let event_system = Arc::new(EventSystem::new());
        let resource_manager = Arc::new(ResourceManager::new());

        let coordinator = SystemCoordinator::new(transport, event_system, resource_manager);

        coordinator.register_system("sys_1".to_string());
        coordinator.register_system("sys_2".to_string());
        coordinator.register_dependency("sys_2".to_string(), "sys_1".to_string());

        let deps = coordinator.get_dependencies("sys_2");
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], "sys_1");
    }
}
