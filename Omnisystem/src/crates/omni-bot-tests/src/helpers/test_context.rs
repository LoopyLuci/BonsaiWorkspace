//! Test context for managing test lifecycle

use std::sync::Arc;
use std::collections::HashMap;
use parking_lot::RwLock;

use crate::helpers::{MockServer, TestClient};

/// Test context managing server and client lifecycle
pub struct TestContext {
    pub server: Arc<MockServer>,
    pub client: TestClient,
    pub metadata: Arc<RwLock<HashMap<String, String>>>,
}

impl TestContext {
    /// Create a new test context
    pub fn new() -> Self {
        let server = Arc::new(MockServer::new());
        let client = TestClient::new(Arc::clone(&server));

        Self {
            server,
            client,
            metadata: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Set metadata for the test
    pub fn set_metadata(&self, key: String, value: String) {
        self.metadata.write().insert(key, value);
    }

    /// Get metadata
    pub fn get_metadata(&self, key: &str) -> Option<String> {
        self.metadata.read().get(key).cloned()
    }

    /// Clean up test resources
    pub async fn cleanup(&self) {
        self.server.reset();
        self.metadata.write().clear();
    }

    /// Create a scoped context for sub-tests
    pub fn scoped(&self) -> Self {
        Self {
            server: Arc::clone(&self.server),
            client: self.client.clone(),
            metadata: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for TestContext {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for TestContext {
    fn clone(&self) -> Self {
        Self {
            server: Arc::clone(&self.server),
            client: self.client.clone(),
            metadata: Arc::new(RwLock::new(self.metadata.read().clone())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_creation() {
        let ctx = TestContext::new();
        assert_eq!(ctx.get_metadata("test"), None);
    }

    #[test]
    fn test_context_metadata() {
        let ctx = TestContext::new();
        ctx.set_metadata("key".to_string(), "value".to_string());
        assert_eq!(ctx.get_metadata("key"), Some("value".to_string()));
    }

    #[tokio::test]
    async fn test_context_cleanup() {
        let ctx = TestContext::new();
        ctx.set_metadata("key".to_string(), "value".to_string());
        ctx.cleanup().await;
        assert_eq!(ctx.get_metadata("key"), None);
    }
}
