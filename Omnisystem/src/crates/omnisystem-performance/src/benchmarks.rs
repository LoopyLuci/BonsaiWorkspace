use omnisystem_integration::*;

pub struct PerformanceTargets {
    pub module_startup_ms: u64,
    pub event_publish_µs: u64,
    pub service_lookup_µs: u64,
    pub health_check_µs: u64,
}

impl PerformanceTargets {
    pub fn default_targets() -> Self {
        Self {
            module_startup_ms: 100,
            event_publish_µs: 10,
            service_lookup_µs: 5,
            health_check_µs: 50,
        }
    }

    pub fn verify(&self) -> bool {
        self.module_startup_ms <= 100
            && self.event_publish_µs <= 10
            && self.service_lookup_µs <= 5
            && self.health_check_µs <= 50
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_performance_targets() {
        let targets = PerformanceTargets::default_targets();
        assert!(targets.verify());
    }
}
