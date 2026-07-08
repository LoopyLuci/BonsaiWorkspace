//! Fault scheduling with deterministic reproducibility.

use crate::error::{FaultError, Result};
use crate::fault::{FaultDefinition, FaultId, FaultType};
use dashmap::DashMap;
use rand_chacha::ChaCha20Rng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, debug, warn};

/// Scheduled fault entry (for ordering in heap).
#[derive(Debug, Clone)]
struct ScheduledFault {
    fault_def: FaultDefinition,
}

impl PartialEq for ScheduledFault {
    fn eq(&self, other: &Self) -> bool {
        self.fault_def.id == other.fault_def.id
    }
}

impl Eq for ScheduledFault {}

impl PartialOrd for ScheduledFault {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScheduledFault {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse so min-heap becomes max-heap (higher priority = earlier injection)
        other.fault_def.inject_at.cmp(&self.fault_def.inject_at)
    }
}

/// Fault schedule with deterministic timing and RNG seeding.
pub struct FaultSchedule {
    schedule: Arc<RwLock<BinaryHeap<ScheduledFault>>>,
    faults: Arc<DashMap<FaultId, FaultDefinition>>,
    rng: Arc<RwLock<ChaCha20Rng>>,
    seed: u64,
    is_running: Arc<RwLock<bool>>,
}

