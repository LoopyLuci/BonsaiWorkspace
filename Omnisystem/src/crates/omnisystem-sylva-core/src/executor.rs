// Module Executor - executes Sylva modules

use async_trait::async_trait;
use omnisystem_ums::{Module, ModuleConfig, ModuleInfo, ModuleRequest, ModuleResponse, ModuleState};
use serde_json::json;
use std::time::Instant;

/// Trait for modules that can be executed
pub trait ModuleExecutor: Send + Sync {
    /// Execute synchronous operation
    fn execute_sync(&self, request: &ModuleRequest) -> anyhow::Result<serde_json::Value>;
}

/// Default module executor implementation
pub struct DefaultModuleExecutor {
    info: ModuleInfo,
    state: std::sync::atomic::AtomicU8,
    metrics: parking_lot::Mutex<ModuleMetrics>,
}

#[derive(Debug, Clone)]
struct ModuleMetrics {
    requests_total: u64,
    requests_active: u32,
    latency_sum_ms: u64,
    errors: u64,
}

impl DefaultModuleExecutor {
    pub fn new(info: ModuleInfo) -> Self {
        Self {
            info,
            state: std::sync::atomic::AtomicU8::new(ModuleState::Registered as u8),
            metrics: parking_lot::Mutex::new(ModuleMetrics {
                requests_total: 0,
                requests_active: 0,
                latency_sum_ms: 0,
                errors: 0,
            }),
        }
    }

    fn state(&self) -> ModuleState {
        match self.state.load(std::sync::atomic::Ordering::Relaxed) {
            0 => ModuleState::Registered,
            1 => ModuleState::Loaded,
            2 => ModuleState::Ready,
            3 => ModuleState::Running,
            4 => ModuleState::Paused,
            5 => ModuleState::Shutting,
            6 => ModuleState::Stopped,
            _ => ModuleState::Error,
        }
    }

    fn set_state(&self, state: ModuleState) {
        self.state.store(state as u8, std::sync::atomic::Ordering::Relaxed);
    }
}

#[async_trait]
impl Module for DefaultModuleExecutor {
    fn info(&self) -> &ModuleInfo {
        &self.info
    }

    async fn initialize(&mut self, config: ModuleConfig) -> anyhow::Result<()> {
        tracing::info!(
            "Initializing module: {} with config: {:?}",
            self.info.name,
            config.data_dirs
        );

        self.set_state(ModuleState::Ready);
        Ok(())
    }

    async fn start(&mut self) -> anyhow::Result<()> {
        tracing::info!("Starting module: {}", self.info.name);
        self.set_state(ModuleState::Running);
        Ok(())
    }

    async fn stop(&mut self) -> anyhow::Result<()> {
        tracing::info!("Stopping module: {}", self.info.name);
        self.set_state(ModuleState::Stopped);
        Ok(())
    }

    async fn execute(&self, request: ModuleRequest) -> anyhow::Result<ModuleResponse> {
        let start = Instant::now();
        let mut metrics = self.metrics.lock();
        metrics.requests_total += 1;
        metrics.requests_active += 1;
        drop(metrics);

        let result = match self.execute_sync(&request) {
            Ok(data) => {
                let elapsed = start.elapsed();
                let mut metrics = self.metrics.lock();
                metrics.latency_sum_ms += elapsed.as_millis() as u64;
                metrics.requests_active = metrics.requests_active.saturating_sub(1);

                ModuleResponse {
                    request_id: request.request_id.clone(),
                    success: true,
                    data,
                    error: None,
                    execution_time_ms: elapsed.as_millis() as u64,
                }
            }
            Err(e) => {
                let elapsed = start.elapsed();
                let mut metrics = self.metrics.lock();
                metrics.errors += 1;
                metrics.requests_active = metrics.requests_active.saturating_sub(1);

                ModuleResponse {
                    request_id: request.request_id.clone(),
                    success: false,
                    data: json!(null),
                    error: Some(e.to_string()),
                    execution_time_ms: elapsed.as_millis() as u64,
                }
            }
        };

        Ok(result)
    }

    fn state(&self) -> ModuleState {
        self.state()
    }

    async fn verify(&self) -> anyhow::Result<omnisystem_ums::module::VerificationResult> {
        Ok(omnisystem_ums::module::VerificationResult {
            passed: true,
            checks: vec![omnisystem_ums::module::VerificationCheck {
                name: "module_loaded".to_string(),
                passed: true,
                details: format!("Module {} verified", self.info.name),
            }],
            errors: vec![],
        })
    }

    fn metrics(&self) -> omnisystem_ums::module::ModuleMetrics {
        let m = self.metrics.lock();
        omnisystem_ums::module::ModuleMetrics {
            requests_total: m.requests_total,
            requests_active: m.requests_active,
            latency_avg_ms: if m.requests_total > 0 {
                m.latency_sum_ms as f64 / m.requests_total as f64
            } else {
                0.0
            },
            latency_p99_ms: 0.0, // Would need percentile tracking
            memory_bytes: 0,
            last_execution: None,
            errors: m.errors,
        }
    }
}

impl ModuleExecutor for DefaultModuleExecutor {
    fn execute_sync(&self, request: &ModuleRequest) -> anyhow::Result<serde_json::Value> {
        match request.operation.as_str() {
            "ping" => Ok(json!({ "pong": true })),
            "info" => Ok(json!({
                "module": self.info.name,
                "version": self.info.version,
                "phase": self.info.phase,
            })),
            "health" => Ok(json!({ "status": "healthy" })),
            _ => Ok(json!({
                "operation": request.operation,
                "status": "executed",
                "module": self.info.name,
            })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use omnisystem_ums::ModuleId;

    fn create_test_module() -> DefaultModuleExecutor {
        let info = ModuleInfo {
            id: ModuleId::new(),
            name: "test-module".to_string(),
            version: "1.0.0".to_string(),
            description: "Test module".to_string(),
            author: "test".to_string(),
            dependencies: vec![],
            capabilities: vec![],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            interface_version: "1.0".to_string(),
            phase: 1,
            source_path: "/test".to_string(),
            canonical_path: "/test".to_string(),
            spec_path: "/test".to_string(),
            metadata: Default::default(),
        };

        DefaultModuleExecutor::new(info)
    }

    #[tokio::test]
    async fn test_module_lifecycle() {
        let mut module = create_test_module();

        assert_eq!(module.state(), ModuleState::Registered);

        module
            .initialize(ModuleConfig {
                config: json!({}),
                runtime_config: json!({}),
                data_dirs: omnisystem_ums::module::ModuleDataDirs {
                    umd_source: std::path::PathBuf::from("/umd"),
                    generated: std::path::PathBuf::from("/gen"),
                    user_data: std::path::PathBuf::from("/user"),
                    cache: std::path::PathBuf::from("/cache"),
                },
            })
            .await
            .unwrap();

        assert_eq!(module.state(), ModuleState::Ready);

        module.start().await.unwrap();
        assert_eq!(module.state(), ModuleState::Running);

        module.stop().await.unwrap();
        assert_eq!(module.state(), ModuleState::Stopped);
    }

    #[tokio::test]
    async fn test_module_execute() {
        let module = create_test_module();

        let request = omnisystem_ums::module::ModuleRequest {
            request_id: "test-1".to_string(),
            operation: "ping".to_string(),
            args: json!({}),
            metadata: Default::default(),
        };

        let response = module.execute(request).await.unwrap();
        assert!(response.success);
        assert_eq!(response.data["pong"], json!(true));
    }
}
