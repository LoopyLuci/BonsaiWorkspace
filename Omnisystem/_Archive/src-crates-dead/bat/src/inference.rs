use anyhow::Result;
use std::path::Path;

use crate::{config::BatConfig, layers::TransformerBlock, scaling::ScaleMap};

pub struct BatEngine {
    config: BatConfig,
    blocks: Vec<TransformerBlock>,
}

impl BatEngine {
    pub fn load(config: BatConfig, _model_dir: &Path) -> Result<Self> {
        let mut blocks = Vec::new();
        for _ in 0..config.depth {
            blocks.push(TransformerBlock::new(config.width as usize, config.use_moe));
        }
        Ok(Self { config, blocks })
    }

    pub fn scale_to(&mut self, target_params: u64) -> Result<()> {
        let map = ScaleMap::default();
        let new_config = map.nearest(target_params);
        self.config = new_config;
        self.blocks.clear();
        for _ in 0..self.config.depth {
            self.blocks.push(TransformerBlock::new(
                self.config.width as usize,
                self.config.use_moe,
            ));
        }
        Ok(())
    }

    pub fn generate(&self, prompt: &[u32]) -> Vec<u32> {
        prompt.to_vec()
    }
}
