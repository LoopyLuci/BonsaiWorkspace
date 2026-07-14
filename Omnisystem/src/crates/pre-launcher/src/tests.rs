#[cfg(test)]
mod integration {
    use crate::*;

    #[tokio::test]
    async fn test_full_bootstrap_sequence() {
        let result = bootstrap::Bootstrap::run().await.expect("bootstrap should succeed");
        assert!(result.initialized);
    }
}
