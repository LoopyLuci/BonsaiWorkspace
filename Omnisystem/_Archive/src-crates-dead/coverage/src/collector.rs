use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Coverage result for a crate or target
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageResult {
    pub crate_name: String,
    pub lines_covered: usize,
    pub lines_total: usize,
    pub branch_coverage: f64,
    pub coverage_percent: f64,
    pub timestamp: DateTime<Utc>,
    pub files: Vec<FileCoverage>,
}

/// Coverage per file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCoverage {
    pub path: String,
    pub lines_covered: usize,
    pub lines_total: usize,
    pub coverage_percent: f64,
}

/// Collects coverage data from test runs
pub struct CoverageCollector {
    results: Arc<RwLock<Vec<CoverageResult>>>,
    by_crate: Arc<RwLock<HashMap<String, CoverageResult>>>,
}

impl CoverageCollector {
    pub fn new() -> Self {
        Self {
            results: Arc::new(RwLock::new(Vec::new())),
            by_crate: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Record coverage result for a crate
    pub fn record_crate(
        &self,
        crate_name: &str,
        lines_covered: usize,
        lines_total: usize,
        branch_coverage: f64,
        files: Vec<FileCoverage>,
    ) {
        let coverage_percent = if lines_total == 0 {
            100.0
        } else {
            (lines_covered as f64 / lines_total as f64) * 100.0
        };

        let result = CoverageResult {
            crate_name: crate_name.to_string(),
            lines_covered,
            lines_total,
            branch_coverage,
            coverage_percent,
            timestamp: Utc::now(),
            files,
        };

        self.results.write().push(result.clone());
        self.by_crate
            .write()
            .insert(crate_name.to_string(), result);

        tracing::info!(
            "Coverage recorded for {}: {:.2}%",
            crate_name,
            coverage_percent
        );
    }

    /// Get coverage for specific crate
    pub fn get_crate_coverage(&self, crate_name: &str) -> Option<CoverageResult> {
        self.by_crate.read().get(crate_name).cloned()
    }

    /// Get all coverage results
    pub fn get_all_results(&self) -> Vec<CoverageResult> {
        self.results.read().clone()
    }

    /// Calculate aggregate coverage across all crates
    pub fn get_aggregate_coverage(&self) -> AggregateCoverage {
        let results = self.results.read();

        if results.is_empty() {
            return AggregateCoverage {
                total_lines_covered: 0,
                total_lines: 0,
                overall_coverage_percent: 0.0,
                crate_count: 0,
                average_coverage_percent: 0.0,
                lowest_coverage_crate: None,
                lowest_coverage_percent: 100.0,
            };
        }

        let total_covered: usize = results.iter().map(|r| r.lines_covered).sum();
        let total_lines: usize = results.iter().map(|r| r.lines_total).sum();

        let overall = if total_lines == 0 {
            100.0
        } else {
            (total_covered as f64 / total_lines as f64) * 100.0
        };

        let average = results.iter().map(|r| r.coverage_percent).sum::<f64>() / results.len() as f64;

        let (lowest_crate, lowest_percent) = results
            .iter()
            .min_by(|a, b| a.coverage_percent.partial_cmp(&b.coverage_percent).unwrap())
            .map(|r| (r.crate_name.clone(), r.coverage_percent))
            .unwrap_or((String::new(), 100.0));

        AggregateCoverage {
            total_lines_covered: total_covered,
            total_lines,
            overall_coverage_percent: overall,
            crate_count: results.len(),
            average_coverage_percent: average,
            lowest_coverage_crate: if lowest_percent < 100.0 {
                Some(lowest_crate)
            } else {
                None
            },
            lowest_coverage_percent: lowest_percent,
        }
    }

    /// Get coverage trend (recent results)
    pub fn get_coverage_trend(&self, limit: usize) -> Vec<CoverageResult> {
        let results = self.results.read();
        let mut trend: Vec<_> = results
            .iter()
            .rev()
            .take(limit)
            .cloned()
            .collect();
        trend.reverse();
        trend
    }

    /// Clear all collected data
    pub fn clear(&self) {
        self.results.write().clear();
        self.by_crate.write().clear();
    }
}

impl Default for CoverageCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// Aggregated coverage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateCoverage {
    pub total_lines_covered: usize,
    pub total_lines: usize,
    pub overall_coverage_percent: f64,
    pub crate_count: usize,
    pub average_coverage_percent: f64,
    pub lowest_coverage_crate: Option<String>,
    pub lowest_coverage_percent: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_coverage() {
        let collector = CoverageCollector::new();
        collector.record_crate(
            "test_crate",
            80,
            100,
            75.0,
            vec![],
        );

        let result = collector.get_crate_coverage("test_crate");
        assert!(result.is_some());
        assert_eq!(result.unwrap().coverage_percent, 80.0);
    }

    #[test]
    fn test_aggregate_coverage() {
        let collector = CoverageCollector::new();
        collector.record_crate("crate1", 80, 100, 75.0, vec![]);
        collector.record_crate("crate2", 90, 100, 85.0, vec![]);

        let agg = collector.get_aggregate_coverage();
        assert_eq!(agg.crate_count, 2);
        assert!(agg.overall_coverage_percent > 80.0);
    }
}
