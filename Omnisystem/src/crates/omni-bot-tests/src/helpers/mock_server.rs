//! Mock Omni-Bot server for testing

use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use omni_bot_core::{ServiceState, ServiceStatus};

/// Mock service state in-memory storage
#[derive(Debug, Clone)]
pub struct MockService {
    pub name: String,
    pub version: String,
    pub state: ServiceState,
    pub status: ServiceStatus,
    pub uptime_seconds: u64,
    pub pid: Option<u32>,
    pub config: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

impl MockService {
    pub fn new(name: String) -> Self {
        Self {
            name,
            version: "1.0.0".to_string(),
            state: ServiceState::Unstarted,
            status: ServiceStatus::Unknown,
            uptime_seconds: 0,
            pid: None,
            config: serde_json::json!({}),
            created_at: Utc::now(),
            last_updated: Utc::now(),
        }
    }
}

/// Mock environment
#[derive(Debug, Clone)]
pub struct MockEnvironment {
    pub id: String,
    pub name: String,
    pub state: String,
    pub services: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub config: serde_json::Value,
    pub snapshots: HashMap<String, serde_json::Value>,
}

impl MockEnvironment {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            state: "active".to_string(),
            services: Vec::new(),
            created_at: Utc::now(),
            config: serde_json::json!({}),
            snapshots: HashMap::new(),
        }
    }
}

/// Mock module
#[derive(Debug, Clone)]
pub struct MockModule {
    pub id: String,
    pub name: String,
    pub version: String,
    pub signature: String,
    pub dependencies: Vec<String>,
    pub installed: bool,
    pub created_at: DateTime<Utc>,
}

impl MockModule {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            version: "1.0.0".to_string(),
            signature: format!("sig-{}", Uuid::new_v4()),
            dependencies: Vec::new(),
            installed: false,
            created_at: Utc::now(),
        }
    }
}

/// Mock asset generation job
#[derive(Debug, Clone)]
pub struct MockAssetJob {
    pub id: String,
    pub name: String,
    pub progress: u32,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub metadata: serde_json::Value,
}

impl MockAssetJob {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            progress: 0,
            status: "pending".to_string(),
            created_at: Utc::now(),
            completed_at: None,
            metadata: serde_json::json!({}),
        }
    }
}

/// Mock validation test result
#[derive(Debug, Clone)]
pub struct MockValidationResult {
    pub id: String,
    pub test_name: String,
    pub status: String,
    pub passed: u32,
    pub failed: u32,
    pub output: String,
    pub timestamp: DateTime<Utc>,
    pub heatmap: HashMap<String, u32>,
}

impl MockValidationResult {
    pub fn new(test_name: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            test_name,
            status: "completed".to_string(),
            passed: 0,
            failed: 0,
            output: String::new(),
            timestamp: Utc::now(),
            heatmap: HashMap::new(),
        }
    }
}

/// Mock workflow
#[derive(Debug, Clone)]
pub struct MockWorkflow {
    pub id: String,
    pub name: String,
    pub steps: Vec<WorkflowStep>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct WorkflowStep {
    pub name: String,
    pub command: String,
    pub depends_on: Vec<String>,
    pub status: String,
}

impl MockWorkflow {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            steps: Vec::new(),
            status: "pending".to_string(),
            created_at: Utc::now(),
            completed_at: None,
        }
    }
}

/// Mock Omni-Bot server
pub struct MockServer {
    pub services: Arc<RwLock<HashMap<String, MockService>>>,
    pub environments: Arc<RwLock<HashMap<String, MockEnvironment>>>,
    pub modules: Arc<RwLock<HashMap<String, MockModule>>>,
    pub assets: Arc<RwLock<HashMap<String, MockAssetJob>>>,
    pub validations: Arc<RwLock<HashMap<String, MockValidationResult>>>,
    pub workflows: Arc<RwLock<HashMap<String, MockWorkflow>>>,
    pub call_counts: Arc<RwLock<HashMap<String, u32>>>,
    pub error_mode: Arc<RwLock<Option<String>>>,
}

impl MockServer {
    /// Create a new mock server
    pub fn new() -> Self {
        Self {
            services: Arc::new(RwLock::new(HashMap::new())),
            environments: Arc::new(RwLock::new(HashMap::new())),
            modules: Arc::new(RwLock::new(HashMap::new())),
            assets: Arc::new(RwLock::new(HashMap::new())),
            validations: Arc::new(RwLock::new(HashMap::new())),
            workflows: Arc::new(RwLock::new(HashMap::new())),
            call_counts: Arc::new(RwLock::new(HashMap::new())),
            error_mode: Arc::new(RwLock::new(None)),
        }
    }

    /// Record a method call for verification
    pub fn record_call(&self, method: impl Into<String>) {
        let method = method.into();
        let mut counts = self.call_counts.write();
        *counts.entry(method).or_insert(0) += 1;
    }

    /// Get call count for a method
    pub fn get_call_count(&self, method: &str) -> u32 {
        self.call_counts.read().get(method).copied().unwrap_or(0)
    }

    /// Set error mode for testing failure paths
    pub fn set_error_mode(&self, error: Option<String>) {
        *self.error_mode.write() = error;
    }

    /// Check if in error mode
    pub fn is_error_mode(&self) -> bool {
        self.error_mode.read().is_some()
    }

