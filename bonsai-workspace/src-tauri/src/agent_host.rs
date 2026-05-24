use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::agent::{Agent, AgentContext, AgentMessage, AgentMetadata, AgentOutput};
use crate::error::BonsaiError;

pub struct AgentHost {
    agents: RwLock<HashMap<String, Arc<dyn Agent>>>,
}

impl AgentHost {
    pub fn new() -> Self {
        Self { agents: RwLock::new(HashMap::new()) }
    }

    pub async fn register(&self, agent: Arc<dyn Agent>) {
        self.agents.write().await.insert(agent.metadata().id.clone(), agent);
    }

    pub async fn unregister(&self, id: &str) {
        self.agents.write().await.remove(id);
    }

    pub async fn list(&self) -> Vec<AgentMetadata> {
        self.agents.read().await.values().map(|a| a.metadata()).collect()
    }

    pub async fn handle(
        &self,
        agent_id: &str,
        ctx: AgentContext,
        msg: AgentMessage,
    ) -> Result<AgentOutput, BonsaiError> {
        let agents = self.agents.read().await;
        let agent  = agents.get(agent_id).ok_or_else(|| {
            BonsaiError::Internal(format!("Agent '{agent_id}' not found"))
        })?;
        agent.handle_message(ctx, msg).await
    }
}
