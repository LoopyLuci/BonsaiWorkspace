use crate::{Result, BuddyError};
use async_trait::async_trait;

#[async_trait]
pub trait InteractionHandler: Send + Sync {
    async fn handle_input(&self, input: &str) -> Result<String>;
    async fn format_output(&self, response: &str) -> Result<String>;
}

pub struct DefaultInteractionHandler;

#[async_trait]
impl InteractionHandler for DefaultInteractionHandler {
    async fn handle_input(&self, input: &str) -> Result<String> {
        tracing::info!("Processing user input: {}", input);
        Ok(input.to_string())
    }

    async fn format_output(&self, response: &str) -> Result<String> {
        Ok(format!("Buddy: {}", response))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_interaction_handler() {
        let handler = DefaultInteractionHandler;
        let result = handler.handle_input("test input").await.unwrap();
        assert_eq!(result, "test input");
        
        let formatted = handler.format_output(&result).await.unwrap();
        assert!(formatted.contains("Buddy:"));
    }
}
