//! Test HTTP client for API testing

use std::sync::Arc;
use std::collections::HashMap;
use serde_json::json;

use omni_bot_core::{ServiceState, ServiceStatus};

use crate::helpers::MockServer;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StartServiceRequest {
    pub name: String,
    pub config: Option<serde_json::Value>,
    pub wait_for_ready: Option<bool>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StopServiceRequest {
    pub name: String,
    pub graceful: Option<bool>,
    pub timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RestartServiceRequest {
    pub name: String,
    pub graceful: Option<bool>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConfigureServiceRequest {
    pub name: String,
    pub config: serde_json::Value,
    pub merge: Option<bool>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SnapshotServiceRequest {
    pub name: String,
    pub snapshot_name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LogsRequest {
    pub name: String,
    pub lines: Option<u32>,
    pub follow: Option<bool>,
    pub filter: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ServiceListResponse {
    pub services: Vec<ServiceSummary>,
    pub total_count: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ServiceSummary {
    pub name: String,
    pub version: String,
    pub state: String,
    pub status: String,
    pub uptime_seconds: u64,
    pub pid: Option<u32>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ServiceDetailResponse {
    pub name: String,
    pub version: String,
    pub state: String,
    pub status: String,
    pub uptime_seconds: u64,
    pub pid: Option<u32>,
    pub cpu_percent: f32,
    pub memory_mb: u32,
    pub disk_mb: u32,
    pub bandwidth_mbps: f32,
    pub last_health_check: chrono::DateTime<chrono::Utc>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StartServiceResponse {
    pub name: String,
    pub state: String,
    pub pid: Option<u32>,
    pub message: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StopServiceResponse {
    pub name: String,
    pub state: String,
    pub uptime_seconds: u64,
    pub message: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RestartServiceResponse {
    pub name: String,
    pub state: String,
    pub old_pid: Option<u32>,
    pub new_pid: Option<u32>,
    pub message: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConfigureServiceResponse {
    pub name: String,
    pub config: serde_json::Value,
    pub applied: bool,
    pub message: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SnapshotResponse {
    pub name: String,
    pub snapshot_id: String,
    pub snapshot_name: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub size_bytes: u64,
    pub message: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LogsResponse {
    pub name: String,
    pub lines: Vec<LogLine>,
    pub total_lines: usize,
    pub truncated: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LogLine {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub level: String,
    pub message: String,
}

/// Test HTTP client for making API calls
pub struct TestClient {
    pub server: Arc<MockServer>,
    pub headers: HashMap<String, String>,
    pub base_url: String,
}

impl TestClient {
    /// Create a new test client
    pub fn new(server: Arc<MockServer>) -> Self {
        Self {
            server,
            headers: HashMap::new(),
            base_url: "http://localhost:8080/api".to_string(),
        }
    }

    /// Add a header
    pub fn with_header(mut self, key: String, value: String) -> Self {
        self.headers.insert(key, value);
        self
    }

    /// Set authorization token
    pub fn with_auth(self, token: String) -> Self {
        self.with_header("Authorization".to_string(), format!("Bearer {}", token))
    }

    /// Set base URL
    pub fn with_base_url(mut self, url: String) -> Self {
        self.base_url = url;
        self
    }

    // Service Management APIs
    pub async fn start_service(
        &self,
        name: &str,
        _config: Option<serde_json::Value>,
    ) -> Result<StartServiceResponse, Box<dyn std::error::Error>> {
        if self.server.is_error_mode() {
            return Err(self.server.get_error_message().unwrap_or_default().into());
        }

        let mut service = omni_bot_core::ServiceInfo::new(name.to_string(), "1.0.0".to_string());
        service.state = ServiceState::Booting;

        let response = StartServiceResponse {
            name: name.to_string(),
            state: "running".to_string(),
            pid: Some(12345),
            message: "Service started successfully".to_string(),
        };

        Ok(response)
    }

    pub async fn stop_service(
        &self,
        name: &str,
        _graceful: Option<bool>,
        _timeout: Option<u64>,
    ) -> Result<StopServiceResponse, Box<dyn std::error::Error>> {
        if self.server.is_error_mode() {
            return Err(self.server.get_error_message().unwrap_or_default().into());
        }

        let response = StopServiceResponse {
            name: name.to_string(),
            state: "stopped".to_string(),
            uptime_seconds: 3600,
            message: "Service stopped successfully".to_string(),
        };

        Ok(response)
    }

    pub async fn restart_service(
        &self,
        name: &str,
        _graceful: Option<bool>,
    ) -> Result<RestartServiceResponse, Box<dyn std::error::Error>> {
        if self.server.is_error_mode() {
            return Err(self.server.get_error_message().unwrap_or_default().into());
        }

        let response = RestartServiceResponse {
            name: name.to_string(),
            state: "running".to_string(),
            old_pid: Some(12345),
            new_pid: Some(12346),
            message: "Service restarted successfully".to_string(),
        };

        Ok(response)
    }

    pub async fn list_services(&self) -> Result<ServiceListResponse, Box<dyn std::error::Error>> {
        if self.server.is_error_mode() {
            return Err(self.server.get_error_message().unwrap_or_default().into());
        }

        let services = self.server.list_services();
        let service_count = services.len();
        let summaries = services
            .into_iter()
            .map(|s| ServiceSummary {
                name: s.name,
                version: s.version,
                state: match s.state {
                    ServiceState::Running => "running".to_string(),
                    ServiceState::Stopped => "stopped".to_string(),
                    ServiceState::Booting => "booting".to_string(),
                    ServiceState::Stopping => "stopping".to_string(),
                    ServiceState::Paused => "paused".to_string(),
                    ServiceState::Pausing => "pausing".to_string(),
                    ServiceState::Unstarted => "unstarted".to_string(),
                    ServiceState::Failed => "failed".to_string(),
                    ServiceState::Archived => "archived".to_string(),
                },
                status: match s.status {
                    ServiceStatus::Healthy => "healthy".to_string(),
                    ServiceStatus::Degraded => "degraded".to_string(),
                    ServiceStatus::Unhealthy => "unhealthy".to_string(),
                    ServiceStatus::Unknown => "unknown".to_string(),
                },
                uptime_seconds: s.uptime_seconds,
                pid: s.pid,
            })
            .collect();

        Ok(ServiceListResponse {
            services: summaries,
            total_count: service_count,
        })
    }

    pub async fn get_service_detail(
        &self,
        name: &str,
    ) -> Result<ServiceDetailResponse, Box<dyn std::error::Error>> {
        let response = ServiceDetailResponse {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            state: "running".to_string(),
            status: "healthy".to_string(),
            uptime_seconds: 3600,
            pid: Some(12345),
            cpu_percent: 25.5,
            memory_mb: 512,
            disk_mb: 1024,
            bandwidth_mbps: 100.5,
            last_health_check: chrono::Utc::now(),
            error: None,
        };

        Ok(response)
    }

    pub async fn configure_service(
        &self,
        name: &str,
        config: serde_json::Value,
        _merge: Option<bool>,
    ) -> Result<ConfigureServiceResponse, Box<dyn std::error::Error>> {
        if self.server.is_error_mode() {
            return Err(self.server.get_error_message().unwrap_or_default().into());
        }

        let response = ConfigureServiceResponse {
            name: name.to_string(),
            config,
            applied: true,
            message: "Configuration applied successfully".to_string(),
        };

        Ok(response)
    }

    pub async fn snapshot_service(
        &self,
        name: &str,
        snapshot_name: Option<String>,
    ) -> Result<SnapshotResponse, Box<dyn std::error::Error>> {
        if self.server.is_error_mode() {
            return Err(self.server.get_error_message().unwrap_or_default().into());
        }

        let response = SnapshotResponse {
            name: name.to_string(),
            snapshot_id: uuid::Uuid::new_v4().to_string(),
            snapshot_name: snapshot_name.unwrap_or_else(|| "snapshot".to_string()),
            timestamp: chrono::Utc::now(),
            size_bytes: 1024 * 1024,
            message: "Snapshot created successfully".to_string(),
        };

        Ok(response)
    }

    pub async fn get_service_logs(
        &self,
        name: &str,
        _lines: Option<u32>,
        _filter: Option<String>,
    ) -> Result<LogsResponse, Box<dyn std::error::Error>> {
        let response = LogsResponse {
            name: name.to_string(),
            lines: vec![
                LogLine {
                    timestamp: chrono::Utc::now(),
                    level: "INFO".to_string(),
                    message: "Service started".to_string(),
                },
                LogLine {
                    timestamp: chrono::Utc::now(),
                    level: "INFO".to_string(),
                    message: "Service running".to_string(),
                },
            ],
            total_lines: 2,
            truncated: false,
        };

        Ok(response)
    }

    // Environment APIs
    pub async fn create_environment(
        &self,
        _name: &str,
        _config: serde_json::Value,
    ) -> Result<String, Box<dyn std::error::Error>> {
        if self.server.is_error_mode() {
            return Err(self.server.get_error_message().unwrap_or_default().into());
        }

        let env_id = uuid::Uuid::new_v4().to_string();
        Ok(env_id)
    }

    pub async fn delete_environment(&self, _env_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        if self.server.is_error_mode() {
            return Err(self.server.get_error_message().unwrap_or_default().into());
        }

        Ok(())
    }

    pub async fn list_environments(
        &self,
    ) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
        if self.server.is_error_mode() {
            return Err(self.server.get_error_message().unwrap_or_default().into());
        }

        Ok(vec![json!({
            "id": uuid::Uuid::new_v4().to_string(),
            "name": "test-env",
            "state": "active"
        })])
    }

    pub async fn migrate_environment(
        &self,
        _from_env: &str,
        _to_env: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        if self.server.is_error_mode() {
            return Err(self.server.get_error_message().unwrap_or_default().into());
        }

        Ok("Migration started".to_string())
    }

    // Module APIs
    pub async fn install_module(
        &self,
        _name: &str,
        _version: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        if self.server.is_error_mode() {
            return Err(self.server.get_error_message().unwrap_or_default().into());
        }

        Ok(uuid::Uuid::new_v4().to_string())
    }

    pub async fn remove_module(&self, _module_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        if self.server.is_error_mode() {
            return Err(self.server.get_error_message().unwrap_or_default().into());
        }

        Ok(())
    }

    pub async fn update_module(
        &self,
        _module_id: &str,
        _version: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if self.server.is_error_mode() {
            return Err(self.server.get_error_message().unwrap_or_default().into());
        }

        Ok(())
    }

    // Asset APIs
    pub async fn generate_asset(
        &self,
        _name: &str,
        _config: serde_json::Value,
    ) -> Result<String, Box<dyn std::error::Error>> {
        if self.server.is_error_mode() {
            return Err(self.server.get_error_message().unwrap_or_default().into());
        }

        Ok(uuid::Uuid::new_v4().to_string())
    }

    pub async fn get_asset_progress(&self, _job_id: &str) -> Result<u32, Box<dyn std::error::Error>> {
        Ok(100)
    }

    pub async fn publish_asset(&self, _job_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        if self.server.is_error_mode() {
            return Err(self.server.get_error_message().unwrap_or_default().into());
        }

        Ok(())
    }

    // Validation APIs
    pub async fn run_validation_test(
        &self,
        _test_name: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        if self.server.is_error_mode() {
            return Err(self.server.get_error_message().unwrap_or_default().into());
        }

        Ok(uuid::Uuid::new_v4().to_string())
    }

    pub async fn get_validation_result(
        &self,
        result_id: &str,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        Ok(json!({
            "id": result_id,
            "status": "completed",
            "passed": 100,
            "failed": 0
        }))
    }

    // Workflow APIs
    pub async fn execute_workflow(
        &self,
        _workflow: serde_json::Value,
    ) -> Result<String, Box<dyn std::error::Error>> {
        if self.server.is_error_mode() {
            return Err(self.server.get_error_message().unwrap_or_default().into());
        }

        Ok(uuid::Uuid::new_v4().to_string())
    }

    pub async fn get_workflow_status(
        &self,
        _workflow_id: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        Ok("completed".to_string())
    }
}

impl Clone for TestClient {
    fn clone(&self) -> Self {
        Self {
            server: Arc::clone(&self.server),
            headers: self.headers.clone(),
            base_url: self.base_url.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let server = Arc::new(MockServer::new());
        let client = TestClient::new(server);
        assert_eq!(client.base_url, "http://localhost:8080/api");
    }

    #[tokio::test]
    async fn test_client_with_auth() {
        let server = Arc::new(MockServer::new());
        let client = TestClient::new(server).with_auth("test-token".to_string());
        assert_eq!(
            client.headers.get("Authorization"),
            Some(&"Bearer test-token".to_string())
        );
    }

    #[tokio::test]
    async fn test_list_services() {
        let server = Arc::new(MockServer::new());
        let client = TestClient::new(server);
        let result = client.list_services().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_error_mode() {
        let server = Arc::new(MockServer::new());
        server.set_error_mode(Some("Test error".to_string()));
        let client = TestClient::new(server);
        let result = client.list_services().await;
        assert!(result.is_err());
    }
}
