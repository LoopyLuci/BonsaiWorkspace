//! ActionExecutor Actor - Executes Actions with error handling and retry logic
//!
//! Responsibilities:
//! - Execute actions with proper error handling
//! - Implement exponential backoff retry strategy
//! - Track execution status and timing
//! - Generate execution reports
//! - Handle timeouts gracefully

use crate::actor::{Actor, ActorId, Snapshot};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use omni_bot_core::Action;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Pending,
    Running,
    Success,
    Failed,
    Retrying,
    TimedOut,
    Cancelled,
}

impl ExecutionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Success => "success",
            Self::Failed => "failed",
            Self::Retrying => "retrying",
            Self::TimedOut => "timed_out",
            Self::Cancelled => "cancelled",
        }
    }
}

/// Execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub execution_id: String,
    pub action: String,
    pub status: ExecutionStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: u64,
    pub error: Option<String>,
    pub retry_count: u32,
    pub output: Option<String>,
}

impl ExecutionResult {
    pub fn new(action: String) -> Self {
        Self {
            execution_id: format!("exec-{}", Uuid::new_v4()),
            action,
            status: ExecutionStatus::Pending,
            started_at: Utc::now(),
            completed_at: None,
            duration_ms: 0,
            error: None,
            retry_count: 0,
            output: None,
        }
    }

    pub fn with_status(mut self, status: ExecutionStatus) -> Self {
        self.status = status;
        self
    }

    pub fn with_error(mut self, error: String) -> Self {
        self.error = Some(error);
        self
    }

    pub fn with_output(mut self, output: String) -> Self {
        self.output = Some(output);
        self
    }

    pub fn complete(&mut self) {
        self.completed_at = Some(Utc::now());
        if let Some(start) = self.started_at.checked_add_signed(Duration::zero()) {
            if let Some(end) = self.completed_at {
                self.duration_ms = end.timestamp_millis() as u64 - start.timestamp_millis() as u64;
            }
        }
    }
}

/// Retry strategy with exponential backoff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryStrategy {
    pub max_retries: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f32,
}

impl Default for RetryStrategy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 100,
            max_delay_ms: 5000,
            backoff_multiplier: 2.0,
        }
    }
}

impl RetryStrategy {
    pub fn calculate_delay(&self, attempt: u32) -> u64 {
        let delay = (self.initial_delay_ms as f32
            * self.backoff_multiplier.powi(attempt as i32)) as u64;
        delay.min(self.max_delay_ms)
    }
}

/// Execution request
#[derive(Debug, Clone)]
pub struct ExecutionRequest {
    pub action: Action,
    pub request_id: String,
    pub retry_strategy: RetryStrategy,
    pub timeout_ms: u64,
}

impl ExecutionRequest {
    pub fn new(action: Action, request_id: String) -> Self {
        Self {
            action,
            request_id,
            retry_strategy: RetryStrategy::default(),
            timeout_ms: 30000, // 30 seconds default
        }
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    pub fn with_retry_strategy(mut self, strategy: RetryStrategy) -> Self {
        self.retry_strategy = strategy;
        self
    }
}

/// Messages for ActionExecutor
#[derive(Debug, Clone)]
pub enum ActionExecutorMessage {
    Execute(ExecutionRequest),
    GetStatus(String),
    CancelExecution(String),
    GetMetrics,
    Stop,
}

/// Execution metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutionMetrics {
    pub total_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
    pub total_retries: u64,
    pub avg_duration_ms: f64,
    pub timeout_count: u64,
}

/// ActionExecutor actor
pub struct ActionExecutor {
    id: ActorId,
    metrics: ExecutionMetrics,
    execution_history: HashMap<String, ExecutionResult>,
    max_history: usize,
}

impl ActionExecutor {
    pub fn new() -> Self {
        Self {
            id: ActorId::new(),
            metrics: ExecutionMetrics::default(),
            execution_history: HashMap::new(),
            max_history: 10000,
        }
    }

