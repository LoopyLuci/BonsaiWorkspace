use async_trait::async_trait;
use crate::{
    Cluster, ClusterId, DeploymentEvent, LifecycleResult, Rollout, RolloutId, RolloutStrategy,
};

#[async_trait]
pub trait RolloutOperations: Send + Sync {
    async fn start_rollout(
        &self,
        deployment_id: &str,
        strategy: RolloutStrategy,
    ) -> LifecycleResult<RolloutId>;

    async fn get_rollout(&self, rollout_id: &RolloutId) -> LifecycleResult<Rollout>;

    async fn list_rollouts(&self) -> LifecycleResult<Vec<Rollout>>;

    async fn pause_rollout(&self, rollout_id: &RolloutId) -> LifecycleResult<()>;

    async fn resume_rollout(&self, rollout_id: &RolloutId) -> LifecycleResult<()>;

    async fn cancel_rollout(&self, rollout_id: &RolloutId) -> LifecycleResult<()>;

    async fn get_rollout_progress(&self, rollout_id: &RolloutId) -> LifecycleResult<u8>;

    async fn get_rollout_events(&self, rollout_id: &RolloutId) -> LifecycleResult<Vec<DeploymentEvent>>;
}

#[async_trait]
pub trait RollbackOperations: Send + Sync {
    async fn rollback_deployment(&self, deployment_id: &str) -> LifecycleResult<RolloutId>;

    async fn rollback_to_revision(&self, deployment_id: &str, revision: u32) -> LifecycleResult<RolloutId>;

    async fn get_revision_history(&self, deployment_id: &str) -> LifecycleResult<Vec<crate::RevisionHistory>>;

    async fn get_previous_revision(&self, deployment_id: &str) -> LifecycleResult<u32>;
}

#[async_trait]
pub trait ClusterOperations: Send + Sync {
    async fn register_cluster(&self, cluster: &Cluster) -> LifecycleResult<ClusterId>;

    async fn get_cluster(&self, cluster_id: &ClusterId) -> LifecycleResult<Cluster>;

    async fn list_clusters(&self) -> LifecycleResult<Vec<Cluster>>;

    async fn remove_cluster(&self, cluster_id: &ClusterId) -> LifecycleResult<()>;

    async fn check_cluster_health(&self, cluster_id: &ClusterId) -> LifecycleResult<bool>;

    async fn sync_deployment(&self, cluster_id: &ClusterId, deployment_id: &str) -> LifecycleResult<()>;

    async fn failover_cluster(&self, from_cluster: &ClusterId, to_cluster: &ClusterId) -> LifecycleResult<()>;
}
