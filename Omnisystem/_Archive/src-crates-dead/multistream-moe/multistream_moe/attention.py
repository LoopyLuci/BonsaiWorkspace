"""Grouped-Query Attention with Rotary Position Embeddings."""

import torch
import torch.nn as nn
import torch.nn.functional as F
from typing import Optional, Tuple
from .config import MultiStreamMoEConfig


class RotaryEmbedding(nn.Module):
    """Rotary Position Embedding (RoPE)."""

    def __init__(self, dim: int, max_seq_len: int = 4096, base: float = 10000.0):
        super().__init__()
        self.dim = dim
        self.max_seq_len = max_seq_len
        self.base = base

        inv_freq = 1.0 / (base ** (torch.arange(0, dim, 2).float() / dim))
        self.register_buffer("inv_freq", inv_freq, persistent=False)

        t = torch.arange(max_seq_len, dtype=self.inv_freq.dtype)
        freqs = torch.outer(t, inv_freq)
        emb = torch.cat([freqs, freqs], dim=-1)
        self.register_buffer("cos_cached", emb.cos()[None, None, :, :], persistent=False)
        self.register_buffer("sin_cached", emb.sin()[None, None, :, :], persistent=False)

    def forward(self, seq_len: int) -> Tuple[torch.Tensor, torch.Tensor]:
        """Return cos and sin embeddings for given sequence length."""
        return self.cos_cached[:, :, :seq_len, :], self.sin_cached[:, :, :seq_len, :]


def rotate_half(x: torch.Tensor) -> torch.Tensor:
    """Rotate half the hidden dims of the input."""
    x1, x2 = x.chunk(2, dim=-1)
    return torch.cat((-x2, x1), dim=-1)


def apply_rotary_pos_emb(
    q: torch.Tensor,
    k: torch.Tensor,
    cos: torch.Tensor,
    sin: torch.Tensor,
) -> Tuple[torch.Tensor, torch.Tensor]:
    """Apply rotary position embeddings to q and k."""
    q_embed = q * cos + rotate_half(q) * sin
    k_embed = k * cos + rotate_half(k) * sin
    return q_embed, k_embed


class GroupedQueryAttention(nn.Module):
    """Grouped-Query Attention with RoPE and causal masking."""

    def __init__(self, config: MultiStreamMoEConfig):
        super().__init__()
        self.hidden_size = config.hidden_size
        self.num_heads = config.num_heads
        self.num_kv_heads = config.num_kv_heads
        self.head_dim = config.head_dim
        self.n_rep = self.num_heads // self.num_kv_heads

        assert self.num_heads % self.num_kv_heads == 0, "num_heads must be divisible by num_kv_heads"
        assert self.head_dim * self.num_heads == self.hidden_size, "head_dim * num_heads must equal hidden_size"

        self.q_proj = nn.Linear(self.hidden_size, self.num_heads * self.head_dim, bias=False)
        self.k_proj = nn.Linear(self.hidden_size, self.num_kv_heads * self.head_dim, bias=False)
        self.v_proj = nn.Linear(self.hidden_size, self.num_kv_heads * self.head_dim, bias=False)
        self.o_proj = nn.Linear(self.num_heads * self.head_dim, self.hidden_size, bias=False)

        self.rotary = RotaryEmbedding(self.head_dim, max_seq_len=config.max_seq_len)
        self.max_seq_len = config.max_seq_len

    def forward(
        self,
        x: torch.Tensor,
        mask: Optional[torch.Tensor] = None,
        past_kv: Optional[Tuple[torch.Tensor, torch.Tensor]] = None,
    ) -> Tuple[torch.Tensor, Optional[Tuple[torch.Tensor, torch.Tensor]]]:
        """
        Args:
            x: (batch, seq, hidden)
            mask: optional causal mask
            past_kv: optional past key/value for efficient inference

        Returns:
            output: (batch, seq, hidden)
            cache: (k, v) for next step
        """
        B, S, _ = x.shape

        q = self.q_proj(x).view(B, S, self.num_heads, self.head_dim).transpose(1, 2)
        k = self.k_proj(x).view(B, S, self.num_kv_heads, self.head_dim).transpose(1, 2)
        v = self.v_proj(x).view(B, S, self.num_kv_heads, self.head_dim).transpose(1, 2)

        cos, sin = self.rotary(S)
        q, k = apply_rotary_pos_emb(q, k, cos, sin)

        # Repeat k and v for grouped query attention
        k = k.unsqueeze(2).expand(-1, -1, self.n_rep, -1, -1).reshape(B, self.num_heads, S, self.head_dim)
        v = v.unsqueeze(2).expand(-1, -1, self.n_rep, -1, -1).reshape(B, self.num_heads, S, self.head_dim)

        # Scaled dot-product attention with causal mask
        attn_out = F.scaled_dot_product_attention(
            q,
            k,
            v,
            attn_mask=mask,
            dropout_p=0.0 if not self.training else 0.1,
            is_causal=mask is None,
        )

        attn_out = attn_out.transpose(1, 2).reshape(B, S, self.num_heads * self.head_dim)
        output = self.o_proj(attn_out)

        # Cache for inference
        cache = (k, v) if past_kv is not None else None

        return output, cache
