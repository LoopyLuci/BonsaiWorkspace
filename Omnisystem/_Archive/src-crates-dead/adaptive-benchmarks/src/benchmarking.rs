use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Configuration for running benchmarks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkConfig {
    pub name: String,
    pub description: String,
    pub scales: Vec<u32>,
    pub batch_sizes: Vec<usize>,
    pub sequence_lengths: Vec<usize>,
    pub devices: Vec<String>,
    pub num_runs: usize,
    pub warmup_runs: usize,
    pub output_dir: PathBuf,
    pub baseline_version: String,
    pub enable_profiling: bool,
    pub enable_memory_tracking: bool,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            name: "default_benchmark".to_string(),
            description: "Default benchmark configuration".to_string(),
            scales: vec![
                100_000_000,
                500_000_000,
                1_000_000_000,
                7_000_000_000,
                30_000_000_000,
                100_000_000_000,
            ],
            batch_sizes: vec![1, 8, 32],
            sequence_lengths: vec![128, 512, 2048],
            devices: vec!["cpu".to_string(), "gpu".to_string()],
            num_runs: 5,
            warmup_runs: 2,
            output_dir: PathBuf::from("./benchmark_results"),
            baseline_version: "v0.1.0".to_string(),
            enable_profiling: false,
            enable_memory_tracking: true,
        }
    }
}

