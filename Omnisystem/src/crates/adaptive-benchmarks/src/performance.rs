use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

/// Performance metrics for adaptive transformer at a given scale
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub scale: u32,
    pub batch_size: usize,
    pub sequence_length: usize,
    pub device: String,

    // Latency metrics
    pub ttft_ms: f32,  // Time to first token
    pub tpt_ms: f32,   // Time per token
    pub e2e_latency_ms: f32,  // End-to-end latency

    // Throughput metrics
    pub throughput_tokens_sec: f32,
    pub throughput_req_sec: f32,

    // Memory metrics
    pub peak_memory_gb: f32,
    pub kv_cache_memory_gb: f32,
    pub model_memory_gb: f32,

    // Energy metrics
    pub estimated_energy_wh: f32,
    pub power_draw_watts: f32,

    // Quality metrics
    pub perplexity: f32,
    pub mmlu_score: f32,
    pub humaneval_pass_at_1: f32,

    // Inference quality
    pub tokens_generated: usize,
    pub num_samples: usize,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            scale: 0,
            batch_size: 1,
            sequence_length: 512,
            device: "cpu".to_string(),
            ttft_ms: 0.0,
            tpt_ms: 0.0,
            e2e_latency_ms: 0.0,
            throughput_tokens_sec: 0.0,
            throughput_req_sec: 0.0,
            peak_memory_gb: 0.0,
            kv_cache_memory_gb: 0.0,
            model_memory_gb: 0.0,
            estimated_energy_wh: 0.0,
            power_draw_watts: 0.0,
            perplexity: 0.0,
            mmlu_score: 0.0,
            humaneval_pass_at_1: 0.0,
            tokens_generated: 0,
            num_samples: 0,
        }
    }
}

/// Benchmark configuration and runner
pub struct PerformanceBenchmark {
    pub scales: Vec<u32>,
    pub batch_sizes: Vec<usize>,
    pub sequence_lengths: Vec<usize>,
    pub devices: Vec<String>,
}

impl PerformanceBenchmark {
    pub fn new(
        scales: Vec<u32>,
        batch_sizes: Vec<usize>,
        sequence_lengths: Vec<usize>,
        devices: Vec<String>,
    ) -> Self {
        Self {
            scales,
            batch_sizes,
            sequence_lengths,
            devices,
        }
    }

    /// Run latency benchmark
    pub async fn benchmark_latency(&self) -> Vec<PerformanceMetrics> {
        let mut results = Vec::new();

        for &scale in &self.scales {
            for &batch_size in &self.batch_sizes {
                for &seq_len in &self.sequence_lengths {
                    for device in &self.devices {
                        let metrics = self.measure_latency(scale, batch_size, seq_len, device).await;
                        results.push(metrics);
                    }
                }
            }
        }

        results
    }

    /// Measure latency for a specific configuration
    pub async fn measure_latency(
        &self,
        scale: u32,
        batch_size: usize,
        seq_len: usize,
        device: &str,
    ) -> PerformanceMetrics {
        let start = Instant::now();

        // Mock: simulate inference
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let elapsed = start.elapsed().as_secs_f32() * 1000.0;

        let tokens_generated = 100;  // Mock
        let ttft_ms = elapsed / 3.0;
        let tpt_ms = elapsed / tokens_generated as f32;

        PerformanceMetrics {
            scale,
            batch_size,
            sequence_length: seq_len,
            device: device.to_string(),
            ttft_ms,
            tpt_ms,
            e2e_latency_ms: elapsed,
            throughput_tokens_sec: tokens_generated as f32 / (elapsed / 1000.0),
            throughput_req_sec: 1.0 / (elapsed / 1000.0),
            peak_memory_gb: self.estimate_memory(scale, batch_size, seq_len),
            kv_cache_memory_gb: self.estimate_kv_cache(seq_len),
            model_memory_gb: self.estimate_model_memory(scale),
            estimated_energy_wh: self.estimate_energy(elapsed, device),
            power_draw_watts: self.estimate_power(scale, device),
            perplexity: self.estimate_perplexity(scale),
            mmlu_score: self.estimate_mmlu(scale),
            humaneval_pass_at_1: self.estimate_humaneval(scale),
            tokens_generated,
            num_samples: 1,
        }
    }

    /// Benchmark memory usage
    pub async fn benchmark_memory(&self) -> HashMap<String, f32> {
        let mut results = HashMap::new();

        for &scale in &self.scales {
            let model_mem = self.estimate_model_memory(scale);
            results.insert(format!("model_memory_{}", scale), model_mem);

            for &seq_len in &self.sequence_lengths {
                let kv_mem = self.estimate_kv_cache(seq_len);
                results.insert(format!("kv_cache_{}_{}", scale, seq_len), kv_mem);
            }
        }

        results
    }

