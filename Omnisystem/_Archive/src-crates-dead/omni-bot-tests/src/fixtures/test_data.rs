//! Test data builders and fixtures

use serde_json::{json, Value};
use uuid::Uuid;
use chrono::Utc;

/// Builder for creating test data
pub struct TestDataBuilder {
    service_name: String,
    service_version: String,
    env_name: String,
    module_count: usize,
    config: Value,
}

impl TestDataBuilder {
    /// Create a new test data builder
    pub fn new() -> Self {
        Self {
            service_name: "test-service".to_string(),
            service_version: "1.0.0".to_string(),
            env_name: "test-env".to_string(),
            module_count: 1,
            config: json!({}),
        }
    }

    /// Set service name
    pub fn with_service_name(mut self, name: String) -> Self {
        self.service_name = name;
        self
    }

    /// Set service version
    pub fn with_service_version(mut self, version: String) -> Self {
        self.service_version = version;
        self
    }

    /// Set environment name
    pub fn with_env_name(mut self, name: String) -> Self {
        self.env_name = name;
        self
    }

    /// Set module count
    pub fn with_module_count(mut self, count: usize) -> Self {
        self.module_count = count;
        self
    }

    /// Set custom config
    pub fn with_config(mut self, config: Value) -> Self {
        self.config = config;
        self
    }

    /// Build service request
    pub fn build_service_config(&self) -> Value {
        json!({
            "name": self.service_name,
            "version": self.service_version,
            "config": self.config
        })
    }

    /// Build environment config
    pub fn build_env_config(&self) -> Value {
        json!({
            "id": Uuid::new_v4().to_string(),
            "name": self.env_name,
            "state": "active",
            "services": [],
            "created_at": Utc::now().to_rfc3339(),
            "config": self.config
        })
    }

    /// Build module list
    pub fn build_modules(&self) -> Vec<Value> {
        (0..self.module_count)
            .map(|i| {
                json!({
                    "id": Uuid::new_v4().to_string(),
                    "name": format!("module-{}", i),
                    "version": "1.0.0",
                    "signature": format!("sig-{}", Uuid::new_v4()),
                    "dependencies": [],
                    "installed": false
                })
            })
            .collect()
    }

    /// Build workflow DAG
    pub fn build_workflow_dag(&self) -> Value {
        json!({
            "id": Uuid::new_v4().to_string(),
            "name": "test-workflow",
            "steps": [
                {
                    "name": "setup",
                    "command": "setup.sh",
                    "depends_on": [],
                    "status": "pending"
                },
                {
                    "name": "execute",
                    "command": "execute.sh",
                    "depends_on": ["setup"],
                    "status": "pending"
                },
                {
                    "name": "cleanup",
                    "command": "cleanup.sh",
                    "depends_on": ["execute"],
                    "status": "pending"
                }
            ]
        })
    }

    /// Build validation test
    pub fn build_validation_test(&self) -> Value {
        json!({
            "id": Uuid::new_v4().to_string(),
            "name": "validation-test",
            "type": "deterministic",
            "replay_enabled": true,
            "heatmap_enabled": true,
            "tests": [
                {
                    "name": "test-1",
                    "script": "test_1.sh",
                    "timeout_seconds": 30
                },
                {
                    "name": "test-2",
                    "script": "test_2.sh",
                    "timeout_seconds": 60
                }
            ]
        })
    }

    /// Build asset generation request
    pub fn build_asset_request(&self) -> Value {
        json!({
            "id": Uuid::new_v4().to_string(),
            "name": "asset-generation",
            "type": "image",
            "config": {
                "format": "png",
                "resolution": "1920x1080"
            }
        })
    }

    /// Build capability token request
    pub fn build_capability_request(&self) -> Value {
        json!({
            "user_id": "test-user",
            "capabilities": [
                "SERVICE:*",
                "ENVIRONMENT:create",
                "ENVIRONMENT:delete"
            ],
            "expires_in_hours": 24
        })
    }

    /// Build nested environment structure
    pub fn build_nested_environments(&self, depth: usize) -> Vec<Value> {
        let mut envs = vec![];
        for i in 0..depth {
            envs.push(json!({
                "id": Uuid::new_v4().to_string(),
                "name": format!("env-{}", i),
                "parent_id": if i > 0 { Some(format!("env-{}", i - 1)) } else { None },
                "state": "active"
            }));
        }
        envs
    }

    /// Build resource limits config
    pub fn build_resource_limits(&self) -> Value {
        json!({
            "cpu_percent": 50.0,
            "memory_mb": 2048,
            "disk_mb": 10240,
            "bandwidth_mbps": 1000.0,
            "timeout_seconds": 3600
        })
    }

    /// Build snapshot metadata
    pub fn build_snapshot(&self) -> Value {
        json!({
            "id": Uuid::new_v4().to_string(),
            "name": "snapshot-1",
            "timestamp": Utc::now().to_rfc3339(),
            "size_bytes": 1024 * 1024,
            "description": "Test snapshot",
            "state": "completed"
        })
    }
}

impl Default for TestDataBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_defaults() {
        let builder = TestDataBuilder::new();
        let config = builder.build_service_config();
        assert_eq!(config["name"], "test-service");
        assert_eq!(config["version"], "1.0.0");
    }

    #[test]
    fn test_builder_customization() {
        let config = TestDataBuilder::new()
            .with_service_name("custom-service".to_string())
            .with_service_version("2.0.0".to_string())
            .build_service_config();

        assert_eq!(config["name"], "custom-service");
        assert_eq!(config["version"], "2.0.0");
    }

    #[test]
    fn test_build_modules() {
        let modules = TestDataBuilder::new()
            .with_module_count(5)
            .build_modules();
        assert_eq!(modules.len(), 5);
    }

    #[test]
    fn test_build_workflow_dag() {
        let workflow = TestDataBuilder::new().build_workflow_dag();
        assert_eq!(workflow["steps"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_build_nested_environments() {
        let envs = TestDataBuilder::new().build_nested_environments(3);
        assert_eq!(envs.len(), 3);
    }

    #[test]
    fn test_snapshot_metadata() {
        let snapshot = TestDataBuilder::new().build_snapshot();
        assert!(snapshot["id"].is_string());
        assert_eq!(snapshot["name"], "snapshot-1");
    }
}
