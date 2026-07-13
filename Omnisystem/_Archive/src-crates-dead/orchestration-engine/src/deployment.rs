use crate::{Deployment, DeploymentId, DeploymentStatus, DeploymentStrategy, OrchestrationError, OrchestrationResult, PodSpec};
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;

pub struct DeploymentManager {
    deployments: Arc<DashMap<String, Deployment>>,
}

impl DeploymentManager {
    pub fn new() -> Self {
        Self {
            deployments: Arc::new(DashMap::new()),
        }
    }

    pub fn deployment_count(&self) -> usize {
        self.deployments.len()
    }

    pub async fn create_deployment(&self, spec: &PodSpec) -> OrchestrationResult<DeploymentId> {
        let deployment_id = DeploymentId(uuid::Uuid::new_v4().to_string());

        let deployment = Deployment {
            id: deployment_id.clone(),
            name: spec.name.clone(),
            spec: spec.clone(),
            strategy: DeploymentStrategy::RollingUpdate,
            desired_replicas: spec.replicas,
            ready_replicas: 0,
            updated_replicas: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            status: DeploymentStatus::Pending,
            revision: 1,
        };

        self.deployments.insert(deployment_id.0.clone(), deployment);
        Ok(deployment_id)
    }

    pub async fn get_deployment(&self, deployment_id: &DeploymentId) -> OrchestrationResult<Deployment> {
        self.deployments
            .get(&deployment_id.0)
            .map(|entry| entry.clone())
            .ok_or_else(|| OrchestrationError::DeploymentNotFound(deployment_id.0.clone()))
    }

    pub async fn list_deployments(&self) -> OrchestrationResult<Vec<Deployment>> {
        Ok(self
            .deployments
            .iter()
            .map(|entry| entry.value().clone())
            .collect())
    }

    pub async fn update_deployment(
        &self,
        deployment_id: &DeploymentId,
        spec: &PodSpec,
    ) -> OrchestrationResult<()> {
        if let Some(mut entry) = self.deployments.get_mut(&deployment_id.0) {
            entry.spec = spec.clone();
            entry.updated_at = Utc::now();
            entry.revision += 1;
            entry.status = DeploymentStatus::Progressing;
            Ok(())
        } else {
            Err(OrchestrationError::DeploymentNotFound(deployment_id.0.clone()))
        }
    }

    pub async fn delete_deployment(&self, deployment_id: &DeploymentId) -> OrchestrationResult<()> {
        if self.deployments.remove(&deployment_id.0).is_some() {
            Ok(())
        } else {
            Err(OrchestrationError::DeploymentNotFound(deployment_id.0.clone()))
        }
    }

    pub async fn scale_deployment(
        &self,
        deployment_id: &DeploymentId,
        replicas: u32,
    ) -> OrchestrationResult<()> {
        if let Some(mut entry) = self.deployments.get_mut(&deployment_id.0) {
            entry.desired_replicas = replicas;
            entry.updated_at = Utc::now();
            Ok(())
        } else {
            Err(OrchestrationError::DeploymentNotFound(deployment_id.0.clone()))
        }
    }

    pub async fn update_status(
        &self,
        deployment_id: &DeploymentId,
        ready_replicas: u32,
        updated_replicas: u32,
    ) -> OrchestrationResult<()> {
        if let Some(mut entry) = self.deployments.get_mut(&deployment_id.0) {
            entry.ready_replicas = ready_replicas;
            entry.updated_replicas = updated_replicas;

            if ready_replicas == entry.desired_replicas && updated_replicas == entry.desired_replicas {
                entry.status = DeploymentStatus::Available;
            } else if ready_replicas > 0 {
                entry.status = DeploymentStatus::Progressing;
            }

            Ok(())
        } else {
            Err(OrchestrationError::DeploymentNotFound(deployment_id.0.clone()))
        }
    }
}

impl Default for DeploymentManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_spec() -> PodSpec {
        PodSpec {
            name: "test-deployment".to_string(),
            image: "ubuntu:20.04".to_string(),
            replicas: 3,
            labels: HashMap::new(),
            cpu_request_millicores: 100,
            memory_request_bytes: 128_000_000,
            cpu_limit_millicores: 500,
            memory_limit_bytes: 512_000_000,
            ports: vec![8080],
        }
    }

    #[tokio::test]
    async fn test_create_deployment() {
        let manager = DeploymentManager::new();
        let spec = create_test_spec();

        let deployment_id = manager.create_deployment(&spec).await.unwrap();
        assert_eq!(manager.deployment_count(), 1);
    }

    #[tokio::test]
    async fn test_get_deployment() {
        let manager = DeploymentManager::new();
        let spec = create_test_spec();

        let deployment_id = manager.create_deployment(&spec).await.unwrap();
        let deployment = manager.get_deployment(&deployment_id).await.unwrap();

        assert_eq!(deployment.id, deployment_id);
        assert_eq!(deployment.desired_replicas, 3);
    }

    #[tokio::test]
    async fn test_list_deployments() {
        let manager = DeploymentManager::new();
        let spec = create_test_spec();

        manager.create_deployment(&spec).await.unwrap();
        manager.create_deployment(&spec).await.unwrap();

        let deployments = manager.list_deployments().await.unwrap();
        assert_eq!(deployments.len(), 2);
    }

    #[tokio::test]
    async fn test_scale_deployment() {
        let manager = DeploymentManager::new();
        let spec = create_test_spec();

        let deployment_id = manager.create_deployment(&spec).await.unwrap();
        manager.scale_deployment(&deployment_id, 5).await.unwrap();

        let deployment = manager.get_deployment(&deployment_id).await.unwrap();
        assert_eq!(deployment.desired_replicas, 5);
    }

    #[tokio::test]
    async fn test_update_status() {
        let manager = DeploymentManager::new();
        let spec = create_test_spec();

        let _deployment_id = manager.create_deployment(&spec).await.unwrap();
        manager.update_status(&_deployment_id, 3, 3).await.unwrap();

        let deployment = manager.get_deployment(&_deployment_id).await.unwrap();
        assert_eq!(deployment.ready_replicas, 3);
        assert_eq!(deployment.status, DeploymentStatus::Available);
    }
}
