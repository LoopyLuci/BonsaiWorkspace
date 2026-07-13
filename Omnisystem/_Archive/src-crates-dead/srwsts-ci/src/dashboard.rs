//! Health dashboard tracking test pass rates, regression frequency, and MTTR

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

/// Health metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthMetrics {
    pub timestamp: DateTime<Utc>,
    pub pass_rate: f64,                    // percentage 0-100
    pub test_count: usize,
    pub passed_count: usize,
    pub failed_count: usize,
    pub regression_count: usize,
    pub mttr_minutes: Option<u64>,         // mean time to resolution
    pub last_regression_at: Option<DateTime<Utc>>,
    pub consecutive_passes: usize,
    pub regression_severity: RegressionLevel,
}

/// Regression severity level
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum RegressionLevel {
    None,
    Low,
    Medium,
    High,
    Critical,
}

impl From<RegressionLevel> for u8 {
    fn from(level: RegressionLevel) -> Self {
        match level {
            RegressionLevel::None => 0,
            RegressionLevel::Low => 1,
            RegressionLevel::Medium => 2,
            RegressionLevel::High => 3,
            RegressionLevel::Critical => 4,
        }
    }
}

/// Dashboard event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardEvent {
    pub timestamp: DateTime<Utc>,
    pub event_type: EventType,
    pub severity: RegressionLevel,
    pub message: String,
    pub details: HashMap<String, String>,
}

/// Event type
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum EventType {
    RegressionDetected,
    RegressionResolved,
    TestStarted,
    TestCompleted,
    AlertTriggered,
    EscalationTriggered,
}

/// Health dashboard
pub struct HealthDashboard {
    current_health: dashmap::DashMap<String, HealthMetrics>,
    history: dashmap::DashMap<String, Vec<HealthMetrics>>,
    events: dashmap::DashMap<String, Vec<DashboardEvent>>,
}

impl HealthDashboard {
    /// Create new dashboard
    pub fn new() -> Self {
        Self {
            current_health: dashmap::DashMap::new(),
            history: dashmap::DashMap::new(),
            events: dashmap::DashMap::new(),
        }
    }

    /// Record health metrics
    pub fn record_metrics(&self, pipeline_stage: &str, metrics: HealthMetrics) {
        self.current_health.insert(pipeline_stage.to_string(), metrics.clone());

        // Add to history
        let mut history = self.history
            .entry(pipeline_stage.to_string())
            .or_insert_with(Vec::new);
        history.push(metrics.clone());

        info!(
            "Health metrics recorded for {}: pass_rate={:.1}%",
            pipeline_stage, metrics.pass_rate
        );
    }

    /// Record event
    pub fn record_event(&self, pipeline_stage: &str, event: DashboardEvent) {
        let mut events = self.events
            .entry(pipeline_stage.to_string())
            .or_insert_with(Vec::new);
        events.push(event);
    }

    /// Get current health
    pub fn get_health(&self, pipeline_stage: &str) -> Option<HealthMetrics> {
        self.current_health.get(pipeline_stage).map(|m| m.clone())
    }

    /// Get health history
    pub fn get_history(&self, pipeline_stage: &str) -> Vec<HealthMetrics> {
        self.history
            .get(pipeline_stage)
            .map(|h| h.clone())
            .unwrap_or_default()
    }

    /// Get events
    pub fn get_events(&self, pipeline_stage: &str) -> Vec<DashboardEvent> {
        self.events
            .get(pipeline_stage)
            .map(|e| e.clone())
            .unwrap_or_default()
    }

    /// Check if escalation needed (regression not resolved in 24h)
    pub fn needs_escalation(&self, pipeline_stage: &str) -> bool {
        if let Some(health) = self.get_health(pipeline_stage) {
            if let Some(last_regression) = health.last_regression_at {
                let elapsed = Utc::now() - last_regression;
                return elapsed > Duration::hours(24);
            }
        }
        false
    }

    /// Get all stages' health
    pub fn get_all_health(&self) -> Vec<(String, HealthMetrics)> {
        self.current_health
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }

