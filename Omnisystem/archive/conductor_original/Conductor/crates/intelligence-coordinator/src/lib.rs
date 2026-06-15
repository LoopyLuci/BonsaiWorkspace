//! IntelligenceCoordinator Agent
#![warn(missing_docs)]
pub mod error;
pub mod types;
use async_trait::async_trait;
use agent_framework_core::{Agent, AgentInput, AgentOutput, Error, Result};
use tracing::info;
use std::collections::HashMap;

pub struct IntelligenceCoordinator;
#[async_trait]
impl Agent for IntelligenceCoordinator {
    fn name(&self) -> &str { "intelligence-coordinator" }
    async fn init(&self) -> Result<()> { info!("Init"); Ok(()) }
    async fn execute(&self, input: AgentInput) -> Result<AgentOutput> {
        Ok(AgentOutput {
            agent_name: "intelligence-coordinator".to_string(),
            status: "success".to_string(),
            result: format!("Executed: {}", input.command),
        })
    }
    async fn health_check(&self) -> Result<bool> { Ok(true) }
}
pub async fn init() -> Result<()> { info!("Module init"); Ok(()) }

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_execute() {
        let r = IntelligenceCoordinator.execute(AgentInput {
            command: "test".to_string(),
            parameters: HashMap::new(),
        }).await;
        assert!(r.is_ok());
    }
    #[tokio::test]
    async fn test_health() { assert!(IntelligenceCoordinator.health_check().await.is_ok()); }
    #[tokio::test]
    async fn test_init() { assert!(init().await.is_ok()); }
}
