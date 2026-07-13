use crate::{
    Container, ContainerConfig, ContainerError, ContainerId, ContainerResult, ContainerState,
    ContainerStats, ImageId,
};
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;

pub struct ContainerRuntime {
    containers: Arc<DashMap<String, Container>>,
}

impl ContainerRuntime {
    pub fn new() -> Self {
        Self {
            containers: Arc::new(DashMap::new()),
        }
    }

    pub fn container_count(&self) -> usize {
        self.containers.len()
    }

    pub async fn create_container(
        &self,
        image_id: &ImageId,
        config: &ContainerConfig,
    ) -> ContainerResult<ContainerId> {
        let container = Container {
            id: ContainerId(uuid::Uuid::new_v4().to_string()),
            image_id: image_id.clone(),
            name: format!("container-{}", uuid::Uuid::new_v4()),
            state: ContainerState::Created,
            created_at: Utc::now(),
            started_at: None,
            stopped_at: None,
            exit_code: None,
            config: config.clone(),
            status: "Created".to_string(),
        };

        let container_id = container.id.clone();
        self.containers.insert(container_id.0.clone(), container);
        Ok(container_id)
    }

    pub async fn start_container(&self, container_id: &ContainerId) -> ContainerResult<()> {
        if let Some(mut entry) = self.containers.get_mut(&container_id.0) {
            if entry.state == ContainerState::Created || entry.state == ContainerState::Stopped {
                entry.state = ContainerState::Running;
                entry.started_at = Some(Utc::now());
                entry.status = "Running".to_string();
                Ok(())
            } else {
                Err(ContainerError::ContainerAlreadyRunning(container_id.0.clone()))
            }
        } else {
            Err(ContainerError::ContainerNotFound(container_id.0.clone()))
        }
    }

    pub async fn stop_container(&self, container_id: &ContainerId, _timeout_secs: u64) -> ContainerResult<()> {
        if let Some(mut entry) = self.containers.get_mut(&container_id.0) {
            if entry.state == ContainerState::Running || entry.state == ContainerState::Paused {
                entry.state = ContainerState::Stopped;
                entry.stopped_at = Some(Utc::now());
                entry.exit_code = Some(0);
                entry.status = "Stopped".to_string();
                Ok(())
            } else {
                Err(ContainerError::ContainerNotRunning(container_id.0.clone()))
            }
        } else {
            Err(ContainerError::ContainerNotFound(container_id.0.clone()))
        }
    }

    pub async fn pause_container(&self, container_id: &ContainerId) -> ContainerResult<()> {
        if let Some(mut entry) = self.containers.get_mut(&container_id.0) {
            if entry.state == ContainerState::Running {
                entry.state = ContainerState::Paused;
                entry.status = "Paused".to_string();
                Ok(())
            } else {
                Err(ContainerError::ContainerNotRunning(container_id.0.clone()))
            }
        } else {
            Err(ContainerError::ContainerNotFound(container_id.0.clone()))
        }
    }

    pub async fn resume_container(&self, container_id: &ContainerId) -> ContainerResult<()> {
        if let Some(mut entry) = self.containers.get_mut(&container_id.0) {
            if entry.state == ContainerState::Paused {
                entry.state = ContainerState::Running;
                entry.status = "Running".to_string();
                Ok(())
            } else {
                Err(ContainerError::ContainerNotRunning(container_id.0.clone()))
            }
        } else {
            Err(ContainerError::ContainerNotFound(container_id.0.clone()))
        }
    }

    pub async fn remove_container(&self, container_id: &ContainerId) -> ContainerResult<()> {
        if self.containers.remove(&container_id.0).is_some() {
            Ok(())
        } else {
            Err(ContainerError::ContainerNotFound(container_id.0.clone()))
        }
    }

    pub async fn get_container(&self, container_id: &ContainerId) -> ContainerResult<Container> {
        self.containers
            .get(&container_id.0)
            .map(|entry| entry.clone())
            .ok_or_else(|| ContainerError::ContainerNotFound(container_id.0.clone()))
    }

    pub async fn list_containers(&self) -> ContainerResult<Vec<Container>> {
        Ok(self
            .containers
            .iter()
            .map(|entry| entry.value().clone())
            .collect())
    }

