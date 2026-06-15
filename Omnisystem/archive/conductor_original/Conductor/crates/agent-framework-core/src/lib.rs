//! Agent Framework Core
//!
//! Provides async trait-based agent system for coordinated container operations.

#![warn(missing_docs)]

pub mod error;
pub mod types;

pub use error::{Error, Result};
pub use types::*;

use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;
use tracing::{info, debug, warn};

/// Agent trait for implementing specialized behaviors
#[async_trait]
pub trait Agent: Send + Sync {
    /// Get agent name
    fn name(&self) -> &str;

    /// Initialize agent
    async fn init(&self) -> Result<()>;

    /// Execute agent task
    async fn execute(&self, input: AgentInput) -> Result<AgentOutput>;

    /// Check agent health
    async fn health_check(&self) -> Result<bool> {
        Ok(true)
    }
}

/// Agent coordination framework
pub struct AgentFramework {
    agents: Arc<DashMap<String, Arc<dyn Agent>>>,
    state: Arc<DashMap<String, String>>,
}

impl AgentFramework {
    /// Create new agent framework
    pub fn new() -> Self {
        info!("Initializing Agent Framework");
        Self {
            agents: Arc::new(DashMap::new()),
            state: Arc::new(DashMap::new()),
        }
    }

    /// Register an agent
    pub async fn register(&self, agent: Arc<dyn Agent>) -> Result<()> {
        let name = agent.name().to_string();
        debug!("Registering agent: {}", name);
        agent.init().await?;
        self.agents.insert(name.clone(), agent);
        self.state.insert(format!("{}_status", name), "ready".to_string());
        Ok(())
    }

    /// Execute agent
    pub async fn execute_agent(
        &self,
        agent_name: &str,
        input: AgentInput,
    ) -> Result<AgentOutput> {
        debug!("Executing agent: {}", agent_name);

        let agent = self
            .agents
            .get(agent_name)
            .ok_or_else(|| Error::Other(format!("Agent not found: {}", agent_name)))?;

        // Update status
        self.state.insert(
            format!("{}_status", agent_name),
            "executing".to_string(),
        );

        // Execute agent
        let result = agent.execute(input).await;

        // Update status
        match &result {
            Ok(_) => {
                self.state.insert(
                    format!("{}_status", agent_name),
                    "success".to_string(),
                );
            }
            Err(e) => {
                warn!("Agent {} failed: {}", agent_name, e);
                self.state.insert(
                    format!("{}_status", agent_name),
                    format!("error: {}", e),
                );
            }
        }

        result
    }

    /// Execute multiple agents in parallel
    pub async fn execute_parallel(
        &self,
        executions: Vec<(String, AgentInput)>,
    ) -> Result<Vec<AgentOutput>> {
        let handles: Vec<_> = executions
            .into_iter()
            .map(|(agent_name, input)| {
                let agent = self.agents.clone();
                let state = self.state.clone();
                tokio::spawn(async move {
                    if let Some(agent_ref) = agent.get(&agent_name) {
                        state.insert(
                            format!("{}_status", &agent_name),
                            "executing".to_string(),
                        );

                        match agent_ref.execute(input).await {
                            Ok(output) => {
                                state.insert(
                                    format!("{}_status", &agent_name),
                                    "success".to_string(),
                                );
                                Ok(output)
                            }
                            Err(e) => {
                                state.insert(
                                    format!("{}_status", &agent_name),
                                    format!("error: {}", e),
                                );
                                Err(e)
                            }
                        }
                    } else {
                        Err(Error::Other(format!("Agent not found: {}", agent_name)))
                    }
                })
            })
            .collect();

        let mut results = Vec::new();
        for handle in handles {
            results.push(handle.await.map_err(|e| Error::Other(e.to_string()))??);
        }

        Ok(results)
    }

    /// Get agent status
    pub fn get_agent_status(&self, agent_name: &str) -> Option<String> {
        self.state
            .get(&format!("{}_status", agent_name))
            .map(|entry| entry.value().clone())
    }

