use crate::Result;

pub struct MetricsAggregator;

impl MetricsAggregator {
    pub fn calculate_percentile(values: &[f32], percentile: u32) -> Result<f32> {
        if values.is_empty() {
            return Err(crate::MonitorError::MetricsError("Empty values".to_string()));
        }
        if percentile > 100 {
            return Err(crate::MonitorError::MetricsError("Invalid percentile".to_string()));
        }
        
        let mut sorted = values.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let idx = ((percentile as f32 / 100.0) * sorted.len() as f32) as usize;
        Ok(sorted[idx.min(sorted.len() - 1)])
    }

    pub fn calculate_moving_average(values: &[f32], window: usize) -> Vec<f32> {
        values
            .windows(window)
            .map(|w| w.iter().sum::<f32>() / window as f32)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percentile() {
        let values = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let p50 = MetricsAggregator::calculate_percentile(&values, 50).unwrap();
        assert!(p50 > 0.0);
    }
}
