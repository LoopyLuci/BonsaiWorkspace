"""Configuration for MultiStreamMoE architecture."""

from dataclasses import dataclass, field
from typing import List


@dataclass
class MultiStreamMoEConfig:
    """Configuration for MultiStreamMoE model."""

    # Attention parameters
    hidden_size: int = 2048
    num_heads: int = 16
    num_kv_heads: int = 4
    head_dim: int = 128

    # KV Compression parameters
    csa_compression_ratio: int = 4
    hca_compression_ratio: int = 128
    sliding_window_size: int = 128

    # MoE parameters
    num_experts: int = 256
    num_active_experts: int = 8

    # Residual stream parameters
    num_residual_streams: int = 2
    is_early_layer: bool = False

    # Training parameters
    sequence_balance_weight: float = 0.01
    use_load_balance_loss: bool = True

    # Model architecture
    num_layers: int = 32
    vocab_size: int = 100_000
    max_seq_len: int = 4096

    # Training hyperparameters
    batch_size: int = 8
    learning_rate: float = 3e-4
    weight_decay: float = 0.1
    num_epochs: int = 10
    gradient_clip_norm: float = 1.0

    # Data parameters
    shard_urls: List[str] = field(default_factory=list)
    tokenizer_path: str = "google/byt5-small"
    output_dir: str = "./checkpoints"
    num_workers: int = 4

    # Monitoring parameters
    use_wandb: bool = True
    wandb_project: str = "multistream-moe"
    wandb_entity: str = None
    tensorboard_dir: str = "./runs"

    # Checkpointing
    resume_checkpoint: str = None
    save_interval: int = 1000

    def __post_init__(self):
        """Validate configuration."""
        if self.csa_compression_ratio <= 1 or not isinstance(self.csa_compression_ratio, int):
            raise ValueError("CSA compression ratio must be integer > 1")
        if self.hca_compression_ratio <= 1 or not isinstance(self.hca_compression_ratio, int):
            raise ValueError("HCA compression ratio must be integer > 1")
        if self.hidden_size % self.num_heads != 0:
            raise ValueError("hidden_size must be divisible by num_heads")
        if self.num_heads % self.num_kv_heads != 0:
            raise ValueError("num_heads must be divisible by num_kv_heads")
        if self.num_residual_streams < 1:
            raise ValueError("At least one residual stream required")
        if self.num_experts < self.num_active_experts:
            raise ValueError("num_experts must be >= num_active_experts")
        if self.head_dim * self.num_heads != self.hidden_size:
            raise ValueError("head_dim * num_heads must equal hidden_size")
