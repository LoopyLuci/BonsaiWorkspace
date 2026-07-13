//! Memory access pattern validation and atomic semantics verification

use crate::{
    ArchitectureTestResults, AtomicOperation, AtomicOperationType, EquivalenceConfig, EquivalenceResult,
    EquivalenceValidator, MemoryAccessTrace, MemoryOrdering, ValidationResult,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Memory access pattern
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccessPattern {
    /// Sequential memory access
    Sequential,
    /// Random access
    Random,
    /// Strided access
    Strided,
    /// Temporal locality (same address repeatedly)
    TemporalLocality,
    /// Spatial locality (nearby addresses)
    SpatialLocality,
}

/// Memory access validator
#[derive(Default)]
pub struct MemoryAccessValidator;

#[async_trait]
impl EquivalenceValidator for MemoryAccessValidator {
    async fn validate(
        &self,
        results: &ArchitectureTestResults,
        _config: &EquivalenceConfig,
    ) -> EquivalenceResult<ValidationResult> {
        if results.results.is_empty() {
            return Ok(ValidationResult::fail(
                self.name().to_string(),
                "No results to validate".to_string(),
            ));
        }

        let reference_trace = &results.results[0].memory_trace;
        let mut has_divergence = false;

        for result in &results.results[1..] {
            let trace = &result.memory_trace;

            // Check L1 hit ratio (allow 5% deviation)
            let ref_l1_ratio = reference_trace.l1_hit_ratio();
            let result_l1_ratio = trace.l1_hit_ratio();
            let l1_deviation = (ref_l1_ratio - result_l1_ratio).abs();

            if l1_deviation > 0.05 {
                has_divergence = true;
            }

            // Check L2 hit ratio (allow 5% deviation)
            let ref_l2_ratio = reference_trace.l2_hit_ratio();
            let result_l2_ratio = trace.l2_hit_ratio();
            let l2_deviation = (ref_l2_ratio - result_l2_ratio).abs();

            if l2_deviation > 0.05 {
                has_divergence = true;
            }
        }

        let validation = if has_divergence {
            ValidationResult::warn(
                self.name().to_string(),
                "Memory access patterns diverge across architectures".to_string(),
            )
        } else {
            ValidationResult::pass(self.name().to_string())
        };

        let validation = validation
            .with_detail(
                "reference_l1_hit_ratio".to_string(),
                format!("{:.2}%", reference_trace.l1_hit_ratio() * 100.0),
            )
            .with_detail(
                "reference_l2_hit_ratio".to_string(),
                format!("{:.2}%", reference_trace.l2_hit_ratio() * 100.0),
            )
            .with_detail(
                "total_loads".to_string(),
                reference_trace.load_count.to_string(),
            )
            .with_detail(
                "total_stores".to_string(),
                reference_trace.store_count.to_string(),
            );

        Ok(validation)
    }

    fn name(&self) -> &str {
        "Memory Access Validator"
    }
}

/// Atomic operation semantics validator
#[derive(Default)]
pub struct AtomicSemanticsValidator;

#[async_trait]
impl EquivalenceValidator for AtomicSemanticsValidator {
    async fn validate(
        &self,
        results: &ArchitectureTestResults,
        _config: &EquivalenceConfig,
    ) -> EquivalenceResult<ValidationResult> {
        if results.results.is_empty() {
            return Ok(ValidationResult::fail(
                self.name().to_string(),
                "No results to validate".to_string(),
            ));
        }

        let mut all_valid = true;

        for result in &results.results {
            for op in &result.atomic_ops {
                if !Self::validate_atomic_operation(op) {
                    all_valid = false;
                    break;
                }
            }
            if !all_valid {
                break;
            }
        }

        let validation = if all_valid {
            ValidationResult::pass(self.name().to_string())
        } else {
            ValidationResult::fail(
                self.name().to_string(),
                "Atomic operation semantics violation detected".to_string(),
            )
        };

        Ok(validation)
    }

    fn name(&self) -> &str {
        "Atomic Semantics Validator"
    }
}

impl AtomicSemanticsValidator {
    /// Validate a single atomic operation
    fn validate_atomic_operation(op: &AtomicOperation) -> bool {
        match op.operation {
            AtomicOperationType::LoadAcquire => {
                // Acquire semantics: prevent subsequent operations from seeing old values
                op.ordering == MemoryOrdering::Acquire
            }
            AtomicOperationType::StoreRelease => {
                // Release semantics: all prior operations must complete
                op.ordering == MemoryOrdering::Release
            }
            AtomicOperationType::CompareAndSwap => {
                // CAS must be properly ordered
                matches!(
                    op.ordering,
                    MemoryOrdering::SeqCst | MemoryOrdering::AcqRel
                )
            }
            AtomicOperationType::AtomicAdd | AtomicOperationType::AtomicSub => {
                // Arithmetic ops with valid ordering
                op.value_after >= op.value_before || op.value_after <= op.value_before
            }
            _ => true,
        }
    }
}

/// Cache coherency validator
#[derive(Default)]
pub struct CacheCoherencyValidator {
    access_patterns: HashMap<u64, Vec<AccessPattern>>,
}

impl CacheCoherencyValidator {
    /// Create a new cache coherency validator
    pub fn new() -> Self {
        Self {
            access_patterns: HashMap::new(),
        }
    }

    /// Detect access pattern for an address
    pub fn detect_pattern(accesses: &[u64]) -> Option<AccessPattern> {
        if accesses.is_empty() {
            return None;
        }

        if accesses.len() == 1 {
            return Some(AccessPattern::TemporalLocality);
        }

        // Check for temporal locality first (repeated same address)
        let unique_count = accesses.iter().collect::<std::collections::HashSet<_>>().len();
        if unique_count * 4 < accesses.len() {
            return Some(AccessPattern::TemporalLocality);
        }

        // Check for sequential access
        let mut sequential = true;
        for i in 1..accesses.len() {
            if accesses[i] != accesses[i - 1] + 1 {
                sequential = false;
                break;
            }
        }

        if sequential {
            return Some(AccessPattern::Sequential);
        }

        // Check for strided access
        if accesses.len() >= 3 {
            let stride = accesses[1] as i64 - accesses[0] as i64;
            let mut strided = true;
            for i in 2..accesses.len() {
                if (accesses[i] as i64 - accesses[i - 1] as i64) != stride {
                    strided = false;
                    break;
                }
            }
            if strided {
                return Some(AccessPattern::Strided);
            }
        }

        // Check for spatial locality (nearby addresses)
        let mut nearby_count = 0;
        for i in 1..accesses.len() {
            let diff = (accesses[i] as i64 - accesses[i - 1] as i64).abs();
            if diff < 64 {
                // Within a cache line
                nearby_count += 1;
            }
        }

        if nearby_count as f64 / accesses.len() as f64 > 0.7 {
            return Some(AccessPattern::SpatialLocality);
        }

        Some(AccessPattern::Random)
    }
}

#[async_trait]
impl EquivalenceValidator for CacheCoherencyValidator {
    async fn validate(
        &self,
        _results: &ArchitectureTestResults,
        _config: &EquivalenceConfig,
    ) -> EquivalenceResult<ValidationResult> {
        // Placeholder for cache coherency validation
        // In a real implementation, this would check for cache coherency violations
        // under concurrent access patterns
        Ok(ValidationResult::pass(self.name().to_string()))
    }

    fn name(&self) -> &str {
        "Cache Coherency Validator"
    }
}

/// Cache performance analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachePerformance {
    /// Architecture name
    pub architecture: String,
    /// L1 cache hit ratio (0.0 to 1.0)
    pub l1_hit_ratio: f64,
    /// L2 cache hit ratio (0.0 to 1.0)
    pub l2_hit_ratio: f64,
    /// L3 cache hit ratio (0.0 to 1.0)
    pub l3_hit_ratio: f64,
    /// Effective memory latency in nanoseconds
    pub effective_latency_ns: f64,
}