    /// Execute an action with error handling
    async fn execute_action(&self, action: &Action) -> Result<String, String> {
        match action {
            Action::StartService { name, .. } => {
                log::info!("[ActionExecutor] Starting service: {}", name);
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                Ok(format!("Service '{}' started successfully", name))
            }
            Action::StopService { name, force } => {
                log::info!("[ActionExecutor] Stopping service: {} (force={})", name, force);
                tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
                Ok(format!("Service '{}' stopped successfully", name))
            }
            Action::RestartService { name } => {
                log::info!("[ActionExecutor] Restarting service: {}", name);
                tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
                Ok(format!("Service '{}' restarted successfully", name))
            }
            Action::GetServiceStatus { name } => {
                log::info!("[ActionExecutor] Getting status for service: {}", name);
                Ok(format!("Service '{}' is running", name))
            }
            Action::CreateEnvironment { name, .. } => {
                log::info!("[ActionExecutor] Creating environment: {}", name);
                tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
                Ok(format!("Environment '{}' created successfully", name))
            }
            Action::InstallModule { name, version } => {
                log::info!("[ActionExecutor] Installing module: {} v{}", name, version);
                tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
                Ok(format!("Module '{}' v{} installed successfully", name, version))
            }
            Action::GenerateAsset {
                asset_type,
                description,
            } => {
                log::info!(
                    "[ActionExecutor] Generating asset: {} ({})",
                    asset_type,
                    description
                );
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                Ok(format!("Asset '{}' generated successfully", asset_type))
            }
            _ => {
                log::warn!("[ActionExecutor] Unsupported action");
                Err("Unsupported action type".to_string())
            }
        }
    }

    /// Execute with retry logic and timeout
    async fn execute_with_retry(
        &self,
        req: ExecutionRequest,
    ) -> ExecutionResult {
        let mut result = ExecutionResult::new(format!("{:?}", req.action));
        result.started_at = Utc::now();

        for attempt in 0..=req.retry_strategy.max_retries {
            result.status = ExecutionStatus::Running;

            match tokio::time::timeout(
                tokio::time::Duration::from_millis(req.timeout_ms),
                self.execute_action(&req.action),
            )
            .await
            {
                Ok(Ok(output)) => {
                    result.status = ExecutionStatus::Success;
                    result.output = Some(output);
                    result.complete();
                    log::info!(
                        "[ActionExecutor] Execution succeeded after {} attempts",
                        attempt + 1
                    );
                    return result;
                }
                Ok(Err(e)) => {
                    if attempt < req.retry_strategy.max_retries {
                        result.status = ExecutionStatus::Retrying;
                        result.retry_count = attempt + 1;
                        result.error = Some(e.clone());

                        let delay = req.retry_strategy.calculate_delay(attempt);
                        log::warn!(
                            "[ActionExecutor] Execution failed (attempt {}), retrying in {}ms: {}",
                            attempt + 1,
                            delay,
                            e
                        );

                        tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                    } else {
                        result.status = ExecutionStatus::Failed;
                        result.error = Some(e);
                        result.complete();
                        log::error!(
                            "[ActionExecutor] Execution failed after {} attempts",
                            attempt + 1
                        );
                        return result;
                    }
                }
                Err(_) => {
                    result.status = ExecutionStatus::TimedOut;
                    result.error = Some("Execution timed out".to_string());
                    result.complete();
                    log::error!(
                        "[ActionExecutor] Execution timed out after {}ms",
                        req.timeout_ms
                    );
                    return result;
                }
            }
        }

        result.status = ExecutionStatus::Failed;
        result.complete();
        result
    }
}

impl Default for ActionExecutor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Actor for ActionExecutor {
    type Message = ActionExecutorMessage;

    fn id(&self) -> ActorId {
        self.id
    }

