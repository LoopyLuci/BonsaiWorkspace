use crate::{MoeConfig, RoutingStrategy};

pub struct Router;

impl Router {
    pub fn new(_config: &MoeConfig) -> Self {
        Router
    }

    pub fn route(
        &self,
        tokens: &[u32],
        _layer: usize,
        strategy: &RoutingStrategy,
    ) -> Vec<Vec<usize>> {
        match strategy {
            RoutingStrategy::HashBased => tokens
                .iter()
                .map(|t| {
                    let hash = blake3::hash(&t.to_le_bytes());
                    vec![hash.as_bytes()[0] as usize % 8]
                })
                .collect(),
            RoutingStrategy::LearnedTopK => tokens.iter().map(|_| vec![0, 1]).collect(),
            RoutingStrategy::SparseActivation => tokens.iter().map(|_| vec![0]).collect(),
        }
    }
}
