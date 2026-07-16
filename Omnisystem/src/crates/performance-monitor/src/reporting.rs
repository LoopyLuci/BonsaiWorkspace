use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceReport {
    pub timestamp: u64,
    pub avg_cpu: f32,
    pub avg_memory: f32,
    pub peak_cpu: f32,
    pub peak_memory: f32,
}

pub struct ReportGenerator;

impl ReportGenerator {
    pub fn generate(metrics: &[(f32, f32)]) -> PerformanceReport {
        let avg_cpu = metrics.iter().map(|(c, _)| c).sum::<f32>() / metrics.len().max(1) as f32;
        let avg_memory = metrics.iter().map(|(_, m)| m).sum::<f32>() / metrics.len().max(1) as f32;
        let peak_cpu = metrics.iter().map(|(c, _)| *c).fold(0.0, f32::max);
        let peak_memory = metrics.iter().map(|(_, m)| *m).fold(0.0, f32::max);
        
        PerformanceReport {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            avg_cpu,
            avg_memory,
            peak_cpu,
            peak_memory,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_report() {
        let metrics = vec![(50.0, 60.0), (55.0, 65.0)];
        let report = ReportGenerator::generate(&metrics);
        assert!(report.avg_cpu > 0.0);
    }
}
