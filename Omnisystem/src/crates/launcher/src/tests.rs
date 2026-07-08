#[cfg(test)]
mod integration {
    use crate::*;

    #[tokio::test]
    async fn test_full_daemon_workflow() {
        let mut daemon = daemon::LauncherDaemon::new();
        daemon.start().await.unwrap();
        assert!(daemon.is_running());
        daemon.stop().await.unwrap();
    }
}