    /// Get error message if in error mode
    pub fn get_error_message(&self) -> Option<String> {
        self.error_mode.read().clone()
    }

    // Service operations
    pub fn create_service(&self, service: MockService) {
        self.record_call("create_service");
        self.services.write().insert(service.name.clone(), service);
    }

    pub fn get_service(&self, name: &str) -> Option<MockService> {
        self.record_call("get_service");
        self.services.read().get(name).cloned()
    }

    pub fn update_service_state(&self, name: &str, state: ServiceState) {
        self.record_call("update_service_state");
        if let Some(service) = self.services.write().get_mut(name) {
            service.state = state;
            service.last_updated = Utc::now();
        }
    }

    pub fn delete_service(&self, name: &str) -> bool {
        self.record_call("delete_service");
        self.services.write().remove(name).is_some()
    }

    pub fn list_services(&self) -> Vec<MockService> {
        self.record_call("list_services");
        self.services.read().values().cloned().collect()
    }

    // Environment operations
    pub fn create_environment(&self, env: MockEnvironment) {
        self.record_call("create_environment");
        self.environments.write().insert(env.id.clone(), env);
    }

    pub fn get_environment(&self, id: &str) -> Option<MockEnvironment> {
        self.record_call("get_environment");
        self.environments.read().get(id).cloned()
    }

    pub fn delete_environment(&self, id: &str) -> bool {
        self.record_call("delete_environment");
        self.environments.write().remove(id).is_some()
    }

    pub fn list_environments(&self) -> Vec<MockEnvironment> {
        self.record_call("list_environments");
        self.environments.read().values().cloned().collect()
    }

    pub fn create_snapshot(&self, env_id: &str, snapshot_id: String) {
        self.record_call("create_snapshot");
        if let Some(env) = self.environments.write().get_mut(env_id) {
            env.snapshots
                .insert(snapshot_id, serde_json::json!({"state": env.state}));
        }
    }

    // Module operations
    pub fn install_module(&self, module: MockModule) {
        self.record_call("install_module");
        self.modules.write().insert(module.id.clone(), module);
    }

    pub fn get_module(&self, id: &str) -> Option<MockModule> {
        self.record_call("get_module");
        self.modules.read().get(id).cloned()
    }

    pub fn remove_module(&self, id: &str) -> bool {
        self.record_call("remove_module");
        self.modules.write().remove(id).is_some()
    }

    pub fn list_modules(&self) -> Vec<MockModule> {
        self.record_call("list_modules");
        self.modules.read().values().cloned().collect()
    }

    // Asset operations
    pub fn create_asset_job(&self, job: MockAssetJob) {
        self.record_call("create_asset_job");
        self.assets.write().insert(job.id.clone(), job);
    }

    pub fn get_asset_job(&self, id: &str) -> Option<MockAssetJob> {
        self.record_call("get_asset_job");
        self.assets.read().get(id).cloned()
    }

    pub fn update_asset_progress(&self, id: &str, progress: u32) {
        self.record_call("update_asset_progress");
        if let Some(job) = self.assets.write().get_mut(id) {
            job.progress = progress;
            if progress >= 100 {
                job.status = "completed".to_string();
                job.completed_at = Some(Utc::now());
            }
        }
    }

    // Validation operations
    pub fn create_validation_result(&self, result: MockValidationResult) {
        self.record_call("create_validation_result");
        self.validations.write().insert(result.id.clone(), result);
    }

    pub fn get_validation_result(&self, id: &str) -> Option<MockValidationResult> {
        self.record_call("get_validation_result");
        self.validations.read().get(id).cloned()
    }

    // Workflow operations
    pub fn create_workflow(&self, workflow: MockWorkflow) {
        self.record_call("create_workflow");
        self.workflows.write().insert(workflow.id.clone(), workflow);
    }

    pub fn get_workflow(&self, id: &str) -> Option<MockWorkflow> {
        self.record_call("get_workflow");
        self.workflows.read().get(id).cloned()
    }

    pub fn update_workflow_status(&self, id: &str, status: String) {
        self.record_call("update_workflow_status");
        if let Some(workflow) = self.workflows.write().get_mut(id) {
            workflow.status = status;
            if workflow.status == "completed" {
                workflow.completed_at = Some(Utc::now());
            }
        }
    }

    /// Clear all state
    pub fn reset(&self) {
        self.services.write().clear();
        self.environments.write().clear();
        self.modules.write().clear();
        self.assets.write().clear();
        self.validations.write().clear();
        self.workflows.write().clear();
        self.call_counts.write().clear();
        self.error_mode.write().take();
    }
}

impl Default for MockServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_server_creation() {
        let server = MockServer::new();
        assert_eq!(server.list_services().len(), 0);
    }

    #[test]
    fn test_mock_service_creation() {
        let server = MockServer::new();
        let service = MockService::new("test-service".to_string());
        server.create_service(service);
        assert_eq!(server.list_services().len(), 1);
    }

    #[test]
    fn test_call_counting() {
        let server = MockServer::new();
        server.record_call("test_method");
        server.record_call("test_method");
        assert_eq!(server.get_call_count("test_method"), 2);
    }

    #[test]
    fn test_error_mode() {
        let server = MockServer::new();
        assert!(!server.is_error_mode());
        server.set_error_mode(Some("Test error".to_string()));
        assert!(server.is_error_mode());
        assert_eq!(server.get_error_message(), Some("Test error".to_string()));
    }
}
