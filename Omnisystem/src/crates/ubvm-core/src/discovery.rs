/// Service discovery trait for worker registry
use async_trait::async_trait;

#[async_trait]
pub trait WorkerRegistry: Send + Sync {
    /// Get list of available workers, optionally filtered by required language/capability
    async fn get_workers(&self, requirement: Option<&str>) -> anyhow::Result<Vec<String>>;

    /// Register a new worker
    async fn register_worker(&self, id: &str, capabilities: Vec<String>) -> anyhow::Result<()>;

    /// Deregister a worker
    async fn deregister_worker(&self, id: &str) -> anyhow::Result<()>;

    /// Health check for a worker
    async fn health_check(&self, id: &str) -> anyhow::Result<bool>;
}

/// Simple in-memory worker registry for testing
pub struct SimpleRegistry {
    workers: dashmap::DashMap<String, WorkerInfo>,
}

#[derive(Clone)]
struct WorkerInfo {
    healthy: bool,
}

impl SimpleRegistry {
    pub fn new() -> Self {
        Self {
            workers: dashmap::DashMap::new(),
        }
    }

    /// Add a test worker
    pub fn add_test_worker(&self, id: &str) {
        self.workers.insert(
            id.to_string(),
            WorkerInfo {
                healthy: true,
            },
        );
    }
}

impl Default for SimpleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WorkerRegistry for SimpleRegistry {
    async fn get_workers(&self, _requirement: Option<&str>) -> anyhow::Result<Vec<String>> {
        Ok(self
            .workers
            .iter()
            .filter(|entry| entry.value().healthy)
            .map(|entry| entry.key().clone())
            .collect())
    }

    async fn register_worker(&self, id: &str, _capabilities: Vec<String>) -> anyhow::Result<()> {
        self.workers.insert(
            id.to_string(),
            WorkerInfo {
                healthy: true,
            },
        );
        Ok(())
    }

    async fn deregister_worker(&self, id: &str) -> anyhow::Result<()> {
        self.workers.remove(id);
        Ok(())
    }

    async fn health_check(&self, id: &str) -> anyhow::Result<bool> {
        Ok(self
            .workers
            .get(id)
            .map(|w| w.value().healthy)
            .unwrap_or(false))
    }
}