    /// Get all agents
    pub fn list_agents(&self) -> Vec<String> {
        self.agents.iter().map(|entry| entry.key().clone()).collect()
    }

    /// Health check all agents
    pub async fn health_check_all(&self) -> Result<Vec<(String, bool)>> {
        let mut results = Vec::new();
        for agent_ref in self.agents.iter() {
            let name = agent_ref.key().clone();
            let agent = agent_ref.value().clone();
            match agent.health_check().await {
                Ok(healthy) => results.push((name, healthy)),
                Err(_) => results.push((name, false)),
            }
        }
        Ok(results)
    }

    /// Wait for agent completion
    pub async fn wait_for_agent(&self, agent_name: &str, timeout_secs: u64) -> Result<bool> {
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_secs(timeout_secs);

        loop {
            if let Some(status) = self.get_agent_status(agent_name) {
                if status == "success" || status.starts_with("error") {
                    return Ok(status == "success");
                }
            }

            if start.elapsed() > timeout {
                return Err(Error::Other("Agent execution timeout".to_string()));
            }

            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }
}

impl Default for AgentFramework {
    fn default() -> Self {
        Self::new()
    }
}

/// Initialize framework
pub async fn init() -> Result<()> {
    info!("Agent Framework initialized");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestAgent;

    #[async_trait]
    impl Agent for TestAgent {
        fn name(&self) -> &str {
            "test-agent"
        }

        async fn init(&self) -> Result<()> {
            Ok(())
        }

        async fn execute(&self, _input: AgentInput) -> Result<AgentOutput> {
            Ok(AgentOutput {
                agent_name: "test-agent".to_string(),
                status: "success".to_string(),
                result: "test output".to_string(),
            })
        }
    }

    #[tokio::test]
    async fn test_framework_creation() {
        let framework = AgentFramework::new();
        assert_eq!(framework.list_agents().len(), 0);
    }

    #[tokio::test]
    async fn test_agent_registration() {
        let framework = AgentFramework::new();
        let agent = Arc::new(TestAgent);
        assert!(framework.register(agent).await.is_ok());
        assert_eq!(framework.list_agents().len(), 1);
    }

    #[tokio::test]
    async fn test_agent_execution() {
        let framework = AgentFramework::new();
        let agent = Arc::new(TestAgent);
        framework.register(agent).await.unwrap();

        let input = AgentInput {
            command: "test".to_string(),
            parameters: std::collections::HashMap::new(),
        };

        let output = framework.execute_agent("test-agent", input).await;
        assert!(output.is_ok());
    }

    #[tokio::test]
    async fn test_agent_status() {
        let framework = AgentFramework::new();
        let agent = Arc::new(TestAgent);
        framework.register(agent).await.unwrap();

        let status = framework.get_agent_status("test-agent");
        assert!(status.is_some());
    }

    #[tokio::test]
    async fn test_health_check() {
        let framework = AgentFramework::new();
        let agent = Arc::new(TestAgent);
        framework.register(agent).await.unwrap();

        let health = framework.health_check_all().await;
        assert!(health.is_ok());
    }

    #[tokio::test]
    async fn test_framework_initialization() {
        assert!(init().await.is_ok());
    }

    #[tokio::test]
    async fn test_default_framework() {
        let framework = AgentFramework::default();
        assert_eq!(framework.list_agents().len(), 0);
    }

    #[tokio::test]
    async fn test_parallel_execution() {
        let framework = AgentFramework::new();
        let agent = Arc::new(TestAgent);
        framework.register(agent).await.unwrap();

        let input = AgentInput {
            command: "test".to_string(),
            parameters: std::collections::HashMap::new(),
        };

        let executions = vec![("test-agent".to_string(), input)];
        let results = framework.execute_parallel(executions).await;
        assert!(results.is_ok());
    }
}