impl FaultSchedule {
    /// Create a new fault schedule with a seed for reproducibility.
    pub fn new(seed: u64) -> Self {
        let rng = ChaCha20Rng::seed_from_u64(seed);
        info!("created fault schedule with seed: {}", seed);

        Self {
            schedule: Arc::new(RwLock::new(BinaryHeap::new())),
            faults: Arc::new(DashMap::new()),
            rng: Arc::new(RwLock::new(rng)),
            seed,
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    /// Add a fault to the schedule.
    pub async fn add_fault(&self, fault_def: FaultDefinition) -> Result<FaultId> {
        // Validate the fault definition
        fault_def.validate()?;

        let fault_id = fault_def.id;
        self.faults.insert(fault_id, fault_def.clone());

        let mut schedule = self.schedule.write().await;
        schedule.push(ScheduledFault {
            fault_def: fault_def.clone(),
        });

        info!("added fault to schedule: {} at {}", fault_id, fault_def.inject_at);
        Ok(fault_id)
    }

    /// Add multiple faults.
    pub async fn add_faults(&self, faults: Vec<FaultDefinition>) -> Result<Vec<FaultId>> {
        let mut ids = Vec::new();
        for fault in faults {
            ids.push(self.add_fault(fault).await?);
        }
        Ok(ids)
    }

    /// Get the next fault to inject.
    pub async fn next_fault(&self) -> Option<FaultDefinition> {
        let mut schedule = self.schedule.write().await;
        schedule.pop().map(|sf| sf.fault_def)
    }

    /// Get faults that should be active at a given time.
    pub async fn faults_at_time(&self, time: u64) -> Vec<FaultDefinition> {
        self.faults
            .iter()
            .filter(|entry| entry.value().is_active_at(time))
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Get a fault by ID.
    pub fn get_fault(&self, id: FaultId) -> Result<FaultDefinition> {
        self.faults
            .get(&id)
            .map(|r| r.clone())
            .ok_or(FaultError::FaultNotFound(id.to_string()))
    }

    /// Remove a fault from the schedule.
    pub fn remove_fault(&self, id: FaultId) -> Result<FaultDefinition> {
        self.faults
            .remove(&id)
            .map(|(_, def)| def)
            .ok_or(FaultError::FaultNotFound(id.to_string()))
    }

    /// Get all scheduled faults.
    pub fn list_faults(&self) -> Vec<FaultDefinition> {
        self.faults.iter().map(|r| r.value().clone()).collect()
    }

    /// Get faults by category.
    pub fn faults_by_category(&self, category: crate::fault::FaultTypeKind) -> Vec<FaultDefinition> {
        self.faults
            .iter()
            .filter(|entry| entry.value().fault_type.category() == category)
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Start the schedule (mark as running).
    pub async fn start(&self) -> Result<()> {
        *self.is_running.write().await = true;
        info!("fault schedule started");
        Ok(())
    }

    /// Stop the schedule.
    pub async fn stop(&self) -> Result<()> {
        *self.is_running.write().await = false;
        info!("fault schedule stopped");
        Ok(())
    }

    /// Check if schedule is running.
    pub async fn is_running(&self) -> bool {
        *self.is_running.read().await
    }

    /// Get schedule statistics.
    pub fn statistics(&self) -> ScheduleStatistics {
        let all_faults = self.list_faults();
        let total = all_faults.len();

        let by_category = [
            crate::fault::FaultTypeKind::Memory,
            crate::fault::FaultTypeKind::Cpu,
            crate::fault::FaultTypeKind::Network,
            crate::fault::FaultTypeKind::Storage,
            crate::fault::FaultTypeKind::Time,
            crate::fault::FaultTypeKind::Hardware,
        ]
        .iter()
        .map(|cat| (cat.to_string(), self.faults_by_category(*cat).len()))
        .collect();

        let destructive = all_faults.iter().filter(|f| f.fault_type.is_destructive()).count();
        let transient = all_faults.iter().filter(|f| f.fault_type.is_transient()).count();

        ScheduleStatistics {
            total_faults: total,
            faults_by_category: by_category,
            destructive_faults: destructive,
            transient_faults: transient,
        }
    }

    /// Generate random faults using the seeded RNG.
    pub async fn generate_random_faults(
        &self,
        count: usize,
        start_time: u64,
        interval_secs: u64,
    ) -> Result<Vec<FaultId>> {
        let mut rng = self.rng.write().await;
        let mut ids = Vec::new();

        for i in 0..count {
            let inject_at = start_time + (i as u64) * interval_secs;
            let duration_secs = rng.gen_range(1..30);

            let fault_type = self.generate_random_fault_type(&mut *rng);
            let fault_def = FaultDefinition::new(fault_type, inject_at, duration_secs);

            drop(rng); // Release lock temporarily
            let id = self.add_fault(fault_def).await?;
            ids.push(id);
            rng = self.rng.write().await;
        }

        info!("generated {} random faults", count);
        Ok(ids)
    }

    /// Generate a random fault type.
    fn generate_random_fault_type(&self, rng: &mut ChaCha20Rng) -> FaultType {
        match rng.gen_range(0..18) {
            0 => FaultType::MemoryPressure {
                pressure_percent: rng.gen_range(10..90),
                page_faults: rng.gen_range(100..10000),
            },
            1 => FaultType::OutOfMemory {
                target_pid: rng.gen_range(0..100),
            },
            2 => FaultType::AllocationFailure {
                failure_rate: rng.gen_range(5..50),
                min_size: rng.gen_range(1024..1048576),
            },
            3 => FaultType::CpuOverload {
                cpu_percent: rng.gen_range(50..100),
                core_count: rng.gen_range(1..8),
            },
            4 => FaultType::CacheContention {
                working_set_size: rng.gen_range(1048576..104857600),
                intensity: rng.gen_range(30..100),
            },
            5 => FaultType::ThermalThrottling {
                temperature: rng.gen_range(80..95),
                throttle_percent: rng.gen_range(20..80),
            },
            6 => FaultType::NetworkPartition {
                isolated_subnets: vec!["192.168.0.0/24".to_string()],
                affect_bidirectional: true,
            },
            7 => FaultType::NetworkLatency {
                latency_ms: rng.gen_range(10..500),
                jitter_ms: rng.gen_range(1..100),
                ports: vec![],
            },
            8 => FaultType::PacketLoss {
                loss_percent: rng.gen_range(1..30),
                correlation_percent: rng.gen_range(0..50),
            },
            9 => FaultType::DiskFull {
                mount_point: "/".to_string(),
                fill_percent: rng.gen_range(70..95),
            },
            10 => FaultType::IoError {
                failure_rate: rng.gen_range(5..30),
                filesystem: None,
                error_codes: vec!["EIO".to_string()],
            },
            11 => FaultType::DataCorruption {
                corruption_rate: rng.gen_range(1..10),
                paths: vec!["/var".to_string()],
            },
            12 => FaultType::ClockSkew {
                offset_secs: rng.gen_range(-3600..3600),
                drift_rate_nanos_per_sec: rng.gen_range(-1000..1000),
            },
            13 => FaultType::TimeJump {
                offset_secs: rng.gen_range(-1800..1800),
            },
            14 => FaultType::GpuReset {
                device_id: rng.gen_range(0..4),
            },
            15 => FaultType::PowerFailure {
                recovery_secs: rng.gen_range(5..60),
            },
            16 => FaultType::ThermalShutdown {
                temperature: rng.gen_range(85..100),
            },
            _ => FaultType::FanFailure {
                fan_id: rng.gen_range(0..4),
            },
        }
    }

    /// Clear all scheduled faults.
    pub async fn clear(&self) {
        self.faults.clear();
        let mut schedule = self.schedule.write().await;
        schedule.clear();
        info!("fault schedule cleared");
    }

    /// Get the schedule seed.
    pub fn seed(&self) -> u64 {
        self.seed
    }
}

/// Schedule statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleStatistics {
    pub total_faults: usize,
    pub faults_by_category: Vec<(String, usize)>,
    pub destructive_faults: usize,
    pub transient_faults: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_schedule_creation() {
        let schedule = FaultSchedule::new(42);
        assert_eq!(schedule.seed(), 42);
        assert!(!schedule.is_running().await);
    }

    #[tokio::test]
    async fn test_add_fault() {
        let schedule = FaultSchedule::new(42);
        let fault = FaultDefinition::new(
            FaultType::MemoryPressure {
                pressure_percent: 50,
                page_faults: 1000,
            },
            10,
            5,
        );

        let id = schedule.add_fault(fault).await.unwrap();
        assert!(schedule.get_fault(id).is_ok());
    }

    #[tokio::test]
    async fn test_faults_at_time() {
        let schedule = FaultSchedule::new(42);
        let fault = FaultDefinition::new(
            FaultType::CpuOverload {
                cpu_percent: 80,
                core_count: 4,
            },
            10,
            5,
        );

        schedule.add_fault(fault).await.unwrap();
        let active = schedule.faults_at_time(12).await;
        assert_eq!(active.len(), 1);
    }

    #[tokio::test]
    async fn test_schedule_start_stop() {
        let schedule = FaultSchedule::new(42);
        assert!(!schedule.is_running().await);

        schedule.start().await.unwrap();
        assert!(schedule.is_running().await);

        schedule.stop().await.unwrap();
        assert!(!schedule.is_running().await);
    }
}
