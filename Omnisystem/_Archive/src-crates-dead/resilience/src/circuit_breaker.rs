use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Duration, Utc};

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitState {
    Closed,      // Normal operation
    Open,        // Failing, rejecting requests
    HalfOpen,    // Testing recovery
}

impl std::fmt::Display for CircuitState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitState::Closed => write!(f, "Closed (healthy)"),
            CircuitState::Open => write!(f, "Open (failing)"),
            CircuitState::HalfOpen => write!(f, "HalfOpen (recovering)"),
        }
    }
}

/// Circuit breaker implementation
pub struct CircuitBreaker {
    state: Arc<RwLock<CircuitState>>,
    failure_count: Arc<RwLock<u32>>,
    success_count: Arc<RwLock<u32>>,
    last_failure_time: Arc<RwLock<Option<DateTime<Utc>>>>,

    failure_threshold: u32,
    success_threshold: u32,
    timeout_secs: u64,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, success_threshold: u32, timeout_secs: u64) -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_count: Arc::new(RwLock::new(0)),
            success_count: Arc::new(RwLock::new(0)),
            last_failure_time: Arc::new(RwLock::new(None)),
            failure_threshold,
            success_threshold,
            timeout_secs,
        }
    }

    /// Get current state
    pub fn state(&self) -> CircuitState {
        *self.state.read()
    }

    /// Record a successful call
    pub fn record_success(&self) {
        match self.state() {
            CircuitState::Closed => {
                // Reset failure count
                *self.failure_count.write() = 0;
                *self.success_count.write() = 0;
            }
            CircuitState::HalfOpen => {
                let mut success = self.success_count.write();
                *success += 1;

                // Check if we should close
                if *success >= self.success_threshold {
                    tracing::info!("Circuit breaker closing (recovered)");
                    *self.state.write() = CircuitState::Closed;
                    *self.failure_count.write() = 0;
                    *success = 0;
                }
            }
            CircuitState::Open => {
                // Still open, ignore success
            }
        }
    }

    /// Record a failed call
    pub fn record_failure(&self) {
        let mut failure_count = self.failure_count.write();
        *failure_count += 1;

        *self.last_failure_time.write() = Some(Utc::now());

        if *failure_count >= self.failure_threshold {
            match self.state() {
                CircuitState::Closed | CircuitState::HalfOpen => {
                    tracing::warn!(
                        "Circuit breaker opening after {} failures",
                        failure_count
                    );
                    *self.state.write() = CircuitState::Open;
                }
                CircuitState::Open => {
                    // Already open
                }
            }
        }
    }

    /// Check if we can execute (or should fail fast)
    pub fn can_execute(&self) -> bool {
        match self.state() {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if timeout has passed
                if let Some(last_failure) = *self.last_failure_time.read() {
                    let elapsed = Utc::now()
                        .signed_duration_since(last_failure)
                        .num_seconds() as u64;

                    if elapsed >= self.timeout_secs {
                        tracing::info!(
                            "Circuit breaker attempting recovery ({}s passed)",
                            elapsed
                        );
                        *self.state.write() = CircuitState::HalfOpen;
                        *self.failure_count.write() = 0;
                        *self.success_count.write() = 0;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    /// Get diagnostics
    pub fn diagnostics(&self) -> CircuitBreakerDiagnostics {
        CircuitBreakerDiagnostics {
            state: self.state(),
            failure_count: *self.failure_count.read(),
            success_count: *self.success_count.read(),
            last_failure_time: *self.last_failure_time.read(),
            failure_threshold: self.failure_threshold,
            success_threshold: self.success_threshold,
        }
    }
}

/// Diagnostics information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerDiagnostics {
    pub state: CircuitState,
    pub failure_count: u32,
    pub success_count: u32,
    pub last_failure_time: Option<DateTime<Utc>>,
    pub failure_threshold: u32,
    pub success_threshold: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circuit_breaker_open() {
        let cb = CircuitBreaker::new(3, 2, 60);
        assert_eq!(cb.state(), CircuitState::Closed);

        // Record failures
        cb.record_failure();
        cb.record_failure();
        cb.record_failure();

        assert_eq!(cb.state(), CircuitState::Open);
        assert!(!cb.can_execute());
    }

    #[test]
    fn test_circuit_breaker_half_open() {
        let cb = CircuitBreaker::new(1, 1, 0);
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        // Should transition to HalfOpen
        assert!(cb.can_execute());
        assert_eq!(cb.state(), CircuitState::HalfOpen);
    }

    #[test]
    fn test_circuit_breaker_recovery() {
        let cb = CircuitBreaker::new(1, 1, 0);
        cb.record_failure();
        assert_eq!(cb.state(), CircuitState::Open);

        // Attempt recovery
        assert!(cb.can_execute());
        assert_eq!(cb.state(), CircuitState::HalfOpen);

        // Record success
        cb.record_success();
        assert_eq!(cb.state(), CircuitState::Closed);
    }
}
