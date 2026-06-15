use crate::{RegisteredModel, ModelVersion, TrainingJob, ModelDeployment, ModelStage, JobStatus, DeploymentStatus, RegistryError, RegistryResult};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;

pub struct ModelRegistry {
    models: Arc<DashMap<Uuid, RegisteredModel>>,
    versions: Arc<DashMap<Uuid, ModelVersion>>,
    jobs: Arc<DashMap<Uuid, TrainingJob>>,
    deployments: Arc<DashMap<Uuid, ModelDeployment>>,
}

impl ModelRegistry {
    pub fn new() -> Self {
        Self {
            models: Arc::new(DashMap::new()),
            versions: Arc::new(DashMap::new()),
            jobs: Arc::new(DashMap::new()),
            deployments: Arc::new(DashMap::new()),
        }
    }

    pub async fn register_model(&self, model: &RegisteredModel) -> RegistryResult<()> {
        self.models.insert(model.model_id, model.clone());
        Ok(())
    }

    pub async fn get_model(&self, model_id: Uuid) -> RegistryResult<RegisteredModel> {
        self.models
            .get(&model_id)
            .map(|m| m.clone())
            .ok_or(RegistryError::ModelNotFound)
    }

    pub async fn create_version(&self, version: &ModelVersion) -> RegistryResult<()> {
        self.versions.insert(version.version_id, version.clone());
        Ok(())
    }

    pub async fn get_version(&self, version_id: Uuid) -> RegistryResult<ModelVersion> {
        self.versions
            .get(&version_id)
            .map(|v| v.clone())
            .ok_or(RegistryError::VersioningFailed)
    }

    pub async fn promote_version(&self, version_id: Uuid, stage: ModelStage) -> RegistryResult<()> {
        if let Some(mut version) = self.versions.get_mut(&version_id) {
            version.stage = stage;
            Ok(())
        } else {
            Err(RegistryError::PromotionFailed)
        }
    }

    pub async fn create_training_job(&self, model_id: Uuid) -> RegistryResult<Uuid> {
        let job = TrainingJob {
            job_id: Uuid::new_v4(),
            model_id,
            status: JobStatus::Pending,
            start_time: Utc::now(),
            end_time: None,
            metrics: Vec::new(),
        };

        let job_id = job.job_id;
        self.jobs.insert(job_id, job);
        Ok(job_id)
    }

    pub async fn get_training_job(&self, job_id: Uuid) -> RegistryResult<TrainingJob> {
        self.jobs
            .get(&job_id)
            .map(|j| j.clone())
            .ok_or(RegistryError::TrainingFailed)
    }

    pub async fn update_job_status(&self, job_id: Uuid, status: JobStatus) -> RegistryResult<()> {
        if let Some(mut job) = self.jobs.get_mut(&job_id) {
            job.status = status;
            if status == JobStatus::Completed || status == JobStatus::Failed {
                job.end_time = Some(Utc::now());
            }
            Ok(())
        } else {
            Err(RegistryError::TrainingFailed)
        }
    }

    pub async fn deploy_model(&self, deployment: &ModelDeployment) -> RegistryResult<()> {
        self.deployments.insert(deployment.deployment_id, deployment.clone());
        Ok(())
    }

    pub async fn get_deployment(&self, deployment_id: Uuid) -> RegistryResult<ModelDeployment> {
        self.deployments
            .get(&deployment_id)
            .map(|d| d.clone())
            .ok_or(RegistryError::DeploymentFailed)
    }

    pub fn model_count(&self) -> usize {
        self.models.len()
    }

    pub fn version_count(&self) -> usize {
        self.versions.len()
    }
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_model() {
        let registry = ModelRegistry::new();
        let model = RegisteredModel {
            model_id: Uuid::new_v4(),
            name: "bert-classifier".to_string(),
            current_version: "1.0".to_string(),
            owner: "ml-team".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        registry.register_model(&model).await.unwrap();
        assert_eq!(registry.model_count(), 1);
    }

    #[tokio::test]
    async fn test_create_version() {
        let registry = ModelRegistry::new();
        let model_id = Uuid::new_v4();
        let version = ModelVersion {
            version_id: Uuid::new_v4(),
            model_id,
            version: "2.0".to_string(),
            stage: ModelStage::Development,
            metrics: vec![("accuracy".to_string(), 0.95)],
            created_at: Utc::now(),
        };

        registry.create_version(&version).await.unwrap();
        assert_eq!(registry.version_count(), 1);
    }

    #[tokio::test]
    async fn test_promote_version() {
        let registry = ModelRegistry::new();
        let model_id = Uuid::new_v4();
        let version = ModelVersion {
            version_id: Uuid::new_v4(),
            model_id,
            version: "1.5".to_string(),
            stage: ModelStage::Development,
            metrics: vec![],
            created_at: Utc::now(),
        };

        registry.create_version(&version).await.unwrap();
        registry.promote_version(version.version_id, ModelStage::Production).await.unwrap();

        let promoted = registry.get_version(version.version_id).await.unwrap();
        assert_eq!(promoted.stage, ModelStage::Production);
    }

    #[tokio::test]
    async fn test_create_training_job() {
        let registry = ModelRegistry::new();
        let model_id = Uuid::new_v4();

        let job_id = registry.create_training_job(model_id).await.unwrap();
        let job = registry.get_training_job(job_id).await.unwrap();
        assert_eq!(job.status, JobStatus::Pending);
    }
}
