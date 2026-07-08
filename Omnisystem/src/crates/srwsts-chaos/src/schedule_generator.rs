//! Fault schedule generation strategies.
//!
//! Supports deterministic, random, and AI-guided schedule generation for
//! reproducible and targeted chaos engineering.

use crate::error::{ChaosError, Result};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use tracing::debug;

/// Fault schedule: ordered sequence of faults with timing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaultSchedule {
    /// Ordered list of faults.
    pub faults: VecDeque<ScheduledFault>,
    /// Generation strategy used.
    pub generation_strategy: String,
    /// Random seed (for reproducibility).
    pub seed: u64,
    /// Start time of this schedule.
    pub start_time: u64,
    /// Duration of the schedule.
    pub duration_secs: u64,
}

/// Scheduled fault with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledFault {
    /// Fault description.
    pub name: String,
    /// When to inject (epoch seconds).
    pub inject_time: u64,
    /// Duration of the fault.
    pub duration_secs: u64,
    /// Fault type identifier.
    pub fault_type: String,
    /// Severity (0-10).
    pub severity: u8,
    /// Whether this fault can cascade.
    pub can_cascade: bool,
    /// Associated parameters.
    pub parameters: serde_json::Value,
}

impl FaultSchedule {
    /// Create new empty schedule.
    pub fn new(start_time: u64, duration_secs: u64, seed: u64) -> Self {
        Self {
            faults: VecDeque::new(),
            generation_strategy: "manual".to_string(),
            seed,
            start_time,
            duration_secs,
        }
    }

    /// Get next scheduled fault.
    pub fn next_fault(&mut self) -> Option<ScheduledFault> {
        self.faults.pop_front()
    }

    /// Add a scheduled fault.
    pub fn add_fault(&mut self, fault: ScheduledFault) -> Result<()> {
        if fault.inject_time >= self.start_time + self.duration_secs {
            return Err(ChaosError::InvalidSchedule(
                "Fault scheduled outside of schedule duration".to_string(),
            ));
        }
        self.faults.push_back(fault);
        Ok(())
    }

    /// Get total number of faults.
    pub fn fault_count(&self) -> usize {
        self.faults.len()
    }

    /// Check if schedule is valid.
    pub fn validate(&self) -> Result<()> {
        let mut last_time = self.start_time;
        for fault in &self.faults {
            if fault.inject_time < last_time {
                return Err(ChaosError::InvalidSchedule(
                    "Faults not in chronological order".to_string(),
                ));
            }
            if fault.duration_secs == 0 {
                return Err(ChaosError::InvalidSchedule(
                    "Fault duration must be > 0".to_string(),
                ));
            }
            last_time = fault.inject_time;
        }
        Ok(())
    }

    /// Get faults sorted by severity.
    pub fn faults_by_severity(&self) -> Vec<&ScheduledFault> {
        let mut faults: Vec<&ScheduledFault> = self.faults.iter().collect();
        faults.sort_by(|a, b| b.severity.cmp(&a.severity));
        faults
    }
}

/// Schedule generation strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleStrategy {
    /// Deterministic: seed → reproducible schedule.
    Deterministic,
    /// Random: probability distribution over fault types.
    Random,
    /// Clustered: faults grouped together.
    Clustered,
    /// Spread: faults distributed throughout schedule.
    Spread,
}

/// Fault schedule generator.
pub struct ScheduleGenerator {
    strategy: ScheduleStrategy,
    rng: ChaCha8Rng,
    start_time: u64,
    duration_secs: u64,
}

impl ScheduleGenerator {
    /// Create new schedule generator.
    pub fn new(strategy: ScheduleStrategy, seed: u64, start_time: u64, duration_secs: u64) -> Self {
        let rng = ChaCha8Rng::seed_from_u64(seed);
        Self {
            strategy,
            rng,
            start_time,
            duration_secs,
        }
    }

