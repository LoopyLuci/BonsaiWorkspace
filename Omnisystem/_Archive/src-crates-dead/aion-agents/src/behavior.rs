use crate::{Action, Perception, Result};
use async_trait::async_trait;

#[async_trait]
pub trait Behavior: Send + Sync {
    async fn execute(&self, perception: &Perception) -> Result<Action>;
    async fn is_applicable(&self, perception: &Perception) -> bool;
}

pub struct ReactiveBehavior;

#[async_trait]
impl Behavior for ReactiveBehavior {
    async fn execute(&self, perception: &Perception) -> Result<Action> {
        tracing::info!("Executing reactive behavior");
        Ok(Action {
            id: uuid::Uuid::new_v4().to_string(),
            action_type: "reactive".to_string(),
            parameters: std::collections::HashMap::new(),
            priority: 8,
        })
    }

    async fn is_applicable(&self, _perception: &Perception) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_reactive_behavior() {
        let behavior = ReactiveBehavior;
        let perception = Perception {
            sensor_data: vec![0.1],
            timestamp: 1000,
            confidence: 0.9,
        };
        assert!(behavior.execute(&perception).await.is_ok());
    }
}
