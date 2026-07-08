//! Agent Assistants — lightweight persistent advisors attached to every hierarchy node.
//!
//! An Assistant learns from an agent's decisions, handles routine communication,
//! and escalates when the main agent is stuck. Assistants are sandboxed: they
//! cannot call tools directly.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, sync::Arc};
use tokio::sync::RwLock;
use uuid::Uuid;

/// A single suggestion from an assistant to its parent agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantSuggestion {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub content: String,
    /// "optimisation" | "warning" | "status_update" | "escalation"
    pub suggestion_type: String,
    pub timestamp: DateTime<Utc>,
    pub acknowledged: bool,
}

/// A preference learned by the assistant over time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnedPreference {
    pub key: String,
    pub value: String,
    /// How many observations support this preference.
    pub confidence: f32,
}

/// Per-agent assistant state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAssistant {
    pub agent_id: Uuid,
    pub swarm_id: Uuid,
    pub suggestions: VecDeque<AssistantSuggestion>,
    pub preferences: Vec<LearnedPreference>,
    /// Number of tasks the assistant has observed.
    pub observations: u64,
    /// Cumulative success score (for LoRA fine-tuning signal).
    pub cumulative_reward: f64,
}

impl AgentAssistant {
    pub fn new(agent_id: Uuid, swarm_id: Uuid) -> Self {
        Self {
            agent_id,
            swarm_id,
            suggestions: VecDeque::new(),
            preferences: Vec::new(),
            observations: 0,
            cumulative_reward: 0.0,
        }
    }

    /// Add a suggestion (capped at 50 most recent).
    pub fn suggest(
        &mut self,
        content: impl Into<String>,
        suggestion_type: impl Into<String>,
    ) -> Uuid {
        let id = Uuid::new_v4();
        self.suggestions.push_back(AssistantSuggestion {
            id,
            agent_id: self.agent_id,
            content: content.into(),
            suggestion_type: suggestion_type.into(),
            timestamp: Utc::now(),
            acknowledged: false,
        });
        if self.suggestions.len() > 50 {
            self.suggestions.pop_front();
        }
        id
    }

    /// Acknowledge (dismiss) a suggestion by id.
    pub fn acknowledge(&mut self, suggestion_id: Uuid) {
        if let Some(s) = self.suggestions.iter_mut().find(|s| s.id == suggestion_id) {
            s.acknowledged = true;
        }
    }

    /// Record a task outcome and update preferences.
    pub fn observe_outcome(&mut self, task_description: &str, success: bool, reward: f64) {
        self.observations += 1;
        self.cumulative_reward += reward;

        // Simple preference learning: track that certain task keywords correlate
        // with success/failure. Real implementation would use a LoRA adapter.
        if success {
            let key = format!("success_pattern_{}", self.observations % 10);
            self.upsert_preference(key, task_description[..task_description.len().min(40)].to_string(), 0.1);
        }

        // Auto-suggest status update if many tasks completed.
        if self.observations % 10 == 0 {
            self.suggest(
                format!(
                    "Agent has completed {} tasks with avg reward {:.2}. Consider summarising progress.",
                    self.observations,
                    self.cumulative_reward / self.observations as f64
                ),
                "status_update",
            );
        }
    }

    fn upsert_preference(&mut self, key: String, value: String, delta: f32) {
        if let Some(p) = self.preferences.iter_mut().find(|p| p.key == key) {
            p.confidence = (p.confidence + delta).min(1.0);
        } else {
            self.preferences.push(LearnedPreference { key, value, confidence: delta });
        }
    }

    /// Unacknowledged suggestions (shown in the inspector).
    pub fn pending_suggestions(&self) -> Vec<&AssistantSuggestion> {
        self.suggestions.iter().filter(|s| !s.acknowledged).collect()
    }
}

/// Registry of all agent assistants for a swarm.
#[derive(Clone)]
pub struct AssistantRegistry {
    assistants: Arc<RwLock<std::collections::HashMap<Uuid, AgentAssistant>>>,
}

impl AssistantRegistry {
    pub fn new() -> Self {
        Self { assistants: Arc::new(RwLock::new(std::collections::HashMap::new())) }
    }

    /// Create an assistant for a new agent.
    pub async fn create(&self, agent_id: Uuid, swarm_id: Uuid) {
        self.assistants
            .write()
            .await
            .insert(agent_id, AgentAssistant::new(agent_id, swarm_id));
    }

    /// Remove assistant when agent stops.
    pub async fn remove(&self, agent_id: Uuid) {
        self.assistants.write().await.remove(&agent_id);
    }

    /// Record a task outcome for an agent's assistant.
    pub async fn observe(
        &self,
        agent_id: Uuid,
        task_description: &str,
        success: bool,
        reward: f64,
    ) {
        if let Some(a) = self.assistants.write().await.get_mut(&agent_id) {
            a.observe_outcome(task_description, success, reward);
        }
    }

    /// Get unacknowledged suggestions for an agent.
    pub async fn pending_suggestions(&self, agent_id: Uuid) -> Vec<AssistantSuggestion> {
        self.assistants
            .read()
            .await
            .get(&agent_id)
            .map(|a| a.pending_suggestions().into_iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Acknowledge a suggestion.
    pub async fn acknowledge(&self, agent_id: Uuid, suggestion_id: Uuid) {
        if let Some(a) = self.assistants.write().await.get_mut(&agent_id) {
            a.acknowledge(suggestion_id);
        }
    }

    /// Snapshot of an assistant (for the inspector panel).
    pub async fn get(&self, agent_id: Uuid) -> Option<AgentAssistant> {
        self.assistants.read().await.get(&agent_id).cloned()
    }
}

impl Default for AssistantRegistry {
    fn default() -> Self {
        Self::new()
    }
}