    /// Calculate trend (improvement/degradation)
    pub fn calculate_trend(&self, pipeline_stage: &str) -> Option<TrendDirection> {
        let history = self.get_history(pipeline_stage);
        if history.len() < 2 {
            return None;
        }

        let recent = &history[history.len() - 1];
        let previous = &history[history.len() - 2];

        let trend = if recent.pass_rate > previous.pass_rate {
            TrendDirection::Improving
        } else if recent.pass_rate < previous.pass_rate {
            TrendDirection::Degrading
        } else {
            TrendDirection::Stable
        };

        Some(trend)
    }

    /// Get MTTR for stage
    pub fn get_mttr(&self, pipeline_stage: &str) -> Option<u64> {
        self.get_health(pipeline_stage).and_then(|m| m.mttr_minutes)
    }

    /// Get regression frequency (per day)
    pub fn get_regression_frequency(&self, pipeline_stage: &str) -> f64 {
        let history = self.get_history(pipeline_stage);
        if history.is_empty() {
            return 0.0;
        }

        // Count regressions in last 24 hours
        let now = Utc::now();
        let day_ago = now - Duration::days(1);

        let regressions = history
            .iter()
            .filter(|h| h.timestamp > day_ago && h.regression_count > 0)
            .count();

        regressions as f64 / 1.0 // per day
    }

    /// Check if health is critical
    pub fn is_critical(&self, pipeline_stage: &str) -> bool {
        if let Some(health) = self.get_health(pipeline_stage) {
            health.pass_rate < 90.0
                && health.regression_severity == RegressionLevel::Critical
        } else {
            false
        }
    }
}

impl Default for HealthDashboard {
    fn default() -> Self {
        Self::new()
    }
}

