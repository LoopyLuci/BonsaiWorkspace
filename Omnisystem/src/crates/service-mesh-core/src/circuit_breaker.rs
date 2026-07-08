use crate::{CircuitBreaker, CircuitBreakerConfig, CircuitBreakerState, MeshError, MeshResult, ServiceId};
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;

pub struct CircuitBreakerManager {
    pub breakers: Arc<DashMap<String, CircuitBreaker>>,
}

impl CircuitBreakerManager {
    pub fn new() -> Self {
        Self {
            breakers: Arc::new(DashMap::new()),
        }
    }

    pub async fn check_circuit(&self, service_id: &ServiceId) -> MeshResult<bool> {
        match self.breakers.get(&service_id.0) {
            Some(breaker) => match breaker.state {
                CircuitBreakerState::Closed => Ok(true),
                CircuitBreakerState::Open => {
                    let now = Utc::now();
                    let opened_at = breaker.opened_at.unwrap_or_else(|| now);
                    let elapsed = (now - opened_at).num_seconds() as u64;

                    if elapsed >= breaker.config.timeout_secs {
                        Ok(true)
                    } else {
                        Err(MeshError::CircuitBreakerOpen(service_id.0.clone()))
                    }
                }
                CircuitBreakerState::HalfOpen => Ok(true),
            },
            None => {
                let breaker = CircuitBreaker {
                    service_id: service_id.clone(),
                    state: CircuitBreakerState::Closed,
                    failure_count: 0,
                    success_count: 0,
                    last_state_change: Utc::now(),
                    opened_at: None,
                    config: CircuitBreakerConfig::default(),
                };
                self.breakers.insert(service_id.0.clone(), breaker);
                Ok(true)
            }
        }
    }

    pub async fn record_success(&self, service_id: &ServiceId) -> MeshResult<()> {
        if let Some(mut breaker) = self.breakers.get_mut(&service_id.0) {
            breaker.success_count += 1;

            match breaker.state {
                CircuitBreakerState::HalfOpen => {
                    if breaker.success_count >= breaker.config.success_threshold {
                        breaker.state = CircuitBreakerState::Closed;
                        breaker.failure_count = 0;
                        breaker.success_count = 0;
                        breaker.last_state_change = Utc::now();
                    }
                }
                CircuitBreakerState::Closed => {
                    breaker.failure_count = breaker.failure_count.saturating_sub(1);
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub async fn record_failure(&self, service_id: &ServiceId) -> MeshResult<()> {
        if let Some(mut breaker) = self.breakers.get_mut(&service_id.0) {
            breaker.failure_count += 1;

            match breaker.state {
                CircuitBreakerState::Closed => {
                    if breaker.failure_count >= breaker.config.failure_threshold {
                        breaker.state = CircuitBreakerState::Open;
                        breaker.opened_at = Some(Utc::now());
                        breaker.last_state_change = Utc::now();
                    }
                }
                CircuitBreakerState::HalfOpen => {
                    breaker.state = CircuitBreakerState::Open;
                    breaker.opened_at = Some(Utc::now());
                    breaker.last_state_change = Utc::now();
                    breaker.success_count = 0;
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub async fn get_circuit_state(&self, service_id: &ServiceId) -> MeshResult<CircuitBreaker> {
        self.breakers
            .get(&service_id.0)
            .map(|entry| entry.clone())
            .ok_or_else(|| MeshError::ServiceNotFound(service_id.0.clone()))
    }

    pub async fn reset_circuit(&self, service_id: &ServiceId) -> MeshResult<()> {
        if let Some(mut breaker) = self.breakers.get_mut(&service_id.0) {
            breaker.state = CircuitBreakerState::Closed;
            breaker.failure_count = 0;
            breaker.success_count = 0;
            breaker.opened_at = None;
            breaker.last_state_change = Utc::now();
        }
        Ok(())
    }

    pub fn breaker_count(&self) -> usize {
        self.breakers.len()
    }
}

impl Default for CircuitBreakerManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_check_circuit_creates_closed_breaker() {
        let manager = CircuitBreakerManager::new();
        let service_id = ServiceId(uuid::Uuid::new_v4().to_string());

        let result = manager.check_circuit(&service_id).await;
        assert!(result.is_ok());
        assert_eq!(manager.breaker_count(), 1);
    }

    #[tokio::test]
    async fn test_record_failure_opens_circuit() {
        let manager = CircuitBreakerManager::new();
        let service_id = ServiceId(uuid::Uuid::new_v4().to_string());

        manager.check_circuit(&service_id).await.unwrap();

        for _ in 0..5 {
            manager.record_failure(&service_id).await.unwrap();
        }

        let breaker = manager.get_circuit_state(&service_id).await.unwrap();
        assert_eq!(breaker.state, CircuitBreakerState::Open);
    }

    #[tokio::test]
    async fn test_circuit_breaker_open_rejects_requests() {
        let manager = CircuitBreakerManager::new();
        let service_id = ServiceId(uuid::Uuid::new_v4().to_string());

        manager.check_circuit(&service_id).await.unwrap();
        for _ in 0..5 {
            manager.record_failure(&service_id).await.unwrap();
        }

        let result = manager.check_circuit(&service_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_half_open_state_with_successes() {
        let manager = CircuitBreakerManager::new();
        let service_id = ServiceId(uuid::Uuid::new_v4().to_string());

        manager.check_circuit(&service_id).await.unwrap();
        for _ in 0..5 {
            manager.record_failure(&service_id).await.unwrap();
        }

        if let Some(mut breaker) = manager.breakers.get_mut(&service_id.0) {
            breaker.state = CircuitBreakerState::HalfOpen;
        }

        manager.record_success(&service_id).await.unwrap();
        manager.record_success(&service_id).await.unwrap();

        let breaker = manager.get_circuit_state(&service_id).await.unwrap();
        assert_eq!(breaker.state, CircuitBreakerState::Closed);
    }

    #[tokio::test]
    async fn test_reset_circuit() {
        let manager = CircuitBreakerManager::new();
        let service_id = ServiceId(uuid::Uuid::new_v4().to_string());

        manager.check_circuit(&service_id).await.unwrap();
        for _ in 0..5 {
            manager.record_failure(&service_id).await.unwrap();
        }

        manager.reset_circuit(&service_id).await.unwrap();

        let breaker = manager.get_circuit_state(&service_id).await.unwrap();
        assert_eq!(breaker.state, CircuitBreakerState::Closed);
        assert_eq!(breaker.failure_count, 0);
    }
}
