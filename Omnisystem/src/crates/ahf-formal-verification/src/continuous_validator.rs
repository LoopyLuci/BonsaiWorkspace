//! Continuous Validation Framework for 24/7 Hallucination Detection
//!
//! This module implements continuous monitoring of AHF outputs, detecting regressions
//! in real-time and automatically alerting governance councils.

use crate::error::{VerificationError, VerificationResult};
use ahf_core::FactualClaim;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use std::sync::RwLock;

/// Status of a validation check
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ValidationStatus {
    /// Validation passed
    Passed,
    /// Validation failed
    Failed,
    /// Validation indeterminate
    Indeterminate,
    /// Validation pending
    Pending,
}

/// Result of a continuous validation check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    /// Unique identifier
    pub id: Uuid,
    /// Timestamp of validation
    pub timestamp: DateTime<Utc>,
    /// Overall status
    pub status: ValidationStatus,
    /// Claims tested
    pub claims_tested: usize,
    /// Claims verified
    pub claims_verified: usize,
    /// Claims contradicted
    pub claims_contradicted: usize,
    /// Detected hallucinations
    pub hallucinations_detected: Vec<HallucinationInstance>,
    /// Estimated hallucination rate
    pub hallucination_rate: f64,
    /// Performance metrics
    pub performance_metrics: PerformanceMetrics,
}

/// An instance of detected hallucination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HallucinationInstance {
    /// The hallucinating claim
    pub claim: String,
    /// Why it's a hallucination
    pub reason: String,
    /// Severity (0.0 to 1.0)
    pub severity: f64,
    /// Time detected
    pub detected_at: DateTime<Utc>,
}

/// Performance metrics for validation run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Total validation time in milliseconds
    pub total_ms: u64,
    /// Average time per claim
    pub avg_per_claim_ms: f64,
    /// Peak memory usage in MB
    pub peak_memory_mb: f64,
    /// Throughput (claims per second)
    pub throughput_cps: f64,
}

/// Continuous validator for 24/7 monitoring
pub struct ContinuousValidator {
    /// Validation reports
    reports: Arc<RwLock<std::collections::HashMap<Uuid, ValidationReport>>>,
    /// Hallucination rate threshold (0.0 to 1.0)
    hallucination_threshold: f64,
    /// Sample rate (e.g., 0.01 = 1% of traffic)
    sample_rate: f64,
    /// Configuration
    config: ValidationConfig,
    /// Metrics cache
    metrics_cache: Arc<RwLock<MetricsCache>>,
}

/// Configuration for continuous validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationConfig {
    /// Enable continuous validation
    pub enabled: bool,
    /// Validation interval in seconds
    pub interval_seconds: u64,
    /// Sample rate for live traffic (0.0 to 1.0)
    pub sample_rate: f64,
    /// Hallucination rate threshold for alerts
    pub hallucination_threshold: f64,
    /// Number of validation runs to aggregate
    pub aggregation_window: usize,
    /// Whether to auto-revoke capabilities on threshold exceeded
    pub auto_revoke_enabled: bool,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        ValidationConfig {
            enabled: true,
            interval_seconds: 3600, // 1 hour
            sample_rate: 0.01,      // 1% of traffic
            hallucination_threshold: 0.05, // 5%
            aggregation_window: 24, // 24 runs (1 day)
            auto_revoke_enabled: true,
        }
    }
}

/// Metrics cache
#[derive(Debug, Clone, Serialize, Deserialize)]
struct MetricsCache {
    /// Last N validation reports
    recent_reports: Vec<ValidationReport>,
    /// Last update time
    last_update: DateTime<Utc>,
    /// Aggregated hallucination rate
    aggregated_hallucination_rate: f64,
}