/// Trend direction
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum TrendDirection {
    Improving,
    Degrading,
    Stable,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_metrics_creation() {
        let metrics = HealthMetrics {
            timestamp: Utc::now(),
            pass_rate: 95.5,
            test_count: 100,
            passed_count: 95,
            failed_count: 5,
            regression_count: 2,
            mttr_minutes: Some(30),
            last_regression_at: Some(Utc::now()),
            consecutive_passes: 10,
            regression_severity: RegressionLevel::Low,
        };

        assert_eq!(metrics.pass_rate, 95.5);
        assert_eq!(metrics.passed_count, 95);
    }

    #[test]
    fn test_record_metrics() {
        let dashboard = HealthDashboard::new();

        let metrics = HealthMetrics {
            timestamp: Utc::now(),
            pass_rate: 98.0,
            test_count: 100,
            passed_count: 98,
            failed_count: 2,
            regression_count: 0,
            mttr_minutes: None,
            last_regression_at: None,
            consecutive_passes: 50,
            regression_severity: RegressionLevel::None,
        };

        dashboard.record_metrics("smoke", metrics.clone());

        let retrieved = dashboard.get_health("smoke").unwrap();
        assert_eq!(retrieved.pass_rate, 98.0);
    }

    #[test]
    fn test_health_history() {
        let dashboard = HealthDashboard::new();

        for i in 0..5 {
            let metrics = HealthMetrics {
                timestamp: Utc::now(),
                pass_rate: 90.0 + i as f64,
                test_count: 100,
                passed_count: 90 + i,
                failed_count: 10 - i,
                regression_count: if i > 2 { 1 } else { 0 },
                mttr_minutes: None,
                last_regression_at: None,
                consecutive_passes: i * 10,
                regression_severity: RegressionLevel::None,
            };

            dashboard.record_metrics("smoke", metrics);
        }

        let history = dashboard.get_history("smoke");
        assert_eq!(history.len(), 5);
    }

    #[test]
    fn test_record_event() {
        let dashboard = HealthDashboard::new();

        let event = DashboardEvent {
            timestamp: Utc::now(),
            event_type: EventType::RegressionDetected,
            severity: RegressionLevel::High,
            message: "Performance regression detected".to_string(),
            details: Default::default(),
        };

        dashboard.record_event("smoke", event);

        let events = dashboard.get_events("smoke");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, EventType::RegressionDetected);
    }

    #[test]
    fn test_get_all_health() {
        let dashboard = HealthDashboard::new();

        for stage in &["smoke", "full", "nightly"] {
            let metrics = HealthMetrics {
                timestamp: Utc::now(),
                pass_rate: 95.0,
                test_count: 100,
                passed_count: 95,
                failed_count: 5,
                regression_count: 0,
                mttr_minutes: None,
                last_regression_at: None,
                consecutive_passes: 20,
                regression_severity: RegressionLevel::None,
            };
            dashboard.record_metrics(stage, metrics);
        }

        let all_health = dashboard.get_all_health();
        assert_eq!(all_health.len(), 3);
    }

    #[test]
    fn test_calculate_trend() {
        let dashboard = HealthDashboard::new();

        // Record declining metrics
        for i in 0..3 {
            let metrics = HealthMetrics {
                timestamp: Utc::now(),
                pass_rate: 100.0 - i as f64 * 5.0,
                test_count: 100,
                passed_count: (100 - i * 5) as usize,
                failed_count: (i * 5) as usize,
                regression_count: 0,
                mttr_minutes: None,
                last_regression_at: None,
                consecutive_passes: 20,
                regression_severity: RegressionLevel::None,
            };
            dashboard.record_metrics("smoke", metrics);
        }

        let trend = dashboard.calculate_trend("smoke");
        assert_eq!(trend, Some(TrendDirection::Degrading));
    }

    #[test]
    fn test_needs_escalation() {
        let dashboard = HealthDashboard::new();

        let old_time = Utc::now() - Duration::hours(25);
        let metrics = HealthMetrics {
            timestamp: Utc::now(),
            pass_rate: 80.0,
            test_count: 100,
            passed_count: 80,
            failed_count: 20,
            regression_count: 1,
            mttr_minutes: None,
            last_regression_at: Some(old_time),
            consecutive_passes: 0,
            regression_severity: RegressionLevel::High,
        };

        dashboard.record_metrics("smoke", metrics);
        assert!(dashboard.needs_escalation("smoke"));
    }

    #[test]
    fn test_is_critical() {
        let dashboard = HealthDashboard::new();

        let metrics = HealthMetrics {
            timestamp: Utc::now(),
            pass_rate: 85.0, // Below 90%
            test_count: 100,
            passed_count: 85,
            failed_count: 15,
            regression_count: 3,
            mttr_minutes: Some(120),
            last_regression_at: Some(Utc::now()),
            consecutive_passes: 0,
            regression_severity: RegressionLevel::Critical,
        };

        dashboard.record_metrics("smoke", metrics);
        assert!(dashboard.is_critical("smoke"));
    }

    #[test]
    fn test_get_regression_frequency() {
        let dashboard = HealthDashboard::new();

        for i in 0..5 {
            let metrics = HealthMetrics {
                timestamp: Utc::now(),
                pass_rate: 90.0,
                test_count: 100,
                passed_count: 90,
                failed_count: 10,
                regression_count: if i % 2 == 0 { 1 } else { 0 },
                mttr_minutes: None,
                last_regression_at: None,
                consecutive_passes: 0,
                regression_severity: RegressionLevel::Low,
            };
            dashboard.record_metrics("smoke", metrics);
        }

        let frequency = dashboard.get_regression_frequency("smoke");
        assert!(frequency > 0.0);
    }

    #[test]
    fn test_get_mttr() {
        let dashboard = HealthDashboard::new();

        let metrics = HealthMetrics {
            timestamp: Utc::now(),
            pass_rate: 95.0,
            test_count: 100,
            passed_count: 95,
            failed_count: 5,
            regression_count: 0,
            mttr_minutes: Some(45),
            last_regression_at: None,
            consecutive_passes: 20,
            regression_severity: RegressionLevel::None,
        };

        dashboard.record_metrics("smoke", metrics);
        assert_eq!(dashboard.get_mttr("smoke"), Some(45));
    }

    #[test]
    fn test_regression_level_conversion() {
        assert_eq!(u8::from(RegressionLevel::None), 0);
        assert_eq!(u8::from(RegressionLevel::Critical), 4);
    }
}