    /// Benchmark quality metrics (perplexity, MMLU, etc.)
    pub async fn benchmark_quality(&self) -> Vec<(u32, f32, f32, f32)> {
        let mut results = Vec::new();

        for &scale in &self.scales {
            let ppl = self.estimate_perplexity(scale);
            let mmlu = self.estimate_mmlu(scale);
            let humaneval = self.estimate_humaneval(scale);

            results.push((scale, ppl, mmlu, humaneval));
        }

        results
    }

    /// Benchmark end-to-end throughput
    pub async fn benchmark_throughput(&self) -> Vec<(u32, f32)> {
        let mut results = Vec::new();

        for &scale in &self.scales {
            let throughput = 1000.0 / self.estimate_latency(scale);
            results.push((scale, throughput));
        }

        results
    }

    // Helper methods for estimation

    fn estimate_memory(&self, scale: u32, batch_size: usize, seq_len: usize) -> f32 {
        let model_mem = (scale as f32) / 1_000_000_000.0 * 4.0;  // Rough: ~4 bytes per parameter
        let activation_mem = batch_size as f32 * seq_len as f32 * 256.0 * 4.0 / 1e9;
        let kv_cache_mem = batch_size as f32 * seq_len as f32 * 256.0 * 2.0 * 4.0 / 1e9;  // 2 for K and V

        model_mem + activation_mem + kv_cache_mem
    }

    fn estimate_kv_cache(&self, seq_len: usize) -> f32 {
        // 2 heads × head_dim × seq_len × 4 bytes
        (seq_len as f32 * 256.0 * 2.0 * 4.0) / 1e9
    }

    fn estimate_model_memory(&self, scale: u32) -> f32 {
        (scale as f32) / 1_000_000_000.0 * 4.0
    }

    fn estimate_latency(&self, scale: u32) -> f32 {
        // Roughly: latency increases with scale
        (scale as f32 / 100_000_000.0) * 10.0  // ~10ms per 100M parameters
    }

    fn estimate_energy(&self, latency_ms: f32, device: &str) -> f32 {
        let power_draw = match device {
            "cpu" => 50.0,
            "gpu" => 250.0,
            "tpu" => 150.0,
            _ => 100.0,
        };

        (latency_ms / 1000.0) * power_draw / 3600.0  // Convert to Wh
    }

    fn estimate_power(&self, scale: u32, device: &str) -> f32 {
        let base_power = match device {
            "cpu" => 50.0,
            "gpu" => 250.0,
            "tpu" => 150.0,
            _ => 100.0,
        };

        base_power + (scale as f32 / 1_000_000_000.0) * 10.0
    }

    fn estimate_perplexity(&self, scale: u32) -> f32 {
        // Rough scaling: larger models have lower perplexity
        100.0 / (1.0 + (scale as f32 / 100_000_000.0).log2())
    }

    fn estimate_mmlu(&self, scale: u32) -> f32 {
        // Rough scaling: 0-100 score
        25.0 + 50.0 * (scale as f32 / 100_000_000_000.0).min(1.0)
    }

    fn estimate_humaneval(&self, scale: u32) -> f32 {
        // Rough scaling: 0-100% pass rate
        (scale as f32 / 100_000_000_000.0).min(1.0) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_latency_measurement() {
        let benchmark = PerformanceBenchmark::new(
            vec![100_000_000],
            vec![1],
            vec![512],
            vec!["cpu".to_string()],
        );

        let metrics = benchmark.measure_latency(100_000_000, 1, 512, "cpu").await;
        assert!(metrics.ttft_ms > 0.0);
        assert!(metrics.tpt_ms > 0.0);
        assert!(metrics.throughput_tokens_sec > 0.0);
    }

    #[tokio::test]
    async fn test_memory_estimation() {
        let benchmark = PerformanceBenchmark::new(
            vec![100_000_000],
            vec![1],
            vec![512],
            vec!["cpu".to_string()],
        );

        let results = benchmark.benchmark_memory().await;
        assert!(!results.is_empty());
        assert!(results.values().all(|&v| v > 0.0));
    }

    #[tokio::test]
    async fn test_quality_scaling() {
        let benchmark = PerformanceBenchmark::new(
            vec![100_000_000, 1_000_000_000],
            vec![1],
            vec![512],
            vec!["cpu".to_string()],
        );

        let results = benchmark.benchmark_quality().await;
        assert_eq!(results.len(), 2);
        // Larger model should have lower perplexity
        assert!(results[0].1 > results[1].1);
    }

    #[tokio::test]
    async fn test_latency_baseline() {
        let benchmark = PerformanceBenchmark::new(
            vec![100_000_000, 7_000_000_000],
            vec![1],
            vec![512],
            vec!["cpu".to_string()],
        );

        let latency_small = benchmark.estimate_latency(100_000_000);
        let latency_large = benchmark.estimate_latency(7_000_000_000);

        // Larger model should take longer
        assert!(latency_large > latency_small);
    }
}
