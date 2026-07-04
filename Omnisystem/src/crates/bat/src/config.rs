use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatConfig {
    pub depth: u32,
    pub width: u32,
    pub num_experts: u32,
    pub num_heads: u32,
    pub head_dim: u32,
    pub vocab_size: u32,
    pub context_length: u32,
    pub use_moe: bool,
    pub use_lora: bool,
}

impl Default for BatConfig {
    fn default() -> Self {
        Self {
            depth: 4,
            width: 256,
            num_experts: 2,
            num_heads: 4,
            head_dim: 64,
            vocab_size: 32000,
            context_length: 4096,
            use_moe: true,
            use_lora: true,
        }
    }
}
