use ndarray::Array2;

pub type ExpertGroup = Vec<Array2<f32>>;

pub fn create_expert_group(num_experts: usize) -> Vec<ExpertGroup> {
    (0..num_experts)
        .map(|_| vec![Array2::zeros((256, 1024))])
        .collect()
}
