use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, Ordering};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ScheduleStrategy {
    Deterministic,
    Randomized,
    Coverage,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConcurrencyConfig {
    pub max_schedules: usize,
    pub timeout_secs: u64,
    pub strategy: ScheduleStrategy,
    pub num_threads: usize,
}

impl Default for ConcurrencyConfig {
    fn default() -> Self {
        Self {
            max_schedules: 1000,
            timeout_secs: 10,
            strategy: ScheduleStrategy::Deterministic,
            num_threads: 8,
        }
    }
}

pub struct ConcurrencyScheduler {
    config: ConcurrencyConfig,
    current_schedule: AtomicU32,
}

impl ConcurrencyScheduler {
    pub fn new(config: ConcurrencyConfig) -> Self {
        Self {
            config,
            current_schedule: AtomicU32::new(0),
        }
    }

    pub fn set_schedule(&self, schedule_id: u32) {
        self.current_schedule.store(schedule_id, Ordering::SeqCst);
    }

    pub fn current_schedule(&self) -> u32 {
        self.current_schedule.load(Ordering::SeqCst)
    }

    pub fn next_thread_choice(&self) -> usize {
        match self.config.strategy {
            ScheduleStrategy::Deterministic => {
                let schedule = self.current_schedule();
                (schedule as usize) % self.config.num_threads
            }
            ScheduleStrategy::Randomized => {
                use rand::Rng;
                let mut rng = rand::thread_rng();
                rng.gen_range(0..self.config.num_threads)
            }
            ScheduleStrategy::Coverage => {
                // Coverage-guided scheduling
                let schedule = self.current_schedule();
                ((schedule ^ (schedule >> 7)) as usize) % self.config.num_threads
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler_creation() {
        let config = ConcurrencyConfig::default();
        let scheduler = ConcurrencyScheduler::new(config);
        assert_eq!(scheduler.current_schedule(), 0);
    }

    #[test]
    fn test_set_schedule() {
        let config = ConcurrencyConfig::default();
        let scheduler = ConcurrencyScheduler::new(config);
        scheduler.set_schedule(42);
        assert_eq!(scheduler.current_schedule(), 42);
    }

    #[test]
    fn test_deterministic_strategy() {
        let mut config = ConcurrencyConfig::default();
        config.strategy = ScheduleStrategy::Deterministic;
        config.num_threads = 4;
        let scheduler = ConcurrencyScheduler::new(config);

        scheduler.set_schedule(0);
        let choice1 = scheduler.next_thread_choice();
        assert!(choice1 < 4);

        scheduler.set_schedule(5);
        let choice2 = scheduler.next_thread_choice();
        assert!(choice2 < 4);
    }
}
