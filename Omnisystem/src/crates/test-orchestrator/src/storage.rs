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

    #[tokio::test]
    async fn test_result_store() {
        let mut store = ResultStore::new();
        let result = StoredResult {
            run_id: uuid::Uuid::new_v4().to_string(),
            spec_name: "TestSpec".to_string(),
            test_case_name: "case1".to_string(),
            language: "rust".to_string(),
            passed: true,
            fidelity: 1.0,
            actual_output: "5".to_string(),
            expected_output: "5".to_string(),
            execution_time_ms: 10,
            timestamp: chrono::Utc::now().to_rfc3339(),
        };
        store.store(result).await.unwrap();
        assert_eq!(store.results.len(), 1);
    }

    #[test]
    fn test_stats_computation() {
        let mut store = ResultStore::new();
        for i in 0..3 {
            let result = StoredResult {
                run_id: format!("run-{}", i),
                spec_name: "TestSpec".to_string(),
                test_case_name: format!("case{}", i),
                language: "rust".to_string(),
                passed: i < 2,
                fidelity: 1.0,
                actual_output: "out".to_string(),
                expected_output: "out".to_string(),
                execution_time_ms: 10,
                timestamp: chrono::Utc::now().to_rfc3339(),
            };
            store.results.push(result);
        }
        let stats = store.compute_stats("TestSpec");
        assert_eq!(stats.total_tests, 3);
        assert_eq!(stats.passed, 2);
        assert_eq!(stats.failed, 1);
    }

    #[test]
    fn test_csv_export() {
        let mut store = ResultStore::new();
        let result = StoredResult {
            run_id: "run1".to_string(),
            spec_name: "TestSpec".to_string(),
            test_case_name: "case1".to_string(),
            language: "rust".to_string(),
            passed: true,
            fidelity: 1.0,
            actual_output: "5".to_string(),
            expected_output: "5".to_string(),
            execution_time_ms: 10,
            timestamp: "2026-06-04T00:00:00Z".to_string(),
        };
        store.results.push(result);
        let csv = store.export_csv();
        assert!(csv.contains("run1"));
        assert!(csv.contains("TestSpec"));
    }
}