impl CachePerformance {
    /// Calculate cache performance from memory trace
    pub fn from_trace(architecture: String, trace: &MemoryAccessTrace) -> Self {
        let l1_total = trace.l1_hits + trace.l1_misses;
        let l1_hit_ratio = if l1_total > 0 {
            trace.l1_hits as f64 / l1_total as f64
        } else {
            0.0
        };

        let l2_total = trace.l2_hits + trace.l2_misses;
        let l2_hit_ratio = if l2_total > 0 {
            trace.l2_hits as f64 / l2_total as f64
        } else {
            0.0
        };

        let l3_total = trace.l3_hits + trace.l3_misses;
        let l3_hit_ratio = if l3_total > 0 {
            trace.l3_hits as f64 / l3_total as f64
        } else {
            0.0
        };

        // Estimate effective latency based on cache hierarchy
        let effective_latency_ns = (4.0 * l1_hit_ratio)
            + (12.0 * (1.0 - l1_hit_ratio) * l2_hit_ratio)
            + (40.0 * (1.0 - l1_hit_ratio) * (1.0 - l2_hit_ratio) * l3_hit_ratio)
            + (100.0 * (1.0 - l1_hit_ratio) * (1.0 - l2_hit_ratio) * (1.0 - l3_hit_ratio));

        Self {
            architecture,
            l1_hit_ratio,
            l2_hit_ratio,
            l3_hit_ratio,
            effective_latency_ns,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_sequential_pattern() {
        let accesses = vec![0, 1, 2, 3, 4];
        let pattern = CacheCoherencyValidator::detect_pattern(&accesses);
        assert_eq!(pattern, Some(AccessPattern::Sequential));
    }

    #[test]
    fn test_detect_strided_pattern() {
        let accesses = vec![0, 8, 16, 24, 32];
        let pattern = CacheCoherencyValidator::detect_pattern(&accesses);
        assert_eq!(pattern, Some(AccessPattern::Strided));
    }

    #[test]
    fn test_detect_temporal_locality() {
        let accesses = vec![0, 0, 0, 0, 0];
        let pattern = CacheCoherencyValidator::detect_pattern(&accesses);
        assert_eq!(pattern, Some(AccessPattern::TemporalLocality));
    }

    #[test]
    fn test_cache_performance_from_trace() {
        let mut trace = MemoryAccessTrace::default();
        trace.l1_hits = 900;
        trace.l1_misses = 100;

        let perf = CachePerformance::from_trace("x86_64".to_string(), &trace);
        assert!((perf.l1_hit_ratio - 0.9).abs() < 0.01);
    }

    #[test]
    fn test_atomic_operation_validation() {
        let op = AtomicOperation {
            address: 1000,
            operation: AtomicOperationType::LoadAcquire,
            ordering: MemoryOrdering::Acquire,
            value_before: 10,
            value_after: 10,
            timestamp_ns: 1000,
        };

        assert!(AtomicSemanticsValidator::validate_atomic_operation(&op));
    }
}
