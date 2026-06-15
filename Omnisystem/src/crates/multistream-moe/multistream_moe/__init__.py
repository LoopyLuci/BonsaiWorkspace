"""
MultiStreamMoE: Production-grade LLM architecture for the Bonsai Ecosystem.

Features:
- Multi-stream residual mixing with learnable stream-coupling matrices
- Grouped-Query Attention with RoPE and causal masking
- Reversible KV compression (CSA/HCA) with defensive padding
- Mixture of Experts with hash-based routing (early) and learned top-k (deep)
- FSDP distributed training with auto-wrap
- WebDataset streaming from object storage
- Comprehensive monitoring via W&B and TensorBoard
"""

__version__ = "1.0.0"

from .config import MultiStreamMoEConfig
from .attention import GroupedQueryAttention, RotaryEmbedding
from .kv_compressor import KVCompressor
from .moe import MultiStreamMoELayer
from .block import MultiStreamMoEBlock, ResidualMixer
from .model import MultiStreamMoEModel
from .causal_lm import MultiStreamMoEForCausalLM

__all__ = [
    "MultiStreamMoEConfig",
    "GroupedQueryAttention",
    "RotaryEmbedding",
    "KVCompressor",
    "MultiStreamMoELayer",
    "MultiStreamMoEBlock",
    "ResidualMixer",
    "MultiStreamMoEModel",
    "MultiStreamMoEForCausalLM",
]
