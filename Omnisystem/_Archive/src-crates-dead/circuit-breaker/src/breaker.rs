use crate::{CircuitBreakerConfig, CircuitBreakerError, CircuitBreakerResult, CircuitBreakerStatus, CircuitState, StateTransition};
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;

pub struct CircuitBreaker {
    config: CircuitBreakerConfig,
    status: Arc<DashMap<String, CircuitBreakerStatus>>,
    transitions: Arc<DashMap<String, Vec<StateTransition>>>,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        let status = CircuitBreakerStatus {
            breaker_id: config.breaker_id.clone(),
            current_state: CircuitState::Closed,
            consecutive_failures: 0,
            consecutive_successes: 0,
            last_failure_time: None,
            opened_at: None,
        };

        let status_map = DashMap::new();
        status_map.insert(config.breaker_id.clone(), status);

        Self {
            config,
            status: Arc::new(status_map),
            transitions: Arc::new(DashMap::new()),
        }
    }

    pub async fn call<F, T>(&self, f: F) -> CircuitBreakerResult<T>
    where
        F: std::future::Future<Output = CircuitBreakerResult<T>>,
    {
        if let Some(status) = self.status.get(&self.config.breaker_id) {
            if status.current_state == CircuitState::Open {
                return Err(CircuitBreakerError::CircuitOpen);
            }
        }

        let result = f.await;

        match &result {
            Ok(_) => {
                self.record_success().await?;
            }
            Err(_) => {
                self.record_failure().await?;
            }
        }

        result
    }

    pub async fn record_success(&self) -> CircuitBreakerResult<()> {
        if let Some(mut status) = self.status.get_mut(&self.config.breaker_id) {
            status.consecutive_failures = 0;
            status.consecutive_successes += 1;

            if status.current_state == CircuitState::HalfOpen {
                if status.consecutive_successes >= self.config.success_threshold {
                    self.transition_state(CircuitState::Closed).await?;
                }
            }
            Ok(())
        } else {
            Err(CircuitBreakerError::CircuitNotFound)
        }
    }

    pub async fn record_failure(&self) -> CircuitBreakerResult<()> {
        if let Some(mut status) = self.status.get_mut(&self.config.breaker_id) {
            status.consecutive_failures += 1;
            status.consecutive_successes = 0;
            status.last_failure_time = Some(Utc::now());

            if status.consecutive_failures >= self.config.failure_threshold {
                self.transition_state(CircuitState::Open).await?;
            }
            Ok(())
        } else {
            Err(CircuitBreakerError::CircuitNotFound)
        }
    }

    async fn transition_state(&self, new_state: CircuitState) -> CircuitBreakerResult<()> {
        if let Some(mut status) = self.status.get_mut(&self.config.breaker_id) {
            let old_state = status.current_state;
            status.current_state = new_state;

            if new_state == CircuitState::Open {
                status.opened_at = Some(Utc::now());
            } else if new_state == CircuitState::Closed {
                status.opened_at = None;
            }

            let transition = StateTransition {
                from_state: old_state,
                to_state: new_state,
                reason: format!("Transitioned from {:?} to {:?}", old_state, new_state),
                timestamp: Utc::now(),
            };

            self.transitions
                .entry(self.config.breaker_id.clone())
                .or_insert_with(Vec::new)
                .push(transition);

            Ok(())
        } else {
            Err(CircuitBreakerError::CircuitNotFound)
        }
    }

    pub async fn attempt_recovery(&self) -> CircuitBreakerResult<()> {
        if let Some(mut status) = self.status.get_mut(&self.config.breaker_id) {
            if status.current_state == CircuitState::Open {
                if let Some(opened_at) = status.opened_at {
                    let elapsed = Utc::now()
                        .signed_duration_since(opened_at)
                        .num_milliseconds() as u64;

                    if elapsed >= self.config.timeout_ms {
                        status.current_state = CircuitState::HalfOpen;
                        status.consecutive_failures = 0;
                        status.consecutive_successes = 0;
                        return Ok(());
                    }
                }
                Err(CircuitBreakerError::RecoveryFailed)
            } else {
                Ok(())
            }
        } else {
            Err(CircuitBreakerError::CircuitNotFound)
        }
    }

    pub async fn get_status(&self) -> CircuitBreakerResult<CircuitBreakerStatus> {
        self.status
            .get(&self.config.breaker_id)
            .map(|entry| entry.clone())
            .ok_or(CircuitBreakerError::CircuitNotFound)
    }

    pub async fn reset(&self) -> CircuitBreakerResult<()> {
        if let Some(mut status) = self.status.get_mut(&self.config.breaker_id) {
            status.current_state = CircuitState::Closed;
            status.consecutive_failures = 0;
            status.consecutive_successes = 0;
            status.last_failure_time = None;
            status.opened_at = None;
            Ok(())
        } else {
            Err(CircuitBreakerError::CircuitNotFound)
        }
    }

    pub async fn get_transitions(&self) -> CircuitBreakerResult<Vec<StateTransition>> {
        if let Some(transitions) = self.transitions.get(&self.config.breaker_id) {
            Ok(transitions.value().clone())
        } else {
            Ok(Vec::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_creation() {
        let config = CircuitBreakerConfig {
            breaker_id: "breaker-1".to_string(),
            failure_threshold: 5,
            success_threshold: 2,
            timeout_ms: 5000,
            half_open_max_calls: 3,
        };

        let breaker = CircuitBreaker::new(config);
        let status = breaker.get_status().await.unwrap();

        assert_eq!(status.current_state, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_record_success() {
        let config = CircuitBreakerConfig {
            breaker_id: "breaker-1".to_string(),
            failure_threshold: 5,
            success_threshold: 2,
            timeout_ms: 5000,
            half_open_max_calls: 3,
        };

        let breaker = CircuitBreaker::new(config);
        breaker.record_success().await.unwrap();

        let status = breaker.get_status().await.unwrap();
        assert_eq!(status.consecutive_successes, 1);
        assert_eq!(status.consecutive_failures, 0);
    }

    #[tokio::test]
    async fn test_record_failure() {
        let config = CircuitBreakerConfig {
            breaker_id: "breaker-1".to_string(),
            failure_threshold: 5,
            success_threshold: 2,
            timeout_ms: 5000,
            half_open_max_calls: 3,
        };

        let breaker = CircuitBreaker::new(config);
        breaker.record_failure().await.unwrap();

        let status = breaker.get_status().await.unwrap();
        assert_eq!(status.consecutive_failures, 1);
        assert_eq!(status.consecutive_successes, 0);
    }

    #[tokio::test]
    async fn test_circuit_opens_on_threshold() {
        let config = CircuitBreakerConfig {
            breaker_id: "breaker-1".to_string(),
            failure_threshold: 3,
            success_threshold: 2,
            timeout_ms: 5000,
            half_open_max_calls: 3,
        };

        let breaker = CircuitBreaker::new(config);

        for _ in 0..3 {
            breaker.record_failure().await.unwrap();
        }

        let status = breaker.get_status().await.unwrap();
        assert_eq!(status.current_state, CircuitState::Open);
    }

    #[tokio::test]
    async fn test_circuit_half_open_after_timeout() {
        let config = CircuitBreakerConfig {
            breaker_id: "breaker-1".to_string(),
            failure_threshold: 3,
            success_threshold: 2,
            timeout_ms: 1,
            half_open_max_calls: 3,
        };

        let breaker = CircuitBreaker::new(config);

        for _ in 0..3 {
            breaker.record_failure().await.unwrap();
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
        breaker.attempt_recovery().await.unwrap();

        let status = breaker.get_status().await.unwrap();
        assert_eq!(status.current_state, CircuitState::HalfOpen);
    }

    #[tokio::test]
    async fn test_reset_circuit() {
        let config = CircuitBreakerConfig {
            breaker_id: "breaker-1".to_string(),
            failure_threshold: 3,
            success_threshold: 2,
            timeout_ms: 5000,
            half_open_max_calls: 3,
        };

        let breaker = CircuitBreaker::new(config);

        for _ in 0..3 {
            breaker.record_failure().await.unwrap();
        }

        breaker.reset().await.unwrap();
        let status = breaker.get_status().await.unwrap();

        assert_eq!(status.current_state, CircuitState::Closed);
        assert_eq!(status.consecutive_failures, 0);
    }

    #[tokio::test]
    async fn test_call_with_open_circuit() {
        let config = CircuitBreakerConfig {
            breaker_id: "breaker-1".to_string(),
            failure_threshold: 1,
            success_threshold: 2,
            timeout_ms: 5000,
            half_open_max_calls: 3,
        };

        let breaker = CircuitBreaker::new(config);
        breaker.record_failure().await.unwrap();

        let result: CircuitBreakerResult<()> = breaker.call(async { Ok(()) }).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_call_with_closed_circuit() {
        let config = CircuitBreakerConfig {
            breaker_id: "breaker-1".to_string(),
            failure_threshold: 5,
            success_threshold: 2,
            timeout_ms: 5000,
            half_open_max_calls: 3,
        };

        let breaker = CircuitBreaker::new(config);

        let result: CircuitBreakerResult<String> = breaker.call(async { Ok("success".to_string()) }).await;
        assert!(result.is_ok());
    }
}
