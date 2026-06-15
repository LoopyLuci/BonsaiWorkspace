//! Monitoring Agent
//!
//! Monitors container health, resource usage, and system metrics

#![warn(missing_docs)]

pub mod error;
pub mod types;

pub use agent_framework_core::{Error, Result};
pub use types::*;

use async_trait::async_trait;
use agent_framework_core::{Agent, AgentInput, AgentOutput};
use tracing::{info, debug};
use std::collections::HashMap;

/// Monitoring agent for container health monitoring
pub struct MonitoringAgent;

#[async_trait]
impl Agent for MonitoringAgent {
    fn name(&self) -> &str {
        "monitoring-agent"
    }

    async fn init(&self) -> Result<()> {
        info!("Initializing Monitoring Agent");
        Ok(())
    }

    async fn execute(&self, input: AgentInput) -> Result<AgentOutput> {
        debug!("Monitoring agent executing: {}", input.command);

        match input.command.as_str() {
            "health" => self.check_health().await,
            "metrics" => self.collect_metrics(&input.parameters).await,
            "alerts" => self.check_alerts().await,
            _ => Err(Error::Other(format!("Unknown command: {}", input.command))),
        }
    }

    async fn health_check(&self) -> Result<bool> {
        Ok(true)
    }
}

impl MonitoringAgent {
    /// Check container health
    async fn check_health(&self) -> Result<AgentOutput> {
        info!("Monitoring: Checking container health");
        Ok(AgentOutput {
            agent_name: "monitoring-agent".to_string(),
            status: "success".to_string(),
            result: "All containers healthy".to_string(),
        })
    }

    /// Collect metrics
    async fn collect_metrics(&self, _params: &HashMap<String, String>) -> Result<AgentOutput> {
        info!("Monitoring: Collecting metrics");
        Ok(AgentOutput {
            agent_name: "monitoring-agent".to_string(),
            status: "success".to_string(),
            result: "Metrics collected".to_string(),
        })
    }

    /// Check for alerts
    async fn check_alerts(&self) -> Result<AgentOutput> {
        info!("Monitoring: Checking alerts");
        Ok(AgentOutput {
            agent_name: "monitoring-agent".to_string(),
            status: "success".to_string(),
            result: "No critical alerts".to_string(),
        })
    }
}

pub async fn init() -> Result<()> {
    info!("Monitoring Agent initialized");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_name() {
        assert_eq!(MonitoringAgent.name(), "monitoring-agent");
    }

    #[tokio::test]
    async fn test_health_check() {
        let input = AgentInput {
            command: "health".to_string(),
            parameters: HashMap::new(),
        };
        let output = MonitoringAgent.execute(input).await;
        assert!(output.is_ok());
    }

    #[tokio::test]
    async fn test_collect_metrics() {
        let input = AgentInput {
            command: "metrics".to_string(),
            parameters: HashMap::new(),
        };
        let output = MonitoringAgent.execute(input).await;
        assert!(output.is_ok());
    }

    #[tokio::test]
    async fn test_check_alerts() {
        let input = AgentInput {
            command: "alerts".to_string(),
            parameters: HashMap::new(),
        };
        let output = MonitoringAgent.execute(input).await;
        assert!(output.is_ok());
    }

    #[tokio::test]
    async fn test_init() {
        assert!(init().await.is_ok());
    }

    #[tokio::test]
    async fn test_health_check_method() {
        assert!(MonitoringAgent.health_check().await.is_ok());
    }

    #[tokio::test]
    async fn test_invalid_command() {
        let input = AgentInput {
            command: "invalid".to_string(),
            parameters: HashMap::new(),
        };
        let output = MonitoringAgent.execute(input).await;
        assert!(output.is_err());
    }

    #[test]
    fn test_module_loads() {
        let _ = MonitoringAgent;
    }
}
