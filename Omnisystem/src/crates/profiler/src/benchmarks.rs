use criterion::Criterion;

/// Benchmark utilities for performance measurement
pub struct BenchmarkRunner {
    results: Vec<BenchmarkResult>,
}

#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub name: String,
    pub mean_ns: f64,
    pub std_dev_ns: f64,
}

impl BenchmarkRunner {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
        }
    }

    /// Add a benchmark result
    pub fn add_result(&mut self, result: BenchmarkResult) {
        self.results.push(result);
    }

    /// Get all results
    pub fn results(&self) -> &[BenchmarkResult] {
        &self.results
    }

    /// Compare against baseline
    pub fn compare_with_baseline(&self, baseline: &str) -> Vec<(String, f64)> {
        let baseline_result = self.results.iter().find(|r| r.name == baseline);

        if let Some(baseline) = baseline_result {
            self.results
                .iter()
                .filter(|r| r.name != baseline.name)
                .map(|r| {
                    let diff_percent = (r.mean_ns - baseline.mean_ns) / baseline.mean_ns * 100.0;
                    (r.name.clone(), diff_percent)
                })
                .collect()
        } else {
            Vec::new()
        }
    }
}

impl Default for BenchmarkRunner {
    fn default() -> Self {
        Self::new()
    }
}

/// Setup Criterion for benchmarking
pub fn setup_criterion() -> Criterion {
    Criterion::default()
        .sample_size(100)
        .warm_up_time(std::time::Duration::from_secs(1))
        .measurement_time(std::time::Duration::from_secs(5))
}
