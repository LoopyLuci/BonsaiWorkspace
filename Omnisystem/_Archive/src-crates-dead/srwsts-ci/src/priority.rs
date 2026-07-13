//! Test priority adaptation with AI advisor suggestions

use crate::errors::CIResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

/// Test priority level
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum TestPriority {
    Low = 1,
    Medium = 2,
    High = 3,
    Critical = 4,
}

/// Test with priority score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrioritizedTest {
    pub test_name: String,
    pub priority: TestPriority,
    pub regression_risk: f64,  // 0-1, predicted risk of regression
    pub historical_failure_rate: f64,
    pub execution_time_ms: u64,
    pub ai_suggested: bool,
}

/// Priority advisor using historical data
pub struct PriorityAdvisor {
    test_history: dashmap::DashMap<String, TestHistory>,
    failure_patterns: dashmap::DashMap<String, FailurePattern>,
}

/// Historical test data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestHistory {
    pub test_name: String,
    pub total_runs: u64,
    pub failure_count: u64,
    pub avg_duration_ms: u64,
    pub last_failed: Option<chrono::DateTime<chrono::Utc>>,
    pub consecutive_passes: u64,
}

/// Failure pattern learned from history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailurePattern {
    pub pattern_id: String,
    pub test_name: String,
    pub failure_reason: String,
    pub frequency: u64,        // How many times seen
    pub related_metrics: Vec<String>,
    pub recovery_time_hours: u64,
}

impl PriorityAdvisor {
    /// Create new advisor
    pub fn new() -> Self {
        Self {
            test_history: dashmap::DashMap::new(),
            failure_patterns: dashmap::DashMap::new(),
        }
    }

    /// Record test result
    pub fn record_test_result(
        &self,
        test_name: &str,
        passed: bool,
        duration_ms: u64,
    ) -> CIResult<()> {
        let mut history = self.test_history
            .entry(test_name.to_string())
            .or_insert_with(|| TestHistory {
                test_name: test_name.to_string(),
                total_runs: 0,
                failure_count: 0,
                avg_duration_ms: 0,
                last_failed: None,
                consecutive_passes: 0,
            });

        history.total_runs += 1;
        if !passed {
            history.failure_count += 1;
            history.last_failed = Some(chrono::Utc::now());
            history.consecutive_passes = 0;
        } else {
            history.consecutive_passes += 1;
        }

        // Update average duration
        let old_avg = history.avg_duration_ms;
        history.avg_duration_ms =
            (old_avg * (history.total_runs - 1) + duration_ms) / history.total_runs;

        Ok(())
    }

    /// Get recommended test priority
    pub fn recommend_priority(&self, test_name: &str) -> TestPriority {
        if let Some(history) = self.test_history.get(test_name) {
            if history.total_runs == 0 {
                return TestPriority::Medium;
            }

            let failure_rate = history.failure_count as f64 / history.total_runs as f64;

            if failure_rate > 0.3 || history.last_failed.is_some() {
                TestPriority::Critical
            } else if failure_rate > 0.1 {
                TestPriority::High
            } else if failure_rate > 0.02 {
                TestPriority::Medium
            } else {
                TestPriority::Low
            }
        } else {
            TestPriority::Medium
        }
    }

    /// Calculate regression risk (0-1)
    pub fn calculate_regression_risk(&self, test_name: &str) -> f64 {
        if let Some(history) = self.test_history.get(test_name) {
            if history.total_runs == 0 {
                return 0.5;
            }

            let failure_rate = history.failure_count as f64 / history.total_runs as f64;
            let recency_factor = if let Some(last_failed) = history.last_failed {
                let age_hours = (chrono::Utc::now() - last_failed).num_hours() as f64;
                (1.0 / (1.0 + (age_hours / 24.0))).max(0.1)
            } else {
                0.1
            };

            failure_rate * recency_factor
        } else {
            0.5
        }
    }

