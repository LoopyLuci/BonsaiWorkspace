use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct TrustScore {
    pub agent_id: String,
    pub reputation: f32,
    pub reliability: f32,
    pub updated_at: u64,
}

pub struct TrustManager {
    trust_scores: Arc<DashMap<String, TrustScore>>,
}

impl TrustManager {
    pub fn new() -> Self {
        Self {
            trust_scores: Arc::new(DashMap::new()),
        }
    }

    pub fn record_interaction(&self, agent_id: &str, success: bool) {
        let mut entry = self.trust_scores.entry(agent_id.to_string()).or_insert(TrustScore {
            agent_id: agent_id.to_string(),
            reputation: 0.5,
            reliability: 0.5,
            updated_at: 0,
        });

        if success {
            entry.reputation = (entry.reputation + 0.1).min(1.0);
            entry.reliability = (entry.reliability + 0.05).min(1.0);
        } else {
            entry.reputation = (entry.reputation - 0.1).max(0.0);
            entry.reliability = (entry.reliability - 0.05).max(0.0);
        }
    }

    pub fn get_trust_score(&self, agent_id: &str) -> Option<TrustScore> {
        self.trust_scores.get(agent_id).map(|s| s.clone())
    }

    pub fn is_trustworthy(&self, agent_id: &str) -> bool {
        self.trust_scores.get(agent_id).map(|s| s.reputation > 0.6).unwrap_or(false)
    }
}

impl Default for TrustManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trust_creation() {
        let tm = TrustManager::new();
        assert!(tm.get_trust_score("agent1").is_none());
    }

    #[test]
    fn test_record_interaction() {
        let tm = TrustManager::new();
        tm.record_interaction("agent1", true);
        assert!(tm.get_trust_score("agent1").is_some());
    }

    #[test]
    fn test_trustworthiness() {
        let tm = TrustManager::new();
        tm.record_interaction("agent1", true);
        tm.record_interaction("agent1", true);
        tm.record_interaction("agent1", true);
        assert!(tm.is_trustworthy("agent1"));
    }
}
