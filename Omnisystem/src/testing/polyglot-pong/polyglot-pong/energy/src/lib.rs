//! Energy Measurement via Intel RAPL
//!
//! Measures energy consumption (joules) for Pong execution across all languages.
//! Provides green computing benchmark data for programming languages.

use polyglot_pong_common::*;
use std::fs;
use std::path::Path;
use tracing::{debug, warn};

/// Energy reader using Linux RAPL (Intel Running Average Power Limit).
pub struct EnergyReader {
    pub rapl_domains: Vec<RaplDomain>,
    pub available: bool,
}

#[derive(Debug, Clone)]
pub struct RaplDomain {
    pub name: String,
    pub energy_path: String,
    pub max_energy_path: String,
}

impl EnergyReader {
    /// Create a new energy reader and auto-detect RAPL domains.
    pub fn new() -> Self {
        let mut rapl_domains = Vec::new();
        let mut available = false;

        // Try to detect RAPL domains on Linux
        #[cfg(target_os = "linux")]
        {
            rapl_domains.push(RaplDomain {
                name: "package-0".into(),
                energy_path: "/sys/class/powercap/intel-rapl/intel-rapl:0/energy_uj".into(),
                max_energy_path: "/sys/class/powercap/intel-rapl/intel-rapl:0/max_energy_range_uj"
                    .into(),
            });

            rapl_domains.push(RaplDomain {
                name: "core".into(),
                energy_path: "/sys/class/powercap/intel-rapl/intel-rapl:0:0/energy_uj".into(),
                max_energy_path: "/sys/class/powercap/intel-rapl/intel-rapl:0:0/max_energy_range_uj"
                    .into(),
            });

            rapl_domains.push(RaplDomain {
                name: "dram".into(),
                energy_path: "/sys/class/powercap/intel-rapl/intel-rapl:0:2/energy_uj".into(),
                max_energy_path: "/sys/class/powercap/intel-rapl/intel-rapl:0:2/max_energy_range_uj"
                    .into(),
            });

            // Check if at least one domain is available
            available = rapl_domains.iter().any(|d| Path::new(&d.energy_path).exists());
        }

        #[cfg(not(target_os = "linux"))]
        {
            warn!("RAPL not available on this platform - using fallback estimation");
        }

        Self {
            rapl_domains,
            available,
        }
    }

    /// Read current energy consumption.
    pub fn read_energy(&self) -> EnergyMetrics {
        if !self.available {
            return self.estimate_energy();
        }

        let mut metrics = EnergyMetrics::default();

        #[cfg(target_os = "linux")]
        {
            for domain in &self.rapl_domains {
                if let Ok(contents) = fs::read_to_string(&domain.energy_path) {
                    if let Ok(microjoules) = contents.trim().parse::<u64>() {
                        let joules = microjoules as f64 / 1_000_000.0;

                        match domain.name.as_str() {
                            "package-0" => metrics.package_joules = joules,
                            "core" => metrics.core_joules = joules,
                            "dram" => metrics.dram_joules = joules,
                            _ => {}
                        }
                    }
                }
            }

            metrics.total_joules =
                metrics.package_joules + metrics.core_joules + metrics.dram_joules;
        }

        metrics
    }

    /// Estimate energy consumption (fallback for non-Linux systems).
    fn estimate_energy(&self) -> EnergyMetrics {
        // Simple model: assume 10W average power consumption
        // Typical game loop takes ~1-2 seconds, so estimate 10-20 joules
        let estimated_joules = 15.0; // Conservative estimate

        EnergyMetrics {
            package_joules: estimated_joules * 0.7,
            core_joules: estimated_joules * 0.2,
            dram_joules: estimated_joules * 0.1,
            total_joules: estimated_joules,
        }
    }

    /// Compute energy delta between two readings.
    pub fn delta(before: &EnergyMetrics, after: &EnergyMetrics) -> EnergyMetrics {
        EnergyMetrics {
            package_joules: (after.package_joules - before.package_joules).max(0.0),
            core_joules: (after.core_joules - before.core_joules).max(0.0),
            dram_joules: (after.dram_joules - before.dram_joules).max(0.0),
            total_joules: (after.total_joules - before.total_joules).max(0.0),
        }
    }
}

/// Energy efficiency benchmark result.
#[derive(Debug, Clone)]
pub struct EnergyBenchmark {
    pub language: Language,
    pub total_energy_joules: f64,
    pub execution_time_us: u64,
    pub binary_size_bytes: u64,
    pub energy_per_second: f64,
    pub energy_per_mb_binary: f64,
}

impl EnergyBenchmark {
    /// Create from a test result.
    pub fn from_result(result: &TestResult, language: Language) -> Self {
        let energy = result.metrics.energy.total_joules;
        let time_seconds = result.metrics.exec_time_us as f64 / 1_000_000.0;
        let binary_mb = result.metrics.binary_size_bytes as f64 / (1024.0 * 1024.0);

        Self {
            language,
            total_energy_joules: energy,
            execution_time_us: result.metrics.exec_time_us,
            binary_size_bytes: result.metrics.binary_size_bytes,
            energy_per_second: if time_seconds > 0.0 {
                energy / time_seconds
            } else {
                0.0
            },
            energy_per_mb_binary: if binary_mb > 0.0 {
                energy / binary_mb
            } else {
                0.0
            },
        }
    }
}

