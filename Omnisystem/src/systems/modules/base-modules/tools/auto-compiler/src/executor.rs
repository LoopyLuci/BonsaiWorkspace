//! Command executor for builds

use crate::{Result, AutoCompileError};
use std::process::Command;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct CommandExecutor {
    working_dir: PathBuf,
}

impl CommandExecutor {
    /// Create new command executor
    pub fn new(working_dir: PathBuf) -> Self {
        Self { working_dir }
    }

    /// Execute command synchronously
    pub fn execute(&self, command: &str, args: &[&str]) -> Result<CommandOutput> {
        log::info!("Executing: {} {:?}", command, args);

        let output = Command::new(command)
            .args(args)
            .current_dir(&self.working_dir)
            .output()
            .map_err(|e| AutoCompileError::CommandFailed(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(CommandOutput {
            exit_code: output.status.code().unwrap_or(-1),
            stdout,
            stderr,
            success: output.status.success(),
        })
    }

    /// Execute command asynchronously
    pub async fn execute_async(&self, command: &str, args: &[&str]) -> Result<CommandOutput> {
        self.execute(command, args)
    }

    /// Timeout-aware execution
    pub fn execute_with_timeout(
        &self,
        command: &str,
        args: &[&str],
        timeout_secs: u64,
    ) -> Result<CommandOutput> {
        log::info!(
            "Executing with timeout {}s: {} {:?}",
            timeout_secs,
            command,
            args
        );

        // Stub: real implementation would use process timeout
        self.execute(command, args)
    }
}

#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub success: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_executor_creation() {
        let executor = CommandExecutor::new(PathBuf::from("."));
        assert_eq!(executor.working_dir, PathBuf::from("."));
    }

    #[test]
    fn test_execute_simple_command() {
        let executor = CommandExecutor::new(PathBuf::from("."));

        // Use a command that works on all platforms
        let result = if cfg!(target_os = "windows") {
            executor.execute("cmd", &["/C", "echo", "test"])
        } else {
            executor.execute("echo", &["test"])
        };

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.success);
    }
}
