//! Test execution context

use crate::errors::ApplicationStressResult;
use crate::ApplicationMetrics;
use std::sync::Arc;

/// Test execution context containing shared resources
#[derive(Clone)]
pub struct TestContext {
    pub run_id: String,
    pub test_id: String,
    pub metrics: Arc<ApplicationMetrics>,
    pub artifact_dir: std::path::PathBuf,
    pub timeout_secs: u64,
}

impl TestContext {
    /// Create a new test context
    pub fn new(
        run_id: impl Into<String>,
        test_id: impl Into<String>,
        metrics: Arc<ApplicationMetrics>,
        artifact_dir: std::path::PathBuf,
        timeout_secs: u64,
    ) -> Self {
        Self {
            run_id: run_id.into(),
            test_id: test_id.into(),
            metrics,
            artifact_dir,
            timeout_secs,
        }
    }

    /// Save test artifact
    pub async fn save_artifact(
        &self,
        filename: impl AsRef<std::path::Path>,
        content: impl AsRef<[u8]>,
    ) -> ApplicationStressResult<std::path::PathBuf> {
        let path = self.artifact_dir.join(filename);

        tokio::fs::create_dir_all(&self.artifact_dir).await?;
        tokio::fs::write(&path, content).await?;

        Ok(path)
    }

    /// Load test artifact
    pub async fn load_artifact(
        &self,
        filename: impl AsRef<std::path::Path>,
    ) -> ApplicationStressResult<Vec<u8>> {
        let path = self.artifact_dir.join(filename);
        Ok(tokio::fs::read(path).await?)
    }

    /// Check artifact exists
    pub async fn artifact_exists(
        &self,
        filename: impl AsRef<std::path::Path>,
    ) -> ApplicationStressResult<bool> {
        let path = self.artifact_dir.join(filename);
        Ok(tokio::fs::try_exists(&path).await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_creation() {
        let metrics = Arc::new(ApplicationMetrics::new());
        let ctx = TestContext::new(
            "run-001",
            "test-001",
            metrics,
            std::path::PathBuf::from("./artifacts"),
            300,
        );

        assert_eq!(ctx.run_id, "run-001");
        assert_eq!(ctx.test_id, "test-001");
        assert_eq!(ctx.timeout_secs, 300);
    }

    #[tokio::test]
    async fn test_artifact_save_and_load() {
        let metrics = Arc::new(ApplicationMetrics::new());
        let temp_dir = tempfile::TempDir::new().unwrap();
        let ctx = TestContext::new(
            "run-001",
            "test-001",
            metrics,
            temp_dir.path().to_path_buf(),
            300,
        );

        let content = b"test content";
        let path = ctx.save_artifact("test.txt", content).await.unwrap();
        assert!(path.exists());

        let loaded = ctx.load_artifact("test.txt").await.unwrap();
        assert_eq!(loaded, content);
    }

    #[tokio::test]
    async fn test_artifact_exists() {
        let metrics = Arc::new(ApplicationMetrics::new());
        let temp_dir = tempfile::TempDir::new().unwrap();
        let ctx = TestContext::new(
            "run-001",
            "test-001",
            metrics,
            temp_dir.path().to_path_buf(),
            300,
        );

        ctx.save_artifact("test.txt", b"content").await.unwrap();
        assert!(ctx.artifact_exists("test.txt").await.unwrap());
        assert!(!ctx.artifact_exists("nonexistent.txt").await.unwrap());
    }
}
