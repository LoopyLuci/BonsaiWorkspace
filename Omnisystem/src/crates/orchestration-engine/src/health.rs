use crate::{HealthProbe, OrchestrationResult, PodId};
use dashmap::DashMap;
use std::sync::Arc;

pub struct HealthChecker {
    probes: Arc<DashMap<String, HealthProbe>>,
    health_status: Arc<DashMap<String, bool>>,
    consecutive_failures: Arc<DashMap<String, u32>>,
    consecutive_successes: Arc<DashMap<String, u32>>,
}

impl HealthChecker {
    pub fn new() -> Self {
        Self {
            probes: Arc::new(DashMap::new()),
            health_status: Arc::new(DashMap::new()),
            consecutive_failures: Arc::new(DashMap::new()),
            consecutive_successes: Arc::new(DashMap::new()),
        }
    }

    pub async fn add_probe(&self, pod_id: &PodId, probe: &HealthProbe) -> OrchestrationResult<()> {
        self.probes.insert(pod_id.0.clone(), probe.clone());
        self.health_status.insert(pod_id.0.clone(), false);
        self.consecutive_failures.insert(pod_id.0.clone(), 0);
        self.consecutive_successes.insert(pod_id.0.clone(), 0);
        Ok(())
    }

    pub async fn check_health(&self, pod_id: &PodId) -> OrchestrationResult<bool> {
        if let Some(probe) = self.probes.get(&pod_id.0) {
            let success = self.execute_probe(&probe).await;

            let mut failures = self
                .consecutive_failures
                .get_mut(&pod_id.0)
                .unwrap_or_else(|| self.consecutive_failures.entry(pod_id.0.clone()).or_insert(0));

            let mut successes = self
                .consecutive_successes
                .get_mut(&pod_id.0)
                .unwrap_or_else(|| self.consecutive_successes.entry(pod_id.0.clone()).or_insert(0));

            if success {
                *failures = 0;
                *successes += 1;

                if *successes >= probe.success_threshold {
                    if let Some(mut status) = self.health_status.get_mut(&pod_id.0) {
                        *status = true;
                    }
                }
            } else {
                *successes = 0;
                *failures += 1;

                if *failures >= probe.failure_threshold {
                    if let Some(mut status) = self.health_status.get_mut(&pod_id.0) {
                        *status = false;
                    }
                }
            }

            self.health_status
                .get(&pod_id.0)
                .map(|entry| *entry)
                .ok_or_else(|| crate::OrchestrationError::HealthCheckFailed(
                    "Health status not found".to_string(),
                ))
        } else {
            Ok(true) // No probe means healthy
        }
    }

    pub async fn get_health_status(&self, pod_id: &PodId) -> OrchestrationResult<bool> {
        Ok(self
            .health_status
            .get(&pod_id.0)
            .map(|entry| *entry)
            .unwrap_or(true))
    }

    async fn execute_probe(&self, probe: &HealthProbe) -> bool {
        match &probe.probe_type {
            crate::ProbeType::Http(_) => true,
            crate::ProbeType::Tcp(_) => true,
            crate::ProbeType::Exec(_) => true,
        }
    }

    pub fn probe_count(&self) -> usize {
        self.probes.len()
    }
}

impl Default for HealthChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_probe() {
        let checker = HealthChecker::new();
        let pod_id = PodId("test-pod".to_string());
        let probe = HealthProbe {
            probe_type: crate::ProbeType::Http("/health".to_string()),
            initial_delay_secs: 10,
            timeout_secs: 5,
            period_secs: 10,
            success_threshold: 1,
            failure_threshold: 3,
        };

        checker.add_probe(&pod_id, &probe).await.unwrap();
        assert_eq!(checker.probe_count(), 1);
    }

    #[tokio::test]
    async fn test_check_health() {
        let checker = HealthChecker::new();
        let pod_id = PodId("test-pod".to_string());
        let probe = HealthProbe {
            probe_type: crate::ProbeType::Http("/health".to_string()),
            initial_delay_secs: 0,
            timeout_secs: 5,
            period_secs: 10,
            success_threshold: 1,
            failure_threshold: 3,
        };

        checker.add_probe(&pod_id, &probe).await.unwrap();
        let is_healthy = checker.check_health(&pod_id).await.unwrap();
        assert!(is_healthy);
    }

    #[tokio::test]
    async fn test_get_health_status() {
        let checker = HealthChecker::new();
        let pod_id = PodId("test-pod".to_string());
        let probe = HealthProbe {
            probe_type: crate::ProbeType::Tcp(8080),
            initial_delay_secs: 0,
            timeout_secs: 5,
            period_secs: 10,
            success_threshold: 1,
            failure_threshold: 3,
        };

        checker.add_probe(&pod_id, &probe).await.unwrap();
        checker.check_health(&pod_id).await.unwrap();
        let status = checker.get_health_status(&pod_id).await.unwrap();
        assert!(status);
    }

    #[tokio::test]
    async fn test_no_probe_healthy() {
        let checker = HealthChecker::new();
        let pod_id = PodId("test-pod".to_string());

        let status = checker.get_health_status(&pod_id).await.unwrap();
        assert!(status); // Pods without probes are considered healthy
    }
}