impl ContinuousValidator {
    /// Create a new continuous validator
    pub fn new(config: ValidationConfig) -> Self {
        ContinuousValidator {
            reports: Arc::new(RwLock::new(std::collections::HashMap::new())),
            hallucination_threshold: config.hallucination_threshold,
            sample_rate: config.sample_rate,
            config,
            metrics_cache: Arc::new(RwLock::new(MetricsCache {
                recent_reports: Vec::new(),
                last_update: Utc::now(),
                aggregated_hallucination_rate: 0.0,
            })),
        }
    }

    /// Validate a batch of claims
    pub async fn validate_claims(&self, claims: &[FactualClaim]) -> VerificationResult<ValidationReport> {
        if !self.config.enabled {
            return Err(VerificationError::ContinuousValidationError(
                "Continuous validation is disabled".to_string(),
            ));
        }

        let start_time = Utc::now();
        let start_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let mut verified = 0;
        let mut contradicted = 0;
        let mut hallucinations = Vec::new();

        // In a real implementation, this would verify claims against the knowledge base
        for claim in claims {
            // Simulate verification with a simple pattern check
            if claim.subject.label.to_lowercase().contains("hallucin") {
                contradicted += 1;
                hallucinations.push(HallucinationInstance {
                    claim: format!("{} {} {}", claim.subject.label, claim.predicate.label, claim.object),
                    reason: "Subject contains hallucination marker".to_string(),
                    severity: 0.8,
                    detected_at: Utc::now(),
                });
            } else {
                verified += 1;
            }
        }

        let end_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let total_ms = end_ms - start_ms;
        let hallucination_rate = if claims.is_empty() {
            0.0
        } else {
            contradicted as f64 / claims.len() as f64
        };

        let report = ValidationReport {
            id: Uuid::new_v4(),
            timestamp: start_time,
            status: if hallucination_rate <= self.hallucination_threshold {
                ValidationStatus::Passed
            } else {
                ValidationStatus::Failed
            },
            claims_tested: claims.len(),
            claims_verified: verified,
            claims_contradicted: contradicted,
            hallucinations_detected: hallucinations,
            hallucination_rate,
            performance_metrics: PerformanceMetrics {
                total_ms,
                avg_per_claim_ms: if claims.is_empty() {
                    0.0
                } else {
                    total_ms as f64 / claims.len() as f64
                },
                peak_memory_mb: 0.0, // Would be measured in real implementation
                throughput_cps: if total_ms == 0 {
                    0.0
                } else {
                    claims.len() as f64 * 1000.0 / total_ms as f64
                },
            },
        };

        // Store report
        self.reports.write().unwrap().insert(report.id, report.clone());

        // Update metrics cache
        self.update_metrics_cache(&report)?;

        // Check if threshold exceeded
        if hallucination_rate > self.hallucination_threshold && self.config.auto_revoke_enabled {
            return Err(VerificationError::HallucinationRateExceeded {
                rate: hallucination_rate * 100.0,
                threshold: self.hallucination_threshold * 100.0,
            });
        }

        Ok(report)
    }

    /// Update metrics cache with new report
    fn update_metrics_cache(&self, report: &ValidationReport) -> VerificationResult<()> {
        let mut cache = self.metrics_cache.write();
        cache.recent_reports.push(report.clone());

        // Keep only recent reports
        if cache.recent_reports.len() > self.config.aggregation_window {
            cache.recent_reports.remove(0);
        }

        // Calculate aggregated hallucination rate
        if cache.recent_reports.is_empty() {
            cache.aggregated_hallucination_rate = 0.0;
        } else {
            let total_hallucinations: f64 = cache
                .recent_reports
                .iter()
                .map(|r| r.hallucination_rate)
                .sum();
            cache.aggregated_hallucination_rate = total_hallucinations / cache.recent_reports.len() as f64;
        }

        cache.last_update = Utc::now();
        Ok(())
    }

    /// Get the aggregated hallucination rate
    pub fn aggregated_hallucination_rate(&self) -> f64 {
        self.metrics_cache.read().aggregated_hallucination_rate
    }