    /// Get prioritized test list
    pub fn prioritize_tests(&self, tests: &[String]) -> Vec<PrioritizedTest> {
        let mut prioritized: Vec<PrioritizedTest> = tests
            .iter()
            .map(|test_name| {
                let priority = self.recommend_priority(test_name);
                let regression_risk = self.calculate_regression_risk(test_name);

                let history = self.test_history.get(test_name);
                let (failure_rate, execution_time) = history
                    .map(|h| {
                        let rate = if h.total_runs > 0 {
                            h.failure_count as f64 / h.total_runs as f64
                        } else {
                            0.0
                        };
                        (rate, h.avg_duration_ms)
                    })
                    .unwrap_or((0.0, 0));

                PrioritizedTest {
                    test_name: test_name.clone(),
                    priority,
                    regression_risk,
                    historical_failure_rate: failure_rate,
                    execution_time_ms: execution_time,
                    ai_suggested: regression_risk > 0.3,
                }
            })
            .collect();

        // Sort by priority (descending) then by regression risk
        prioritized.sort_by(|a, b| {
            if a.priority != b.priority {
                b.priority.cmp(&a.priority)
            } else {
                b.regression_risk.partial_cmp(&a.regression_risk).unwrap()
            }
        });

        info!(
            "Prioritized {} tests, {} AI-suggested",
            prioritized.len(),
            prioritized.iter().filter(|t| t.ai_suggested).count()
        );

        prioritized
    }

    /// Record failure pattern
    pub fn record_failure_pattern(
        &self,
        test_name: &str,
        reason: &str,
        related_metrics: Vec<String>,
        recovery_hours: u64,
    ) -> CIResult<()> {
        let pattern_id = uuid::Uuid::new_v4().to_string();

        let pattern = FailurePattern {
            pattern_id,
            test_name: test_name.to_string(),
            failure_reason: reason.to_string(),
            frequency: 1,
            related_metrics,
            recovery_time_hours: recovery_hours,
        };

        self.failure_patterns.insert(pattern.pattern_id.clone(), pattern);
        Ok(())
    }

    /// Find common failure patterns
    pub fn find_failure_patterns(&self) -> Vec<(String, u64)> {
        let mut patterns: HashMap<String, u64> = HashMap::new();

        for entry in self.failure_patterns.iter() {
            let pattern = entry.value();
            *patterns
                .entry(pattern.failure_reason.clone())
                .or_insert(0) += 1;
        }

        let mut sorted: Vec<_> = patterns.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));

        sorted
    }

    /// Get test history
    pub fn get_history(&self, test_name: &str) -> Option<TestHistory> {
        self.test_history.get(test_name).map(|h| h.clone())
    }

    /// Get all test priorities
    pub fn get_all_priorities(&self) -> Vec<(String, TestPriority)> {
        self.test_history
            .iter()
            .map(|entry| {
                let test_name = entry.key().clone();
                let priority = self.recommend_priority(&test_name);
                (test_name, priority)
            })
            .collect()
    }

    /// Estimate execution time for test set
    pub fn estimate_execution_time(&self, tests: &[String]) -> u64 {
        tests
            .iter()
            .map(|test| {
                self.test_history
                    .get(test)
                    .map(|h| h.avg_duration_ms)
                    .unwrap_or(1000)
            })
            .sum()
    }
}

