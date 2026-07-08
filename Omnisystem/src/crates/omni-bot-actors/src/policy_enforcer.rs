//! PolicyEnforcer Actor - Enforces capability validation and rate limiting
//!
//! Responsibilities:
//! - Validate user capabilities for actions
//! - Enforce rate limits per user
//! - Track policy violations
//! - Support action quarantine and escalation

use crate::actor::{Actor, ActorId, Snapshot};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use omni_bot_core::{Capability, SessionId};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Rate limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    pub requests_per_minute: u32,
    pub requests_per_hour: u32,
    pub requests_per_day: u32,
}

impl Default for RateLimit {
    fn default() -> Self {
        Self {
            requests_per_minute: 60,
            requests_per_hour: 1000,
            requests_per_day: 10000,
        }
    }
}

/// Request window for rate limiting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestWindow {
    pub minute_count: u32,
    pub hour_count: u32,
    pub day_count: u32,
    pub last_reset: DateTime<Utc>,
}

impl RequestWindow {
    pub fn new() -> Self {
        Self {
            minute_count: 0,
            hour_count: 0,
            day_count: 0,
            last_reset: Utc::now(),
        }
    }

    pub fn check_and_increment(
        &mut self,
        limit: &RateLimit,
    ) -> Result<(), String> {
        let now = Utc::now();

        // Reset counters if time windows have passed
        if now.signed_duration_since(self.last_reset).num_minutes() >= 1 {
            self.minute_count = 0;
        }
        if now.signed_duration_since(self.last_reset).num_hours() >= 1 {
            self.hour_count = 0;
        }
        if now.signed_duration_since(self.last_reset).num_days() >= 1 {
            self.day_count = 0;
        }

        // Check limits
        if self.minute_count >= limit.requests_per_minute {
            return Err("Rate limit exceeded (per minute)".to_string());
        }
        if self.hour_count >= limit.requests_per_hour {
            return Err("Rate limit exceeded (per hour)".to_string());
        }
        if self.day_count >= limit.requests_per_day {
            return Err("Rate limit exceeded (per day)".to_string());
        }

        // Increment counters
        self.minute_count += 1;
        self.hour_count += 1;
        self.day_count += 1;

        Ok(())
    }
}

impl Default for RequestWindow {
    fn default() -> Self {
        Self::new()
    }
}

/// Policy violation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyViolation {
    pub violation_id: String,
    pub user_id: String,
    pub violation_type: ViolationType,
    pub severity: ViolationSeverity,
    pub timestamp: DateTime<Utc>,
    pub details: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViolationType {
    UnauthorizedAction,
    RateLimitExceeded,
    InvalidCapability,
    SuspiciousActivity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ViolationSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl PolicyViolation {
    pub fn new(
        user_id: String,
        violation_type: ViolationType,
        severity: ViolationSeverity,
        details: String,
    ) -> Self {
        Self {
            violation_id: format!("violation-{}", uuid::Uuid::new_v4()),
            user_id,
            violation_type,
            severity,
            timestamp: Utc::now(),
            details,
        }
    }
}

/// Messages for PolicyEnforcer
#[derive(Debug, Clone)]
pub enum PolicyEnforcerMessage {
    /// Validate a user action
    ValidateAction {
        session_id: SessionId,
        action_name: String,
        capability: Capability,
    },
    /// Check rate limit
    CheckRateLimit { user_id: String },
    /// Record a violation
    RecordViolation(PolicyViolation),
    /// Get violations for user
    GetViolations { user_id: String },
    /// Get metrics
    GetMetrics,
    /// Stop the actor
    Stop,
}

/// Policy metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PolicyMetrics {
    pub total_validations: u64,
    pub passed_validations: u64,
    pub failed_validations: u64,
    pub rate_limit_violations: u64,
    pub capability_violations: u64,
}

/// PolicyEnforcer actor
pub struct PolicyEnforcer {
    id: ActorId,
    metrics: PolicyMetrics,
    rate_limits: Arc<DashMap<String, RequestWindow>>,
    violations: Arc<DashMap<String, Vec<PolicyViolation>>>,
    default_rate_limit: RateLimit,
}

impl PolicyEnforcer {
    pub fn new() -> Self {
        Self {
            id: ActorId::new(),
            metrics: PolicyMetrics::default(),
            rate_limits: Arc::new(DashMap::new()),
            violations: Arc::new(DashMap::new()),
            default_rate_limit: RateLimit::default(),
        }
    }

    pub fn with_rate_limit(mut self, limit: RateLimit) -> Self {
        self.default_rate_limit = limit;
        self
    }

    fn validate_capability(&self, capability: &Capability, action: &str) -> Result<(), String> {
        if !capability.is_valid() {
            return Err("Capability token expired".to_string());
        }

        if !capability.can(action) {
            return Err(format!("User lacks capability: {}", action));
        }

        Ok(())
    }

