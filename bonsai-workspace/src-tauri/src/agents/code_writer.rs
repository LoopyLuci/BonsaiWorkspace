use async_trait::async_trait;

use crate::agent::{Agent, AgentCapability, AgentContext, AgentMessage, AgentMetadata, AgentOutput};
use crate::error::BonsaiError;

pub struct CodeWriter;

#[async_trait]
impl Agent for CodeWriter {
    fn metadata(&self) -> AgentMetadata {
        AgentMetadata {
            id:           "code-writer".into(),
            name:         "Code Writer".into(),
            description:  "Generates and writes code files based on user descriptions.".into(),
            capabilities: vec![
                AgentCapability::TextGeneration,
                AgentCapability::CodeEditing,
                AgentCapability::FileManipulation,
            ],
        }
    }

    async fn handle_message(&self, _ctx: AgentContext, msg: AgentMessage) -> Result<AgentOutput, BonsaiError> {
        Ok(AgentOutput {
            content:  format!("[CodeWriter] Received: {}", msg.content),
            actions:  vec![],
            metadata: None,
        })
    }

    async fn shutdown(&self) {}
}
