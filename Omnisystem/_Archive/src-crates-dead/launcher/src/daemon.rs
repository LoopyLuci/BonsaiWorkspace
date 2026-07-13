use anyhow::Result;

pub struct LauncherDaemon {
    name: String,
    running: bool,
}

impl LauncherDaemon {
    pub fn new() -> Self {
        Self {
            name: "launcher-daemon".to_string(),
            running: false,
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        self.running = true;
        tracing::info!("Launcher daemon started");
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        self.running = false;
        tracing::info!("Launcher daemon stopped");
        Ok(())
    }

    pub fn is_running(&self) -> bool {
        self.running
    }
}

impl Default for LauncherDaemon {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_daemon_lifecycle() {
        let mut daemon = LauncherDaemon::new();
        assert!(!daemon.is_running());
        
        daemon.start().await.unwrap();
        assert!(daemon.is_running());
        
        daemon.stop().await.unwrap();
        assert!(!daemon.is_running());
    }
}
