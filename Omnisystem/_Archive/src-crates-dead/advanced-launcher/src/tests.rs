#[cfg(test)]
mod advanced_tests {
    use crate::*;

    #[test]
    fn test_plugin_system() {
        let mut manager = plugins::PluginManager::new();
        assert_eq!(manager.get_plugins(), 0);
    }

    #[tokio::test]
    async fn test_hotreload() {
        let manager = hotreload::HotReloadManager;
        assert!(manager.enable("app").await.is_ok());
    }
}
