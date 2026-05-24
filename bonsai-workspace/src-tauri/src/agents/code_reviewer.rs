use async_trait::async_trait;

use crate::agent::{Agent, AgentCapability, AgentContext, AgentMessage, AgentMetadata, AgentOutput};
use crate::error::BonsaiError;

pub struct CodeReviewer;

#[async_trait]
impl Agent for CodeReviewer {
    fn metadata(&self) -> AgentMetadata {
        AgentMetadata {
            id:           "code-reviewer".into(),
            name:         "Code Reviewer".into(),
            description:  "Reviews code files and provides suggestions.".into(),
            capabilities: vec![
                AgentCapability::TextGeneration,
                AgentCapability::CodeEditing,
            ],
        }
    }

    async fn handle_message(&self, _ctx: AgentContext, msg: AgentMessage) -> Result<AgentOutput, BonsaiError> {
        Ok(AgentOutput {
            content:  format!("[CodeReviewer] Received: {}", msg.content),
            actions:  vec![],
            metadata: None,
        })
    }

    async fn shutdown(&self) {}
}
