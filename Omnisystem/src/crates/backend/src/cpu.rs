use ndarray::{Array1, Array2};

/// CPU-optimized matrix multiplication.
pub fn matmul(a: &Array2<f32>, b: &Array2<f32>) -> Array2<f32> {
    let (m, k) = a.dim();
    let (_, n) = b.dim();
    assert_eq!(a.ncols(), b.nrows(), "Incompatible matrix dimensions");

    let mut result = Array2::zeros((m, n));

    for i in 0..m {
        for j in 0..n {
            let mut sum = 0.0f32;
            for p in 0..k {
                sum += a[[i, p]] * b[[p, j]];
            }
            result[[i, j]] = sum;
        }
    }

    result
}

/// Batched matrix multiplication for inference.
pub fn batched_matmul(
    a: &Array2<f32>,  // (batch_size * seq_len, hidden_dim)
    b: &Array2<f32>,  // (hidden_dim, output_dim)
) -> Array2<f32> {
    matmul(a, b)
}

/// Element-wise operations.
pub fn elementwise_add(a: &Array1<f32>, b: &Array1<f32>) -> Array1<f32> {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| x + y)
        .collect()
}

pub fn elementwise_mul(a: &Array1<f32>, b: &Array1<f32>) -> Array1<f32> {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| x * y)
        .collect()
}

pub fn elementwise_relu(a: &Array1<f32>) -> Array1<f32> {
    a.iter()
        .map(|&x| x.max(0.0))
        .collect()
}

/// Check if SIMD features are available.
pub fn has_simd() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        is_x86_feature_detected!("avx2")
    }
    #[cfg(target_arch = "aarch64")]
    {
        true // NEON is always available on ARM64
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        false
    }
}

/// Reduce a vector to a scalar (e.g., sum).
pub fn sum(a: &Array1<f32>) -> f32 {
    a.iter().sum()
}

pub fn mean(a: &Array1<f32>) -> f32 {
    let sum: f32 = a.iter().sum();
    sum / a.len() as f32
}

pub fn max(a: &Array1<f32>) -> f32 {
    a.iter().copied().fold(f32::NEG_INFINITY, |a, b| a.max(b))
}

pub fn min(a: &Array1<f32>) -> f32 {
    a.iter().copied().fold(f32::INFINITY, |a, b| a.min(b))
}

/// Softmax with numerical stability.
pub fn softmax(a: &Array1<f32>) -> Array1<f32> {
    let max_val = max(a);
    let exp_a: Array1<f32> = a.iter()
        .map(|&x| (x - max_val).exp())
        .collect();
    let sum_exp: f32 = exp_a.iter().sum();
    exp_a.iter()
        .map(|&x| x / sum_exp)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ndarray::arr1;

    #[test]
    fn test_elementwise_ops() {
        let a = arr1(&[1.0, 2.0, 3.0]);
        let b = arr1(&[2.0, 3.0, 4.0]);
        let sum = elementwise_add(&a, &b);
        assert_eq!(sum.to_vec(), vec![3.0, 5.0, 7.0]);
    }

    #[test]
    fn test_reduction() {
        let a = arr1(&[1.0, 2.0, 3.0, 4.0]);
        assert_eq!(sum(&a), 10.0);
        assert_eq!(mean(&a), 2.5);
        assert_eq!(max(&a), 4.0);
        assert_eq!(min(&a), 1.0);
    }
}
