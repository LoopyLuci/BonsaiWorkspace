pub fn compute_load_balancing_loss(routing_counts: &[usize]) -> f32 {
    let total: usize = routing_counts.iter().sum();
    if total == 0 {
        return 0.0;
    }
    let mean = total as f32 / routing_counts.len() as f32;
    let variance = routing_counts
        .iter()
        .map(|&c| (c as f32 - mean).powi(2))
        .sum::<f32>()
        / routing_counts.len() as f32;
    variance.sqrt() / mean
}