/// Benchmark runner orchestrating all tests
pub struct BenchmarkRunner {
    pub config: BenchmarkConfig,
    pub results: HashMap<String, BenchmarkResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub name: String,
    pub scale: u32,
    pub batch_size: usize,
    pub sequence_length: usize,
    pub device: String,
    pub runs: Vec<SingleRun>,
    pub stats: RunStatistics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SingleRun {
    pub run_id: usize,
    pub latency_ms: f32,
    pub throughput: f32,
    pub memory_peak_mb: f32,
    pub power_draw_watts: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunStatistics {
    pub mean_latency_ms: f32,
    pub std_dev_latency_ms: f32,
    pub min_latency_ms: f32,
    pub max_latency_ms: f32,
    pub p50_latency_ms: f32,
    pub p95_latency_ms: f32,
    pub p99_latency_ms: f32,
    pub mean_throughput: f32,
    pub mean_memory_mb: f32,
    pub cv_coefficient: f32,  // Coefficient of variation
}

impl BenchmarkRunner {
    pub fn new(config: BenchmarkConfig) -> Self {
        Self {
            config,
            results: HashMap::new(),
        }
    }

    /// Run comprehensive benchmark suite
    pub async fn run_all(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        for &scale in &self.config.scales.clone() {
            for &batch_size in &self.config.batch_sizes.clone() {
                for &seq_len in &self.config.sequence_lengths.clone() {
                    for device in &self.config.devices.clone() {
                        self.run_benchmark_config(scale, batch_size, seq_len, device)
                            .await?;
                    }
                }
            }
        }

        // Save results
        self.save_results().await?;
        Ok(())
    }

    async fn run_benchmark_config(
        &mut self,
        scale: u32,
        batch_size: usize,
        seq_len: usize,
        device: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let key = format!("{}_{}_{}_{}", scale, batch_size, seq_len, device);

        // Warmup runs
        for _ in 0..self.config.warmup_runs {
            self.simulate_inference(scale, batch_size, seq_len).await?;
        }

        // Actual runs
        let mut runs = Vec::new();
        for run_id in 0..self.config.num_runs {
            let (latency, throughput, memory) = self.simulate_inference(scale, batch_size, seq_len).await?;

            runs.push(SingleRun {
                run_id,
                latency_ms: latency,
                throughput,
                memory_peak_mb: memory,
                power_draw_watts: None,
            });
        }

        let stats = self.compute_statistics(&runs);

        self.results.insert(
            key,
            BenchmarkResult {
                name: format!("{}_{}_{}_{}", scale, batch_size, seq_len, device),
                scale,
                batch_size,
                sequence_length: seq_len,
                device: device.to_string(),
                runs,
                stats,
            },
        );

        Ok(())
    }

    async fn simulate_inference(
        &self,
        _scale: u32,
        _batch_size: usize,
        _seq_len: usize,
    ) -> Result<(f32, f32, f32), Box<dyn std::error::Error>> {
        // Mock: simulate inference timing
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        Ok((
            15.5,   // latency ms
            64.5,   // throughput tokens/sec
            2048.0, // memory mb
        ))
    }

    fn compute_statistics(&self, runs: &[SingleRun]) -> RunStatistics {
        if runs.is_empty() {
            return RunStatistics::default();
        }

        let latencies: Vec<f32> = runs.iter().map(|r| r.latency_ms).collect();
        let throughputs: Vec<f32> = runs.iter().map(|r| r.throughput).collect();
        let memory: Vec<f32> = runs.iter().map(|r| r.memory_peak_mb).collect();

        let mean_latency = latencies.iter().sum::<f32>() / latencies.len() as f32;
        let mean_throughput = throughputs.iter().sum::<f32>() / throughputs.len() as f32;
        let mean_memory = memory.iter().sum::<f32>() / memory.len() as f32;

        // Standard deviation
        let variance = latencies.iter()
            .map(|l| (l - mean_latency).powi(2))
            .sum::<f32>() / latencies.len() as f32;
        let std_dev = variance.sqrt();

        // Percentiles (simplified)
        let mut sorted_latencies = latencies.clone();
        sorted_latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let p50_idx = sorted_latencies.len() / 2;
        let p95_idx = (sorted_latencies.len() * 95) / 100;
        let p99_idx = (sorted_latencies.len() * 99) / 100;

        let cv = if mean_latency.abs() > 1e-9 {
            std_dev / mean_latency
        } else {
            0.0
        };

        RunStatistics {
            mean_latency_ms: mean_latency,
            std_dev_latency_ms: std_dev,
            min_latency_ms: sorted_latencies[0],
            max_latency_ms: sorted_latencies[sorted_latencies.len() - 1],
            p50_latency_ms: sorted_latencies[p50_idx],
            p95_latency_ms: sorted_latencies[p95_idx.min(sorted_latencies.len() - 1)],
            p99_latency_ms: sorted_latencies[p99_idx.min(sorted_latencies.len() - 1)],
            mean_throughput,
            mean_memory_mb: mean_memory,
            cv_coefficient: cv,
        }
    }

    async fn save_results(&self) -> Result<(), Box<dyn std::error::Error>> {
        std::fs::create_dir_all(&self.config.output_dir)?;

        let results_path = self.config.output_dir.join("results.json");
        let json = serde_json::to_string_pretty(&self.results)?;
        std::fs::write(results_path, json)?;

        Ok(())
    }

    pub fn generate_report(&self) -> BenchmarkReport {
        let mut scale_summaries = HashMap::new();

        for (key, result) in &self.results {
            scale_summaries
                .entry(result.scale)
                .or_insert_with(Vec::new)
                .push(result.clone());
        }

        BenchmarkReport {
            config: self.config.clone(),
            results_count: self.results.len(),
            scale_summaries,
            timestamp: chrono::Local::now().to_rfc3339(),
        }
    }

    pub fn find_bottlenecks(&self) -> Vec<Bottleneck> {
        let mut bottlenecks = Vec::new();

        for result in self.results.values() {
            let cv = result.stats.cv_coefficient;

            if cv > 0.2 {
                bottlenecks.push(Bottleneck {
                    location: format!("scale_{}_batch_{}", result.scale, result.batch_size),
                    reason: format!("High variability: CV = {:.2}", cv),
                    severity: if cv > 0.5 { "high" } else { "medium" }.to_string(),
                    metric_affected: "latency".to_string(),
                });
            }

            if result.stats.mean_memory_mb > 8000.0 {
                bottlenecks.push(Bottleneck {
                    location: format!("scale_{}_seq_{}", result.scale, result.sequence_length),
                    reason: "High memory usage".to_string(),
                    severity: "high".to_string(),
                    metric_affected: "memory".to_string(),
                });
            }
        }

        bottlenecks
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkReport {
    pub config: BenchmarkConfig,
    pub results_count: usize,
    pub scale_summaries: HashMap<u32, Vec<BenchmarkResult>>,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bottleneck {
    pub location: String,
    pub reason: String,
    pub severity: String,
    pub metric_affected: String,
}

impl Default for RunStatistics {
    fn default() -> Self {
        Self {
            mean_latency_ms: 0.0,
            std_dev_latency_ms: 0.0,
            min_latency_ms: 0.0,
            max_latency_ms: 0.0,
            p50_latency_ms: 0.0,
            p95_latency_ms: 0.0,
            p99_latency_ms: 0.0,
            mean_throughput: 0.0,
            mean_memory_mb: 0.0,
            cv_coefficient: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_config_default() {
        let config = BenchmarkConfig::default();
        assert!(!config.scales.is_empty());
        assert!(!config.batch_sizes.is_empty());
    }

    #[test]
    fn test_run_statistics_computation() {
        let runs = vec![
            SingleRun {
                run_id: 0,
                latency_ms: 10.0,
                throughput: 100.0,
                memory_peak_mb: 2048.0,
                power_draw_watts: None,
            },
            SingleRun {
                run_id: 1,
                latency_ms: 12.0,
                throughput: 95.0,
                memory_peak_mb: 2100.0,
                power_draw_watts: None,
            },
        ];

        let runner = BenchmarkRunner::new(BenchmarkConfig::default());
        let stats = runner.compute_statistics(&runs);

        assert!(stats.mean_latency_ms > 10.0);
        assert!(stats.std_dev_latency_ms > 0.0);
    }

    #[tokio::test]
    async fn test_benchmark_runner_creation() {
        let config = BenchmarkConfig::default();
        let runner = BenchmarkRunner::new(config);
        assert!(runner.results.is_empty());
    }

    #[test]
    fn test_report_generation() {
        let mut runner = BenchmarkRunner::new(BenchmarkConfig::default());

        // Add a sample result
        runner.results.insert(
            "test_key".to_string(),
            BenchmarkResult {
                name: "test".to_string(),
                scale: 100_000_000,
                batch_size: 1,
                sequence_length: 512,
                device: "cpu".to_string(),
                runs: vec![],
                stats: RunStatistics::default(),
            },
        );

        let report = runner.generate_report();
        assert_eq!(report.results_count, 1);
    }
}

// Add chrono for timestamps
use chrono;
