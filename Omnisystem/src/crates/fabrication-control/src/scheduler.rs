use crate::{Job, Result};
use std::collections::BinaryHeap;
use std::cmp::Ordering;

// `Job` and `estimated_duration` involve `f32` fields, which don't
// implement `Eq` (no total ordering across NaN). `BinaryHeap<T>`
// requires `T: Ord`, and `Ord` requires `Eq` as a supertrait, so
// `PartialEq`/`Eq` are implemented manually below over just the fields
// that determine ordering (priority + estimated_duration), consistent
// with the `Ord` impl, rather than deriving a field-by-field
// comparison across `Job` that couldn't compile.
#[derive(Debug, Clone)]
pub struct ScheduledJob {
    pub job: Job,
    pub priority: u8,
    pub estimated_duration: f32,
}

impl PartialEq for ScheduledJob {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.estimated_duration == other.estimated_duration
    }
}

impl Eq for ScheduledJob {}

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
            state: crate::JobState::Pending,
            progress: 0.0,
        };
        assert!(scheduler.schedule_job(job, 5, 120.0).is_ok());
        assert_eq!(scheduler.queue_size(), 1);
    }

    #[test]
    fn test_scheduler_orders_by_priority() {
        // ScheduledJob::cmp compares `other.priority.cmp(&self.priority)`
        // (reversed), so smaller priority numbers sort as "greater" and
        // come out of the max-heap first - i.e. priority is Unix
        // nice-style: 0 is most urgent. This test locks in that
        // behavior rather than assuming either direction.
        let mut scheduler = JobScheduler::new();
        let make_job = |id: &str| Job {
            id: id.to_string(),
            device_id: "d1".to_string(),
            material: crate::MaterialType::PLA,
            state: crate::JobState::Pending,
            progress: 0.0,
        };

        scheduler.schedule_job(make_job("p9"), 9, 10.0).unwrap();
        scheduler.schedule_job(make_job("p1"), 1, 10.0).unwrap();
        scheduler.schedule_job(make_job("p5"), 5, 10.0).unwrap();

        let order: Vec<String> = std::iter::from_fn(|| scheduler.get_next_job())
            .map(|j| j.id)
            .collect();
        assert_eq!(order, vec!["p1", "p5", "p9"]);
    }
}
