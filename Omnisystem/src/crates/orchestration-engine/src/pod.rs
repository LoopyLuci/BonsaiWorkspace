use crate::{OrchestrationError, OrchestrationResult, Pod, PodId, PodPhase, PodSpec, DeploymentId};
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;

pub struct PodManager {
    pods: Arc<DashMap<String, Pod>>,
}

impl PodManager {
    pub fn new() -> Self {
        Self {
            pods: Arc::new(DashMap::new()),
        }
    }

    pub fn pod_count(&self) -> usize {
        self.pods.len()
    }

    pub async fn create_pod(&self, spec: &PodSpec) -> OrchestrationResult<PodId> {
        let pod_id = PodId(uuid::Uuid::new_v4().to_string());

        let pod = Pod {
            id: pod_id.clone(),
            name: spec.name.clone(),
            deployment_id: None,
            phase: PodPhase::Pending,
            containers: vec![spec.image.clone()],
            node_id: None,
            created_at: Utc::now(),
            started_at: None,
            ready: false,
            restart_count: 0,
        };

        self.pods.insert(pod_id.0.clone(), pod);
        Ok(pod_id)
    }

    pub async fn create_deployment_pod(
        &self,
        spec: &PodSpec,
        deployment_id: &DeploymentId,
    ) -> OrchestrationResult<PodId> {
        let pod_id = PodId(uuid::Uuid::new_v4().to_string());

        let pod = Pod {
            id: pod_id.clone(),
            name: format!("{}-pod", spec.name),
            deployment_id: Some(deployment_id.clone()),
            phase: PodPhase::Pending,
            containers: vec![spec.image.clone()],
            node_id: None,
            created_at: Utc::now(),
            started_at: None,
            ready: false,
            restart_count: 0,
        };

        self.pods.insert(pod_id.0.clone(), pod);
        Ok(pod_id)
    }

    pub async fn get_pod(&self, pod_id: &PodId) -> OrchestrationResult<Pod> {
        self.pods
            .get(&pod_id.0)
            .map(|entry| entry.clone())
            .ok_or_else(|| OrchestrationError::PodNotFound(pod_id.0.clone()))
    }

    pub async fn list_pods(&self) -> OrchestrationResult<Vec<Pod>> {
        Ok(self
            .pods
            .iter()
            .map(|entry| entry.value().clone())
            .collect())
    }

    pub async fn delete_pod(&self, pod_id: &PodId) -> OrchestrationResult<()> {
        if self.pods.remove(&pod_id.0).is_some() {
            Ok(())
        } else {
            Err(OrchestrationError::PodNotFound(pod_id.0.clone()))
        }
    }

    pub async fn list_deployment_pods(&self, deployment_id: &DeploymentId) -> OrchestrationResult<Vec<Pod>> {
        let pods: Vec<Pod> = self
            .pods
            .iter()
            .filter(|entry| {
                if let Some(dep_id) = &entry.value().deployment_id {
                    dep_id == deployment_id
                } else {
                    false
                }
            })
            .map(|entry| entry.value().clone())
            .collect();

        Ok(pods)
    }

    pub async fn update_pod_phase(&self, pod_id: &PodId, phase: PodPhase) -> OrchestrationResult<()> {
        if let Some(mut entry) = self.pods.get_mut(&pod_id.0) {
            entry.phase = phase;
            if phase == PodPhase::Running {
                entry.started_at = Some(Utc::now());
                entry.ready = true;
            }
            Ok(())
        } else {
            Err(OrchestrationError::PodNotFound(pod_id.0.clone()))
        }
    }
}

impl Default for PodManager {
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
            name: "test-pod".to_string(),
            image: "ubuntu:20.04".to_string(),
            replicas: 1,
            labels: HashMap::new(),
            cpu_request_millicores: 100,
            memory_request_bytes: 128_000_000,
            cpu_limit_millicores: 500,
            memory_limit_bytes: 512_000_000,
            ports: vec![8080],
        }
    }

    #[tokio::test]
    async fn test_create_pod() {
        let manager = PodManager::new();
        let spec = create_test_spec();

        let pod_id = manager.create_pod(&spec).await.unwrap();
        assert_eq!(manager.pod_count(), 1);
    }

    #[tokio::test]
    async fn test_get_pod() {
        let manager = PodManager::new();
        let spec = create_test_spec();

        let pod_id = manager.create_pod(&spec).await.unwrap();
        let pod = manager.get_pod(&pod_id).await.unwrap();

        assert_eq!(pod.id, pod_id);
        assert_eq!(pod.phase, PodPhase::Pending);
    }

    #[tokio::test]
    async fn test_list_pods() {
        let manager = PodManager::new();
        let spec = create_test_spec();

        manager.create_pod(&spec).await.unwrap();
        manager.create_pod(&spec).await.unwrap();

        let pods = manager.list_pods().await.unwrap();
        assert_eq!(pods.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_pod() {
        let manager = PodManager::new();
        let spec = create_test_spec();

        let pod_id = manager.create_pod(&spec).await.unwrap();
        assert_eq!(manager.pod_count(), 1);

        manager.delete_pod(&pod_id).await.unwrap();
        assert_eq!(manager.pod_count(), 0);
    }

    #[tokio::test]
    async fn test_update_pod_phase() {
        let manager = PodManager::new();
        let spec = create_test_spec();

        let _pod_id = manager.create_pod(&spec).await.unwrap();
        manager
            .update_pod_phase(&_pod_id, PodPhase::Running)
            .await
            .unwrap();

        let pod = manager.get_pod(&_pod_id).await.unwrap();
        assert_eq!(pod.phase, PodPhase::Running);
        assert!(pod.ready);
    }
}
