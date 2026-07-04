use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Retry strategy for backoff calculation
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum RetryStrategy {
    Immediate,           // No delay
    Linear { increment_ms: u64 }, // delay = attempt * increment
    Exponential {
        initial_ms: u64,
        multiplier: f64,
        max_ms: u64,
    },
    Fibonacci { initial_ms: u64, max_ms: u64 },
}

/// Retry policy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub strategy: RetryStrategy,
    pub jitter: bool,
    pub retry_on_timeout: bool,
}

impl RetryPolicy {
    /// Create with exponential backoff (recommended)
    pub fn exponential(max_attempts: u32, initial_ms: u64, max_ms: u64) -> Self {
        Self {
            max_attempts,
            strategy: RetryStrategy::Exponential {
                initial_ms,
                multiplier: 2.0,
                max_ms,
            },
            jitter: true,
            retry_on_timeout: true,
        }
    }

    /// Create with immediate retries
    pub fn immediate(max_attempts: u32) -> Self {
        Self {
            max_attempts,
            strategy: RetryStrategy::Immediate,
            jitter: false,
            retry_on_timeout: false,
        }
    }

    /// Calculate backoff for attempt
    pub fn backoff_for_attempt(&self, attempt: u32) -> Duration {
        let ms = match self.strategy {
            RetryStrategy::Immediate => 0,
            RetryStrategy::Linear { increment_ms } => (attempt as u64) * increment_ms,
            RetryStrategy::Exponential {
                initial_ms,
                multiplier,
                max_ms,
            } => {
                let delay = initial_ms as f64 * multiplier.powi(attempt as i32 - 1);
                (delay as u64).min(max_ms)
            }
            RetryStrategy::Fibonacci {
                initial_ms,
                max_ms,
            } => {
                let fib = fibonacci(attempt);
                ((initial_ms as f64 * fib as f64) as u64).min(max_ms)
            }
        };

        let final_ms = if self.jitter {
            // Add jitter: random between 0 and calculated value
            let jitter = (rand_percent() * ms as f64) as u64;
            (ms + jitter) / 2
        } else {
            ms
        };

        Duration::from_millis(final_ms)
    }

    /// Check if should retry
    pub fn should_retry(&self, attempt: u32) -> bool {
        attempt < self.max_attempts
    }
}

/// Retry context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryContext {
    pub max_attempts: u32,
    pub current_attempt: u32,
    pub last_error: Option<String>,
}

impl RetryContext {
    pub fn new(max_attempts: u32) -> Self {
        Self {
            max_attempts,
            current_attempt: 0,
            last_error: None,
        }
    }

    pub fn next_attempt(&mut self) {
        self.current_attempt += 1;
    }

    pub fn set_error(&mut self, error: String) {
        self.last_error = Some(error);
    }

    pub fn is_exhausted(&self) -> bool {
        self.current_attempt >= self.max_attempts
    }
}

// Helper functions
fn fibonacci(n: u32) -> f64 {
    let mut a = 0.0;
    let mut b = 1.0;

    for _ in 0..n {
        let temp = a + b;
        a = b;
        b = temp;
    }

    a
}

fn rand_percent() -> f64 {
    // Simple pseudo-random (0.0 to 1.0)
    // In production, use proper random crate
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    ((nanos % 1000) as f64) / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exponential_backoff() {
        let policy = RetryPolicy::exponential(5, 100, 10000);

        let delay1 = policy.backoff_for_attempt(1);
        let delay2 = policy.backoff_for_attempt(2);
        let delay3 = policy.backoff_for_attempt(3);

        // Each should be roughly 2x the previous
        assert!(delay2.as_millis() >= delay1.as_millis());
        assert!(delay3.as_millis() >= delay2.as_millis());
    }

    #[test]
    fn test_retry_context() {
        let mut ctx = RetryContext::new(3);
        assert!(!ctx.is_exhausted());

        ctx.next_attempt();
        assert!(!ctx.is_exhausted());

        ctx.next_attempt();
        ctx.next_attempt();
        assert!(ctx.is_exhausted());
    }

    #[test]
    fn test_immediate_backoff() {
        let policy = RetryPolicy::immediate(3);
        let delay = policy.backoff_for_attempt(1);
        assert_eq!(delay.as_millis(), 0);
    }
}