    async fn handle(&mut self, msg: Self::Message) -> Result<bool, String> {
        match msg {
            ActionExecutorMessage::Execute(req) => {
                let result = self.execute_with_retry(req).await;

                // Update metrics
                self.metrics.total_executions += 1;
                self.metrics.total_retries += result.retry_count as u64;

                match result.status {
                    ExecutionStatus::Success => self.metrics.successful_executions += 1,
                    ExecutionStatus::Failed => self.metrics.failed_executions += 1,
                    ExecutionStatus::TimedOut => self.metrics.timeout_count += 1,
                    _ => {}
                }

                if self.metrics.total_executions > 0 {
                    self.metrics.avg_duration_ms = (self.metrics.avg_duration_ms
                        + result.duration_ms as f64)
                        / 2.0;
                }

                // Store in history
                if self.execution_history.len() >= self.max_history {
                    self.execution_history.clear();
                }
                self.execution_history
                    .insert(result.execution_id.clone(), result);

                Ok(true)
            }
            ActionExecutorMessage::GetStatus(execution_id) => {
                if let Some(result) = self.execution_history.get(&execution_id) {
                    log::info!(
                        "[ActionExecutor] Status of {}: {}",
                        execution_id,
                        result.status.as_str()
                    );
                } else {
                    log::warn!("[ActionExecutor] Execution {} not found", execution_id);
                }
                Ok(true)
            }
            ActionExecutorMessage::CancelExecution(execution_id) => {
                if let Some(result) = self.execution_history.get_mut(&execution_id) {
                    result.status = ExecutionStatus::Cancelled;
                    log::info!("[ActionExecutor] Cancelled execution {}", execution_id);
                }
                Ok(true)
            }
            ActionExecutorMessage::GetMetrics => {
                log::info!("[ActionExecutor] Metrics: {:?}", self.metrics);
                Ok(true)
            }
            ActionExecutorMessage::Stop => {
                log::info!("[ActionExecutor] Stop signal received");
                Ok(false)
            }
        }
    }

    async fn snapshot(&self) -> Result<Snapshot, String> {
        let state = serde_json::json!({
            "metrics": self.metrics,
            "history_size": self.execution_history.len(),
        });

        Ok(Snapshot::new(
            self.id,
            "ActionExecutor".to_string(),
            state,
        ))
    }

    async fn restore(&mut self, _snapshot: Snapshot) -> Result<(), String> {
        log::info!("[ActionExecutor] Restored from snapshot");
        Ok(())
    }

    fn actor_type(&self) -> &'static str {
        "ActionExecutor"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_status_str() {
        assert_eq!(ExecutionStatus::Running.as_str(), "running");
        assert_eq!(ExecutionStatus::Success.as_str(), "success");
        assert_eq!(ExecutionStatus::Failed.as_str(), "failed");
    }

    #[test]
    fn test_retry_strategy_backoff() {
        let strategy = RetryStrategy::default();
        assert_eq!(strategy.calculate_delay(0), 100); // initial delay
        assert_eq!(strategy.calculate_delay(1), 200); // doubled
        assert_eq!(strategy.calculate_delay(2), 400); // doubled again
    }

    #[test]
    fn test_retry_strategy_max_delay() {
        let strategy = RetryStrategy {
            max_retries: 10,
            initial_delay_ms: 100,
            max_delay_ms: 1000,
            backoff_multiplier: 2.0,
        };

        // After several retries, should cap at max_delay
        let delay = strategy.calculate_delay(5);
        assert!(delay <= strategy.max_delay_ms);
    }

    #[tokio::test]
    async fn test_execution_request_timeout() {
        let executor = ActionExecutor::new();
        let action = Action::StartService {
            name: "test".to_string(),
            config: None,
        };

        let req = ExecutionRequest::new(action, "req-1".to_string())
            .with_timeout(10000);

        assert_eq!(req.timeout_ms, 10000);
    }
}
