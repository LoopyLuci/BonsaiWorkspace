pub struct HotReloadManager;

impl HotReloadManager {
    pub async fn enable(&self, _app_id: &str) -> anyhow::Result<()> {
        Ok(())
    }

    pub async fn trigger_reload(&self, _app_id: &str) -> anyhow::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hotreload() {
        let manager = HotReloadManager;
        assert!(manager.enable("test-app").await.is_ok());
        assert!(manager.trigger_reload("test-app").await.is_ok());
    }
}
