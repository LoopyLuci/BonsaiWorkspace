//! Autonomous Agents Framework

use crate::cdp::Customer;
use std::sync::Arc;

pub trait Agent: Send + Sync {
    fn name(&self) -> &str;
    fn execute(&self, customer: &Customer) -> AgentDecision;
}

pub struct AgentDecision {
    pub action: AgentAction,
    pub confidence: f64,
    pub reasoning: String,
}

pub enum AgentAction {
    Qualify,
    Nurture,
    Reach,
    Skip,
}

pub struct LeadQualificationAgent;

impl Agent for LeadQualificationAgent {
    fn name(&self) -> &str {
        "lead-qualification"
    }

    fn execute(&self, customer: &Customer) -> AgentDecision {
        let score = customer.health_score();
        let confidence = if score > 0.7 { 0.95 } else { 0.6 };

        AgentDecision {
            action: if score > 0.7 { AgentAction::Qualify } else { AgentAction::Nurture },
            confidence,
            reasoning: format!("Health score: {}", score),
        }
    }
}

pub struct ChurnPredictionAgent;

impl Agent for ChurnPredictionAgent {
    fn name(&self) -> &str {
        "churn-prediction"
    }

    fn execute(&self, customer: &Customer) -> AgentDecision {
        let risk = customer.churn_risk();

        AgentDecision {
            action: if risk > 0.7 { AgentAction::Reach } else { AgentAction::Skip },
            confidence: risk,
            reasoning: format!("Churn risk: {}", risk),
        }
    }
}

pub struct NextBestActionAgent;

impl Agent for NextBestActionAgent {
    fn name(&self) -> &str {
        "next-best-action"
    }

    fn execute(&self, customer: &Customer) -> AgentDecision {
        let segments = customer.get_segments();
        
        AgentDecision {
            action: AgentAction::Reach,
            confidence: 0.8,
            reasoning: format!("Segments: {}", segments.join(",")),
        }
    }
}

pub struct AgentOrchestrator {
    agents: Vec<Arc<dyn Agent>>,
}

impl AgentOrchestrator {
    pub fn new() -> Self {
        Self {
            agents: vec![
                Arc::new(LeadQualificationAgent) as Arc<dyn Agent>,
                Arc::new(ChurnPredictionAgent),
                Arc::new(NextBestActionAgent),
            ],
        }
    }

    pub fn execute(&self, customer: &Customer) -> Vec<AgentDecision> {
        self.agents
            .iter()
            .map(|agent| agent.execute(customer))
            .collect()
    }
}

impl Default for AgentOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_execution() {
        let customer = Customer::new(crate::cdp::CustomerId::Email("test@example.com".to_string()));
        let agent = LeadQualificationAgent;
        let decision = agent.execute(&customer);
        assert!(decision.confidence >= 0.0 && decision.confidence <= 1.0);
    }

    #[test]
    fn test_orchestrator() {
        let orchestrator = AgentOrchestrator::new();
        let customer = Customer::new(crate::cdp::CustomerId::Email("test@example.com".to_string()));
        let decisions = orchestrator.execute(&customer);
        assert_eq!(decisions.len(), 3);
    }
}
