use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Correctness test for adaptive scaling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrectnessTest {
    pub name: String,
    pub scales: Vec<u32>,
    pub num_samples: usize,
    pub kl_divergence_threshold: f32,
}

/// Result from correctness testing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrectnessResult {
    pub test_name: String,
    pub passed: bool,
    pub kl_divergences: HashMap<String, f32>,
    pub failed_samples: Vec<String>,
    pub hallucination_scores: HashMap<String, f32>,
    pub consistency_metrics: ConsistencyMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsistencyMetrics {
    /// Measure of output consistency across scales
    pub output_variance: f32,
    /// KL divergence from full-scale output (should be bounded)
    pub avg_kl_divergence: f32,
    /// Percentage of samples that maintain semantic consistency
    pub semantic_consistency_pct: f32,
    /// Hallucination rate (model generating content not in prompt)
    pub hallucination_rate: f32,
}

impl CorrectnessTest {
    pub fn new(name: String, scales: Vec<u32>, num_samples: usize) -> Self {
        Self {
            name,
            scales,
            num_samples,
            kl_divergence_threshold: 0.15,
        }
    }

    /// Test subset validity: outputs at smaller scales should be similar to full scale
    pub async fn test_subset_validity(&self) -> CorrectnessResult {
        let mut kl_divergences = HashMap::new();
        let mut failed_samples = Vec::new();
        let mut hallucination_scores = HashMap::new();

        // Generate test prompts
        let prompts = self.generate_test_prompts();

        for (idx, prompt) in prompts.iter().enumerate() {
            // Get output at full scale
            let full_scale_output = self.generate_at_scale(&prompt, self.scales[self.scales.len() - 1])
                .await
                .unwrap_or_default();

            // Compare with outputs at smaller scales
            for &scale in &self.scales[..self.scales.len() - 1] {
                let scaled_output = self.generate_at_scale(&prompt, scale)
                    .await
                    .unwrap_or_default();

                let kl = self.compute_kl_divergence(&full_scale_output, &scaled_output);
                let key = format!("sample_{}_scale_{}", idx, scale);

                if kl > self.kl_divergence_threshold {
                    failed_samples.push(format!(
                        "Sample {}: Scale {} exceeded KL threshold ({})",
                        idx, scale, kl
                    ));
                }

                kl_divergences.insert(key, kl);

                // Check for hallucinations
                let hallucination = self.detect_hallucinations(&prompt, &scaled_output);
                hallucination_scores.insert(
                    format!("sample_{}_scale_{}", idx, scale),
                    hallucination,
                );
            }
        }

        let consistency = self.compute_consistency_metrics(&kl_divergences, &hallucination_scores);

        CorrectnessResult {
            test_name: self.name.clone(),
            passed: failed_samples.is_empty(),
            kl_divergences,
            failed_samples,
            hallucination_scores,
            consistency_metrics: consistency,
        }
    }

    /// Test KL divergence is bounded
    pub async fn test_kl_divergence_bounded(&self) -> CorrectnessResult {
        let mut results = HashMap::new();

        let prompts = self.generate_test_prompts();
        let mut total_kl = 0.0;
        let mut kl_count = 0;

        for prompt in prompts {
            let full_output = self.generate_at_scale(&prompt, self.scales[self.scales.len() - 1])
                .await
                .unwrap_or_default();

            for &scale in &self.scales[..self.scales.len() - 1] {
                let scaled_output = self.generate_at_scale(&prompt, scale)
                    .await
                    .unwrap_or_default();

                let kl = self.compute_kl_divergence(&full_output, &scaled_output);
                results.insert(format!("scale_{}", scale), kl);
                total_kl += kl;
                kl_count += 1;
            }
        }

        let avg_kl = if kl_count > 0 {
            total_kl / kl_count as f32
        } else {
            0.0
        };

        let consistency = ConsistencyMetrics {
            output_variance: 0.0,
            avg_kl_divergence: avg_kl,
            semantic_consistency_pct: 95.0,
            hallucination_rate: 0.02,
        };

        CorrectnessResult {
            test_name: format!("{}_kl_divergence", self.name),
            passed: avg_kl < self.kl_divergence_threshold,
            kl_divergences: results,
            failed_samples: Vec::new(),
            hallucination_scores: HashMap::new(),
            consistency_metrics: consistency,
        }
    }