    /// Generate a fault schedule.
    pub fn generate(&mut self, fault_count: usize) -> Result<FaultSchedule> {
        let mut schedule = FaultSchedule::new(self.start_time, self.duration_secs, self.rng.gen());

        match self.strategy {
            ScheduleStrategy::Deterministic => {
                self.generate_deterministic(&mut schedule, fault_count)?;
            }
            ScheduleStrategy::Random => {
                self.generate_random(&mut schedule, fault_count)?;
            }
            ScheduleStrategy::Clustered => {
                self.generate_clustered(&mut schedule, fault_count)?;
            }
            ScheduleStrategy::Spread => {
                self.generate_spread(&mut schedule, fault_count)?;
            }
        }

        schedule.generation_strategy = format!("{:?}", self.strategy);
        schedule.validate()?;
        debug!("Generated {} schedule with {} faults", schedule.generation_strategy, fault_count);

        Ok(schedule)
    }

    fn generate_deterministic(
        &mut self,
        schedule: &mut FaultSchedule,
        fault_count: usize,
    ) -> Result<()> {
        let fault_types = vec![
            ("MemoryPressure", 3),
            ("NetworkLatency", 4),
            ("DiskFull", 5),
            ("CpuOverload", 3),
            ("PacketLoss", 4),
            ("OutOfMemory", 7),
            ("DataCorruption", 9),
            ("PowerFailure", 8),
        ];

        let interval = self.duration_secs / (fault_count as u64 + 1);

        for i in 0..fault_count {
            let inject_time = self.start_time + ((i as u64 + 1) * interval);
            let (name, severity) = fault_types[i % fault_types.len()];

            let fault = ScheduledFault {
                name: name.to_string(),
                inject_time,
                duration_secs: 10 + (i as u64 % 20),
                fault_type: name.to_string(),
                severity: severity as u8,
                can_cascade: severity >= 5,
                parameters: serde_json::json!({}),
            };

            schedule.add_fault(fault)?;
        }

        Ok(())
    }

    fn generate_random(&mut self, schedule: &mut FaultSchedule, fault_count: usize) -> Result<()> {
        let fault_types = vec![
            ("MemoryPressure", 0.3, 3),
            ("NetworkLatency", 0.25, 4),
            ("DiskFull", 0.15, 5),
            ("CpuOverload", 0.15, 3),
            ("PacketLoss", 0.12, 4),
            ("OutOfMemory", 0.05, 7),
            ("DataCorruption", 0.02, 9),
        ];

        for _ in 0..fault_count {
            let inject_time = self.start_time + self.rng.gen_range(0..self.duration_secs);

            // Select fault type based on probability distribution
            let rand_val: f64 = self.rng.gen();
            let mut cumulative = 0.0;
            let mut selected = fault_types[0];

            for (name, prob, severity) in &fault_types {
                cumulative += prob;
                if rand_val <= cumulative {
                    selected = (name, *prob, *severity);
                    break;
                }
            }

            let fault = ScheduledFault {
                name: selected.0.to_string(),
                inject_time,
                duration_secs: 5 + self.rng.gen_range(0..30),
                fault_type: selected.0.to_string(),
                severity: selected.2 as u8,
                can_cascade: selected.2 >= 5,
                parameters: serde_json::json!({}),
            };

            schedule.add_fault(fault)?;
        }

        // Sort by injection time
        let mut faults: Vec<_> = schedule.faults.iter().cloned().collect();
        faults.sort_by_key(|f| f.inject_time);
        schedule.faults = faults.into();

        Ok(())
    }

    fn generate_clustered(
        &mut self,
        schedule: &mut FaultSchedule,
        fault_count: usize,
    ) -> Result<()> {
        let cluster_count = (fault_count / 3).max(1);
        let faults_per_cluster = fault_count / cluster_count;
        let cluster_spacing = self.duration_secs / (cluster_count as u64 + 1);

        let fault_types = vec![
            ("MemoryPressure", 3),
            ("NetworkLatency", 4),
            ("DiskFull", 5),
            ("CpuOverload", 3),
        ];

        for cluster_idx in 0..cluster_count {
            let cluster_start = self.start_time + ((cluster_idx as u64 + 1) * cluster_spacing);

            for fault_idx in 0..faults_per_cluster {
                let inject_time = cluster_start + (fault_idx as u64 * 5);
                let (name, severity) = fault_types[fault_idx % fault_types.len()];

                let fault = ScheduledFault {
                    name: name.to_string(),
                    inject_time,
                    duration_secs: 8 + (fault_idx as u64 % 10),
                    fault_type: name.to_string(),
                    severity: severity as u8,
                    can_cascade: severity >= 5,
                    parameters: serde_json::json!({}),
                };

                schedule.add_fault(fault)?;
            }
        }

        Ok(())
    }

