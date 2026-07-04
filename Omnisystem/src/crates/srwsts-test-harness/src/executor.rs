//! Test executor: runs tests inside vaults

use crate::errors::{HarnessError, HarnessResult};
use crate::result::ResultCapture;
use crate::vault::Vault;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Test execution mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionMode {
    /// Normal execution
    Normal,
    /// Deterministic replay from trace
    Replay,
    /// Debug mode with enhanced logging
    Debug,
    /// Stress test mode
    Stress,
}

impl std::fmt::Display for ExecutionMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Normal => write!(f, "Normal"),
            Self::Replay => write!(f, "Replay"),
            Self::Debug => write!(f, "Debug"),
            Self::Stress => write!(f, "Stress"),
        }
    }
}

/// Test execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    /// Path to test binary
    pub binary_path: PathBuf,
    /// Arguments to pass to the binary
    pub args: Vec<String>,
    /// Environment variables
    pub env: std::collections::HashMap<String, String>,
    /// Execution timeout
    pub timeout: Duration,
    /// Execution mode
    pub mode: ExecutionMode,
    /// Enable stdout capture
    pub capture_stdout: bool,
    /// Enable stderr capture
    pub capture_stderr: bool,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            binary_path: PathBuf::new(),
            args: Vec::new(),
            env: std::collections::HashMap::new(),
            timeout: Duration::from_secs(60),
            mode: ExecutionMode::Normal,
            capture_stdout: true,
            capture_stderr: true,
        }
    }
}

impl ExecutionConfig {
    /// Create a new execution configuration
    pub fn new(binary_path: impl Into<PathBuf>) -> Self {
        Self {
            binary_path: binary_path.into(),
            ..Default::default()
        }
    }

    /// Add an argument
    pub fn with_arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    /// Add environment variable
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set execution mode
    pub fn with_mode(mut self, mode: ExecutionMode) -> Self {
        self.mode = mode;
        self
    }
}

/// Test executor: runs tests inside vaults
pub struct TestExecutor {
    /// Current execution configuration
    config: ExecutionConfig,
}

impl TestExecutor {
    /// Create a new test executor
    pub fn new(config: ExecutionConfig) -> Self {
        Self { config }
    }

    /// Execute a test inside a vault
    pub async fn execute(&self, vault: &Vault) -> HarnessResult<ResultCapture> {
        if !vault.is_available() {
            return Err(HarnessError::ExecutionFailed(
                "vault not available".to_string(),
            ));
        }

        tracing::info!(
            "Executing test: {} in vault: {}",
            self.config.binary_path.display(),
            vault.id
        );

        // In a real implementation, this would:
        // 1. Enter the vault using Sanctum
        // 2. Spawn the test process
        // 3. Monitor execution with timeouts
        // 4. Capture output and metrics
        // 5. Exit the vault on completion

        let result = ResultCapture::new(
            vault.id,
            self.config.binary_path.clone(),
            self.config.timeout,
        );

        tracing::debug!("Test execution completed for vault: {}", vault.id);

        Ok(result)
    }

    /// Execute with deterministic replay from trace
    pub async fn execute_replay(
        &self,
        vault: &Vault,
        trace_data: &[u8],
    ) -> HarnessResult<ResultCapture> {
        if !vault.is_available() {
            return Err(HarnessError::ExecutionFailed(
                "vault not available".to_string(),
            ));
        }

        tracing::info!(
            "Executing test in replay mode for vault: {}",
            vault.id
        );

        let mut result = ResultCapture::new(
            vault.id,
            self.config.binary_path.clone(),
            self.config.timeout,
        );

        result.trace_data = Some(trace_data.to_vec());

        Ok(result)
    }

    /// Get current configuration
    pub fn config(&self) -> &ExecutionConfig {
        &self.config
    }

    /// Update configuration
    pub fn set_config(&mut self, config: ExecutionConfig) {
        self.config = config;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_mode_display() {
        assert_eq!(ExecutionMode::Normal.to_string(), "Normal");
        assert_eq!(ExecutionMode::Replay.to_string(), "Replay");
        assert_eq!(ExecutionMode::Debug.to_string(), "Debug");
    }

    #[test]
    fn test_execution_config_creation() {
        let config = ExecutionConfig::new("/path/to/test");
        assert_eq!(config.mode, ExecutionMode::Normal);
        assert!(config.capture_stdout);
    }

    #[test]
    fn test_execution_config_builder() {
        let config = ExecutionConfig::new("/path/to/test")
            .with_arg("--verbose")
            .with_env("TEST", "true")
            .with_mode(ExecutionMode::Debug);

        assert_eq!(config.args.len(), 1);
        assert_eq!(config.mode, ExecutionMode::Debug);
    }

    #[test]
    fn test_test_executor_creation() {
        let config = ExecutionConfig::new("/path/to/test");
        let executor = TestExecutor::new(config);
        assert_eq!(executor.config().mode, ExecutionMode::Normal);
    }
}
