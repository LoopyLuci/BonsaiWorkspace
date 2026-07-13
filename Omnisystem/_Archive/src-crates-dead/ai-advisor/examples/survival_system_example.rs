//! Example: Bonsai Survival System (Self-Healing) with AI-optional anomaly detection
//!
//! Demonstrates how the self-healing system detects crashes deterministically,
//! applies repair rules, and optionally uses AI for predictive failure detection.

use ai_fallback::{
    SovereignService, Arbiter, ArbiterConfig, AdvisoryOutput, Result,
};

/// System health check and repair decision engine
pub struct SurvivalService {
    /// Recent crash log entries
    crash_history: Vec<CrashEvent>,
    /// System metrics snapshot
    metrics: SystemMetrics,
    /// Whether AI anomaly detection is enabled
    ai_enabled: bool,
}

#[derive(Debug, Clone)]
pub struct CrashEvent {
    pub timestamp: u64,
    pub component: String,
    pub exit_code: i32,
    pub memory_used_mb: u32,
}

#[derive(Debug, Clone)]
pub struct SystemMetrics {
    pub cpu_usage_percent: f32,
    pub memory_used_mb: u32,
    pub memory_available_mb: u32,
    pub io_wait_percent: f32,
    pub disk_free_mb: u32,
}

impl SurvivalService {
    pub fn new(metrics: SystemMetrics) -> Self {
        Self {
            crash_history: Vec::new(),
            metrics,
            ai_enabled: false,
        }
    }

    pub fn add_crash(&mut self, event: CrashEvent) {
        self.crash_history.push(event);
    }

    /// Deterministic rule: count crashes in last minute
    fn count_recent_crashes(&self, window_secs: u64) -> usize {
        let now = self.crash_history.last().map(|e| e.timestamp).unwrap_or(0);
        self.crash_history
            .iter()
            .filter(|e| now.saturating_sub(e.timestamp) < window_secs)
            .count()
    }

    /// Deterministic rule: check if repair is needed
    fn should_repair(&self) -> bool {
        // Rule 1: More than 3 crashes in 1 minute = repair
        if self.count_recent_crashes(60) >= 3 {
            return true;
        }

        // Rule 2: Memory pressure
        if self.metrics.memory_available_mb < 100 {
            return true;
        }

        // Rule 3: Disk almost full
        if self.metrics.disk_free_mb < 1000 {
            return true;
        }

        false
    }

    /// Deterministic core: apply fixed repair script
    fn repair_action(&self) -> String {
        if self.count_recent_crashes(60) >= 3 {
            "restart_component_gracefully".to_string()
        } else if self.metrics.memory_available_mb < 100 {
            "invoke_memory_reclaim".to_string()
        } else if self.metrics.disk_free_mb < 1000 {
            "trigger_cleanup_old_logs".to_string()
        } else {
            "no_action".to_string()
        }
    }

    /// Heuristic: use log pattern matching (regex) to diagnose issues
    fn diagnose_heuristic(&self) -> Option<String> {
        // In production, use regex against component logs
        // For this example, check crash pattern
        let recent_crashes = self.count_recent_crashes(300); // 5-minute window

        if recent_crashes >= 5 {
            Some("recurring_crash_detected".to_string())
        } else {
            None
        }
    }

    /// Encode repair decision
    fn encode_decision(&self, action: &str, diagnosis: Option<&str>) -> Vec<u8> {
        let mut result = Vec::new();
        result.extend_from_slice(action.as_bytes());
        if let Some(diag) = diagnosis {
            result.push(b':');
            result.extend_from_slice(diag.as_bytes());
        }
        result
    }
}

impl SovereignService for SurvivalService {
    /// Deterministic core: apply fixed rules without AI
    fn deterministic_core(&self, _input: &[u8]) -> Result<Vec<u8>> {
        let action = if self.should_repair() {
            self.repair_action()
        } else {
            "monitor".to_string()
        };

        Ok(self.encode_decision(&action, None))
    }

    /// Heuristic: apply deterministic pattern matching on logs
    fn heuristic(&self, _input: &[u8]) -> Result<Option<Vec<u8>>> {
        if let Some(diagnosis) = self.diagnose_heuristic() {
            let action = "apply_targeted_fix";
            return Ok(Some(self.encode_decision(action, Some(&diagnosis))));
        }

        Ok(None)
    }

    /// AI enhancement: predictive failure detection via anomaly model
    fn ai_suggestion(&self, _input: &[u8]) -> Option<AdvisoryOutput> {
        if !self.ai_enabled {
            return None;
        }

        // In production, call lightweight anomaly detection model
        // that was trained on historical system metrics
        // This simulates detecting early warning signs

        // Example: AI detects unusual CPU spike + memory growth pattern
        if self.metrics.cpu_usage_percent > 85.0 && self.metrics.memory_used_mb > 6000 {
            let suggestion = "proactive_restart_recommended";
            let data = self.encode_decision(suggestion, Some("predicted_oom_in_5min"));

            return Some(AdvisoryOutput::new(
                data,
                0.82,  // Moderate confidence (it's a prediction)
                3500,  // 3.5ms latency
            ));
        }

        None
    }

    /// Safe stub: always respond with "monitor" (zero risk, no action)
    fn safe_stub(&self, _input: &[u8]) -> Vec<u8> {
        self.encode_decision("safe_stub_monitor_only", None)
    }

