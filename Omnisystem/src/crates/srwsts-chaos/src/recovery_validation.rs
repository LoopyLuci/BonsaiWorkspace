//! Fault recovery validation and measurement.
//!
//! Measures and validates recovery from injected faults:
//! - Time to detect fault
//! - Time to recover
//! - Data loss verification
//! - Consistency violations

use serde::{Deserialize, Serialize};
use tracing::{debug, warn};
use uuid::Uuid;

/// Recovery metrics for a single fault.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryMetrics {
    /// Fault ID this metrics track.
    pub fault_id: Uuid,
    /// Time from injection to detection (millis).
    pub detection_time_ms: u64,
    /// Time from detection to recovery (millis).
    pub recovery_time_ms: u64,
    /// Total time from injection to full recovery (millis).
    pub total_time_ms: u64,
    /// Whether recovery was successful (no data loss).
    pub successful: bool,
    /// Number of failures during recovery.
    pub failure_count: u64,
    /// Number of consistency violations detected.
    pub consistency_violations: u64,
    /// Data integrity result.
    pub data_integrity_ok: bool,
    /// Optional detailed failure reason.
    pub failure_reason: Option<String>,
}

impl RecoveryMetrics {
    /// Create new recovery metrics.
    pub fn new(fault_id: Uuid) -> Self {
        Self {
            fault_id,
            detection_time_ms: 0,
            recovery_time_ms: 0,
            total_time_ms: 0,
            successful: true,
            failure_count: 0,
            consistency_violations: 0,
            data_integrity_ok: true,
            failure_reason: None,
        }
    }

    /// Mark as successful with timing.
    pub fn mark_successful(
        mut self,
        detection_ms: u64,
        recovery_ms: u64,
    ) -> Self {
        self.detection_time_ms = detection_ms;
        self.recovery_time_ms = recovery_ms;
        self.total_time_ms = detection_ms + recovery_ms;
        self.successful = true;
        self.data_integrity_ok = true;
        debug!(
            "Recovery successful for {:?}: detect={}ms, recover={}ms",
            self.fault_id, detection_ms, recovery_ms
        );
        self
    }

    /// Mark as failed.
    pub fn mark_failed(mut self, reason: String) -> Self {
        self.successful = false;
        self.failure_reason = Some(reason.clone());
        warn!("Recovery failed for {:?}: {}", self.fault_id, reason);
        self
    }

    /// Mark data loss.
    pub fn with_data_loss(mut self) -> Self {
        self.data_integrity_ok = false;
        warn!("Data loss detected for {:?}", self.fault_id);
        self
    }

    /// Add consistency violations.
    pub fn with_consistency_violations(mut self, count: u64) -> Self {
        self.consistency_violations = count;
        if count > 0 {
            warn!("Consistency violations in {:?}: count={}", self.fault_id, count);
        }
        self
    }

    /// Is recovery MTTR acceptable? (< 5 seconds by default).
    pub fn is_mttr_acceptable(&self, max_ms: u64) -> bool {
        self.recovery_time_ms <= max_ms
    }

    /// Is total TTR acceptable? (< 30 seconds by default).
    pub fn is_ttr_acceptable(&self, max_ms: u64) -> bool {
        self.total_time_ms <= max_ms
    }
}

/// Recovery validator for batch analysis.
pub struct RecoveryValidator {
    pub metrics: Vec<RecoveryMetrics>,
}

impl RecoveryValidator {
    /// Create new validator.
    pub fn new() -> Self {
        Self {
            metrics: Vec::new(),
        }
    }

    /// Add recovery metrics.
    pub fn add_metrics(&mut self, metrics: RecoveryMetrics) {
        self.metrics.push(metrics);
    }

    /// Get statistics across all metrics.
    pub fn get_stats(&self) -> RecoveryStats {
        if self.metrics.is_empty() {
            return RecoveryStats::default();
        }

        let total_count = self.metrics.len();
        let successful_count = self.metrics.iter().filter(|m| m.successful).count();
        let data_loss_count = self.metrics.iter().filter(|m| !m.data_integrity_ok).count();
        let consistency_violation_count: u64 =
            self.metrics.iter().map(|m| m.consistency_violations).sum();

        let mut detection_times: Vec<u64> = self.metrics.iter().map(|m| m.detection_time_ms).collect();
        let mut recovery_times: Vec<u64> = self.metrics.iter().map(|m| m.recovery_time_ms).collect();
        let total_times: Vec<u64> = self.metrics.iter().map(|m| m.total_time_ms).collect();

        let avg_detection = detection_times.iter().sum::<u64>() / total_count as u64;
        let avg_recovery = recovery_times.iter().sum::<u64>() / total_count as u64;
        let avg_total = total_times.iter().sum::<u64>() / total_count as u64;

        detection_times.sort_unstable();
        recovery_times.sort_unstable();

        let p99_idx = (total_count * 99) / 100;
        let p99_idx = p99_idx.max(1) - 1;

        RecoveryStats {
            total_faults: total_count,
            successful_recoveries: successful_count,
            failed_recoveries: total_count - successful_count,
            faults_with_data_loss: data_loss_count,
            total_consistency_violations: consistency_violation_count,
            avg_detection_time_ms: avg_detection,
            avg_recovery_time_ms: avg_recovery,
            avg_total_time_ms: avg_total,
            p99_detection_time_ms: detection_times[p99_idx],
            p99_recovery_time_ms: recovery_times[p99_idx],
            success_rate_percent: (successful_count * 100) / total_count,
        }
    }