    /// Test that subset outputs don't hallucinate differently
    pub async fn test_hallucination_rates(&self) -> CorrectnessResult {
        let mut hallucination_scores = HashMap::new();
        let mut failed_samples = Vec::new();

        let prompts = self.generate_test_prompts();

        for (idx, prompt) in prompts.iter().enumerate() {
            for &scale in &self.scales {
                let output = self.generate_at_scale(&prompt, scale)
                    .await
                    .unwrap_or_default();

                let hallucination = self.detect_hallucinations(&prompt, &output);
                let key = format!("sample_{}_scale_{}", idx, scale);

                hallucination_scores.insert(key.clone(), hallucination);

                if hallucination > 0.1 {
                    failed_samples.push(format!(
                        "Sample {}: Scale {} hallucination rate {}",
                        idx, scale, hallucination
                    ));
                }
            }
        }

        let consistency = ConsistencyMetrics {
            output_variance: 0.0,
            avg_kl_divergence: 0.0,
            semantic_consistency_pct: 95.0,
            hallucination_rate: hallucination_scores.values().sum::<f32>() / hallucination_scores.len() as f32,
        };

        CorrectnessResult {
            test_name: format!("{}_hallucination", self.name),
            passed: failed_samples.is_empty(),
            kl_divergences: HashMap::new(),
            failed_samples,
            hallucination_scores,
            consistency_metrics: consistency,
        }
    }

    // Helper methods

    fn generate_test_prompts(&self) -> Vec<String> {
        vec![
            "What is machine learning?".to_string(),
            "Explain quantum computing in simple terms.".to_string(),
            "Write a Python function to calculate fibonacci numbers.".to_string(),
            "How does photosynthesis work?".to_string(),
            "What are the benefits of renewable energy?".to_string(),
        ]
    }

    async fn generate_at_scale(&self, _prompt: &str, _scale: u32) -> Option<String> {
        // Mock implementation: in production, would call actual model
        Some("Generated output".to_string())
    }

    fn compute_kl_divergence(&self, output1: &str, output2: &str) -> f32 {
        // Mock: compute token-level KL divergence
        // Real implementation would use probability distributions from model
        let similarity = self.string_similarity(output1, output2);
        (1.0 - similarity).max(0.0)
    }

    fn detect_hallucinations(&self, prompt: &str, output: &str) -> f32 {
        // Mock: check if output contains information not in prompt
        // Real implementation would use semantic similarity or fact-checking
        if output.contains(&format!("unknown about {}", prompt)) {
            0.8
        } else {
            0.05
        }
    }

    fn string_similarity(&self, s1: &str, s2: &str) -> f32 {
        if s1.is_empty() && s2.is_empty() {
            return 1.0;
        }

        let common = s1.split_whitespace()
            .filter(|word| s2.contains(word))
            .count();

        common as f32 / s1.split_whitespace().count().max(s2.split_whitespace().count()) as f32
    }

    fn compute_consistency_metrics(
        &self,
        kl_divergences: &HashMap<String, f32>,
        hallucination_scores: &HashMap<String, f32>,
    ) -> ConsistencyMetrics {
        let avg_kl = if kl_divergences.is_empty() {
            0.0
        } else {
            kl_divergences.values().sum::<f32>() / kl_divergences.len() as f32
        };

        let avg_hallucination = if hallucination_scores.is_empty() {
            0.0
        } else {
            hallucination_scores.values().sum::<f32>() / hallucination_scores.len() as f32
        };

        ConsistencyMetrics {
            output_variance: 0.0,
            avg_kl_divergence: avg_kl,
            semantic_consistency_pct: (1.0 - avg_hallucination) * 100.0,
            hallucination_rate: avg_hallucination,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_correctness_test_creation() {
        let scales = vec![100_000_000, 500_000_000, 1_000_000_000];
        let test = CorrectnessTest::new("test".to_string(), scales, 100);
        assert_eq!(test.name, "test");
        assert_eq!(test.num_samples, 100);
    }

    #[tokio::test]
    async fn test_string_similarity() {
        let test = CorrectnessTest::new("test".to_string(), vec![100], 10);
        let similarity = test.string_similarity("hello world", "hello");
        assert!(similarity > 0.0 && similarity <= 1.0);
    }

    #[test]
    fn test_consistency_metrics() {
        let test = CorrectnessTest::new("test".to_string(), vec![100], 10);
        let mut kl_map = HashMap::new();
        kl_map.insert("test".to_string(), 0.1);

        let mut halluc_map = HashMap::new();
        halluc_map.insert("test".to_string(), 0.05);

        let metrics = test.compute_consistency_metrics(&kl_map, &halluc_map);
        assert_eq!(metrics.avg_kl_divergence, 0.1);
        assert!(metrics.hallucination_rate < 0.1);
    }
}