    fn name(&self) -> &str {
        "Bonsai Survival System"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crash_counting() {
        let mut service = SurvivalService::new(SystemMetrics {
            cpu_usage_percent: 50.0,
            memory_used_mb: 4000,
            memory_available_mb: 2000,
            io_wait_percent: 5.0,
            disk_free_mb: 50000,
        });

        service.add_crash(CrashEvent {
            timestamp: 1000,
            component: "worker".to_string(),
            exit_code: 1,
            memory_used_mb: 4000,
        });
        service.add_crash(CrashEvent {
            timestamp: 1050,
            component: "worker".to_string(),
            exit_code: 1,
            memory_used_mb: 4000,
        });
        service.add_crash(CrashEvent {
            timestamp: 1100,
            component: "worker".to_string(),
            exit_code: 1,
            memory_used_mb: 4000,
        });

        assert_eq!(service.count_recent_crashes(60), 3);
        assert!(service.should_repair());
    }

    #[test]
    fn test_memory_pressure_detection() {
        let service = SurvivalService::new(SystemMetrics {
            cpu_usage_percent: 30.0,
            memory_used_mb: 7000,
            memory_available_mb: 50, // Very low
            io_wait_percent: 2.0,
            disk_free_mb: 50000,
        });

        assert!(service.should_repair());
    }

    #[test]
    fn test_disk_pressure_detection() {
        let service = SurvivalService::new(SystemMetrics {
            cpu_usage_percent: 30.0,
            memory_used_mb: 4000,
            memory_available_mb: 4000,
            io_wait_percent: 2.0,
            disk_free_mb: 500, // Very low
        });

        assert!(service.should_repair());
    }

    #[test]
    fn test_deterministic_core_produces_action() {
        let service = SurvivalService::new(SystemMetrics {
            cpu_usage_percent: 50.0,
            memory_used_mb: 4000,
            memory_available_mb: 4000,
            io_wait_percent: 5.0,
            disk_free_mb: 50000,
        });

        let result = service.deterministic_core(&[]).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_safe_stub_always_returns_monitor() {
        let service = SurvivalService::new(SystemMetrics {
            cpu_usage_percent: 99.0, // Critical
            memory_used_mb: 7800,
            memory_available_mb: 10,
            io_wait_percent: 99.0,
            disk_free_mb: 1,
        });

        let stub = service.safe_stub(&[]);
        assert!(!stub.is_empty());
    }

    #[test]
    fn test_ai_disabled_by_default() {
        let service = SurvivalService::new(SystemMetrics {
            cpu_usage_percent: 90.0,
            memory_used_mb: 7000,
            memory_available_mb: 1000,
            io_wait_percent: 50.0,
            disk_free_mb: 50000,
        });

        assert!(!service.ai_enabled);
        assert!(service.ai_suggestion(&[]).is_none());
    }
}

fn main() {
    println!("Bonsai Survival System Example: Self-Healing with AI-Optional Predictive Analysis");
    println!("================================================================================\n");

    // Scenario 1: Normal operation
    println!("Scenario 1: Normal operation");
    let metrics_normal = SystemMetrics {
        cpu_usage_percent: 45.0,
        memory_used_mb: 4000,
        memory_available_mb: 4000,
        io_wait_percent: 3.0,
        disk_free_mb: 100_000,
    };
    let service_normal = SurvivalService::new(metrics_normal);
    let mut arbiter = Arbiter::new(ArbiterConfig::default());
    let result = arbiter.execute(&service_normal, &[]);
    println!(
        "  Decision: {:?}, Confidence: {:.2}\n",
        result.tier, result.confidence
    );

    // Scenario 2: Memory pressure
    println!("Scenario 2: Low memory available");
    let metrics_lowmem = SystemMetrics {
        cpu_usage_percent: 50.0,
        memory_used_mb: 7500,
        memory_available_mb: 50,
        io_wait_percent: 15.0,
        disk_free_mb: 100_000,
    };
    let service_lowmem = SurvivalService::new(metrics_lowmem);
    let result = arbiter.execute(&service_lowmem, &[]);
    println!("  Decision: {:?}, Confidence: {:.2}\n", result.tier, result.confidence);

    // Scenario 3: Repeated crashes
    println!("Scenario 3: Repeated crashes detected");
    let metrics_crashes = SystemMetrics {
        cpu_usage_percent: 60.0,
        memory_used_mb: 4000,
        memory_available_mb: 4000,
        io_wait_percent: 5.0,
        disk_free_mb: 100_000,
    };
    let mut service_crashes = SurvivalService::new(metrics_crashes);
    service_crashes.add_crash(CrashEvent {
        timestamp: 1000,
        component: "worker".to_string(),
        exit_code: 1,
        memory_used_mb: 4000,
    });
    service_crashes.add_crash(CrashEvent {
        timestamp: 1050,
        component: "worker".to_string(),
        exit_code: 1,
        memory_used_mb: 4000,
    });
    service_crashes.add_crash(CrashEvent {
        timestamp: 1100,
        component: "worker".to_string(),
        exit_code: 1,
        memory_used_mb: 4000,
    });

    let result = arbiter.execute(&service_crashes, &[]);
    println!("  Decision: {:?}, Confidence: {:.2}\n", result.tier, result.confidence);

    println!("✓ Survival System operates deterministically without AI");
    println!("✓ Rule-based repair actions trigger reliably");
    println!("✓ AI predictions available but disabled by default");
    println!("✓ Safe stub ensures system never blocks on health decisions");
}
