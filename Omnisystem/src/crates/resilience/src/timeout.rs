use std::time::Duration;
use serde::{Deserialize, Serialize};

/// Timeout policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutPolicy {
    pub default_timeout: Duration,
    pub connect_timeout: Duration,
    pub read_timeout: Duration,
    pub write_timeout: Duration,
    pub request_timeout: Duration,
}

impl TimeoutPolicy {
    /// Create with all same timeout
    pub fn uniform(duration: Duration) -> Self {
        Self {
            default_timeout: duration,
            connect_timeout: duration,
            read_timeout: duration,
            write_timeout: duration,
            request_timeout: duration,
        }
    }

    /// Create with HTTP-like defaults
    pub fn http_defaults() -> Self {
        Self {
            default_timeout: Duration::from_secs(30),
            connect_timeout: Duration::from_secs(10),
            read_timeout: Duration::from_secs(30),
            write_timeout: Duration::from_secs(30),
            request_timeout: Duration::from_secs(60),
        }
    }

    /// Create with strict defaults (for critical paths)
    pub fn strict() -> Self {
        Self {
            default_timeout: Duration::from_secs(5),
            connect_timeout: Duration::from_secs(2),
            read_timeout: Duration::from_secs(5),
            write_timeout: Duration::from_secs(5),
            request_timeout: Duration::from_secs(10),
        }
    }
}

impl Default for TimeoutPolicy {
    fn default() -> Self {
        Self::http_defaults()
    }
}

/// Timeout enforcement
pub struct TimeoutEnforcer {
    policy: TimeoutPolicy,
}

impl TimeoutEnforcer {
    pub fn new(policy: TimeoutPolicy) -> Self {
        Self { policy }
    }

    /// Get applicable timeout for operation
    pub fn timeout_for(&self, op_type: &str) -> Duration {
        match op_type {
            "connect" => self.policy.connect_timeout,
            "read" => self.policy.read_timeout,
            "write" => self.policy.write_timeout,
            "request" => self.policy.request_timeout,
            _ => self.policy.default_timeout,
        }
    }

    /// Check if operation exceeded timeout
    pub fn is_timeout_exceeded(
        &self,
        op_type: &str,
        elapsed: Duration,
    ) -> bool {
        elapsed > self.timeout_for(op_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeout_policy() {
        let policy = TimeoutPolicy::uniform(Duration::from_secs(30));
        assert_eq!(policy.default_timeout.as_secs(), 30);
        assert_eq!(policy.connect_timeout.as_secs(), 30);
    }

    #[test]
    fn test_timeout_enforcer() {
        let policy = TimeoutPolicy::strict();
        let enforcer = TimeoutEnforcer::new(policy);

        let connect_timeout = enforcer.timeout_for("connect");
        assert_eq!(connect_timeout.as_secs(), 2);

        let elapsed = Duration::from_secs(3);
        assert!(enforcer.is_timeout_exceeded("connect", elapsed));
    }

    #[test]
    fn test_http_defaults() {
        let policy = TimeoutPolicy::http_defaults();
        assert_eq!(policy.request_timeout.as_secs(), 60);
    }
}
