//! Performance validation across architectures
//!
//! Validates that performance is within expected tolerances, accounting for
//! architecture-specific clock frequencies and capabilities.

use crate::{
    ArchitectureTarget, ArchitectureTestResults, EquivalenceConfig, EquivalenceResult,
    EquivalenceValidator, ValidationResult,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Performance profile for an architecture
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceProfile {
    /// Architecture identifier
    pub architecture: String,
    /// Baseline latency in nanoseconds
    pub baseline_ns: u64,
    /// Normalized latency (adjusted for clock frequency)
    pub normalized_ns: u64,
    /// Actual measured time
    pub measured_ns: u64,
    /// Deviation from baseline
    pub deviation_percent: f64,
    /// Whether this is within tolerance
    pub within_tolerance: bool,
}

/// Performance validator
#[derive(Default)]
pub struct PerformanceValidator {
    profiles: HashMap<String, PerformanceProfile>,
}

impl PerformanceValidator {
    /// Create a new performance validator
    pub fn new() -> Self {
        Self {
            profiles: HashMap::new(),
        }
    }

    /// Normalize execution time from one architecture to another
    fn normalize_time(arch: &ArchitectureTarget, time_ns: u64) -> u64 {
        let cpu_freq = arch.cpu_frequency_mhz() as f64;
        let ref_freq = 3600.0; // Reference frequency (Skylake)
        ((time_ns as f64) * (ref_freq / cpu_freq)) as u64
    }

    /// Calculate acceptable latency bounds
    fn calculate_bounds(
        baseline_ns: u64,
        tolerance_percent: f64,
    ) -> (u64, u64) {
        let tolerance_ns = (baseline_ns as f64 * tolerance_percent / 100.0) as u64;
        (
            baseline_ns.saturating_sub(tolerance_ns),
            baseline_ns.saturating_add(tolerance_ns),
        )
    }
}

#[async_trait]
impl EquivalenceValidator for PerformanceValidator {
    async fn validate(
        &self,
        results: &ArchitectureTestResults,
        config: &EquivalenceConfig,
    ) -> EquivalenceResult<ValidationResult> {
        if results.results.is_empty() {
            return Ok(ValidationResult::fail(
                self.name().to_string(),
                "No results to validate".to_string(),
            ));
        }

        // Get reference (first) result
        let reference = &results.results[0];
        let reference_normalized = Self::normalize_time(&reference.architecture, reference.exec_time_ns);

        let mut has_outliers = false;
        let mut profiles = HashMap::new();

        for result in &results.results {
            let measured = result.exec_time_ns;
            let normalized = Self::normalize_time(&result.architecture, measured);
            let deviation = if reference_normalized > 0 {
                ((normalized as f64 - reference_normalized as f64) / reference_normalized as f64) * 100.0
            } else {
                0.0
            };

            let (lower_bound, upper_bound) =
                Self::calculate_bounds(reference_normalized, config.performance_tolerance_percent);
            let within_tolerance = normalized >= lower_bound && normalized <= upper_bound;

            if !within_tolerance {
                has_outliers = true;
            }

            profiles.insert(
                result.architecture.to_string(),
                PerformanceProfile {
                    architecture: result.architecture.to_string(),
                    baseline_ns: reference_normalized,
                    normalized_ns: normalized,
                    measured_ns: measured,
                    deviation_percent: deviation,
                    within_tolerance,
                },
            );
        }

        let mut validation = if has_outliers {
            ValidationResult::warn(
                self.name().to_string(),
                "Some architectures have performance deviation".to_string(),
            )
        } else {
            ValidationResult::pass(self.name().to_string())
        };

        // Add profile details
        for (arch, profile) in profiles {
            validation = ValidationResult {
                validator_name: validation.validator_name.clone(),
                status: validation.status,
                message: validation.message.clone(),
                details: {
                    let mut d = validation.details.clone();
                    d.insert(
                        format!("{}_normalized_ns", arch),
                        profile.normalized_ns.to_string(),
                    );
                    d.insert(
                        format!("{}_deviation_percent", arch),
                        format!("{:.2}", profile.deviation_percent),
                    );
                    d
                },
                is_critical: validation.is_critical,
            };
        }

        Ok(validation)
    }

    fn name(&self) -> &str {
        "Performance Validator"
    }
}

/// Latency characterization for different operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyCharacterization {
    /// Operation name
    pub operation: String,
    /// Architecture-specific latencies in nanoseconds
    pub latencies: HashMap<String, u64>,
    /// Average latency across all architectures
    pub average_ns: u64,
    /// Minimum latency
    pub min_ns: u64,
    /// Maximum latency
    pub max_ns: u64,
    /// Variance
    pub variance: f64,
}

impl LatencyCharacterization {
    /// Create a new latency characterization
    pub fn new(operation: String) -> Self {
        Self {
            operation,
            latencies: HashMap::new(),
            average_ns: 0,
            min_ns: 0,
            max_ns: 0,
            variance: 0.0,
        }
    }

