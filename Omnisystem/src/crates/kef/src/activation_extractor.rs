//! Activation vector extraction and clustering

use crate::{KefError, Result};
use ndarray::{Array1, Array2};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for activation extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivationExtractorConfig {
    /// Sparsity threshold: only record layers where >N% neurons fire
    pub sparsity_threshold: f32,
    /// Number of clusters to form from activations
    pub num_clusters: usize,
    /// Maximum number of activations to collect
    pub max_activations: usize,
    /// Layer indices to extract from (empty = all)
    pub target_layers: Vec<usize>,
}

impl Default for ActivationExtractorConfig {
    fn default() -> Self {
        Self {
            sparsity_threshold: 0.3,
            num_clusters: 16,
            max_activations: 10000,
            target_layers: Vec::new(),
        }
    }
}

/// A single activation vector with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivationSample {
    /// Layer index
    pub layer: usize,
    /// Activation vector
    pub vector: Vec<f32>,
    /// Sparsity (fraction of non-zero activations)
    pub sparsity: f32,
    /// Input text that produced this activation
    pub input_text: String,
}

/// A cluster of related activations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivationCluster {
    /// Cluster centroid
    pub centroid: Vec<f32>,
    /// Members of this cluster
    pub members: Vec<ActivationSample>,
    /// Natural language description of this concept
    pub description: Option<String>,
}

impl ActivationCluster {
    /// Create a new cluster from samples
    pub fn new(samples: Vec<ActivationSample>) -> Result<Self> {
        if samples.is_empty() {
            return Err(KefError::ClusteringFailed(
                "cannot create cluster from empty samples".to_string(),
            ));
        }

        // Compute centroid as mean of vectors
        let dim = samples[0].vector.len();
        let mut centroid = vec![0.0f32; dim];

        for sample in &samples {
            if sample.vector.len() != dim {
                return Err(KefError::DimensionMismatch {
                    expected: dim,
                    got: sample.vector.len(),
                });
            }

            for (i, val) in sample.vector.iter().enumerate() {
                centroid[i] += val;
            }
        }

        for val in &mut centroid {
            *val /= samples.len() as f32;
        }

        Ok(ActivationCluster {
            centroid,
            members: samples,
            description: None,
        })
    }

    /// Compute distance from centroid to a vector
    pub fn distance_to_centroid(&self, vector: &[f32]) -> Result<f32> {
        if vector.len() != self.centroid.len() {
            return Err(KefError::DimensionMismatch {
                expected: self.centroid.len(),
                got: vector.len(),
            });
        }

        let dist: f32 = self
            .centroid
            .iter()
            .zip(vector.iter())
            .map(|(a, b)| (a - b).powi(2))
            .sum();

        Ok(dist.sqrt())
    }
}

/// Extracts and clusters activation vectors from neural networks
pub struct ActivationExtractor {
    config: ActivationExtractorConfig,
    samples: Vec<ActivationSample>,
}

impl ActivationExtractor {
    /// Create a new activation extractor
    pub fn new(config: ActivationExtractorConfig) -> Self {
        Self {
            config,
            samples: Vec::new(),
        }
    }

    /// Add an activation sample
    pub fn add_sample(&mut self, sample: ActivationSample) -> Result<()> {
        if self.samples.len() >= self.config.max_activations {
            return Err(KefError::ExtractionFailed(
                "maximum activation samples reached".to_string(),
            ));
        }

        // Check sparsity threshold
        if sample.sparsity < self.config.sparsity_threshold {
            // Only record sparse activations
            return Ok(());
        }

        self.samples.push(sample);
        Ok(())
    }

    /// Perform K-means clustering on collected activations
    ///
    /// Returns clusters of related activations
    pub fn cluster(&self) -> Result<Vec<ActivationCluster>> {
        if self.samples.is_empty() {
            return Err(KefError::ClusteringFailed(
                "no samples to cluster".to_string(),
            ));
        }

        let num_clusters = self.config.num_clusters.min(self.samples.len());

        // Simple K-means implementation
        let clusters = self.kmeans(num_clusters)?;

        Ok(clusters)
    }

    /// Get collected samples
    pub fn samples(&self) -> &[ActivationSample] {
        &self.samples
    }

