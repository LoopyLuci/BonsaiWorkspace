// SYLVA NEURAL NETWORK MODULE

pub struct Tensor {
    pub shape: Vec<usize>,
    pub data: Vec<f64>,
}

pub struct NeuralNetwork {
    pub layers: Vec<Layer>,
}

pub struct Layer {
    pub weights: Tensor,
    pub bias: Tensor,
}

impl NeuralNetwork {
    pub fn forward(&self, input: &Tensor) -> Tensor {
        Tensor {
            shape: vec![10],
            data: vec![0.0; 10],
        }
    }

    pub fn backward(&mut self, gradient: &Tensor) {
        // Automatic differentiation
    }
}
