use crate::{Device, Result};
use dashmap::DashMap;
use std::sync::Arc;

pub struct DeviceMonitor {
    device_stats: Arc<DashMap<String, DeviceStats>>,
}

#[derive(Debug, Clone)]
pub struct DeviceStats {
    pub device_id: String,
    pub uptime: u64,
    pub jobs_completed: u32,
    pub errors: u32,
    pub avg_temp: f32,
}

impl DeviceMonitor {
    pub fn new() -> Self {
        Self {
            device_stats: Arc::new(DashMap::new()),
        }
    }

    pub fn register_device(&self, device_id: String) -> Result<()> {
        self.device_stats.insert(device_id.clone(), DeviceStats {
            device_id,
            uptime: 0,
            jobs_completed: 0,
            errors: 0,
            avg_temp: 25.0,
        });
        Ok(())
    }

    pub fn update_stats(&self, device_id: &str, jobs: u32, errors: u32, temp: f32) -> Result<()> {
        if let Some(mut stats) = self.device_stats.get_mut(device_id) {
            stats.jobs_completed = jobs;
            stats.errors = errors;
            stats.avg_temp = temp;
            Ok(())
        } else {
            Err(crate::FabricationError::DeviceError("Device not found".to_string()))
        }
    }

    pub fn get_healthy_devices(&self) -> Vec<DeviceStats> {
        self.device_stats
            .iter()
            .filter(|s| s.value().errors < 5)
            .map(|s| s.value().clone())
            .collect()
    }

    pub fn device_count(&self) -> usize {
        self.device_stats.len()
    }
}

impl Default for DeviceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monitor() {
        let monitor = DeviceMonitor::new();
        assert!(monitor.register_device("dev1".to_string()).is_ok());
        assert!(monitor.update_stats("dev1", 5, 0, 200.0).is_ok());
        assert_eq!(monitor.device_count(), 1);
    }
}