    /// Basic K-means clustering
    fn kmeans(&self, k: usize) -> Result<Vec<ActivationCluster>> {
        if self.samples.is_empty() {
            return Ok(Vec::new());
        }

        // Initialize centroids randomly from samples
        let mut rng = rand::thread_rng();
        let dim = self.samples[0].vector.len();
        let mut centroids: Vec<Vec<f32>> = Vec::new();

        // Select k random samples as initial centroids
        use rand::seq::SliceRandom;
        let selected = self
            .samples
            .choose_multiple(&mut rng, k.min(self.samples.len()));

        for sample in selected {
            centroids.push(sample.vector.clone());
        }

        // Ensure we have k centroids
        while centroids.len() < k {
            let mut new_centroid = vec![0.0f32; dim];
            for sample in &self.samples {
                for (i, val) in sample.vector.iter().enumerate() {
                    new_centroid[i] += val;
                }
            }
            for val in &mut new_centroid {
                *val /= self.samples.len() as f32;
            }
            centroids.push(new_centroid);
        }

        // Run k-means iterations
        let max_iterations = 10;
        for _iteration in 0..max_iterations {
            // Assign samples to nearest centroid
            let mut clusters: Vec<Vec<ActivationSample>> = vec![Vec::new(); k];

            for sample in &self.samples {
                let mut best_cluster = 0;
                let mut best_distance = f32::INFINITY;

                for (j, centroid) in centroids.iter().enumerate() {
                    let dist = Self::euclidean_distance(&sample.vector, centroid);
                    if dist < best_distance {
                        best_distance = dist;
                        best_cluster = j;
                    }
                }

                clusters[best_cluster].push(sample.clone());
            }

            // Update centroids
            let mut updated_centroids = Vec::new();
            for cluster_samples in &clusters {
                if cluster_samples.is_empty() {
                    // Keep old centroid if cluster is empty
                    updated_centroids.push(centroids[updated_centroids.len()].clone());
                } else {
                    let mut new_centroid = vec![0.0f32; dim];
                    for sample in cluster_samples {
                        for (i, val) in sample.vector.iter().enumerate() {
                            new_centroid[i] += val;
                        }
                    }
                    for val in &mut new_centroid {
                        *val /= cluster_samples.len() as f32;
                    }
                    updated_centroids.push(new_centroid);
                }
            }

            centroids = updated_centroids;
        }

        // Create final clusters
        let mut result = Vec::new();
        let mut clusters: Vec<Vec<ActivationSample>> = vec![Vec::new(); k];

        for sample in &self.samples {
            let mut best_cluster = 0;
            let mut best_distance = f32::INFINITY;

            for (j, centroid) in centroids.iter().enumerate() {
                let dist = Self::euclidean_distance(&sample.vector, centroid);
                if dist < best_distance {
                    best_distance = dist;
                    best_cluster = j;
                }
            }

            clusters[best_cluster].push(sample.clone());
        }

        for cluster_samples in clusters {
            if !cluster_samples.is_empty() {
                result.push(ActivationCluster::new(cluster_samples)?);
            }
        }

        Ok(result)
    }

    /// Compute Euclidean distance between two vectors
    fn euclidean_distance(a: &[f32], b: &[f32]) -> f32 {
        let dist: f32 = a
            .iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y).powi(2))
            .sum();
        dist.sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_activation_extractor_creation() {
        let config = ActivationExtractorConfig::default();
        let extractor = ActivationExtractor::new(config);
        assert_eq!(extractor.samples().len(), 0);
    }

    #[test]
    fn test_add_sample() -> Result<()> {
        let config = ActivationExtractorConfig::default();
        let mut extractor = ActivationExtractor::new(config);

        let sample = ActivationSample {
            layer: 0,
            vector: vec![0.5, 0.6, 0.7],
            sparsity: 0.5,
            input_text: "test input".to_string(),
        };

        extractor.add_sample(sample)?;
        assert_eq!(extractor.samples().len(), 1);

        Ok(())
    }

    #[test]
    fn test_activation_cluster_creation() -> Result<()> {
        let sample = ActivationSample {
            layer: 0,
            vector: vec![0.5, 0.6, 0.7],
            sparsity: 0.5,
            input_text: "test".to_string(),
        };

        let cluster = ActivationCluster::new(vec![sample])?;
        assert_eq!(cluster.centroid.len(), 3);

        Ok(())
    }

    #[test]
    fn test_distance_calculation() -> Result<()> {
        let sample = ActivationSample {
            layer: 0,
            vector: vec![0.5, 0.6, 0.7],
            sparsity: 0.5,
            input_text: "test".to_string(),
        };

        let cluster = ActivationCluster::new(vec![sample])?;
        let dist = cluster.distance_to_centroid(&vec![0.5, 0.6, 0.7])?;
        assert!(dist < 0.01); // Should be very close to centroid

        Ok(())
    }
}
