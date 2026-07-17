use anyhow::Result;
use ndarray::Array2;
use rand::Rng;
use std::path::Path;

use crate::{config::BatConfig, layers::TransformerBlock, scaling::ScaleMap};

pub struct BatEngine {
    config: BatConfig,
    blocks: Vec<TransformerBlock>,
    /// Token embedding table, shape (vocab_size, width). Random-initialized
    /// like the transformer blocks -- there is no real trained model here,
    /// but the forward pass and decode loop are genuine.
    embedding: Array2<f32>,
}

impl BatEngine {
    pub fn load(config: BatConfig, _model_dir: &Path) -> Result<Self> {
        let blocks = Self::build_blocks(&config);
        let embedding = Self::build_embedding(&config);
        Ok(Self { config, blocks, embedding })
    }

    fn build_blocks(config: &BatConfig) -> Vec<TransformerBlock> {
        (0..config.depth)
            .map(|_| TransformerBlock::new(config.width as usize, config.use_moe))
            .collect()
    }

    fn build_embedding(config: &BatConfig) -> Array2<f32> {
        let mut rng = rand::thread_rng();
        let scale = 1.0 / (config.width as f32).sqrt();
        Array2::from_shape_fn((config.vocab_size as usize, config.width as usize), |_| rng.gen::<f32>() * scale)
    }

    pub fn scale_to(&mut self, target_params: u64) -> Result<()> {
        let map = ScaleMap::default();
        let new_config = map.nearest(target_params);
        self.blocks = Self::build_blocks(&new_config);
        self.embedding = Self::build_embedding(&new_config);
        self.config = new_config;
        Ok(())
    }

    /// Greedily decode up to `max_new_tokens` additional tokens from
    /// `prompt` by running the real embed -> transformer-blocks ->
    /// unembed -> argmax pipeline, autoregressively feeding each
    /// generated token back in. With randomly-initialized (untrained)
    /// weights the output tokens are not meaningful text, but every step
    /// of the mechanism is real matrix computation, not a passthrough.
    pub fn generate(&self, prompt: &[u32], max_new_tokens: usize) -> Vec<u32> {
        let mut tokens: Vec<u32> = prompt.to_vec();
        if tokens.is_empty() || self.blocks.is_empty() {
            return tokens;
        }

        for _ in 0..max_new_tokens {
            let seq_len = tokens.len();
            let width = self.config.width as usize;

            // Embed the current sequence.
            let mut x = Array2::<f32>::zeros((seq_len, width));
            for (i, &tok) in tokens.iter().enumerate() {
                let row = (tok as usize).min(self.embedding.nrows() - 1);
                x.row_mut(i).assign(&self.embedding.row(row));
            }

            // Run through every transformer block.
            for block in &self.blocks {
                x = block.forward(&x);
            }

            // Unembed the final position's hidden state into vocab logits
            // (weight-tied with the embedding table) and take the argmax.
            let last_hidden = x.row(seq_len - 1);
            let logits = self.embedding.dot(&last_hidden);
            let next_token = logits
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(idx, _)| idx as u32)
                .unwrap_or(0);

            tokens.push(next_token);
        }

        tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_generate_appends_requested_token_count() {
        let config = BatConfig { depth: 2, width: 16, vocab_size: 64, ..BatConfig::default() };
        let engine = BatEngine::load(config, &PathBuf::from(".")).unwrap();

        let prompt = vec![1, 2, 3];
        let out = engine.generate(&prompt, 5);

        assert_eq!(out.len(), prompt.len() + 5);
        assert_eq!(&out[..3], &prompt[..]);
    }

    #[test]
    fn test_generate_tokens_are_in_vocab_range() {
        let config = BatConfig { depth: 2, width: 16, vocab_size: 32, ..BatConfig::default() };
        let engine = BatEngine::load(config.clone(), &PathBuf::from(".")).unwrap();

        let out = engine.generate(&[0], 10);
        assert!(out.iter().all(|&t| (t as u32) < config.vocab_size));
    }

    #[test]
    fn test_generate_empty_prompt_returns_empty() {
        let config = BatConfig::default();
        let engine = BatEngine::load(config, &PathBuf::from(".")).unwrap();
        assert!(engine.generate(&[], 5).is_empty());
    }

    #[test]
    fn test_scale_to_changes_block_count() {
        // Deliberately targets a small/medium bucket (depth=12, width=256)
        // rather than the largest one: TransformerBlock::new eagerly
        // materializes full random weight matrices per block, so scaling
        // to the 100-layer/2048-width bucket allocates and randomly
        // initializes several billion f32s, which is far too slow for a
        // unit test (and a real scalability concern in its own right).
        let config = BatConfig::default();
        let mut engine = BatEngine::load(config, &PathBuf::from(".")).unwrap();
        assert_eq!(engine.blocks.len(), 4); // default depth bucket

        engine.scale_to(600_000_000).unwrap();
        assert_eq!(engine.blocks.len(), 12);
    }
}
