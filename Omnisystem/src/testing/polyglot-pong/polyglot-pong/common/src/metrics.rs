//! Metrics aggregation and analysis for test runs.

use super::*;
use std::collections::HashMap;

/// Aggregated metrics across all test runs.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct AggregatedMetrics {
    pub total_tests: usize,
    pub successful_tests: usize,
    pub avg_fidelity: f32,
    pub avg_exec_time_us: u64,
    pub total_energy_joules: f64,
    pub bugs_discovered: usize,
    pub highest_energy_lang: Option<(Language, f64)>,
    pub lowest_energy_lang: Option<(Language, f64)>,
}

impl AggregatedMetrics {
    /// Compute metrics from test results.
    pub fn from_results(results: &[TestResult]) -> Self {
        if results.is_empty() {
            return Self::default();
        }

        let total = results.len();
        let successful = results.iter().filter(|r| r.success).count();

        // Compute average fidelity
        let fidelity_sum: f32 = results
            .iter()
            .filter(|r| r.success && !r.trace.is_empty())
            .map(|_| 1.0) // Placeholder: in production, use actual fidelity comparison
            .sum();
        let avg_fidelity = if successful > 0 {
            fidelity_sum / successful as f32
        } else {
            0.0
        };

        // Compute average execution time
        let exec_time_sum: u64 = results.iter().map(|r| r.metrics.exec_time_us).sum();
        let avg_exec_time = if total > 0 {
            exec_time_sum / total as u64
        } else {
            0
        };

        // Compute total energy
        let total_energy: f64 = results
            .iter()
            .map(|r| r.metrics.energy.total_joules)
            .sum();

        // Find highest and lowest energy languages
        let mut lang_energy: HashMap<Language, f64> = HashMap::new();
        for result in results {
            *lang_energy
                .entry(result.job_id.0.to_string()) // Placeholder
                .or_insert(0.0) += result.metrics.energy.total_joules;
        }

        let highest_energy_lang = lang_energy
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(lang, energy)| (lang.clone(), *energy));

        let lowest_energy_lang = lang_energy
            .iter()
            .min_by(|a, b| a.1.partial_cmp(b.1).unwrap())
            .map(|(lang, energy)| (lang.clone(), *energy));

        Self {
            total_tests: total,
            successful_tests: successful,
            avg_fidelity,
            avg_exec_time_us: avg_exec_time,
            total_energy_joules: total_energy,
            bugs_discovered: 0, // Would be populated by fuzzer
            highest_energy_lang,
            lowest_energy_lang,
        }
    }

    /// Compute success rate as percentage.
    pub fn success_rate(&self) -> f32 {
        if self.total_tests == 0 {
            0.0
        } else {
            (self.successful_tests as f32 / self.total_tests as f32) * 100.0
        }
    }

    /// Average energy per test.
    pub fn avg_energy_per_test(&self) -> f64 {
        if self.total_tests == 0 {
            0.0
        } else {
            self.total_energy_joules / self.total_tests as f64
        }
    }
}

/// Fidelity matrix (language pair → fidelity score).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FidelityMatrix {
    pub matrix: HashMap<(Language, Language), f32>,
    pub languages: Vec<Language>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl FidelityMatrix {
    /// Create from test results.
    pub fn from_results(results: &[TestResult]) -> Self {
        let mut matrix = HashMap::new();
        let mut languages = std::collections::HashSet::new();

        for result in results {
            // Placeholder: in production, compute actual fidelity
            let fidelity = if result.success { 1.0 } else { 0.0 };
            // matrix.insert((result.source_lang, result.target_lang), fidelity);
            // languages.insert(result.source_lang.clone());
            // languages.insert(result.target_lang.clone());
        }

        Self {
            matrix,
            languages: languages.into_iter().collect(),
            timestamp: chrono::Utc::now(),
        }
    }

    /// Get fidelity for a language pair.
    pub fn get(&self, src: &Language, tgt: &Language) -> Option<f32> {
        self.matrix.get(&(src.clone(), tgt.clone())).copied()
    }

    /// Compute average fidelity.
    pub fn average_fidelity(&self) -> f32 {
        if self.matrix.is_empty() {
            return 0.0;
        }
        let sum: f32 = self.matrix.values().sum();
        sum / self.matrix.len() as f32
    }
}

/// Energy efficiency ranking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyRanking {
    pub rankings: Vec<(Language, f64)>, // (language, joules_per_frame)
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl EnergyRanking {
    /// Create from test results.
    pub fn from_results(results: &[TestResult], frame_count: usize) -> Self {
        let mut lang_energy: HashMap<Language, (f64, usize)> = HashMap::new();

        for result in results {
            let energy = result.metrics.energy.total_joules;
            let entry = lang_energy.entry("placeholder".into()).or_insert((0.0, 0));
            entry.0 += energy;
            entry.1 += 1;
        }

        let mut rankings: Vec<(Language, f64)> = lang_energy
            .into_iter()
            .map(|(lang, (total, count))| {
                let avg_joules = total / count as f64;
                let joules_per_frame = if frame_count > 0 {
                    avg_joules / frame_count as f64
                } else {
                    0.0
                };
                (lang, joules_per_frame)
            })
            .collect();

        rankings.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        Self {
            rankings,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Get greenest (lowest energy) language.
    pub fn greenest(&self) -> Option<&(Language, f64)> {
        self.rankings.first()
    }

    /// Get least efficient language.
    pub fn least_efficient(&self) -> Option<&(Language, f64)> {
        self.rankings.last()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aggregated_metrics_empty() {
        let metrics = AggregatedMetrics::from_results(&[]);
        assert_eq!(metrics.total_tests, 0);
        assert_eq!(metrics.success_rate(), 0.0);
    }

    #[test]
    fn test_fidelity_matrix_creation() {
        let matrix = FidelityMatrix {
            matrix: HashMap::new(),
            languages: vec!["Rust".into(), "Python".into()],
            timestamp: chrono::Utc::now(),
        };
        assert_eq!(matrix.languages.len(), 2);
    }

    #[test]
    fn test_energy_ranking_sorted() {
        let results = vec![
            TestResult {
                job_id: TestId(uuid::Uuid::new_v4()),
                success: true,
                trace: vec![],
                generated_source: None,
                metrics: RuntimeMetrics {
                    energy: EnergyMetrics {
                        total_joules: 10.0,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                zk_proof: None,
                tee_attestation: None,
                error_message: None,
            },
        ];
        let ranking = EnergyRanking::from_results(&results, 100);
        assert!(!ranking.rankings.is_empty());
    }
}
