/// Content-addressed test result storage backend
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub test_id: String,
    pub test_name: String,
    pub result: TestStatus,
    pub duration_ms: u128,
    pub output: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestStatus {
    Passed,
    Failed,
    Skipped,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageEntry {
    pub content_hash: String,
    pub result: TestResult,
    pub storage_path: String,
}

pub struct TestStorage {
    results: Arc<RwLock<HashMap<String, StorageEntry>>>,
    storage_dir: String,
}

impl TestStorage {
    pub fn new(storage_dir: String) -> Self {
        Self {
            results: Arc::new(RwLock::new(HashMap::new())),
            storage_dir,
        }
    }

    pub async fn store_result(&self, result: TestResult) -> Result<String> {
        let content_hash = Self::hash_content(&result);
        let storage_path = format!("{}/{}.json", self.storage_dir, content_hash);

        let entry = StorageEntry {
            content_hash: content_hash.clone(),
            result,
            storage_path: storage_path.clone(),
        };

        let mut results = self.results.write().await;
        results.insert(content_hash.clone(), entry);

        tracing::info!("Stored test result: {} at {}", content_hash, storage_path);
        Ok(content_hash)
    }

    pub async fn retrieve_result(&self, content_hash: &str) -> Result<Option<TestResult>> {
        let results = self.results.read().await;
        Ok(results.get(content_hash).map(|e| e.result.clone()))
    }

    pub async fn get_all_results(&self) -> Result<Vec<TestResult>> {
        let results = self.results.read().await;
        Ok(results.values().map(|e| e.result.clone()).collect())
    }

    pub async fn get_results_by_status(&self, status: TestStatus) -> Result<Vec<TestResult>> {
        let results = self.results.read().await;
        Ok(results
            .values()
            .filter(|e| e.result.result == status)
            .map(|e| e.result.clone())
            .collect())
    }

    pub async fn delete_result(&self, content_hash: &str) -> Result<()> {
        let mut results = self.results.write().await;
        results.remove(content_hash);
        tracing::info!("Deleted result: {}", content_hash);
        Ok(())
    }

    pub async fn purge_old_results(&self, days_old: i64) -> Result<usize> {
        let now = chrono::Utc::now().timestamp();
        let cutoff = now - (days_old * 86400);

        let mut results = self.results.write().await;
        let original_count = results.len();
        results.retain(|_, e| e.result.timestamp > cutoff);
        let deleted_count = original_count - results.len();

        tracing::info!("Purged {} old results", deleted_count);
        Ok(deleted_count)
    }

    fn hash_content(result: &TestResult) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        result.test_name.hash(&mut hasher);
        result.output.hash(&mut hasher);
        result.timestamp.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

impl Default for TestStorage {
    fn default() -> Self {
        Self::new("./test_storage".to_string())
    }
}

/// Statistics for a test spec run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecStats {
    pub spec_name: String,
    pub total_tests: usize,
    pub passed: usize,
    pub failed: usize,
    pub success_rate: f64,
    pub avg_fidelity: f64,
    pub total_execution_time_ms: u64,
}

/// Log an event to Universe
pub async fn log_event_to_universe(event: &str) -> Result<()> {
    tracing::info!("[Universe] {}", event);
    Ok(())
}

/// Store a BLAKE3 hash of test artifacts (for content-addressed storage)
pub async fn store_artifact_hash(hash: &str, _content: &[u8]) -> Result<()> {
    tracing::debug!("[CAS] Stored artifact with hash: {}", hash);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_result(test_id: &str, name: &str, status: TestStatus) -> TestResult {
        TestResult {
            test_id: test_id.to_string(),
            test_name: name.to_string(),
            result: status,
            duration_ms: 10,
            output: "5".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    #[tokio::test]
    async fn test_store_and_retrieve_result() {
        let storage = TestStorage::new("./test_storage_tmp".to_string());
        let result = sample_result("t1", "case1", TestStatus::Passed);

        let hash = storage.store_result(result.clone()).await.unwrap();
        let retrieved = storage.retrieve_result(&hash).await.unwrap();

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().test_name, "case1");
    }

    #[tokio::test]
    async fn test_get_all_results() {
        let storage = TestStorage::new("./test_storage_tmp".to_string());
        storage
            .store_result(sample_result("t1", "case1", TestStatus::Passed))
            .await
            .unwrap();

        let all = storage.get_all_results().await.unwrap();
        assert_eq!(all.len(), 1);
    }

    #[tokio::test]
    async fn test_get_results_by_status() {
        let storage = TestStorage::new("./test_storage_tmp".to_string());
        storage
            .store_result(sample_result("t1", "case1", TestStatus::Passed))
            .await
            .unwrap();
        storage
            .store_result(sample_result("t2", "case2", TestStatus::Failed))
            .await
            .unwrap();
        storage
            .store_result(sample_result("t3", "case3", TestStatus::Passed))
            .await
            .unwrap();

        let passed = storage.get_results_by_status(TestStatus::Passed).await.unwrap();
        assert_eq!(passed.len(), 2);

        let failed = storage.get_results_by_status(TestStatus::Failed).await.unwrap();
        assert_eq!(failed.len(), 1);
    }

    #[tokio::test]
    async fn test_delete_result() {
        let storage = TestStorage::new("./test_storage_tmp".to_string());
        let hash = storage
            .store_result(sample_result("t1", "case1", TestStatus::Passed))
            .await
            .unwrap();

        storage.delete_result(&hash).await.unwrap();
        let retrieved = storage.retrieve_result(&hash).await.unwrap();
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_log_event_and_store_artifact_hash_do_not_error() {
        log_event_to_universe("test event").await.unwrap();
        store_artifact_hash("abc123", b"content").await.unwrap();
    }
}