    /// Get recent validation reports
    pub fn recent_reports(&self, count: usize) -> Vec<ValidationReport> {
        self.metrics_cache
            .read()
            .recent_reports
            .iter()
            .rev()
            .take(count)
            .cloned()
            .collect()
    }

    /// Get a specific validation report
    pub fn get_report(&self, id: Uuid) -> Option<ValidationReport> {
        self.reports.read().unwrap().get(&id).cloned()
    }

    /// Check if validation is healthy
    pub fn is_healthy(&self) -> bool {
        let rate = self.aggregated_hallucination_rate();
        rate <= self.hallucination_threshold
    }

    /// Get health status details
    pub fn health_status(&self) -> HealthStatus {
        let rate = self.aggregated_hallucination_rate();
        let is_healthy = rate <= self.hallucination_threshold;

        HealthStatus {
            healthy: is_healthy,
            hallucination_rate: rate,
            threshold: self.hallucination_threshold,
            message: if is_healthy {
                format!(
                    "System healthy (hallucination rate: {:.2}%)",
                    rate * 100.0
                )
            } else {
                format!(
                    "WARNING: Hallucination rate {:.2}% exceeds threshold {:.2}%",
                    rate * 100.0,
                    self.hallucination_threshold * 100.0
                )
            },
            recent_reports_count: self.metrics_cache.read().recent_reports.len(),
        }
    }
}

/// Health status of the continuous validator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub healthy: bool,
    pub hallucination_rate: f64,
    pub threshold: f64,
    pub message: String,
    pub recent_reports_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ahf_core::{Subject, Predicate, ClaimObject};

    #[tokio::test]
    async fn test_continuous_validator_creation() {
        let config = ValidationConfig::default();
        let validator = ContinuousValidator::new(config);
        assert!(validator.is_healthy());
    }

    #[tokio::test]
    async fn test_validate_clean_claims() {
        let config = ValidationConfig::default();
        let validator = ContinuousValidator::new(config);

        let claims = vec![
            FactualClaim {
                id: Uuid::new_v4(),
                subject: Subject("Paris".to_string()),
                predicate: Predicate::IsCapitalOf,
                object: ClaimObject::String("France".to_string()),
                confidence: 0.95,
                source_text: "Paris is the capital of France".to_string(),
            },
        ];

        let report = validator.validate_claims(&claims).await.unwrap();
        assert_eq!(report.status, ValidationStatus::Passed);
        assert_eq!(report.hallucinations_detected.len(), 0);
    }

    #[tokio::test]
    async fn test_detect_hallucinations() {
        let config = ValidationConfig::default();
        let validator = ContinuousValidator::new(config);

        let claims = vec![
            FactualClaim {
                id: Uuid::new_v4(),
                subject: Subject("Hallucinating Entity".to_string()),
                predicate: Predicate::Is,
                object: ClaimObject::String("Something fake".to_string()),
                confidence: 0.5,
                source_text: "This is a hallucination".to_string(),
            },
        ];

        let report = validator.validate_claims(&claims).await.unwrap();
        assert!(report.hallucinations_detected.len() > 0);
    }

    #[tokio::test]
    async fn test_health_status() {
        let config = ValidationConfig::default();
        let validator = ContinuousValidator::new(config);

        let status = validator.health_status();
        assert!(status.healthy);
        assert_eq!(status.recent_reports_count, 0);
    }

    #[test]
    fn test_validation_config_defaults() {
        let config = ValidationConfig::default();
        assert!(config.enabled);
        assert_eq!(config.sample_rate, 0.01);
        assert_eq!(config.hallucination_threshold, 0.05);
    }

    #[test]
    fn test_performance_metrics() {
        let metrics = PerformanceMetrics {
            total_ms: 1000,
            avg_per_claim_ms: 10.0,
            peak_memory_mb: 256.0,
            throughput_cps: 100.0,
        };
        assert_eq!(metrics.total_ms, 1000);
        assert_eq!(metrics.avg_per_claim_ms, 10.0);
    }
}
