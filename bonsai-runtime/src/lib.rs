use anyhow::Result;
use tokio::process::Command;

pub struct RuntimeManager {}

impl RuntimeManager {
    pub fn new() -> Self { Self {} }

    /// Start a Python worker by spawning the given script path with the provided port.
    /// Returns the spawned child process handle (not awaited).
    pub async fn start_python_worker(&self, script_path: &str, port: u16) -> Result<tokio::process::Child> {
        let mut cmd = Command::new("python");
        cmd.arg(script_path).arg(port.to_string());
        let child = cmd.spawn()?;
        Ok(child)
    }

    /// Start a Babashka (Clojure) worker by spawning `bb` with the given script.
    pub async fn start_babashka_worker(&self, script_path: &str) -> Result<tokio::process::Child> {
        let mut cmd = Command::new("bb");
        cmd.arg(script_path);
        let child = cmd.spawn()?;
        Ok(child)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn create_manager() {
        let m = RuntimeManager::new();
        let _ = m;
        assert!(true);
    }
}