    /// Add a latency measurement
    pub fn add_measurement(&mut self, arch: String, latency_ns: u64) {
        self.latencies.insert(arch, latency_ns);
        self.recalculate_stats();
    }

    /// Recalculate statistics
    fn recalculate_stats(&mut self) {
        if self.latencies.is_empty() {
            return;
        }

        let values: Vec<u64> = self.latencies.values().copied().collect();

        // Calculate average
        self.average_ns = values.iter().sum::<u64>() / values.len() as u64;

        // Calculate min/max
        self.min_ns = *values.iter().min().unwrap_or(&0);
        self.max_ns = *values.iter().max().unwrap_or(&0);

        // Calculate variance
        let variance_sum: f64 = values
            .iter()
            .map(|&v| {
                let diff = v as f64 - self.average_ns as f64;
                diff * diff
            })
            .sum();
        self.variance = variance_sum / values.len() as f64;
    }

    /// Get variance as standard deviation
    pub fn standard_deviation(&self) -> f64 {
        self.variance.sqrt()
    }

    /// Get coefficient of variation
    pub fn coefficient_of_variation(&self) -> f64 {
        if self.average_ns == 0 {
            return 0.0;
        }
        (self.standard_deviation() / self.average_ns as f64) * 100.0
    }
}

/// Performance characterization suite
pub struct PerformanceCharacterization {
    /// Per-operation characterizations
    pub operations: HashMap<String, LatencyCharacterization>,
}

impl PerformanceCharacterization {
    /// Create a new characterization
    pub fn new() -> Self {
        Self {
            operations: HashMap::new(),
        }
    }

    /// Add a characterization for an operation
    pub fn add_operation(&mut self, operation: String, char: LatencyCharacterization) {
        self.operations.insert(operation, char);
    }

    /// Get performance profile baseline
    pub fn get_baseline(&self, operation: &str, arch: &ArchitectureTarget) -> Option<u64> {
        self.operations
            .get(operation)
            .and_then(|char| char.latencies.get(&arch.to_string()))
            .copied()
    }

    /// Check if all operations meet tolerance
    pub fn all_operations_within_tolerance(&self, tolerance_percent: f64) -> bool {
        self.operations.values().all(|char| {
            let range = char.max_ns - char.min_ns;
            let tolerance = (char.average_ns as f64 * tolerance_percent / 100.0) as u64;
            range <= tolerance
        })
    }
}

impl Default for PerformanceCharacterization {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ArchVariant;

    #[test]
    fn test_normalize_time() {
        let skylake = ArchitectureTarget::X86_64(ArchVariant::Skylake);
        let time = 1000u64;

        // Skylake is 3600 MHz (reference), so normalized should be same
        let normalized = PerformanceValidator::normalize_time(&skylake, time);
        assert_eq!(normalized, time);
    }

    #[test]
    fn test_normalize_time_different_freq() {
        let cortex = ArchitectureTarget::ARMv8(ArchVariant::CortexA72);
        let time = 1000u64;

        // Cortex-A72 is 1500 MHz, so normalized should be higher
        let normalized = PerformanceValidator::normalize_time(&cortex, time);
        assert!(normalized > time);
    }

    #[test]
    fn test_calculate_bounds() {
        let baseline = 1000u64;
        let (lower, upper) = PerformanceValidator::calculate_bounds(baseline, 10.0);

        assert_eq!(lower, 900);
        assert_eq!(upper, 1100);
    }

    #[test]
    fn test_latency_characterization() {
        let mut char = LatencyCharacterization::new("read".to_string());

        char.add_measurement("x86_64".to_string(), 100);
        char.add_measurement("armv8".to_string(), 110);
        char.add_measurement("riscv64".to_string(), 95);

        assert_eq!(char.average_ns, 101); // (100 + 110 + 95) / 3
        assert_eq!(char.min_ns, 95);
        assert_eq!(char.max_ns, 110);
        assert!(char.variance > 0.0);
    }

    #[test]
    fn test_latency_std_deviation() {
        let mut char = LatencyCharacterization::new("write".to_string());

        char.add_measurement("x86_64".to_string(), 100);
        char.add_measurement("armv8".to_string(), 100);
        char.add_measurement("riscv64".to_string(), 100);

        assert_eq!(char.standard_deviation(), 0.0);
    }

    #[test]
    fn test_performance_characterization() {
        let mut perf = PerformanceCharacterization::new();

        let mut read_char = LatencyCharacterization::new("read".to_string());
        read_char.add_measurement("x86_64-skylake".to_string(), 100);

        perf.add_operation("read".to_string(), read_char);

        let baseline = perf.get_baseline("read", &ArchitectureTarget::X86_64(ArchVariant::Skylake));
        assert_eq!(baseline, Some(100));
    }
}
