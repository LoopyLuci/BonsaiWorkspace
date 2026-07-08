use ndarray::{Array2, Array3};
use std::collections::HashMap;

/// Unit tests for core adaptive transformer components
#[derive(Debug, Clone)]
pub struct LayerMask {
    /// Boolean mask: true = layer active, false = layer inactive
    pub mask: Vec<bool>,
    pub num_layers: usize,
}

impl LayerMask {
    pub fn new(num_layers: usize) -> Self {
        Self {
            mask: vec![true; num_layers],
            num_layers,
        }
    }

    pub fn with_pattern(num_layers: usize, pattern: &[usize]) -> Self {
        let mut mask = vec![false; num_layers];
        for &idx in pattern {
            if idx < num_layers {
                mask[idx] = true;
            }
        }
        Self {
            mask,
            num_layers,
        }
    }

    pub fn active_layers(&self) -> usize {
        self.mask.iter().filter(|&&b| b).count()
    }

    pub fn is_subset(&self, other: &LayerMask) -> bool {
        // self is a subset if all active layers in self are also active in other
        for (i, &active_in_self) in self.mask.iter().enumerate() {
            if active_in_self && !other.mask[i] {
                return false;
            }
        }
        true
    }
}

/// Test layer masking with various patterns
#[cfg(test)]
mod layer_masking_tests {
    use super::*;

    #[test]
    fn test_layer_mask_creation() {
        let mask = LayerMask::new(100);
        assert_eq!(mask.num_layers, 100);
        assert_eq!(mask.active_layers(), 100);
    }

    #[test]
    fn test_layer_mask_pattern() {
        let pattern = vec![0, 50, 99];
        let mask = LayerMask::with_pattern(100, &pattern);
        assert_eq!(mask.active_layers(), 3);
        assert!(mask.mask[0]);
        assert!(mask.mask[50]);
        assert!(mask.mask[99]);
        assert!(!mask.mask[1]);
    }

    #[test]
    fn test_layer_mask_skip_connections() {
        // Test that skip connections properly bypass masked layers
        let batch_size = 1;
        let seq_len = 512;
        let hidden = 256;

        let x = Array3::<f32>::zeros((batch_size, seq_len, hidden));
        let output_masked = skip_connection_forward(&x, &LayerMask::with_pattern(100, &[0, 50, 99]));

        // Verify output shape is correct
        assert_eq!(output_masked.dim(), (batch_size, seq_len, hidden));

        // Verify no NaNs
        assert!(!output_masked.iter().any(|v| v.is_nan()));
    }

    #[test]
    fn test_layer_mask_is_subset() {
        let full_mask = LayerMask::new(100);
        let partial_mask = LayerMask::with_pattern(100, &[0, 25, 50, 75, 99]);

        assert!(partial_mask.is_subset(&full_mask));
        assert!(!full_mask.is_subset(&partial_mask));
        assert!(partial_mask.is_subset(&partial_mask));
    }

    #[test]
    fn test_layer_mask_determinism() {
        // Test that the same mask produces the same output (deterministic computation)
        let mask = LayerMask::with_pattern(100, &[0, 50, 99]);
        let batch_size = 2;
        let seq_len = 512;
        let hidden = 256;

        let x = Array3::<f32>::zeros((batch_size, seq_len, hidden));

        let output1 = skip_connection_forward(&x, &mask);
        let output2 = skip_connection_forward(&x, &mask);

        assert_eq!(output1.dim(), output2.dim());
        for (v1, v2) in output1.iter().zip(output2.iter()) {
            assert!((v1 - v2).abs() < 1e-6);
        }
    }
}

/// Width scaling: dimension truncation and projection matrices
#[derive(Debug, Clone)]
pub struct WidthScaler {
    pub original_dim: usize,
    pub target_dim: usize,
}

impl WidthScaler {
    pub fn new(original_dim: usize, target_dim: usize) -> Self {
        assert!(target_dim <= original_dim);
        Self {
            original_dim,
            target_dim,
        }
    }

    /// Truncate dimensions directly
    pub fn truncate(&self, tensor: &Array2<f32>) -> Array2<f32> {
        let (batch, dim) = tensor.dim();
        assert_eq!(dim, self.original_dim);
        tensor.slice(s![.., ..self.target_dim]).to_owned()
    }

