#[cfg(test)]
mod ui_tests {
    use crate::*;

    #[tokio::test]
    async fn test_all_uis() {
        assert!(desktop::UI::render().await.is_ok());
        assert!(web::UI::render().await.is_ok());
        assert!(cli::UI::render().await.is_ok());
        assert!(client::UI::render().await.is_ok());
    }
}
