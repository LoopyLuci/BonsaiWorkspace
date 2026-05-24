use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::BonsaiError;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentCapability {
    TextGeneration,
    CodeEditing,
    FileManipulation,
    WebSearch,
    ToolUse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMetadata {
    pub id:           String,
    pub name:         String,
    pub description:  String,
    pub capabilities: Vec<AgentCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub content:  String,
    pub role:     Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContext {
    pub model_url:    Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAction {
    pub kind:    String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOutput {
    pub content:  String,
    pub actions:  Vec<AgentAction>,
    pub metadata: Option<serde_json::Value>,
}

#[async_trait]
pub trait Agent: Send + Sync {
    fn metadata(&self) -> AgentMetadata;
    async fn handle_message(&self, ctx: AgentContext, msg: AgentMessage) -> Result<AgentOutput, BonsaiError>;
    async fn shutdown(&self);
}
