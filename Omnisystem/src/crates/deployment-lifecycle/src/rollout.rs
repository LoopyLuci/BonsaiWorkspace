use crate::{
    DeploymentEvent, LifecycleError, LifecycleResult, Rollout, RolloutId, RolloutStatus, RolloutStrategy,
};
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;

pub struct RolloutManager {
    rollouts: Arc<DashMap<String, Rollout>>,
    events: Arc<DashMap<String, Vec<DeploymentEvent>>>,
}

impl RolloutManager {
    pub fn new() -> Self {
        Self {
            rollouts: Arc::new(DashMap::new()),
            events: Arc::new(DashMap::new()),
        }
    }

    pub fn rollout_count(&self) -> usize {
        self.rollouts.len()
    }

    pub async fn start_rollout(
        &self,
        deployment_id: &str,
        strategy: RolloutStrategy,
    ) -> LifecycleResult<RolloutId> {
        let rollout_id = RolloutId(uuid::Uuid::new_v4().to_string());

        let rollout = Rollout {
            id: rollout_id.clone(),
            deployment_id: deployment_id.to_string(),
            strategy,
            old_revision: 1,
            new_revision: 2,
            status: RolloutStatus::Pending,
            progress_percent: 0,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            surge_replicas: 0,
            unavailable_replicas: 0,
        };

        self.rollouts.insert(rollout_id.0.clone(), rollout);
        self.events.insert(rollout_id.0.clone(), Vec::new());

        self.log_event(
            &rollout_id,
            "RolloutStarted",
            &format!("Rollout started with strategy: {}", strategy.to_string()),
            "Info",
        )
        .await?;

        Ok(rollout_id)
    }

    pub async fn get_rollout(&self, rollout_id: &RolloutId) -> LifecycleResult<Rollout> {
        self.rollouts
            .get(&rollout_id.0)
            .map(|entry| entry.clone())
            .ok_or_else(|| LifecycleError::RolloutNotFound(rollout_id.0.clone()))
    }

    pub async fn list_rollouts(&self) -> LifecycleResult<Vec<Rollout>> {
        Ok(self
            .rollouts
            .iter()
            .map(|entry| entry.value().clone())
            .collect())
    }

    pub async fn update_progress(&self, rollout_id: &RolloutId, progress: u8) -> LifecycleResult<()> {
        if let Some(mut entry) = self.rollouts.get_mut(&rollout_id.0) {
            entry.progress_percent = progress;

            if progress == 100 {
                entry.status = RolloutStatus::Completed;
                entry.completed_at = Some(Utc::now());
            }

            Ok(())
        } else {
            Err(LifecycleError::RolloutNotFound(rollout_id.0.clone()))
        }
    }

    pub async fn pause_rollout(&self, rollout_id: &RolloutId) -> LifecycleResult<()> {
        if let Some(mut entry) = self.rollouts.get_mut(&rollout_id.0) {
            entry.status = RolloutStatus::Paused;
            self.log_event(rollout_id, "RolloutPaused", "Rollout paused", "Info")
                .await?;
            Ok(())
        } else {
            Err(LifecycleError::RolloutNotFound(rollout_id.0.clone()))
        }
    }

    pub async fn resume_rollout(&self, rollout_id: &RolloutId) -> LifecycleResult<()> {
        if let Some(mut entry) = self.rollouts.get_mut(&rollout_id.0) {
            entry.status = RolloutStatus::InProgress;
            self.log_event(rollout_id, "RolloutResumed", "Rollout resumed", "Info")
                .await?;
            Ok(())
        } else {
            Err(LifecycleError::RolloutNotFound(rollout_id.0.clone()))
        }
    }

    pub async fn cancel_rollout(&self, rollout_id: &RolloutId) -> LifecycleResult<()> {
        if let Some(mut entry) = self.rollouts.get_mut(&rollout_id.0) {
            entry.status = RolloutStatus::Failed;
            self.log_event(rollout_id, "RolloutCancelled", "Rollout cancelled", "Warning")
                .await?;
            Ok(())
        } else {
            Err(LifecycleError::RolloutNotFound(rollout_id.0.clone()))
        }
    }

    pub async fn log_event(
        &self,
        rollout_id: &RolloutId,
        event_type: &str,
        message: &str,
        severity: &str,
    ) -> LifecycleResult<()> {
        let event = DeploymentEvent {
            timestamp: Utc::now(),
            event_type: event_type.to_string(),
            message: message.to_string(),
            severity: severity.to_string(),
        };

        self.events
            .entry(rollout_id.0.clone())
            .or_insert_with(Vec::new)
            .push(event);

        Ok(())
    }

    pub async fn get_rollout_events(&self, rollout_id: &RolloutId) -> LifecycleResult<Vec<DeploymentEvent>> {
        Ok(self
            .events
            .get(&rollout_id.0)
            .map(|entry| entry.clone())
            .unwrap_or_default())
    }
}

impl Default for RolloutManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_start_rollout() {
        let manager = RolloutManager::new();
        let rollout_id = manager
            .start_rollout("test-deployment", RolloutStrategy::RollingUpdate)
            .await
            .unwrap();

        assert_eq!(manager.rollout_count(), 1);
    }

    #[tokio::test]
    async fn test_get_rollout() {
        let manager = RolloutManager::new();
        let rollout_id = manager
            .start_rollout("test-deployment", RolloutStrategy::BlueGreen)
            .await
            .unwrap();

        let rollout = manager.get_rollout(&rollout_id).await.unwrap();
        assert_eq!(rollout.id, rollout_id);
    }

    #[tokio::test]
    async fn test_pause_resume_rollout() {
        let manager = RolloutManager::new();
        let rollout_id = manager
            .start_rollout("test-deployment", RolloutStrategy::Canary)
            .await
            .unwrap();

        manager.pause_rollout(&rollout_id).await.unwrap();
        let paused = manager.get_rollout(&rollout_id).await.unwrap();
        assert_eq!(paused.status, RolloutStatus::Paused);

        manager.resume_rollout(&rollout_id).await.unwrap();
        let resumed = manager.get_rollout(&rollout_id).await.unwrap();
        assert_eq!(resumed.status, RolloutStatus::InProgress);
    }

    #[tokio::test]
    async fn test_update_progress() {
        let manager = RolloutManager::new();
        let rollout_id = manager
            .start_rollout("test-deployment", RolloutStrategy::RollingUpdate)
            .await
            .unwrap();

        manager.update_progress(&rollout_id, 50).await.unwrap();
        let rollout = manager.get_rollout(&rollout_id).await.unwrap();
        assert_eq!(rollout.progress_percent, 50);

        manager.update_progress(&rollout_id, 100).await.unwrap();
        let completed = manager.get_rollout(&rollout_id).await.unwrap();
        assert_eq!(completed.status, RolloutStatus::Completed);
    }

    #[tokio::test]
    async fn test_log_events() {
        let manager = RolloutManager::new();
        let rollout_id = manager
            .start_rollout("test-deployment", RolloutStrategy::RollingUpdate)
            .await
            .unwrap();

        manager
            .log_event(&rollout_id, "TestEvent", "Test message", "Info")
            .await
            .unwrap();

        let events = manager.get_rollout_events(&rollout_id).await.unwrap();
        assert_eq!(events.len(), 2); // RolloutStarted + TestEvent
    }
}