impl Default for PriorityAdvisor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_advisor_creation() {
        let advisor = PriorityAdvisor::new();
        assert_eq!(advisor.test_history.len(), 0);
    }

    #[test]
    fn test_record_test_result() {
        let advisor = PriorityAdvisor::new();

        advisor.record_test_result("test_a", true, 100).unwrap();
        advisor.record_test_result("test_a", true, 120).unwrap();
        advisor.record_test_result("test_a", false, 110).unwrap();

        let history = advisor.get_history("test_a").unwrap();
        assert_eq!(history.total_runs, 3);
        assert_eq!(history.failure_count, 1);
    }

    #[test]
    fn test_recommend_priority_high_failure_rate() {
        let advisor = PriorityAdvisor::new();

        for _ in 0..10 {
            advisor.record_test_result("flaky_test", false, 100).unwrap();
        }

        let priority = advisor.recommend_priority("flaky_test");
        assert_eq!(priority, TestPriority::Critical);
    }

    #[test]
    fn test_recommend_priority_low_failure_rate() {
        let advisor = PriorityAdvisor::new();

        for _ in 0..100 {
            advisor.record_test_result("stable_test", true, 100).unwrap();
        }
        advisor.record_test_result("stable_test", false, 100).unwrap();

        let priority = advisor.recommend_priority("stable_test");
        assert!(priority <= TestPriority::Medium);
    }

    #[test]
    fn test_prioritize_tests() {
        let advisor = PriorityAdvisor::new();

        // Create test scenarios
        for _ in 0..5 {
            advisor.record_test_result("critical_test", false, 100).unwrap();
        }

        for _ in 0..100 {
            advisor.record_test_result("stable_test", true, 100).unwrap();
        }

        let tests = vec![
            "critical_test".to_string(),
            "stable_test".to_string(),
        ];

        let prioritized = advisor.prioritize_tests(&tests);
        assert_eq!(prioritized[0].test_name, "critical_test");
        assert_eq!(prioritized[1].test_name, "stable_test");
    }

    #[test]
    fn test_calculate_regression_risk() {
        let advisor = PriorityAdvisor::new();

        advisor.record_test_result("test_a", true, 100).unwrap();
        let risk1 = advisor.calculate_regression_risk("test_a");

        for _ in 0..10 {
            advisor.record_test_result("test_a", false, 100).unwrap();
        }

        let risk2 = advisor.calculate_regression_risk("test_a");
        assert!(risk2 > risk1);
    }

    #[test]
    fn test_record_failure_pattern() {
        let advisor = PriorityAdvisor::new();

        advisor
            .record_failure_pattern(
                "test_a",
                "Timeout on database connection",
                vec!["latency_p99".to_string()],
                2,
            )
            .unwrap();

        assert_eq!(advisor.failure_patterns.len(), 1);
    }

    #[test]
    fn test_find_failure_patterns() {
        let advisor = PriorityAdvisor::new();

        advisor
            .record_failure_pattern("test_a", "Timeout", vec![], 1)
            .unwrap();
        advisor
            .record_failure_pattern("test_b", "Timeout", vec![], 1)
            .unwrap();
        advisor
            .record_failure_pattern("test_c", "AssertionError", vec![], 1)
            .unwrap();

        let patterns = advisor.find_failure_patterns();
        assert!(patterns[0].1 > 1); // Timeout appears more than once
    }

    #[test]
    fn test_estimate_execution_time() {
        let advisor = PriorityAdvisor::new();

        advisor.record_test_result("test_a", true, 100).unwrap();
        advisor.record_test_result("test_b", true, 200).unwrap();

        let total = advisor.estimate_execution_time(&[
            "test_a".to_string(),
            "test_b".to_string(),
        ]);

        assert!(total >= 300);
    }

    #[test]
    fn test_get_all_priorities() {
        let advisor = PriorityAdvisor::new();

        for _ in 0..5 {
            advisor.record_test_result("critical", false, 100).unwrap();
        }

        for _ in 0..10 {
            advisor.record_test_result("normal", true, 100).unwrap();
        }

        let priorities = advisor.get_all_priorities();
        assert_eq!(priorities.len(), 2);
    }

    #[test]
    fn test_test_priority_ordering() {
        assert!(TestPriority::Critical > TestPriority::High);
        assert!(TestPriority::High > TestPriority::Medium);
        assert!(TestPriority::Medium > TestPriority::Low);
    }

    #[test]
    fn test_ai_suggested_flag() {
        let advisor = PriorityAdvisor::new();

        // High regression risk
        for _ in 0..5 {
            advisor.record_test_result("risky", false, 100).unwrap();
        }

        let tests = advisor.prioritize_tests(&["risky".to_string()]);
        assert!(tests[0].ai_suggested);
    }

    #[test]
    fn test_prioritized_test_serialization() {
        let test = PrioritizedTest {
            test_name: "test_a".to_string(),
            priority: TestPriority::High,
            regression_risk: 0.75,
            historical_failure_rate: 0.3,
            execution_time_ms: 150,
            ai_suggested: true,
        };

        let json = serde_json::to_string(&test).unwrap();
        let deserialized: PrioritizedTest = serde_json::from_str(&json).unwrap();

        assert_eq!(test.test_name, deserialized.test_name);
        assert_eq!(test.priority, deserialized.priority);
    }
}