    pub async fn get_container_stats(&self, container_id: &ContainerId) -> ContainerResult<ContainerStats> {
        let container = self.get_container(container_id).await?;

        Ok(ContainerStats {
            container_id: container_id.clone(),
            cpu_usage_percent: 0.0,
            memory_usage_bytes: 0,
            memory_limit_bytes: container.config.memory_limit_bytes,
            network_in_bytes: 0,
            network_out_bytes: 0,
            block_read_bytes: 0,
            block_write_bytes: 0,
        })
    }

    pub async fn kill_container(&self, container_id: &ContainerId, _signal: u32) -> ContainerResult<()> {
        if let Some(mut entry) = self.containers.get_mut(&container_id.0) {
            entry.state = ContainerState::Dead;
            entry.exit_code = Some(137);
            entry.status = "Dead".to_string();
            Ok(())
        } else {
            Err(ContainerError::ContainerNotFound(container_id.0.clone()))
        }
    }
}

impl Default for ContainerRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_config() -> ContainerConfig {
        ContainerConfig {
            image: "ubuntu:20.04".to_string(),
            cmd: vec!["bash".to_string()],
            env: HashMap::new(),
            working_dir: "/".to_string(),
            ports: vec![],
            volumes: vec![],
            cpu_limit_millicores: 1000,
            memory_limit_bytes: 512_000_000,
        }
    }

    #[tokio::test]
    async fn test_create_container() {
        let runtime = ContainerRuntime::new();
        let image_id = ImageId("test-image".to_string());
        let config = create_test_config();

        let container_id = runtime.create_container(&image_id, &config).await.unwrap();
        assert_eq!(runtime.container_count(), 1);
    }

    #[tokio::test]
    async fn test_start_container() {
        let runtime = ContainerRuntime::new();
        let image_id = ImageId("test-image".to_string());
        let config = create_test_config();

        let container_id = runtime.create_container(&image_id, &config).await.unwrap();
        runtime.start_container(&container_id).await.unwrap();

        let container = runtime.get_container(&container_id).await.unwrap();
        assert_eq!(container.state, ContainerState::Running);
    }

    #[tokio::test]
    async fn test_stop_container() {
        let runtime = ContainerRuntime::new();
        let image_id = ImageId("test-image".to_string());
        let config = create_test_config();

        let container_id = runtime.create_container(&image_id, &config).await.unwrap();
        runtime.start_container(&container_id).await.unwrap();
        runtime.stop_container(&container_id, 10).await.unwrap();

        let container = runtime.get_container(&container_id).await.unwrap();
        assert_eq!(container.state, ContainerState::Stopped);
    }

    #[tokio::test]
    async fn test_pause_resume_container() {
        let runtime = ContainerRuntime::new();
        let image_id = ImageId("test-image".to_string());
        let config = create_test_config();

        let container_id = runtime.create_container(&image_id, &config).await.unwrap();
        runtime.start_container(&container_id).await.unwrap();
        runtime.pause_container(&container_id).await.unwrap();

        let paused = runtime.get_container(&container_id).await.unwrap();
        assert_eq!(paused.state, ContainerState::Paused);

        runtime.resume_container(&container_id).await.unwrap();
        let resumed = runtime.get_container(&container_id).await.unwrap();
        assert_eq!(resumed.state, ContainerState::Running);
    }

    #[tokio::test]
    async fn test_list_containers() {
        let runtime = ContainerRuntime::new();
        let image_id = ImageId("test-image".to_string());
        let config = create_test_config();

        runtime.create_container(&image_id, &config).await.unwrap();
        runtime.create_container(&image_id, &config).await.unwrap();

        let containers = runtime.list_containers().await.unwrap();
        assert_eq!(containers.len(), 2);
    }

    #[tokio::test]
    async fn test_remove_container() {
        let runtime = ContainerRuntime::new();
        let image_id = ImageId("test-image".to_string());
        let config = create_test_config();

        let container_id = runtime.create_container(&image_id, &config).await.unwrap();
        assert_eq!(runtime.container_count(), 1);

        runtime.remove_container(&container_id).await.unwrap();
        assert_eq!(runtime.container_count(), 0);
    }

    #[tokio::test]
    async fn test_get_container_stats() {
        let runtime = ContainerRuntime::new();
        let image_id = ImageId("test-image".to_string());
        let config = create_test_config();

        let _container_id = runtime.create_container(&image_id, &config).await.unwrap();
        let stats = runtime.get_container_stats(&_container_id).await.unwrap();

        assert_eq!(stats.memory_limit_bytes, config.memory_limit_bytes);
    }
}