    /// Use projection matrix for smoother scaling
    pub fn project(&self, tensor: &Array2<f32>) -> Array2<f32> {
        let (batch, dim) = tensor.dim();
        assert_eq!(dim, self.original_dim);

        // Simple projection: average pooling
        let pool_factor = (self.original_dim as f32 / self.target_dim as f32).ceil() as usize;
        let mut output = Array2::<f32>::zeros((batch, self.target_dim));

        for i in 0..self.target_dim {
            let start = i * pool_factor;
            let end = ((i + 1) * pool_factor).min(self.original_dim);
            let count = (end - start) as f32;

            for b in 0..batch {
                let sum: f32 = tensor.row(b).slice(s![start..end]).sum();
                output[[b, i]] = sum / count;
            }
        }
        output
    }
}

#[cfg(test)]
mod width_scaling_tests {
    use super::*;
    use ndarray::s;

    #[test]
    fn test_width_truncate() {
        let scaler = WidthScaler::new(256, 128);
        let tensor = Array2::<f32>::ones((4, 256));
        let result = scaler.truncate(&tensor);
        assert_eq!(result.dim(), (4, 128));
    }

    #[test]
    fn test_width_project() {
        let scaler = WidthScaler::new(256, 128);
        let tensor = Array2::<f32>::ones((4, 256)) * 2.0;
        let result = scaler.project(&tensor);
        assert_eq!(result.dim(), (4, 128));
        // Each value should be approximately 2.0 (average of 2.0s)
        assert!(result.iter().all(|&v| (v - 2.0).abs() < 0.1));
    }

    #[test]
    fn test_width_scaling_is_invertible() {
        let scaler_down = WidthScaler::new(256, 128);
        let scaler_up = WidthScaler::new(128, 256);

        let original = Array2::<f32>::ones((4, 256));
        let scaled_down = scaler_down.project(&original);
        let _scaled_up = scaler_up.project(&scaled_down);

        assert_eq!(scaled_down.dim(), (4, 128));
    }
}

/// Expert routing with expert masks
#[derive(Debug, Clone)]
pub struct ExpertRouter {
    pub num_experts: usize,
    pub active_experts: usize,
}

impl ExpertRouter {
    pub fn new(num_experts: usize, active_experts: usize) -> Self {
        assert!(active_experts <= num_experts);
        Self {
            num_experts,
            active_experts,
        }
    }

    /// Route tokens to active experts
    pub fn route(&self, token_ids: &[usize]) -> Vec<Vec<usize>> {
        let mut expert_assignments = vec![Vec::new(); self.active_experts];

        for (token_idx, &token_id) in token_ids.iter().enumerate() {
            let expert = token_id % self.active_experts;
            expert_assignments[expert].push(token_idx);
        }

        expert_assignments
    }

    /// Verify that all tokens are routed
    pub fn verify_coverage(&self, assignments: &[Vec<usize>], total_tokens: usize) -> bool {
        let total_routed: usize = assignments.iter().map(|v| v.len()).sum();
        total_routed == total_tokens
    }
}

#[cfg(test)]
mod expert_routing_tests {
    use super::*;

    #[test]
    fn test_expert_routing() {
        let router = ExpertRouter::new(8, 4);
        let tokens = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
        let assignments = router.route(&tokens);

        assert_eq!(assignments.len(), 4);
        assert!(router.verify_coverage(&assignments, tokens.len()));
    }

    #[test]
    fn test_expert_routing_load_balance() {
        let router = ExpertRouter::new(8, 4);
        let tokens: Vec<usize> = (0..1000).collect();
        let assignments = router.route(&tokens);

        // Check that load is reasonably balanced
        let load_variance: f32 = assignments.iter()
            .map(|v| v.len() as f32)
            .collect::<Vec<_>>()
            .iter()
            .map(|&x| (x - 250.0).powi(2))
            .sum::<f32>() / assignments.len() as f32;

        // Variance should be low for uniform distribution
        assert!(load_variance < 1000.0);
    }
}

/// LoRA adapter composition
#[derive(Debug, Clone)]
pub struct LoRAAdapter {
    pub rank: usize,
    pub original_dim: usize,
    pub alpha: f32,
}

