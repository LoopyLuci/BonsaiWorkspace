use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq, Hash, Copy)]
#[repr(u8)]
pub enum Priority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ThreadInfo {
    pub thread_id: u64,
    pub priority: Priority,
    pub cpu_affinity: Vec<u32>,
    pub state: ThreadState,
    pub cpu_time_ms: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum ThreadState {
    Ready,
    Running,
    Blocked,
    Suspended,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub process_id: u64,
    pub threads: Vec<u64>,
    pub total_cpu_time_ms: u64,
    pub memory_mb: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SchedulerConfig {
    pub max_threads: usize,
    pub timeslice_ms: u32,
    pub enable_preemption: bool,
    pub cpu_cores: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SchedulingDecision {
    pub thread_id: u64,
    pub cpu_core: u32,
    pub priority: Priority,
    pub timeslice_ms: u32,
}
