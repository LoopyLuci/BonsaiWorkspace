use crate::{Cluster, ClusterId, ClusterStatus, LifecycleError, LifecycleResult};
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;

pub struct ClusterFederation {
    clusters: Arc<DashMap<String, Cluster>>,
    deployments: Arc<DashMap<String, Vec<String>>>,
}

impl ClusterFederation {
    pub fn new() -> Self {
        Self {
            clusters: Arc::new(DashMap::new()),
            deployments: Arc::new(DashMap::new()),
        }
    }

    pub fn cluster_count(&self) -> usize {
        self.clusters.len()
    }

    pub async fn register_cluster(&self, cluster: &Cluster) -> LifecycleResult<ClusterId> {
        if self.clusters.contains_key(&cluster.id.0) {
            return Err(LifecycleError::ClusterAlreadyRegistered(cluster.id.0.clone()));
        }

        self.clusters.insert(cluster.id.0.clone(), cluster.clone());
        Ok(cluster.id.clone())
    }

    pub async fn get_cluster(&self, cluster_id: &ClusterId) -> LifecycleResult<Cluster> {
        self.clusters
            .get(&cluster_id.0)
            .map(|entry| entry.clone())
            .ok_or_else(|| LifecycleError::ClusterNotFound(cluster_id.0.clone()))
    }

    pub async fn list_clusters(&self) -> LifecycleResult<Vec<Cluster>> {
        Ok(self
            .clusters
            .iter()
            .map(|entry| entry.value().clone())
            .collect())
    }

    pub async fn remove_cluster(&self, cluster_id: &ClusterId) -> LifecycleResult<()> {
        if self.clusters.remove(&cluster_id.0).is_some() {
            self.deployments.remove(&cluster_id.0);
            Ok(())
        } else {
            Err(LifecycleError::ClusterNotFound(cluster_id.0.clone()))
        }
    }

    pub async fn check_cluster_health(&self, cluster_id: &ClusterId) -> LifecycleResult<bool> {
        if let Some(mut entry) = self.clusters.get_mut(&cluster_id.0) {
            let now = Utc::now();
            let elapsed = (now - entry.last_heartbeat).num_seconds();

            if elapsed > 60 {
                entry.status = ClusterStatus::Disconnected;
                Ok(false)
            } else {
                entry.status = ClusterStatus::Ready;
                entry.last_heartbeat = now;
                Ok(true)
            }
        } else {
            Err(LifecycleError::ClusterNotFound(cluster_id.0.clone()))
        }
    }

    pub async fn sync_deployment(
        &self,
        cluster_id: &ClusterId,
        deployment_id: &str,
    ) -> LifecycleResult<()> {
        if !self.clusters.contains_key(&cluster_id.0) {
            return Err(LifecycleError::ClusterNotFound(cluster_id.0.clone()));
        }

        self.deployments
            .entry(cluster_id.0.clone())
            .or_insert_with(Vec::new)
            .push(deployment_id.to_string());

        Ok(())
    }

    pub async fn get_cluster_deployments(&self, cluster_id: &ClusterId) -> LifecycleResult<Vec<String>> {
        Ok(self
            .deployments
            .get(&cluster_id.0)
            .map(|entry| entry.clone())
            .unwrap_or_default())
    }

    pub async fn failover_cluster(
        &self,
        from_cluster: &ClusterId,
        to_cluster: &ClusterId,
    ) -> LifecycleResult<()> {
        if !self.clusters.contains_key(&from_cluster.0) {
            return Err(LifecycleError::ClusterNotFound(from_cluster.0.clone()));
        }

        if !self.clusters.contains_key(&to_cluster.0) {
            return Err(LifecycleError::ClusterNotFound(to_cluster.0.clone()));
        }

        let deployments = self
            .deployments
            .get(&from_cluster.0)
            .map(|entry| entry.clone())
            .unwrap_or_default();

        for deployment_id in deployments {
            self.sync_deployment(to_cluster, &deployment_id).await?;
        }

        self.deployments.remove(&from_cluster.0);

        Ok(())
    }

    pub async fn get_healthy_clusters(&self) -> LifecycleResult<Vec<Cluster>> {
        let mut healthy = Vec::new();

        for entry in self.clusters.iter() {
            if entry.status == ClusterStatus::Ready {
                healthy.push(entry.clone());
            }
        }

        Ok(healthy)
    }
}

impl Default for ClusterFederation {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_cluster() -> Cluster {
        Cluster {
            id: ClusterId("test-cluster".to_string()),
            name: "test".to_string(),
            api_url: "http://localhost:6443".to_string(),
            status: ClusterStatus::Ready,
            capacity_replicas: 100,
            available_replicas: 100,
            region: "us-east-1".to_string(),
            created_at: Utc::now(),
            last_heartbeat: Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_register_cluster() {
        let federation = ClusterFederation::new();
        let cluster = create_test_cluster();

        let cluster_id = federation.register_cluster(&cluster).await.unwrap();
        assert_eq!(federation.cluster_count(), 1);
    }

    #[tokio::test]
    async fn test_get_cluster() {
        let federation = ClusterFederation::new();
        let cluster = create_test_cluster();

        federation.register_cluster(&cluster).await.unwrap();
        let retrieved = federation.get_cluster(&cluster.id).await.unwrap();
        assert_eq!(retrieved.id, cluster.id);
    }

    #[tokio::test]
    async fn test_list_clusters() {
        let federation = ClusterFederation::new();
        let cluster1 = create_test_cluster();
        let mut cluster2 = create_test_cluster();
        cluster2.id = ClusterId("cluster-2".to_string());

        federation.register_cluster(&cluster1).await.unwrap();
        federation.register_cluster(&cluster2).await.unwrap();

        let clusters = federation.list_clusters().await.unwrap();
        assert_eq!(clusters.len(), 2);
    }

    #[tokio::test]
    async fn test_sync_deployment() {
        let federation = ClusterFederation::new();
        let cluster = create_test_cluster();

        federation.register_cluster(&cluster).await.unwrap();
        federation.sync_deployment(&cluster.id, "test-deployment").await.unwrap();

        let deployments = federation.get_cluster_deployments(&cluster.id).await.unwrap();
        assert_eq!(deployments.len(), 1);
    }

    #[tokio::test]
    async fn test_failover_cluster() {
        let federation = ClusterFederation::new();
        let mut cluster1 = create_test_cluster();
        let mut cluster2 = create_test_cluster();

        cluster2.id = ClusterId("cluster-2".to_string());

        federation.register_cluster(&cluster1).await.unwrap();
        federation.register_cluster(&cluster2).await.unwrap();

        federation.sync_deployment(&cluster1.id, "test-deployment").await.unwrap();

        federation.failover_cluster(&cluster1.id, &cluster2.id).await.unwrap();

        let cluster2_deployments = federation.get_cluster_deployments(&cluster2.id).await.unwrap();
        assert_eq!(cluster2_deployments.len(), 1);
    }

    #[tokio::test]
    async fn test_get_healthy_clusters() {
        let federation = ClusterFederation::new();
        let cluster = create_test_cluster();

        federation.register_cluster(&cluster).await.unwrap();

        let healthy = federation.get_healthy_clusters().await.unwrap();
        assert_eq!(healthy.len(), 1);
    }
}
