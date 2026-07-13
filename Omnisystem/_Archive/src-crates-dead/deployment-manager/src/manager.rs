use crate::{Deployment, DeploymentError, DeploymentResult};
use dashmap::DashMap;
use chrono::Utc;
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

    pub async fn deploy(&self, version: &str) -> DeploymentResult<Deployment> {
        let deployment = Deployment {
            deployment_id: uuid::Uuid::new_v4().to_string(),
            version: version.to_string(),
            status: "deployed".to_string(),
            deployed_at: Utc::now(),
        };

        self.deployments.insert(deployment.deployment_id.clone(), deployment.clone());
        Ok(deployment)
    }

    pub async fn get_status(&self, deployment_id: &str) -> DeploymentResult<String> {
        if let Some(depl) = self.deployments.get(deployment_id) {
            Ok(depl.status.clone())
        } else {
            Err(DeploymentError::StatusCheckFailed)
        }
    }

    pub fn deployment_count(&self) -> usize {
        self.deployments.len()
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

    #[tokio::test]
    async fn test_deploy() {
        let mgr = DeploymentManager::new();
        let depl = mgr.deploy("1.0.0").await.unwrap();
        assert_eq!(depl.version, "1.0.0");
    }

    #[tokio::test]
    async fn test_get_status() {
        let mgr = DeploymentManager::new();
        let depl = mgr.deploy("1.0.0").await.unwrap();
        let status = mgr.get_status(&depl.deployment_id).await.unwrap();
        assert_eq!(status, "deployed");
    }
}