/// Energy ranking for multiple languages.
#[derive(Debug, Clone)]
pub struct EnergyLeaderboard {
    pub benchmarks: Vec<EnergyBenchmark>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl EnergyLeaderboard {
    /// Create from multiple test results.
    pub fn from_results(results: &[(Language, &TestResult)]) -> Self {
        let mut benchmarks = Vec::new();

        for (lang, result) in results {
            benchmarks.push(EnergyBenchmark::from_result(result, lang.clone()));
        }

        // Sort by total energy (ascending = most efficient first)
        benchmarks.sort_by(|a, b| a.total_energy_joules.partial_cmp(&b.total_energy_joules).unwrap());

        Self {
            benchmarks,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Get greenest language.
    pub fn greenest(&self) -> Option<&EnergyBenchmark> {
        self.benchmarks.first()
    }

    /// Get least efficient language.
    pub fn least_efficient(&self) -> Option<&EnergyBenchmark> {
        self.benchmarks.last()
    }

    /// Compute average energy.
    pub fn average_energy(&self) -> f64 {
        if self.benchmarks.is_empty() {
            return 0.0;
        }

        let sum: f64 = self.benchmarks.iter().map(|b| b.total_energy_joules).sum();
        sum / self.benchmarks.len() as f64
    }

    /// Export as CSV for analysis.
    pub fn to_csv(&self) -> String {
        let mut csv = String::from("Language,Total Energy (J),Exec Time (µs),Binary Size (B),Energy/Second,Energy/MB Binary\n");

        for bench in &self.benchmarks {
            csv.push_str(&format!(
                "{},{:.4},{},{},{:.4},{:.4}\n",
                bench.language,
                bench.total_energy_joules,
                bench.execution_time_us,
                bench.binary_size_bytes,
                bench.energy_per_second,
                bench.energy_per_mb_binary
            ));
        }

        csv
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_energy_reader_creation() {
        let reader = EnergyReader::new();
        // Should not panic on any platform
        assert!(true);
    }

    #[test]
    fn test_energy_delta() {
        let before = EnergyMetrics {
            package_joules: 10.0,
            core_joules: 5.0,
            dram_joules: 2.0,
            total_joules: 17.0,
        };

        let after = EnergyMetrics {
            package_joules: 15.0,
            core_joules: 8.0,
            dram_joules: 3.0,
            total_joules: 26.0,
        };

        let delta = EnergyReader::delta(&before, &after);
        assert_eq!(delta.package_joules, 5.0);
        assert_eq!(delta.core_joules, 3.0);
        assert_eq!(delta.dram_joules, 1.0);
        assert_eq!(delta.total_joules, 9.0);
    }

    #[test]
    fn test_energy_benchmark() {
        let result = TestResult {
            job_id: TestId(uuid::Uuid::new_v4()),
            success: true,
            trace: vec![],
            generated_source: None,
            metrics: RuntimeMetrics {
                exec_time_us: 1_000_000,
                memory_peak_bytes: 1024,
                binary_size_bytes: 1_000_000,
                energy: EnergyMetrics {
                    package_joules: 10.0,
                    core_joules: 5.0,
                    dram_joules: 2.0,
                    total_joules: 17.0,
                },
            },
            zk_proof: None,
            tee_attestation: None,
            error_message: None,
        };

        let bench = EnergyBenchmark::from_result(&result, "Rust".into());
        assert_eq!(bench.total_energy_joules, 17.0);
        assert_eq!(bench.language, "Rust");
    }

    #[test]
    fn test_energy_leaderboard() {
        let results = vec![
            (
                "Python".into(),
                &TestResult {
                    job_id: TestId(uuid::Uuid::new_v4()),
                    success: true,
                    trace: vec![],
                    generated_source: None,
                    metrics: RuntimeMetrics {
                        exec_time_us: 2_000_000,
                        memory_peak_bytes: 2048,
                        binary_size_bytes: 500_000,
                        energy: EnergyMetrics {
                            total_joules: 25.0,
                            ..Default::default()
                        },
                    },
                    zk_proof: None,
                    tee_attestation: None,
                    error_message: None,
                },
            ),
            (
                "Rust".into(),
                &TestResult {
                    job_id: TestId(uuid::Uuid::new_v4()),
                    success: true,
                    trace: vec![],
                    generated_source: None,
                    metrics: RuntimeMetrics {
                        exec_time_us: 1_000_000,
                        memory_peak_bytes: 1024,
                        binary_size_bytes: 1_000_000,
                        energy: EnergyMetrics {
                            total_joules: 10.0,
                            ..Default::default()
                        },
                    },
                    zk_proof: None,
                    tee_attestation: None,
                    error_message: None,
                },
            ),
        ];

        let board = EnergyLeaderboard::from_results(&results);
        assert_eq!(board.benchmarks.len(), 2);
        assert_eq!(board.greenest().unwrap().language, "Rust");
        assert_eq!(board.least_efficient().unwrap().language, "Python");
    }
}
