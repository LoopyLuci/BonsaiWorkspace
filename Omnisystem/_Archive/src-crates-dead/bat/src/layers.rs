use ndarray::Array2;
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

    pub fn forward(&self, x: &Array2<f32>) -> Array2<f32> {
        x.clone()
    }
}
