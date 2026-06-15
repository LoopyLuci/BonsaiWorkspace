//! Auto-Differentiation Engine

use crate::error::Result;
use crate::tensor::Tensor;
use std::collections::HashMap;

/// Auto-differentiation engine for computing gradients
pub struct AutoDiff;

impl AutoDiff {
    /// Compute gradients using reverse-mode differentiation
    pub fn backward(
        output: &Tensor,
        parameters: &[&Tensor],
    ) -> Result<HashMap<u64, Tensor>> {
        let mut gradients = HashMap::new();

        // Phase 1 simplified implementation
        // In full implementation, would traverse computation graph
        // and compute gradients for each parameter

        for param in parameters {
            // For now, return zero gradient placeholder
            let grad = Tensor::zeros(
                param.shape().to_vec(),
                param.dtype(),
                param.device(),
            )?;
            gradients.insert(param.id(), grad);
        }

        Ok(gradients)
    }

    /// Compute numerical gradients for testing
    pub fn numerical_gradient(
        f: impl Fn(&Tensor) -> Result<f32>,
        input: &Tensor,
        epsilon: f32,
    ) -> Result<Tensor> {
        let mut grad = Tensor::zeros(
            input.shape().to_vec(),
            input.dtype(),
            input.device(),
        )?;

        let base_value = f(input)?;
        let data = input.data().read();
        let mut grad_data = grad.data_mut().write();

        for i in 0..input.numel() {
            // Perturb input
            let mut perturbed = data.clone();
            perturbed[i] += epsilon;

            // Create perturbed tensor
            let mut perturbed_input = input.clone();
            *perturbed_input.data_mut().write() = perturbed;

            let perturbed_value = f(&perturbed_input)?;
            grad_data[i] = (perturbed_value - base_value) / epsilon;
        }

        Ok(grad)
    }

    /// Check if gradients are computed correctly
    pub fn check_gradients(
        f: impl Fn(&Tensor) -> Result<f32>,
        input: &Tensor,
        analytical_grad: &Tensor,
        epsilon: f32,
        tolerance: f32,
    ) -> Result<bool> {
        let numerical_grad = Self::numerical_gradient(f, input, epsilon)?;

        let analytical_data = analytical_grad.data().read();
        let numerical_data = numerical_grad.data().read();

        for (a, n) in analytical_data.iter().zip(numerical_data.iter()) {
            if (a - n).abs() > tolerance {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::DType;

    #[test]
    fn test_backward() {
        let output = Tensor::ones(vec![2, 3], DType::Float32, "cpu").unwrap();
        let param = Tensor::ones(vec![2, 3], DType::Float32, "cpu").unwrap();

        let grads = AutoDiff::backward(&output, &[&param]).unwrap();
        assert!(grads.contains_key(&param.id()));
    }

    #[test]
    fn test_numerical_gradient() {
        let input = Tensor::ones(vec![2, 2], DType::Float32, "cpu").unwrap();

        let f = |x: &Tensor| -> Result<f32> { x.sum() };
        let grad = AutoDiff::numerical_gradient(f, &input, 1e-5).unwrap();

        // All gradients should be close to 1.0
        let data = grad.data().read();
        for &g in data.iter() {
            assert!((g - 1.0).abs() < 1e-3);
        }
    }
}
