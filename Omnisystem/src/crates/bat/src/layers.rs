use ndarray::{Array2, Axis};
use rand::Rng;

pub struct TransformerBlock {
    pub attn_qkv: Array2<f32>,
    pub attn_out: Array2<f32>,
    pub ffn_w1: Array2<f32>,
    pub ffn_w2: Array2<f32>,
    pub use_moe: bool,
}

impl TransformerBlock {
    pub fn new(dim: usize, use_moe: bool) -> Self {
        let mut rng = rand::thread_rng();
        let scale = 1.0 / (dim as f32).sqrt();
        Self {
            attn_qkv: Array2::from_shape_fn((dim, dim * 3), |_| rng.gen::<f32>() * scale),
            attn_out: Array2::from_shape_fn((dim, dim), |_| rng.gen::<f32>() * scale),
            ffn_w1: Array2::from_shape_fn((dim, dim * 4), |_| rng.gen::<f32>() * scale),
            ffn_w2: Array2::from_shape_fn((dim * 4, dim), |_| rng.gen::<f32>() * scale),
            use_moe,
        }
    }

    /// A real (single-head, no layer norm, no MoE routing) transformer
    /// block forward pass: scaled dot-product self-attention followed by
    /// a ReLU feed-forward network, each with a residual connection.
    /// `x` has shape (seq_len, dim); the previous implementation ignored
    /// every weight matrix and returned `x` unchanged.
    pub fn forward(&self, x: &Array2<f32>) -> Array2<f32> {
        let dim = x.ncols();

        // Project to Q, K, V and split the concatenated (seq_len, 3*dim)
        // output into three (seq_len, dim) matrices.
        let qkv = x.dot(&self.attn_qkv);
        let q = qkv.slice(ndarray::s![.., 0..dim]).to_owned();
        let k = qkv.slice(ndarray::s![.., dim..2 * dim]).to_owned();
        let v = qkv.slice(ndarray::s![.., 2 * dim..3 * dim]).to_owned();

        // Scaled dot-product attention: softmax(Q K^T / sqrt(d)) V
        let scale = 1.0 / (dim as f32).sqrt();
        let scores = q.dot(&k.t()) * scale;
        let attn_weights = softmax_rows(&scores);
        let attn_output = attn_weights.dot(&v);

        let projected = attn_output.dot(&self.attn_out);
        let residual1 = x + &projected;

        // Position-wise feed-forward network with ReLU activation.
        let hidden = residual1.dot(&self.ffn_w1).mapv(|v| v.max(0.0));
        let ffn_output = hidden.dot(&self.ffn_w2);

        residual1 + &ffn_output
    }
}

/// Numerically-stable row-wise softmax.
fn softmax_rows(x: &Array2<f32>) -> Array2<f32> {
    let mut out = x.clone();
    for mut row in out.axis_iter_mut(Axis(0)) {
        let max = row.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
        row.mapv_inplace(|v| (v - max).exp());
        let sum: f32 = row.sum();
        if sum > 0.0 {
            row.mapv_inplace(|v| v / sum);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forward_preserves_shape() {
        let block = TransformerBlock::new(8, false);
        let x = Array2::<f32>::zeros((4, 8));
        let out = block.forward(&x);
        assert_eq!(out.shape(), x.shape());
    }

    #[test]
    fn test_forward_is_not_identity() {
        // The old implementation returned `x` unchanged regardless of the
        // block's weights; a real forward pass must actually transform it.
        let block = TransformerBlock::new(8, false);
        let x = Array2::<f32>::from_shape_fn((4, 8), |(i, j)| (i * 8 + j) as f32 * 0.01);
        let out = block.forward(&x);
        assert_ne!(out, x, "forward() must not be a no-op that just returns the input");
    }

    #[test]
    fn test_softmax_rows_sums_to_one() {
        let x = Array2::from_shape_vec((2, 3), vec![1.0, 2.0, 3.0, -1.0, 0.0, 1.0]).unwrap();
        let sm = softmax_rows(&x);
        for row in sm.axis_iter(Axis(0)) {
            let sum: f32 = row.sum();
            assert!((sum - 1.0).abs() < 1e-5, "row must sum to 1, got {sum}");
            assert!(row.iter().all(|&v| v >= 0.0), "softmax outputs must be non-negative");
        }
    }

    #[test]
    fn test_forward_deterministic_for_fixed_weights() {
        let block = TransformerBlock::new(4, false);
        let x = Array2::<f32>::from_shape_fn((2, 4), |(i, j)| (i + j) as f32);
        let out1 = block.forward(&x);
        let out2 = block.forward(&x);
        assert_eq!(out1, out2, "forward() must be deterministic for the same weights and input");
    }
}
