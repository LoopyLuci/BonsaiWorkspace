//! Artifact collection and storage in the Universe audit log

use crate::errors::{CIError, CIResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, info};

/// Metadata about a test artifact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactMetadata {
    pub artifact_id: String,
    pub test_name: String,
    pub timestamp: DateTime<Utc>,
    pub commit_hash: String,
    pub pipeline_stage: String,
    pub tags: Vec<String>,
    pub retention_days: u32,
}

/// Complete test artifact with logs and traces
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestArtifact {
    pub metadata: ArtifactMetadata,
    pub stdout_log: String,
    pub stderr_log: String,
    pub trace_data: Option<String>,
    pub metrics_snapshot: Option<String>,
    pub status: ArtifactStatus,
    pub created_at: DateTime<Utc>,
}

/// Artifact status
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum ArtifactStatus {
    Collected,
    Indexed,
    Archived,
    Deleted,
}

/// Collector for test artifacts
pub struct ArtifactCollector {
    storage_path: PathBuf,
    artifacts: dashmap::DashMap<String, TestArtifact>,
}

impl ArtifactCollector {
    /// Create new artifact collector
    pub async fn new(storage_path: impl AsRef<Path>) -> CIResult<Self> {
        let storage_path = storage_path.as_ref().to_path_buf();
        fs::create_dir_all(&storage_path).await?;

        info!("ArtifactCollector initialized at {:?}", storage_path);

        Ok(Self {
            storage_path,
            artifacts: dashmap::DashMap::new(),
        })
    }

    /// Collect artifact from test execution
    pub async fn collect(
        &self,
        test_name: &str,
        stdout: String,
        stderr: String,
        commit_hash: &str,
        stage: &str,
    ) -> CIResult<String> {
        let artifact_id = uuid::Uuid::new_v4().to_string();

        let metadata = ArtifactMetadata {
            artifact_id: artifact_id.clone(),
            test_name: test_name.to_string(),
            timestamp: Utc::now(),
            commit_hash: commit_hash.to_string(),
            pipeline_stage: stage.to_string(),
            tags: vec![],
            retention_days: 30,
        };

        let artifact = TestArtifact {
            metadata: metadata.clone(),
            stdout_log: stdout,
            stderr_log: stderr,
            trace_data: None,
            metrics_snapshot: None,
            status: ArtifactStatus::Collected,
            created_at: Utc::now(),
        };

        // Store artifact
        self.artifacts.insert(artifact_id.clone(), artifact.clone());

        // Write to disk
        self.write_artifact_to_disk(&artifact).await?;

        debug!("Artifact {} collected for {}", artifact_id, test_name);
        Ok(artifact_id)
    }

    /// Collect artifact with full context
    pub async fn collect_full(
        &self,
        test_name: &str,
        stdout: String,
        stderr: String,
        trace_data: Option<String>,
        metrics: Option<String>,
        commit_hash: &str,
        stage: &str,
    ) -> CIResult<String> {
        let artifact_id = uuid::Uuid::new_v4().to_string();

        let metadata = ArtifactMetadata {
            artifact_id: artifact_id.clone(),
            test_name: test_name.to_string(),
            timestamp: Utc::now(),
            commit_hash: commit_hash.to_string(),
            pipeline_stage: stage.to_string(),
            tags: vec!["full_context".to_string()],
            retention_days: 30,
        };

        let artifact = TestArtifact {
            metadata: metadata.clone(),
            stdout_log: stdout,
            stderr_log: stderr,
            trace_data,
            metrics_snapshot: metrics,
            status: ArtifactStatus::Collected,
            created_at: Utc::now(),
        };

        self.artifacts.insert(artifact_id.clone(), artifact.clone());
        self.write_artifact_to_disk(&artifact).await?;

        info!(
            "Full artifact {} collected for {} with traces and metrics",
            artifact_id, test_name
        );
        Ok(artifact_id)
    }

    /// Get artifact by ID
    pub fn get_artifact(&self, artifact_id: &str) -> Option<TestArtifact> {
        self.artifacts.get(artifact_id).map(|a| a.clone())
    }

