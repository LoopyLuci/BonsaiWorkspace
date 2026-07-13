//! Dynamic state knowledge backend
//!
//! Queries the Service Lifecycle Manager for current system state and runtime facts.
//! Includes TTL-based caching for performance.

use super::KnowledgeBackend;
use ahf_core::{FactualClaim, VerificationResult, VerificationStatus};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

/// Dynamic state entry with TTL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateEntry {
    /// The state value
    pub value: String,
    /// When this entry expires
    pub expires_at: SystemTime,
    /// Reliability of this state fact (0.0 to 1.0)
    pub reliability: f64,
}

impl StateEntry {
    /// Check if this entry is still valid
    pub fn is_valid(&self) -> bool {
        SystemTime::now() < self.expires_at
    }
}

/// Dynamic state knowledge base
///
/// Tracks system state and runtime facts with configurable TTL.
pub struct DynamicStateBackend {
    state: std::sync::Arc<std::sync::RwLock<HashMap<String, StateEntry>>>,
    default_ttl: Duration,
    reliability: f64,
}

impl DynamicStateBackend {
    /// Create a new dynamic state backend
    pub fn new() -> Self {
        DynamicStateBackend {
            state: std::sync::Arc::new(std::sync::RwLock::new(HashMap::new())),
            default_ttl: Duration::from_secs(60),
            reliability: 0.85,
        }
    }

    /// Create with custom TTL
    pub fn with_ttl(ttl: Duration) -> Self {
        DynamicStateBackend {
            state: std::sync::Arc::new(std::sync::RwLock::new(HashMap::new())),
            default_ttl: ttl,
            reliability: 0.85,
        }
    }

    /// Set a system state fact
    pub fn set_state(&self, key: String, value: String, reliability: f64) {
        let entry = StateEntry {
            value,
            expires_at: SystemTime::now() + self.default_ttl,
            reliability: reliability.clamp(0.0, 1.0),
        };

        let mut state = self.state.write().unwrap();
        state.insert(key, entry);
    }

    /// Get a state fact (if not expired)
    pub fn get_state(&self, key: &str) -> Option<String> {
        let state = self.state.read().unwrap();
        if let Some(entry) = state.get(key) {
            if entry.is_valid() {
                return Some(entry.value.clone());
            }
        }
        None
    }

    /// Clear all expired entries
    pub fn cleanup_expired(&self) {
        let mut state = self.state.write().unwrap();
        state.retain(|_, entry| entry.is_valid());
    }
}

impl Default for DynamicStateBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl KnowledgeBackend for DynamicStateBackend {
    async fn lookup(&self, claim: &FactualClaim) -> crate::KgsResult<VerificationResult> {
        // Cleanup expired entries
        self.cleanup_expired();

        // Try to find matching state
        let state = self.state.read().unwrap();

        for (_key, entry) in state.iter() {
            if entry.is_valid() && entry.value.to_lowercase().contains(&claim.object.to_lowercase())
            {
                return Ok(VerificationResult {
                    status: VerificationStatus::Valid,
                    proof: None,
                    reasoning: format!("Found in system state: {}", entry.value),
                    confidence: entry.reliability,
                });
            }
        }

        Ok(VerificationResult {
            status: VerificationStatus::Inconclusive,
            proof: None,
            reasoning: "Not found in dynamic state".to_string(),
            confidence: 0.0,
        })
    }

    fn name(&self) -> &str {
        "Dynamic State Backend (System Lifecycle Manager)"
    }

    fn reliability_score(&self) -> f64 {
        self.reliability
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_entry_valid() {
        let entry = StateEntry {
            value: "test".to_string(),
            expires_at: SystemTime::now() + Duration::from_secs(60),
            reliability: 0.9,
        };

        assert!(entry.is_valid());
    }

    #[test]
    fn test_state_entry_expired() {
        let entry = StateEntry {
            value: "test".to_string(),
            expires_at: SystemTime::now() - Duration::from_secs(1),
            reliability: 0.9,
        };

        assert!(!entry.is_valid());
    }

    #[test]
    fn test_set_and_get_state() {
        let backend = DynamicStateBackend::new();
        backend.set_state(
            "test_key".to_string(),
            "test_value".to_string(),
            0.9,
        );

        assert_eq!(backend.get_state("test_key"), Some("test_value".to_string()));
    }

    #[test]
    fn test_state_cleanup() {
        let backend = DynamicStateBackend::with_ttl(Duration::from_millis(100));
        backend.set_state("key1".to_string(), "value1".to_string(), 0.9);

        // Let it expire
        std::thread::sleep(Duration::from_millis(150));

        backend.cleanup_expired();

        assert_eq!(backend.get_state("key1"), None);
    }

    #[tokio::test]
    async fn test_dynamic_state_lookup() {
        use ahf_core::{Subject, Predicate};
        use chrono::Utc;

        let backend = DynamicStateBackend::new();
        backend.set_state(
            "system_ready".to_string(),
            "System is ready".to_string(),
            0.95,
        );

        let claim = FactualClaim {
            id: uuid::Uuid::new_v4(),
            subject: Subject::new("system", "System"),
            predicate: Predicate::new("is", "is"),
            object: "ready".to_string(),
            context: None,
            source_confidence: 0.9,
            timestamp: Utc::now(),
            source_reference: None,
        };

        let result = backend.lookup(&claim).await.unwrap();
        assert_eq!(result.status, VerificationStatus::Valid);
    }

    #[tokio::test]
    async fn test_dynamic_state_not_found() {
        use ahf_core::{Subject, Predicate};
        use chrono::Utc;

        let backend = DynamicStateBackend::new();

        let claim = FactualClaim {
            id: uuid::Uuid::new_v4(),
            subject: Subject::new("unknown", "Unknown"),
            predicate: Predicate::new("is", "is"),
            object: "state".to_string(),
            context: None,
            source_confidence: 0.9,
            timestamp: Utc::now(),
            source_reference: None,
        };

        let result = backend.lookup(&claim).await.unwrap();
        assert_eq!(result.status, VerificationStatus::Inconclusive);
    }

    #[test]
    fn test_reliability_score() {
        let backend = DynamicStateBackend::new();
        assert!(backend.reliability_score() > 0.8);
    }
}