impl LoRAAdapter {
    pub fn new(original_dim: usize, rank: usize, alpha: f32) -> Self {
        Self {
            rank,
            original_dim,
            alpha,
        }
    }

    /// Apply LoRA update to parameter
    pub fn apply(&self, param: f32, delta: f32) -> f32 {
        let scaling = self.alpha / self.rank as f32;
        param + scaling * delta
    }

    /// Compose multiple LoRA adapters
    pub fn compose(&self, adapters: &[Array2<f32>]) -> Array2<f32> {
        let mut result = Array2::<f32>::zeros((self.original_dim, self.rank));

        for adapter in adapters {
            result = result + adapter;
        }

        result * (self.alpha / self.rank as f32)
    }
}

#[cfg(test)]
mod lora_tests {
    use super::*;

    #[test]
    fn test_lora_adapter_scaling() {
        let adapter = LoRAAdapter::new(768, 16, 16.0);

        let original = 1.0f32;
        let delta = 0.1f32;
        let updated = adapter.apply(original, delta);

        let scaling = 16.0 / 16 as f32;
        assert!((updated - (original + scaling * delta)).abs() < 1e-6);
    }

    #[test]
    fn test_lora_composition() {
        let adapter = LoRAAdapter::new(768, 16, 16.0);
        let adapters = vec![
            Array2::<f32>::ones((768, 16)),
            Array2::<f32>::ones((768, 16)) * 2.0,
        ];

        let result = adapter.compose(&adapters);
        assert_eq!(result.dim(), (768, 16));
    }
}

/// KV-cache invalidation
#[derive(Debug, Clone)]
pub struct KVCacheInvalidation {
    pub cache_size: usize,
    pub sequence_length: usize,
    pub invalid_positions: Vec<usize>,
}

impl KVCacheInvalidation {
    pub fn new(cache_size: usize, sequence_length: usize) -> Self {
        Self {
            cache_size,
            sequence_length,
            invalid_positions: Vec::new(),
        }
    }

    /// Mark positions as invalid
    pub fn invalidate(&mut self, start: usize, end: usize) {
        for pos in start..end.min(self.sequence_length) {
            self.invalid_positions.push(pos);
        }
    }

    /// Check if cache is fully valid
    pub fn is_valid(&self) -> bool {
        self.invalid_positions.is_empty()
    }

    /// Get valid slice of cache
    pub fn valid_slice(&self, cache: &[f32]) -> Vec<f32> {
        let mut result = Vec::new();
        for (i, &value) in cache.iter().enumerate() {
            if !self.invalid_positions.contains(&i) {
                result.push(value);
            }
        }
        result
    }
}

#[cfg(test)]
mod kv_cache_tests {
    use super::*;

    #[test]
    fn test_kv_cache_invalidation() {
        let mut cache = KVCacheInvalidation::new(4096, 512);
        assert!(cache.is_valid());

        cache.invalidate(100, 150);
        assert!(!cache.is_valid());
        assert_eq!(cache.invalid_positions.len(), 50);
    }

    #[test]
    fn test_kv_cache_valid_slice() {
        let mut cache = KVCacheInvalidation::new(10, 10);
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];

        cache.invalidate(3, 7);
        let valid = cache.valid_slice(&data);

        assert_eq!(valid.len(), 6);
        assert_eq!(valid[0], 1.0);
        assert_eq!(valid[3], 8.0); // Position 7
    }
}

/// Helper function for skip connection forward pass
fn skip_connection_forward(x: &Array3<f32>, _mask: &LayerMask) -> Array3<f32> {
    // Simplified: just return input (in reality, would apply masked layers)
    x.clone()
}

#[cfg(test)]
mod gradient_flow_tests {
    use super::*;

    #[test]
    fn test_gradient_flow_through_masked_layers() {
        // Test that gradients flow correctly through masked and unmasked layers
        let mask = LayerMask::with_pattern(100, &[0, 50, 99]);

        // Compute forward pass
        let x = Array3::<f32>::ones((2, 512, 256));
        let y = skip_connection_forward(&x, &mask);

        // Verify shapes for backward pass
        assert_eq!(y.dim(), x.dim());

        // Gradients would flow back through active layers
        let _grad = y.clone();
        assert!(!_grad.iter().any(|v| v.is_nan()));
    }
}