    /// Validate all metrics pass criteria.
    pub fn validate_all(
        &self,
        require_success: bool,
        max_detection_ms: Option<u64>,
        max_recovery_ms: Option<u64>,
        allow_data_loss: bool,
    ) -> ValidationResult {
        let mut violations = Vec::new();

        for metrics in &self.metrics {
            if require_success && !metrics.successful {
                violations.push(format!("Fault {:?} recovery not successful", metrics.fault_id));
            }

            if !metrics.data_integrity_ok && !allow_data_loss {
                violations.push(format!("Fault {:?} caused data loss", metrics.fault_id));
            }

            if let Some(max) = max_detection_ms {
                if metrics.detection_time_ms > max {
                    violations.push(format!(
                        "Fault {:?} detection time {}ms exceeds max {}ms",
                        metrics.fault_id, metrics.detection_time_ms, max
                    ));
                }
            }

            if let Some(max) = max_recovery_ms {
                if metrics.recovery_time_ms > max {
                    violations.push(format!(
                        "Fault {:?} recovery time {}ms exceeds max {}ms",
                        metrics.fault_id, metrics.recovery_time_ms, max
                    ));
                }
            }

            if metrics.consistency_violations > 0 {
                violations.push(format!(
                    "Fault {:?} caused {} consistency violations",
                    metrics.fault_id, metrics.consistency_violations
                ));
            }
        }

        ValidationResult {
            is_valid: violations.is_empty(),
            violations,
        }
    }

    /// Get detailed report.
    pub fn generate_report(&self) -> String {
        let stats = self.get_stats();
        format!(
            "Recovery Validation Report\n\
             ===========================\n\
             Total Faults: {}\n\
             Successful Recoveries: {} ({}%)\n\
             Failed Recoveries: {}\n\
             Data Loss Incidents: {}\n\
             Consistency Violations: {}\n\
             \n\
             Timing (milliseconds):\n\
             - Avg Detection: {}\n\
             - Avg Recovery: {}\n\
             - Avg Total: {}\n\
             - P99 Detection: {}\n\
             - P99 Recovery: {}\n",
            stats.total_faults,
            stats.successful_recoveries,
            stats.success_rate_percent,
            stats.failed_recoveries,
            stats.faults_with_data_loss,
            stats.total_consistency_violations,
            stats.avg_detection_time_ms,
            stats.avg_recovery_time_ms,
            stats.avg_total_time_ms,
            stats.p99_detection_time_ms,
            stats.p99_recovery_time_ms,
        )
    }
}

impl Default for RecoveryValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics across multiple recovery attempts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryStats {
    /// Total faults tested.
    pub total_faults: usize,
    /// Faults that recovered successfully.
    pub successful_recoveries: usize,
    /// Faults that failed to recover.
    pub failed_recoveries: usize,
    /// Faults that caused data loss.
    pub faults_with_data_loss: usize,
    /// Total consistency violations across all faults.
    pub total_consistency_violations: u64,
    /// Average time to detect fault.
    pub avg_detection_time_ms: u64,
    /// Average time to recover.
    pub avg_recovery_time_ms: u64,
    /// Average total time (detection + recovery).
    pub avg_total_time_ms: u64,
    /// P99 detection time.
    pub p99_detection_time_ms: u64,
    /// P99 recovery time.
    pub p99_recovery_time_ms: u64,
    /// Success rate percentage.
    pub success_rate_percent: usize,
}

impl Default for RecoveryStats {
    fn default() -> Self {
        Self {
            total_faults: 0,
            successful_recoveries: 0,
            failed_recoveries: 0,
            faults_with_data_loss: 0,
            total_consistency_violations: 0,
            avg_detection_time_ms: 0,
            avg_recovery_time_ms: 0,
            avg_total_time_ms: 0,
            p99_detection_time_ms: 0,
            p99_recovery_time_ms: 0,
            success_rate_percent: 0,
        }
    }
}

/// Validation result.
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether validation passed.
    pub is_valid: bool,
    /// List of violations if validation failed.
    pub violations: Vec<String>,
}

impl ValidationResult {
    /// Check if validation passed.
    pub fn passed(&self) -> bool {
        self.is_valid
    }

    /// Get violation count.
    pub fn violation_count(&self) -> usize {
        self.violations.len()
    }

    /// Get formatted report.
    pub fn report(&self) -> String {
        if self.is_valid {
            "✓ All validations passed".to_string()
        } else {
            format!(
                "✗ {} validation failures:\n{}",
                self.violations.len(),
                self.violations
                    .iter()
                    .enumerate()
                    .map(|(i, v)| format!("  {}. {}", i + 1, v))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recovery_metrics() {
        let fault_id = Uuid::new_v4();
        let metrics = RecoveryMetrics::new(fault_id)
            .mark_successful(100, 200)
            .with_consistency_violations(0);

        assert_eq!(metrics.fault_id, fault_id);
        assert_eq!(metrics.detection_time_ms, 100);
        assert_eq!(metrics.recovery_time_ms, 200);
        assert_eq!(metrics.total_time_ms, 300);
        assert!(metrics.successful);
    }

    #[test]
    fn test_recovery_validator() {
        let mut validator = RecoveryValidator::new();

        for i in 0..10 {
            let fault_id = Uuid::new_v4();
            let metrics = RecoveryMetrics::new(fault_id)
                .mark_successful(50 + (i * 10), 100 + (i * 20));
            validator.add_metrics(metrics);
        }

        let stats = validator.get_stats();
        assert_eq!(stats.total_faults, 10);
        assert_eq!(stats.successful_recoveries, 10);
        assert_eq!(stats.failed_recoveries, 0);
    }

    #[test]
    fn test_validation_result() {
        let result = ValidationResult {
            is_valid: true,
            violations: Vec::new(),
        };
        assert!(result.passed());
        assert_eq!(result.violation_count(), 0);
    }
}
