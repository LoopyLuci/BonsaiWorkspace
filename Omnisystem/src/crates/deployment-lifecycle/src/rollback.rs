use crate::{LifecycleError, LifecycleResult, RevisionHistory, RolloutId};
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;

pub struct RollbackManager {
    revision_history: Arc<DashMap<String, Vec<RevisionHistory>>>,
    current_revision: Arc<DashMap<String, u32>>,
}

impl RollbackManager {
    pub fn new() -> Self {
        Self {
            revision_history: Arc::new(DashMap::new()),
            current_revision: Arc::new(DashMap::new()),
        }
    }

    pub async fn add_revision(
        &self,
        deployment_id: &str,
        revision: u32,
        image: &str,
    ) -> LifecycleResult<()> {
        let history = RevisionHistory {
            revision,
            image: image.to_string(),
            timestamp: Utc::now(),
            rollout_status: "Completed".to_string(),
        };

        self.revision_history
            .entry(deployment_id.to_string())
            .or_insert_with(Vec::new)
            .push(history);

        self.current_revision.insert(deployment_id.to_string(), revision);

        Ok(())
    }

    pub async fn rollback_deployment(&self, deployment_id: &str) -> LifecycleResult<RolloutId> {
        let history = self
            .revision_history
            .get(deployment_id)
            .ok_or_else(|| LifecycleError::NoPreviousRevision)?;

        if history.len() < 2 {
            return Err(LifecycleError::NoPreviousRevision);
        }

        let previous_revision = history[history.len() - 2].clone();

        self.current_revision
            .insert(deployment_id.to_string(), previous_revision.revision);

        Ok(RolloutId(uuid::Uuid::new_v4().to_string()))
    }

    pub async fn rollback_to_revision(
        &self,
        deployment_id: &str,
        target_revision: u32,
    ) -> LifecycleResult<RolloutId> {
        let history = self
            .revision_history
            .get(deployment_id)
            .ok_or_else(|| LifecycleError::NoPreviousRevision)?;

        if !history.iter().any(|h| h.revision == target_revision) {
            return Err(LifecycleError::RollbackFailed(format!(
                "Revision {} not found",
                target_revision
            )));
        }

        self.current_revision
            .insert(deployment_id.to_string(), target_revision);

        Ok(RolloutId(uuid::Uuid::new_v4().to_string()))
    }

    pub async fn get_revision_history(&self, deployment_id: &str) -> LifecycleResult<Vec<RevisionHistory>> {
        Ok(self
            .revision_history
            .get(deployment_id)
            .map(|entry| entry.clone())
            .unwrap_or_default())
    }

    pub async fn get_previous_revision(&self, deployment_id: &str) -> LifecycleResult<u32> {
        let history = self
            .revision_history
            .get(deployment_id)
            .ok_or_else(|| LifecycleError::NoPreviousRevision)?;

        if history.len() < 2 {
            return Err(LifecycleError::NoPreviousRevision);
        }

        Ok(history[history.len() - 2].revision)
    }

    pub fn history_count(&self, deployment_id: &str) -> usize {
        self.revision_history
            .get(deployment_id)
            .map(|entry| entry.len())
            .unwrap_or(0)
    }
}

impl Default for RollbackManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_revision() {
        let manager = RollbackManager::new();
        manager.add_revision("test-deployment", 1, "image:v1").await.unwrap();
        assert_eq!(manager.history_count("test-deployment"), 1);
    }

    #[tokio::test]
    async fn test_rollback_deployment() {
        let manager = RollbackManager::new();
        manager.add_revision("test-deployment", 1, "image:v1").await.unwrap();
        manager.add_revision("test-deployment", 2, "image:v2").await.unwrap();

        let _rollout_id = manager.rollback_deployment("test-deployment").await.unwrap();
        let history = manager.get_revision_history("test-deployment").await.unwrap();
        assert_eq!(history.len(), 2);
    }

    #[tokio::test]
    async fn test_rollback_to_revision() {
        let manager = RollbackManager::new();
        manager.add_revision("test-deployment", 1, "image:v1").await.unwrap();
        manager.add_revision("test-deployment", 2, "image:v2").await.unwrap();
        manager.add_revision("test-deployment", 3, "image:v3").await.unwrap();

        let _rollout_id = manager
            .rollback_to_revision("test-deployment", 1)
            .await
            .unwrap();

        assert_eq!(manager.history_count("test-deployment"), 3);
    }

    #[tokio::test]
    async fn test_no_previous_revision() {
        let manager = RollbackManager::new();
        manager.add_revision("test-deployment", 1, "image:v1").await.unwrap();

        let result = manager.rollback_deployment("test-deployment").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_previous_revision() {
        let manager = RollbackManager::new();
        manager.add_revision("test-deployment", 1, "image:v1").await.unwrap();
        manager.add_revision("test-deployment", 2, "image:v2").await.unwrap();

        let prev = manager.get_previous_revision("test-deployment").await.unwrap();
        assert_eq!(prev, 1);
    }
}
