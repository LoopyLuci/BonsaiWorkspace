use crate::{CpuSample, PerformanceMetric, FlameGraphNode, ProfilerError, ProfilerResult};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;

pub struct PerformanceAnalyzer {
    metrics: Arc<DashMap<Uuid, PerformanceMetric>>,
}

impl PerformanceAnalyzer {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(DashMap::new()),
        }
    }

    pub async fn analyze_samples(&self, samples: &[CpuSample]) -> ProfilerResult<Vec<PerformanceMetric>> {
        if samples.is_empty() {
            return Err(ProfilerError::InvalidSampleCount);
        }

        let mut metrics = Vec::new();

        let total_duration: u64 = samples.iter().map(|s| s.duration_us).sum();
        let avg_duration = total_duration as f64 / samples.len() as f64;

        let metric_id = Uuid::new_v4();
        let metric = PerformanceMetric {
            metric_id,
            name: "average_duration_us".to_string(),
            value: avg_duration,
            unit: "microseconds".to_string(),
            timestamp: chrono::Utc::now(),
        };

        self.metrics.insert(metric_id, metric.clone());
        metrics.push(metric);

        Ok(metrics)
    }

    pub async fn generate_flamegraph(&self, samples: &[CpuSample]) -> ProfilerResult<Vec<FlameGraphNode>> {
        if samples.is_empty() {
            return Err(ProfilerError::InvalidSampleCount);
        }

        let mut function_times: std::collections::HashMap<String, u64> = std::collections::HashMap::new();

        for sample in samples {
            for frame in &sample.stack_trace {
                *function_times.entry(frame.function_name.clone()).or_insert(0) += sample.duration_us;
            }
        }

        let total_time: u64 = function_times.values().sum();

        let nodes: Vec<FlameGraphNode> = function_times
            .into_iter()
            .map(|(name, time)| FlameGraphNode {
                function_name: name,
                time_percent: (time as f32 / total_time as f32) * 100.0,
                sample_count: 1,
            })
            .collect();

        Ok(nodes)
    }

    pub fn metric_count(&self) -> usize {
        self.metrics.len()
    }
}

impl Default for PerformanceAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_analyze_samples() {
        let analyzer = PerformanceAnalyzer::new();
        let samples = vec![
            CpuSample {
                sample_id: Uuid::new_v4(),
                timestamp: chrono::Utc::now(),
                stack_trace: vec![],
                duration_us: 100,
            },
            CpuSample {
                sample_id: Uuid::new_v4(),
                timestamp: chrono::Utc::now(),
                stack_trace: vec![],
                duration_us: 200,
            },
        ];

        let metrics = analyzer.analyze_samples(&samples).await.unwrap();
        assert!(!metrics.is_empty());
    }

    #[tokio::test]
    async fn test_generate_flamegraph() {
        let analyzer = PerformanceAnalyzer::new();
        let samples = vec![CpuSample {
            sample_id: Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            stack_trace: vec![
                crate::StackFrame {
                    function_name: "foo".to_string(),
                    module_name: "lib".to_string(),
                    line_number: 1,
                    offset: 0,
                },
            ],
            duration_us: 150,
        }];

        let nodes = analyzer.generate_flamegraph(&samples).await.unwrap();
        assert!(!nodes.is_empty());
    }
}
