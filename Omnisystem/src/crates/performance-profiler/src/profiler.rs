use crate::{CpuSample, ProfileReport, ProfilerResult, StackFrame};
use dashmap::DashMap;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

pub struct CpuProfiler {
    samples: Arc<DashMap<Uuid, CpuSample>>,
    profile_id: Uuid,
    started_at: Arc<Mutex<Option<Instant>>>,
}

impl CpuProfiler {
    pub fn new() -> Self {
        Self {
            samples: Arc::new(DashMap::new()),
            profile_id: Uuid::new_v4(),
            started_at: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn start_sampling(&self) -> ProfilerResult<Uuid> {
        *self.started_at.lock() = Some(Instant::now());
        Ok(self.profile_id)
    }

    pub async fn record_sample(
        &self,
        duration_us: u64,
        stack_trace: Vec<StackFrame>,
    ) -> ProfilerResult<Uuid> {
        let sample_id = Uuid::new_v4();

        let sample = CpuSample {
            sample_id,
            timestamp: chrono::Utc::now(),
            stack_trace,
            duration_us,
        };

        self.samples.insert(sample_id, sample);
        Ok(sample_id)
    }

    pub async fn stop_sampling(&self) -> ProfilerResult<ProfileReport> {
        let total_samples = self.samples.len() as u64;
        let mut total_duration = 0u64;
        let mut frame_totals: HashMap<String, u64> = HashMap::new();

        for entry in self.samples.iter() {
            total_duration += entry.value().duration_us;
            for frame in &entry.value().stack_trace {
                *frame_totals.entry(frame.function_name.clone()).or_insert(0) +=
                    entry.value().duration_us;
            }
        }

        // cpu_time_percent is real: share of wall-clock time since
        // start_sampling() that was spent in recorded samples.
        let elapsed_us = self
            .started_at
            .lock()
            .map(|start| start.elapsed().as_micros() as u64)
            .unwrap_or(0);
        let cpu_time_percent = if elapsed_us > 0 {
            ((total_duration as f64 / elapsed_us as f64) * 100.0).min(100.0) as f32
        } else {
            0.0
        };

        // hotspots: real top-N frames by cumulative sampled duration.
        let mut hotspots: Vec<(String, u64)> = frame_totals.into_iter().collect();
        hotspots.sort_by(|a, b| b.1.cmp(&a.1));
        let hotspots: Vec<StackFrame> = hotspots
            .into_iter()
            .take(5)
            .map(|(name, _)| StackFrame {
                function_name: name,
                module_name: String::new(),
                line_number: 0,
                offset: 0,
            })
            .collect();

        Ok(ProfileReport {
            profile_id: self.profile_id,
            total_samples,
            duration_ms: total_duration / 1000,
            cpu_time_percent,
            memory_peak_mb: current_process_memory_mb(),
            hotspots,
        })
    }

    pub async fn get_samples(&self) -> ProfilerResult<Vec<CpuSample>> {
        let mut samples = Vec::new();
        for entry in self.samples.iter() {
            samples.push(entry.value().clone());
        }
        Ok(samples)
    }

    pub fn sample_count(&self) -> usize {
        self.samples.len()
    }
}

impl Default for CpuProfiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Best-effort real reading of this process's current resident memory, in
/// megabytes. Returns 0 if the platform sampler can't produce a value
/// rather than fabricating a number.
fn current_process_memory_mb() -> u64 {
    use sysinfo::{Pid, System};

    let pid = match sysinfo::get_current_pid() {
        Ok(pid) => pid,
        Err(_) => return 0,
    };

    let mut system = System::new();
    system.refresh_process(pid);

    system
        .process(Pid::from(pid.as_u32() as usize))
        .map(|process| process.memory() / (1024 * 1024))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_start_sampling() {
        let profiler = CpuProfiler::new();
        let profile_id = profiler.start_sampling().await.unwrap();
        assert!(!profile_id.is_nil());
    }

    #[tokio::test]
    async fn test_record_sample() {
        let profiler = CpuProfiler::new();
        profiler.start_sampling().await.unwrap();

        let stack = vec![StackFrame {
            function_name: "main".to_string(),
            module_name: "app".to_string(),
            line_number: 42,
            offset: 0x1000,
        }];

        let result = profiler.record_sample(100, stack).await;
        assert!(result.is_ok());
        assert_eq!(profiler.sample_count(), 1);
    }

    #[tokio::test]
    async fn test_stop_sampling() {
        let profiler = CpuProfiler::new();
        profiler.start_sampling().await.unwrap();

        let stack = vec![StackFrame {
            function_name: "compute".to_string(),
            module_name: "lib".to_string(),
            line_number: 10,
            offset: 0x2000,
        }];

        profiler.record_sample(15000, stack).await.unwrap();

        let report = profiler.stop_sampling().await.unwrap();
        assert_eq!(report.total_samples, 1);
        assert!(report.duration_ms >= 15);
    }

    #[tokio::test]
    async fn test_stop_sampling_computes_hotspots_from_real_samples() {
        let profiler = CpuProfiler::new();
        profiler.start_sampling().await.unwrap();

        profiler
            .record_sample(
                1000,
                vec![StackFrame {
                    function_name: "hot_fn".to_string(),
                    module_name: "lib".to_string(),
                    line_number: 1,
                    offset: 0,
                }],
            )
            .await
            .unwrap();
        profiler
            .record_sample(
                10,
                vec![StackFrame {
                    function_name: "cold_fn".to_string(),
                    module_name: "lib".to_string(),
                    line_number: 2,
                    offset: 0,
                }],
            )
            .await
            .unwrap();

        let report = profiler.stop_sampling().await.unwrap();
        assert_eq!(report.hotspots.first().unwrap().function_name, "hot_fn");
    }
}
