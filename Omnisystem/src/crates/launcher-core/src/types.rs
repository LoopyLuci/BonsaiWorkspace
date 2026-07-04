use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInstance {
    pub instance_id: Uuid,
    pub app_id: String,
    pub session_id: Uuid,
    pub pid: Option<u32>,
    pub status: AppStatus,
    pub started_at: SystemTime,
    pub resource_usage: ResourceMetrics,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ResourceMetrics {
    pub cpu_percent: f64,
    pub memory_mb: u64,
    pub resident_set_mb: u64,
    pub open_files: u32,
    pub threads: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum AppStatus {
    Pending,
    Starting,
    Running,
    Paused,
    Stopping,
    Stopped,
    Failed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_instance_creation() {
        let instance = AppInstance {
            instance_id: Uuid::new_v4(),
            app_id: "test".to_string(),
            session_id: Uuid::new_v4(),
            pid: Some(1234),
            status: AppStatus::Running,
            started_at: SystemTime::now(),
            resource_usage: ResourceMetrics {
                cpu_percent: 5.5,
                memory_mb: 128,
                resident_set_mb: 100,
                open_files: 10,
                threads: 2,
            },
        };
        assert_eq!(instance.app_id, "test");
        assert_eq!(instance.status, AppStatus::Running);
    }
}
