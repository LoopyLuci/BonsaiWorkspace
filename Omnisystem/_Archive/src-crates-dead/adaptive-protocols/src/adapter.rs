use crate::{ProtocolError, ProtocolResult, ProtocolTransition, ProtocolType};
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;

pub struct ProtocolAdapter {
    transitions: Arc<DashMap<String, ProtocolTransition>>,
}

impl ProtocolAdapter {
    pub fn new() -> Self {
        Self {
            transitions: Arc::new(DashMap::new()),
        }
    }

    pub async fn initiate_transition(
        &self,
        transition_id: &str,
        from_protocol: ProtocolType,
        to_protocol: ProtocolType,
        reason: &str,
    ) -> ProtocolResult<ProtocolTransition> {
        let transition = ProtocolTransition {
            from_protocol,
            to_protocol,
            reason: reason.to_string(),
            initiated_at: Utc::now(),
            completed_at: None,
            success: false,
        };

        self.transitions.insert(transition_id.to_string(), transition.clone());
        Ok(transition)
    }

    pub async fn complete_transition(
        &self,
        transition_id: &str,
        success: bool,
    ) -> ProtocolResult<()> {
        if let Some(mut transition) = self.transitions.get_mut(transition_id) {
            transition.completed_at = Some(Utc::now());
            transition.success = success;
            Ok(())
        } else {
            Err(ProtocolError::ProtocolNotFound)
        }
    }

    pub async fn get_transition(&self, transition_id: &str) -> ProtocolResult<ProtocolTransition> {
        self.transitions
            .get(transition_id)
            .map(|entry| entry.clone())
            .ok_or(ProtocolError::ProtocolNotFound)
    }

    pub async fn get_active_transitions(&self) -> ProtocolResult<Vec<ProtocolTransition>> {
        let active: Vec<ProtocolTransition> = self
            .transitions
            .iter()
            .filter(|entry| entry.value().completed_at.is_none())
            .map(|entry| entry.value().clone())
            .collect();

        Ok(active)
    }

    pub fn transition_count(&self) -> usize {
        self.transitions.len()
    }
}

impl Default for ProtocolAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_initiate_transition() {
        let adapter = ProtocolAdapter::new();
        let transition = adapter
            .initiate_transition("t-1", ProtocolType::Http1, ProtocolType::Http2, "latency improvement")
            .await
            .unwrap();

        assert_eq!(transition.from_protocol, ProtocolType::Http1);
        assert_eq!(transition.to_protocol, ProtocolType::Http2);
        assert!(!transition.success);
    }

    #[tokio::test]
    async fn test_complete_transition_success() {
        let adapter = ProtocolAdapter::new();
        adapter
            .initiate_transition("t-1", ProtocolType::Http1, ProtocolType::Http2, "latency improvement")
            .await
            .unwrap();

        adapter.complete_transition("t-1", true).await.unwrap();
        let transition = adapter.get_transition("t-1").await.unwrap();

        assert!(transition.success);
        assert!(transition.completed_at.is_some());
    }

    #[tokio::test]
    async fn test_complete_transition_failure() {
        let adapter = ProtocolAdapter::new();
        adapter
            .initiate_transition("t-1", ProtocolType::Http1, ProtocolType::Http2, "latency improvement")
            .await
            .unwrap();

        adapter.complete_transition("t-1", false).await.unwrap();
        let transition = adapter.get_transition("t-1").await.unwrap();

        assert!(!transition.success);
    }

    #[tokio::test]
    async fn test_get_transition() {
        let adapter = ProtocolAdapter::new();
        adapter
            .initiate_transition("t-1", ProtocolType::Http1, ProtocolType::Http2, "latency improvement")
            .await
            .unwrap();

        let transition = adapter.get_transition("t-1").await.unwrap();
        assert_eq!(transition.from_protocol, ProtocolType::Http1);
    }

    #[tokio::test]
    async fn test_get_active_transitions() {
        let adapter = ProtocolAdapter::new();
        adapter
            .initiate_transition("t-1", ProtocolType::Http1, ProtocolType::Http2, "reason1")
            .await
            .unwrap();
        adapter
            .initiate_transition("t-2", ProtocolType::Http2, ProtocolType::Http3, "reason2")
            .await
            .unwrap();

        adapter.complete_transition("t-1", true).await.unwrap();

        let active = adapter.get_active_transitions().await.unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].from_protocol, ProtocolType::Http2);
    }

    #[tokio::test]
    async fn test_transition_not_found() {
        let adapter = ProtocolAdapter::new();
        let result = adapter.get_transition("nonexistent").await;

        assert!(result.is_err());
    }
}