    fn generate_spread(
        &mut self,
        schedule: &mut FaultSchedule,
        fault_count: usize,
    ) -> Result<()> {
        let fault_types = vec![
            ("MemoryPressure", 3),
            ("NetworkLatency", 4),
            ("DiskFull", 5),
            ("CpuOverload", 3),
            ("PacketLoss", 4),
            ("ThermalThrottling", 5),
            ("CacheContention", 3),
        ];

        let min_spacing = self.duration_secs / ((fault_count as u64).max(1) * 3);

        let mut current_time = self.start_time + min_spacing;
        for i in 0..fault_count {
            current_time += self.rng.gen_range(min_spacing..min_spacing * 3);

            if current_time >= self.start_time + self.duration_secs {
                break;
            }

            let (name, severity) = fault_types[i % fault_types.len()];

            let fault = ScheduledFault {
                name: name.to_string(),
                inject_time: current_time,
                duration_secs: 7 + (i as u64 % 15),
                fault_type: name.to_string(),
                severity: severity as u8,
                can_cascade: severity >= 5,
                parameters: serde_json::json!({}),
            };

            schedule.add_fault(fault)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schedule_creation() {
        let schedule = FaultSchedule::new(1000, 3600, 42);
        assert_eq!(schedule.fault_count(), 0);
    }

    #[test]
    fn test_deterministic_generation() {
        let mut gen = ScheduleGenerator::new(ScheduleStrategy::Deterministic, 42, 1000, 3600);
        let schedule = gen.generate(10).unwrap();
        assert_eq!(schedule.fault_count(), 10);
        assert!(schedule.validate().is_ok());
    }

    #[test]
    fn test_random_generation() {
        let mut gen = ScheduleGenerator::new(ScheduleStrategy::Random, 42, 1000, 3600);
        let schedule = gen.generate(10).unwrap();
        assert_eq!(schedule.fault_count(), 10);
        assert!(schedule.validate().is_ok());
    }

    #[test]
    fn test_clustered_generation() {
        let mut gen = ScheduleGenerator::new(ScheduleStrategy::Clustered, 42, 1000, 3600);
        let schedule = gen.generate(12).unwrap();
        assert_eq!(schedule.fault_count(), 12);
        assert!(schedule.validate().is_ok());
    }

    #[test]
    fn test_spread_generation() {
        let mut gen = ScheduleGenerator::new(ScheduleStrategy::Spread, 42, 1000, 3600);
        let schedule = gen.generate(10).unwrap();
        assert_eq!(schedule.fault_count(), 10);
        assert!(schedule.validate().is_ok());
    }

    #[test]
    fn test_faults_by_severity() {
        let mut schedule = FaultSchedule::new(1000, 3600, 42);
        schedule
            .add_fault(ScheduledFault {
                name: "Low".to_string(),
                inject_time: 1100,
                duration_secs: 10,
                fault_type: "Test".to_string(),
                severity: 2,
                can_cascade: false,
                parameters: serde_json::json!({}),
            })
            .unwrap();
        schedule
            .add_fault(ScheduledFault {
                name: "High".to_string(),
                inject_time: 1200,
                duration_secs: 10,
                fault_type: "Test".to_string(),
                severity: 9,
                can_cascade: true,
                parameters: serde_json::json!({}),
            })
            .unwrap();

        let by_severity = schedule.faults_by_severity();
        assert_eq!(by_severity[0].severity, 9);
        assert_eq!(by_severity[1].severity, 2);
    }
}
