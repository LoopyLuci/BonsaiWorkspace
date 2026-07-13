use crate::{Priority, SchedulerError, SchedulerResult, ThreadInfo, ThreadState, SchedulingDecision, ProcessInfo};
use dashmap::DashMap;
use std::sync::Arc;

pub struct CpuScheduler {
    threads: Arc<DashMap<u64, ThreadInfo>>,
    processes: Arc<DashMap<u64, ProcessInfo>>,
    ready_queue: Arc<DashMap<Priority, Vec<u64>>>,
    cpu_cores: usize,
}

impl CpuScheduler {
    pub fn new(cpu_cores: usize) -> Self {
        let mut ready_queue = DashMap::new();
        ready_queue.insert(Priority::Low, Vec::new());
        ready_queue.insert(Priority::Normal, Vec::new());
        ready_queue.insert(Priority::High, Vec::new());
        ready_queue.insert(Priority::Critical, Vec::new());

        Self {
            threads: Arc::new(DashMap::new()),
            processes: Arc::new(DashMap::new()),
            ready_queue: Arc::new(ready_queue),
            cpu_cores,
        }
    }

    pub async fn create_thread(
        &self,
        thread_id: u64,
        priority: Priority,
        cpu_affinity: Vec<u32>,
    ) -> SchedulerResult<()> {
        let thread_info = ThreadInfo {
            thread_id,
            priority,
            cpu_affinity,
            state: ThreadState::Ready,
            cpu_time_ms: 0,
        };

        self.threads.insert(thread_id, thread_info);

        if let Some(mut queue) = self.ready_queue.get_mut(&priority) {
            queue.push(thread_id);
        }

        Ok(())
    }

    pub async fn schedule_next(&self) -> SchedulerResult<SchedulingDecision> {
        for priority_level in [Priority::Critical, Priority::High, Priority::Normal, Priority::Low].iter() {
            if let Some(mut queue) = self.ready_queue.get_mut(priority_level) {
                if !queue.is_empty() {
                    let thread_id = queue.remove(0);

                    if let Some(mut thread) = self.threads.get_mut(&thread_id) {
                        thread.state = ThreadState::Running;

                        let cpu_core = (thread_id as u32) % (self.cpu_cores as u32);
                        return Ok(SchedulingDecision {
                            thread_id,
                            cpu_core,
                            priority: thread.priority,
                            timeslice_ms: 10,
                        });
                    }
                }
            }
        }

        Err(SchedulerError::SchedulingFailed)
    }

    pub async fn set_thread_priority(
        &self,
        thread_id: u64,
        new_priority: Priority,
    ) -> SchedulerResult<()> {
        if let Some(mut thread) = self.threads.get_mut(&thread_id) {
            thread.priority = new_priority;
            Ok(())
        } else {
            Err(SchedulerError::ThreadNotFound)
        }
    }

    pub async fn update_thread_time(&self, thread_id: u64, elapsed_ms: u64) -> SchedulerResult<()> {
        if let Some(mut thread) = self.threads.get_mut(&thread_id) {
            thread.cpu_time_ms += elapsed_ms;
            Ok(())
        } else {
            Err(SchedulerError::ThreadNotFound)
        }
    }

    pub async fn get_thread_info(&self, thread_id: u64) -> SchedulerResult<ThreadInfo> {
        self.threads
            .get(&thread_id)
            .map(|t| t.clone())
            .ok_or(SchedulerError::ThreadNotFound)
    }

    pub fn thread_count(&self) -> usize {
        self.threads.len()
    }
}

impl Default for CpuScheduler {
    fn default() -> Self {
        Self::new(4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_thread() {
        let scheduler = CpuScheduler::new(4);
        let result = scheduler
            .create_thread(1, Priority::Normal, vec![0, 1])
            .await;
        assert!(result.is_ok());
        assert_eq!(scheduler.thread_count(), 1);
    }

    #[tokio::test]
    async fn test_schedule_next() {
        let scheduler = CpuScheduler::new(4);
        scheduler
            .create_thread(1, Priority::Normal, vec![0])
            .await
            .unwrap();

        let decision = scheduler.schedule_next().await.unwrap();
        assert_eq!(decision.thread_id, 1);
        assert_eq!(decision.priority, Priority::Normal);
    }

    #[tokio::test]
    async fn test_set_thread_priority() {
        let scheduler = CpuScheduler::new(4);
        scheduler
            .create_thread(1, Priority::Normal, vec![0])
            .await
            .unwrap();

        let result = scheduler.set_thread_priority(1, Priority::High).await;
        assert!(result.is_ok());

        let thread = scheduler.get_thread_info(1).await.unwrap();
        assert_eq!(thread.priority, Priority::High);
    }

    #[tokio::test]
    async fn test_update_thread_time() {
        let scheduler = CpuScheduler::new(4);
        scheduler
            .create_thread(1, Priority::Normal, vec![0])
            .await
            .unwrap();

        scheduler.update_thread_time(1, 50).await.unwrap();

        let thread = scheduler.get_thread_info(1).await.unwrap();
        assert_eq!(thread.cpu_time_ms, 50);
    }
}