    /// List all artifacts
    pub fn list_artifacts(&self) -> Vec<TestArtifact> {
        self.artifacts
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// List artifacts for specific test
    pub fn list_test_artifacts(&self, test_name: &str) -> Vec<TestArtifact> {
        self.artifacts
            .iter()
            .filter(|entry| entry.value().metadata.test_name == test_name)
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// List artifacts from specific stage
    pub fn list_stage_artifacts(&self, stage: &str) -> Vec<TestArtifact> {
        self.artifacts
            .iter()
            .filter(|entry| entry.value().metadata.pipeline_stage == stage)
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Mark artifact as indexed
    pub fn mark_indexed(&self, artifact_id: &str) -> CIResult<()> {
        if let Some(mut artifact) = self.artifacts.get_mut(artifact_id) {
            artifact.status = ArtifactStatus::Indexed;
            Ok(())
        } else {
            Err(CIError::ArtifactCollectionFailed(format!(
                "Artifact {} not found",
                artifact_id
            )))
        }
    }

    /// Archive artifact (compress and move to cold storage)
    pub async fn archive(&self, artifact_id: &str) -> CIResult<()> {
        if let Some(mut artifact) = self.artifacts.get_mut(artifact_id) {
            artifact.status = ArtifactStatus::Archived;

            let archive_path = self.storage_path.join(format!("{}.archive", artifact_id));
            let data = serde_json::to_vec(&*artifact)
                .map_err(|e| CIError::SerializationError(e))?;

            // In a real implementation, this would compress with zstd or gzip
            fs::write(&archive_path, data).await?;

            debug!("Artifact {} archived to {:?}", artifact_id, archive_path);
            Ok(())
        } else {
            Err(CIError::ArtifactCollectionFailed(format!(
                "Artifact {} not found",
                artifact_id
            )))
        }
    }

    /// Delete old artifacts based on retention policy
    pub async fn cleanup_expired(&self) -> CIResult<usize> {
        let now = Utc::now();
        let mut deleted = 0;

        let mut to_delete = Vec::new();
        for entry in self.artifacts.iter() {
            let artifact = entry.value();
            let age_days = (now - artifact.created_at).num_days() as u32;

            if age_days > artifact.metadata.retention_days {
                to_delete.push(entry.key().clone());
            }
        }

        for artifact_id in to_delete {
            self.artifacts.remove(&artifact_id);
            deleted += 1;
        }

        info!("Cleaned up {} expired artifacts", deleted);
        Ok(deleted)
    }

    /// Get artifact storage path
    pub fn storage_path(&self) -> &Path {
        &self.storage_path
    }

    /// Add tag to artifact
    pub fn add_tag(&self, artifact_id: &str, tag: &str) -> CIResult<()> {
        if let Some(mut artifact) = self.artifacts.get_mut(artifact_id) {
            if !artifact.metadata.tags.contains(&tag.to_string()) {
                artifact.metadata.tags.push(tag.to_string());
            }
            Ok(())
        } else {
            Err(CIError::ArtifactCollectionFailed(format!(
                "Artifact {} not found",
                artifact_id
            )))
        }
    }

    /// Enable deterministic replay: capture all needed info
    pub async fn enable_replay(
        &self,
        artifact_id: &str,
        test_params: &str,
    ) -> CIResult<()> {
        self.add_tag(artifact_id, "replay_enabled")?;

        if let Some(mut artifact) = self.artifacts.get_mut(artifact_id) {
            artifact.metadata.tags.push(format!("params:{}", test_params));
        }

        Ok(())
    }

    async fn write_artifact_to_disk(&self, artifact: &TestArtifact) -> CIResult<()> {
        let path = self
            .storage_path
            .join(format!("{}.json", artifact.metadata.artifact_id));
        let data = serde_json::to_string(artifact)
            .map_err(|e| CIError::SerializationError(e))?;
        fs::write(&path, &data).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_artifact_collection() {
        let collector = ArtifactCollector::new("/tmp/artifacts_test").await.unwrap();

        let artifact_id = collector
            .collect(
                "test_basic",
                "Test passed".to_string(),
                String::new(),
                "abc123",
                "smoke",
            )
            .await
            .unwrap();

        assert!(!artifact_id.is_empty());
        assert!(collector.get_artifact(&artifact_id).is_some());
    }

    #[tokio::test]
    async fn test_artifact_full_collection() {
        let collector = ArtifactCollector::new("/tmp/artifacts_test").await.unwrap();

        let artifact_id = collector
            .collect_full(
                "test_perf",
                "Latency: 100ms".to_string(),
                String::new(),
                Some("trace data".to_string()),
                Some("metrics data".to_string()),
                "def456",
                "full_suite",
            )
            .await
            .unwrap();

        let artifact = collector.get_artifact(&artifact_id).unwrap();
        assert!(artifact.trace_data.is_some());
        assert!(artifact.metrics_snapshot.is_some());
    }

    #[tokio::test]
    async fn test_list_test_artifacts() {
        let collector = ArtifactCollector::new("/tmp/artifacts_test").await.unwrap();

        let _ = collector
            .collect("test_a", "pass".to_string(), String::new(), "abc", "smoke")
            .await
            .unwrap();
        let _ = collector
            .collect("test_b", "pass".to_string(), String::new(), "abc", "smoke")
            .await
            .unwrap();

        let artifacts = collector.list_test_artifacts("test_a");
        assert_eq!(artifacts.len(), 1);
        assert_eq!(artifacts[0].metadata.test_name, "test_a");
    }

    #[tokio::test]
    async fn test_list_stage_artifacts() {
        let collector = ArtifactCollector::new("/tmp/artifacts_test").await.unwrap();

        let _ = collector
            .collect("test_a", "pass".to_string(), String::new(), "abc", "smoke")
            .await
            .unwrap();
        let _ = collector
            .collect("test_b", "pass".to_string(), String::new(), "abc", "full")
            .await
            .unwrap();

        let smoke_artifacts = collector.list_stage_artifacts("smoke");
        assert_eq!(smoke_artifacts.len(), 1);
        assert_eq!(smoke_artifacts[0].metadata.pipeline_stage, "smoke");
    }

    #[tokio::test]
    async fn test_mark_indexed() {
        let collector = ArtifactCollector::new("/tmp/artifacts_test").await.unwrap();

        let artifact_id = collector
            .collect("test_a", "pass".to_string(), String::new(), "abc", "smoke")
            .await
            .unwrap();

        assert!(collector.mark_indexed(&artifact_id).is_ok());
        let artifact = collector.get_artifact(&artifact_id).unwrap();
        assert_eq!(artifact.status, ArtifactStatus::Indexed);
    }

    #[tokio::test]
    async fn test_archive_artifact() {
        let collector = ArtifactCollector::new("/tmp/artifacts_test").await.unwrap();

        let artifact_id = collector
            .collect("test_a", "pass".to_string(), String::new(), "abc", "smoke")
            .await
            .unwrap();

        assert!(collector.archive(&artifact_id).await.is_ok());
        let artifact = collector.get_artifact(&artifact_id).unwrap();
        assert_eq!(artifact.status, ArtifactStatus::Archived);
    }

    #[tokio::test]
    async fn test_add_tag() {
        let collector = ArtifactCollector::new("/tmp/artifacts_test").await.unwrap();

        let artifact_id = collector
            .collect("test_a", "pass".to_string(), String::new(), "abc", "smoke")
            .await
            .unwrap();

        assert!(collector.add_tag(&artifact_id, "critical").is_ok());
        let artifact = collector.get_artifact(&artifact_id).unwrap();
        assert!(artifact.metadata.tags.contains(&"critical".to_string()));
    }

    #[tokio::test]
    async fn test_enable_replay() {
        let collector = ArtifactCollector::new("/tmp/artifacts_test").await.unwrap();

        let artifact_id = collector
            .collect("test_a", "pass".to_string(), String::new(), "abc", "smoke")
            .await
            .unwrap();

        assert!(collector
            .enable_replay(&artifact_id, "seed=42")
            .await
            .is_ok());

        let artifact = collector.get_artifact(&artifact_id).unwrap();
        assert!(artifact
            .metadata
            .tags
            .iter()
            .any(|t| t.contains("replay_enabled")));
    }

    #[tokio::test]
    async fn test_cleanup_expired() {
        let collector = ArtifactCollector::new("/tmp/artifacts_test").await.unwrap();

        let artifact_id = collector
            .collect("test_a", "pass".to_string(), String::new(), "abc", "smoke")
            .await
            .unwrap();

        // Manually set retention to 0 days
        if let Some(mut artifact) = collector.artifacts.get_mut(&artifact_id) {
            artifact.metadata.retention_days = 0;
        }

        let deleted = collector.cleanup_expired().await.unwrap();
        assert_eq!(deleted, 1);
        assert!(collector.get_artifact(&artifact_id).is_none());
    }

    #[test]
    fn test_artifact_metadata_creation() {
        let metadata = ArtifactMetadata {
            artifact_id: "test123".to_string(),
            test_name: "test_basic".to_string(),
            timestamp: Utc::now(),
            commit_hash: "abc123".to_string(),
            pipeline_stage: "smoke".to_string(),
            tags: vec!["critical".to_string()],
            retention_days: 30,
        };

        assert_eq!(metadata.artifact_id, "test123");
        assert_eq!(metadata.test_name, "test_basic");
        assert_eq!(metadata.tags.len(), 1);
    }

    #[tokio::test]
    async fn test_list_all_artifacts() {
        let collector = ArtifactCollector::new("/tmp/artifacts_test").await.unwrap();

        let _ = collector
            .collect("test_a", "pass".to_string(), String::new(), "abc", "smoke")
            .await
            .unwrap();
        let _ = collector
            .collect("test_b", "fail".to_string(), String::new(), "abc", "full")
            .await
            .unwrap();

        let all = collector.list_artifacts();
        assert!(all.len() >= 2);
    }
}
