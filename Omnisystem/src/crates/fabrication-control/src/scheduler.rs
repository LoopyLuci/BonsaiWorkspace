use crate::{Job, JobState, Result};
use std::collections::BinaryHeap;
use std::cmp::Ordering;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ScheduledJob {
    pub job: Job,
    pub priority: u8,
    pub estimated_duration: f32,
}

impl Ord for ScheduledJob {
    fn cmp(&self, other: &Self) -> Ordering {
        other.priority.cmp(&self.priority)
            .then_with(|| self.estimated_duration.partial_cmp(&other.estimated_duration).unwrap_or(Ordering::Equal))
    }
}

impl PartialOrd for ScheduledJob {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub struct JobScheduler {
    queue: BinaryHeap<ScheduledJob>,
}

impl JobScheduler {
    pub fn new() -> Self {
        Self {
            queue: BinaryHeap::new(),
        }
    }

    pub fn schedule_job(&mut self, job: Job, priority: u8, duration: f32) -> Result<()> {
        let scheduled = ScheduledJob {
            job,
            priority,
            estimated_duration: duration,
        };
        self.queue.push(scheduled);
        tracing::info!("Job scheduled");
        Ok(())
    }

    pub fn get_next_job(&mut self) -> Option<Job> {
        self.queue.pop().map(|s| s.job)
    }

    pub fn queue_size(&self) -> usize {
        self.queue.len()
    }
}

impl Default for JobScheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler() {
        let mut scheduler = JobScheduler::new();
        let job = Job {
            id: "j1".to_string(),
            device_id: "d1".to_string(),
            material: crate::MaterialType::PLA,
            state: JobState::Pending,
            progress: 0.0,
        };
        assert!(scheduler.schedule_job(job, 5, 120.0).is_ok());
        assert_eq!(scheduler.queue_size(), 1);
    }
}
