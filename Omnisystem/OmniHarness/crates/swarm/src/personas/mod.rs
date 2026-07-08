pub mod bug_fixer;
pub mod feature_developer;

pub use bug_fixer::bug_fixer_persona;
pub use feature_developer::feature_developer_persona;

use crate::role::{AgentProfile, Capability, SwarmRole};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Extended persona definition: AgentProfile + runtime guidance fields.
/// `AgentProfile` is the in-registry identity; `PersonaDef` adds the LLM
/// system prompt and execution preferences that drive the agent's behaviour.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonaDef {
    pub profile: AgentProfile,
    /// System prompt injected at the start of every inference turn.
    pub system_prompt: String,
    /// Preferred model slug (falls back to swarm default if None).
    pub model_preference: Option<String>,
    /// Max output tokens per turn.
    pub max_tokens: u32,
    /// Sampling temperature.
    pub temperature: f32,
    /// Max retries before escalation.
    pub max_retries: u32,
}

impl PersonaDef {
    pub fn new(
        role: SwarmRole,
        display_name: impl Into<String>,
        capabilities: Vec<Capability>,
        system_prompt: impl Into<String>,
        model_preference: Option<&str>,
    ) -> Self {
        Self {
            profile: AgentProfile {
                agent_id: Uuid::new_v4(),
                role,
                display_name: display_name.into(),
                capabilities,
                cost_per_minute: 0.0,
                reliability: 1.0,
                current_load: 0.0,
                is_remote: false,
            },
            system_prompt: system_prompt.into(),
            model_preference: model_preference.map(|s| s.to_string()),
            max_tokens: 8192,
            temperature: 0.3,
            max_retries: 3,
        }
    }
}