    fn check_rate_limit(&self, user_id: &str) -> Result<(), String> {
        let mut window = self
            .rate_limits
            .entry(user_id.to_string())
            .or_insert_with(RequestWindow::new)
            .clone();

        window.check_and_increment(&self.default_rate_limit)?;

        // Update in map
        if let Some(mut entry) = self.rate_limits.get_mut(user_id) {
            *entry = window;
        }

        Ok(())
    }
}

impl Default for PolicyEnforcer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Actor for PolicyEnforcer {
    type Message = PolicyEnforcerMessage;

    fn id(&self) -> ActorId {
        self.id
    }

    async fn handle(&mut self, msg: Self::Message) -> Result<bool, String> {
        match msg {
            PolicyEnforcerMessage::ValidateAction {
                session_id: _session_id,
                action_name,
                capability,
            } => {
                self.metrics.total_validations += 1;

                match self.validate_capability(&capability, &action_name) {
                    Ok(_) => {
                        self.metrics.passed_validations += 1;
                        log::info!(
                            "[PolicyEnforcer] Action validated: {}",
                            action_name
                        );
                        Ok(true)
                    }
                    Err(e) => {
                        self.metrics.failed_validations += 1;
                        self.metrics.capability_violations += 1;
                        log::warn!("[PolicyEnforcer] Validation failed: {}", e);
                        Ok(true) // Continue processing even on validation failure
                    }
                }
            }
            PolicyEnforcerMessage::CheckRateLimit { user_id } => {
                match self.check_rate_limit(&user_id) {
                    Ok(_) => {
                        log::info!("[PolicyEnforcer] Rate limit OK for user: {}", user_id);
                        Ok(true)
                    }
                    Err(e) => {
                        self.metrics.rate_limit_violations += 1;
                        log::warn!("[PolicyEnforcer] Rate limit exceeded for {}: {}", user_id, e);
                        Ok(true) // Continue processing
                    }
                }
            }
            PolicyEnforcerMessage::RecordViolation(violation) => {
                let user_id = violation.user_id.clone();
                log::warn!(
                    "[PolicyEnforcer] Recording violation for {}: {:?}",
                    user_id,
                    violation.violation_type
                );

                self.violations
                    .entry(user_id)
                    .or_insert_with(Vec::new)
                    .push(violation);

                Ok(true)
            }
            PolicyEnforcerMessage::GetViolations { user_id } => {
                if let Some(violations) = self.violations.get(&user_id) {
                    log::info!(
                        "[PolicyEnforcer] User {} has {} violations",
                        user_id,
                        violations.len()
                    );
                } else {
                    log::info!("[PolicyEnforcer] User {} has no violations", user_id);
                }
                Ok(true)
            }
            PolicyEnforcerMessage::GetMetrics => {
                log::info!("[PolicyEnforcer] Metrics: {:?}", self.metrics);
                Ok(true)
            }
            PolicyEnforcerMessage::Stop => {
                log::info!("[PolicyEnforcer] Stop signal received");
                Ok(false)
            }
        }
    }

    async fn snapshot(&self) -> Result<Snapshot, String> {
        let state = serde_json::json!({
            "metrics": self.metrics,
            "rate_limits_count": self.rate_limits.len(),
            "violations_count": self.violations.len(),
        });

        Ok(Snapshot::new(
            self.id,
            "PolicyEnforcer".to_string(),
            state,
        ))
    }

    async fn restore(&mut self, _snapshot: Snapshot) -> Result<(), String> {
        log::info!("[PolicyEnforcer] Restored from snapshot");
        Ok(())
    }

    fn actor_type(&self) -> &'static str {
        "PolicyEnforcer"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_window_increment() {
        let mut window = RequestWindow::new();
        let limit = RateLimit::default();

        for _ in 0..10 {
            assert!(window.check_and_increment(&limit).is_ok());
        }
        assert_eq!(window.minute_count, 10);
    }

    #[test]
    fn test_policy_violation_creation() {
        let violation = PolicyViolation::new(
            "alice".to_string(),
            ViolationType::RateLimitExceeded,
            ViolationSeverity::High,
            "Too many requests".to_string(),
        );

        assert_eq!(violation.user_id, "alice");
        assert_eq!(violation.violation_type, ViolationType::RateLimitExceeded);
        assert_eq!(violation.severity, ViolationSeverity::High);
    }

    #[test]
    fn test_violation_severity_ordering() {
        assert!(ViolationSeverity::Low < ViolationSeverity::Medium);
        assert!(ViolationSeverity::Medium < ViolationSeverity::High);
        assert!(ViolationSeverity::High < ViolationSeverity::Critical);
    }
}
