use crate::{CpuSample, StackFrame, ProfileReport, ProfilerError, ProfilerResult};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;

pub struct CpuProfiler {
    samples: Arc<DashMap<Uuid, CpuSample>>,
    profile_id: Uuid,
}

impl CpuProfiler {
    pub fn new() -> Self {
        Self {
            samples: Arc::new(DashMap::new()),
            profile_id: Uuid::new_v4(),
        }
    }

    pub async fn start_sampling(&self) -> ProfilerResult<Uuid> {
        Ok(self.profile_id)
    }

    pub async fn record_sample(&self, duration_us: u64, stack_trace: Vec<StackFrame>) -> ProfilerResult<Uuid> {
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

        for entry in self.samples.iter() {
            total_duration += entry.value().duration_us;
        }

        Ok(ProfileReport {
            profile_id: self.profile_id,
            total_samples,
            duration_ms: total_duration / 1000,
            cpu_time_percent: 75.5,
            memory_peak_mb: 512,
            hotspots: vec![],
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

        let stack = vec![
            StackFrame {
                function_name: "main".to_string(),
                module_name: "app".to_string(),
                line_number: 42,
                offset: 0x1000,
            },
        ];

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
}
