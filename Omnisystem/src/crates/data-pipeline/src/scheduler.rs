use crate::Result;
use dashmap::DashMap;
use std::sync::Arc;

pub struct PipelineScheduler {
    schedules: Arc<DashMap<String, Schedule>>,
}

#[derive(Debug, Clone)]
pub struct Schedule {
    pub pipeline_id: String,
    pub frequency: ScheduleFrequency,
    pub last_run: u64,
    pub next_run: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleFrequency {
    Hourly,
    Daily,
    Weekly,
    Monthly,
    OnDemand,
}

impl PipelineScheduler {
    pub fn new() -> Self {
        Self {
            schedules: Arc::new(DashMap::new()),
        }
    }

    pub fn schedule_pipeline(&self, schedule: Schedule) -> Result<()> {
        self.schedules.insert(schedule.pipeline_id.clone(), schedule);
        tracing::info!("Pipeline scheduled");
        Ok(())
    }

    pub fn get_due_pipelines(&self) -> Vec<String> {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        self.schedules
            .iter()
            .filter(|s| s.value().next_run <= current_time)
            .map(|s| s.value().pipeline_id.clone())
            .collect()
    }

    pub fn schedule_count(&self) -> usize {
        self.schedules.len()
    }
}

impl Default for PipelineScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler() {
        let scheduler = PipelineScheduler::new();
        let schedule = Schedule {
            pipeline_id: "p1".to_string(),
            frequency: ScheduleFrequency::Daily,
            last_run: 0,
            next_run: 0,
        };
        assert!(scheduler.schedule_pipeline(schedule).is_ok());
        assert_eq!(scheduler.schedule_count(), 1);
    }
}
