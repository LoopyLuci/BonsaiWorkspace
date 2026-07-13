// Natural Language Understanding (BonsAI V2)

use serde::{Deserialize, Serialize};
use crate::Capability;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    pub name: String,
    pub tool_name: String,
    pub parameters: serde_json::Value,
    pub confidence: f32,
    pub required_capability: Capability,
}

impl Intent {
    pub fn description(&self) -> String {
        format!("{} (confidence: {:.0}%)", self.name, self.confidence * 100.0)
    }
}

pub struct IntentClassifier {
    // In production, this would load a BonsAI V2 model
}

impl IntentClassifier {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn classify(&self, text: &str) -> anyhow::Result<Intent> {
        // In a real implementation, this would call BonsAI V2 MCP tool
        // For now, simple pattern matching
        let (name, tool_name, capability, confidence) = if text.contains("poe") && text.contains("ac") {
            ("PoeAcPersonality", "set_narrative_mode", Capability::ModelChat, 0.95)
        } else if text.contains("poe") {
            ("PoeChatIntent", "poe_chat", Capability::ModelChat, 0.90)
        } else if text.contains("sweep") && text.contains("repo") {
            ("BugHunterSweep", "bug_hunter_sweep_repo", Capability::BugHunterSweep, 0.92)
        } else if text.contains("status") {
            ("GetStatus", "bonsai_status", Capability::View, 0.88)
        } else {
            ("Unknown", "help", Capability::View, 0.5)
        };

        Ok(Intent {
            name: name.into(),
            tool_name: tool_name.into(),
            parameters: serde_json::json!({ "text": text }),
            confidence,
            required_capability: capability,
        })
    }
}

impl Default for IntentClassifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_intent_classification() {
        let classifier = IntentClassifier::new();

        let intent = classifier.classify("poe ac").await.unwrap();
        assert!(intent.confidence > 0.8);
        assert_eq!(intent.name, "PoeAcPersonality");
    }
}
