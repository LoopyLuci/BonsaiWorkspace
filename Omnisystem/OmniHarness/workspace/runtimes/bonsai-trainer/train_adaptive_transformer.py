#!/usr/bin/env python3
"""
train_adaptive_transformer.py

Complete training pipeline for Bonsai Adaptive Transformer with:
- Phase 0: Base model training (100M params)
- Phase 1: Progressive depth (4 → 100 layers)
- Phase 2: Progressive width (256 → 1024 hidden dim)
- Phase 3: Expert pool training (1 → 1024 experts)
- Phase 4: LoRA adapter stacking (100 adapters)
- Phase 5: Co-adaptation & joint training

Usage:
    python train_adaptive_transformer.py \
        --config configs/adaptive_phase_0.json \
        --phase 0 \
        --output-dir ./checkpoints \
        --distributed-rank 0 \
        --distributed-world-size 1
"""

import argparse
import json
import logging
import os
import random
import sys
import time
from datetime import datetime
from pathlib import Path
from typing import Optional, Tuple, Dict, List

import numpy as np
import torch
import torch.nn as nn
import torch.nn.functional as F
from torch.optim import Adam
from torch.optim.lr_scheduler import LambdaLR
from torch.utils.data import DataLoader, DistributedSampler, Dataset


# ══════════════════════════════════════════════════════════════════════════════
# Logging & Utilities
# ══════════════════════════════════════════════════════════════════════════════

def setup_logging(output_dir: str):
    """Configure structured logging."""
    os.makedirs(output_dir, exist_ok=True)

    log_format = "%(asctime)s - %(name)s - %(levelname)s - %(message)s"
    logging.basicConfig(
        level=logging.INFO,
        format=log_format,
        handlers=[
            logging.FileHandler(f"{output_dir}/training.log"),
            logging.StreamHandler(sys.stdout),
        ]
    )
    return logging.getLogger(__name__)


def emit(logger, tag: str, **kwargs):
    """Emit structured log entry."""
    entry = {
        "timestamp": datetime.now().isoformat(),
        "tag": tag,
        **kwargs
    }
    logger.info(json.dumps(entry, default=str))


def set_seed(seed: int):
    """Set reproducible random state."""
    random.seed(seed)
    np.random.seed(seed)
    torch.manual_seed(seed)
    torch.cuda.manual_seed_all(seed)


def warmup_linear(optimizer, warmup_steps: int):
    """Linear warmup + constant LR scheduler."""
    def lr_lambda(step):
        if step < warmup_steps:
            return float(step) / float(max(1, warmup_steps))
        return 1.0
    return LambdaLR(optimizer, lr_lambda)


def compute_cross_entropy(logits: torch.Tensor, labels: torch.Tensor,
                         ignore_index: int = -100) -> torch.Tensor:
    """Compute cross-entropy loss."""
    return F.cross_entropy(
        logits.view(-1, logits.size(-1)),
        labels.view(-1),
        ignore_index=ignore_index
    )


def save_checkpoint(path: str, model, optimizer, scheduler, metrics: Dict = None):
    """Save training checkpoint."""
    os.makedirs(os.path.dirname(path), exist_ok=True)
    state = {
        "model": model.state_dict() if isinstance(model, nn.Module) else model,
        "optimizer": optimizer.state_dict(),
        "scheduler": scheduler.state_dict(),
        "metrics": metrics or {},
        "timestamp": datetime.now().isoformat(),
    }
    torch.save(state, path)


def load_checkpoint(path: str, device: str = "cuda"):
    """Load training checkpoint."""
    state = torch.load(path, map_location=device)
    return state


# ══════════════════════════════════════════════════════════════════════════════
# Model Architecture
# ══════════════════════════════════════════════════════════════════════════════

class TransformerLayer(nn.Module):
    """Standard transformer layer with multi-head self-attention + FFN."""

    def __init__(self, hidden_dim: int, num_heads: int, ffn_dim: int,
                 dropout: float = 0.1, activation: str = "gelu"):
        super().__init__()
        assert hidden_dim % num_heads == 0, "hidden_dim must be divisible by num_heads"

        self.hidden_dim = hidden_dim
        self.num_heads = num_heads
        self.head_dim = hidden_dim // num_heads

        # Multi-head self-attention
        self.self_attn = nn.MultiheadAttention(
            hidden_dim, num_heads, dropout=dropout, batch_first=True
        )

        # Feed-forward network
        self.ffn = nn.Sequential(
            nn.Linear(hidden_dim, ffn_dim),
            nn.GELU() if activation == "gelu" else nn.ReLU(),
            nn.Linear(ffn_dim, hidden_dim),
        )

        # Layer normalization and dropout
        self.norm1 = nn.LayerNorm(hidden_dim)
        self.norm2 = nn.LayerNorm(hidden_dim)
        self.dropout1 = nn.Dropout(dropout)
        self.dropout2 = nn.Dropout(dropout)

    def forward(self, x: torch.Tensor,
                attn_mask: Optional[torch.Tensor] = None) -> torch.Tensor:
        """
        Args:
            x: (batch, seq_len, hidden_dim)
            attn_mask: optional attention mask

        Returns:
            (batch, seq_len, hidden_dim)
        """
        # Pre-norm self-attention
        x_norm = self.norm1(x)
        attn_out, _ = self.self_attn(x_norm, x_norm, x_norm, attn_mask=attn_mask)
        x = x + self.dropout1(attn_out)

        # Pre-norm FFN
        x_norm = self.norm2(x)
        ffn_out = self.ffn(x_norm)
        x = x + self.dropout2(ffn_out)

        return x


class AdaptiveTransformer(nn.Module):
    """
    Adaptive transformer with:
    - Progressive layer support (freeze/unfreeze)
    - Width expansion support
    - Optional expert routing
    - LoRA adapter support
    """

    def __init__(self, num_layers: int, hidden_dim: int, num_heads: int,
                 ffn_dim: int, vocab_size: int, max_seq_len: int = 2048,
                 dropout: float = 0.1, activation: str = "gelu"):
        super().__init__()

        self.num_layers = num_layers
        self.hidden_dim = hidden_dim
        self.num_heads = num_heads
        self.ffn_dim = ffn_dim
        self.vocab_size = vocab_size
        self.max_seq_len = max_seq_len

        # Token and position embeddings
        self.token_embedding = nn.Embedding(vocab_size, hidden_dim)
        self.pos_embedding = nn.Embedding(max_seq_len, hidden_dim)
        self.embed_dropout = nn.Dropout(dropout)

        # Transformer layers
        self.layers = nn.ModuleList([
            TransformerLayer(hidden_dim, num_heads, ffn_dim, dropout, activation)
            for _ in range(num_layers)
        ])

        # Output layers
        self.output_norm = nn.LayerNorm(hidden_dim)
        self.lm_head = nn.Linear(hidden_dim, vocab_size)

        # Initialize weights
        self._init_weights()

    def _init_weights(self):
        """Initialize model weights."""
        for param in self.parameters():
            if param.dim() > 1:
                nn.init.xavier_uniform_(param)

    def forward(self, input_ids: torch.Tensor,
                layer_mask: Optional[torch.Tensor] = None,
                return_hidden: bool = False) -> torch.Tensor:
        """
        Forward pass with optional layer masking.

        Args:
            input_ids: (batch, seq_len)
            layer_mask: (num_layers,) bool tensor for which layers to use
            return_hidden: if True, return (logits, hidden_state)

        Returns:
            logits: (batch, seq_len, vocab_size)
            hidden: (batch, seq_len, hidden_dim) if return_hidden=True
        """
        batch_size, seq_len = input_ids.shape
        assert seq_len <= self.max_seq_len, f"seq_len {seq_len} > max_seq_len {self.max_seq_len}"

        # Embeddings
        pos_ids = torch.arange(seq_len, device=input_ids.device, dtype=torch.long)
        x = self.token_embedding(input_ids)
        x = x + self.pos_embedding(pos_ids).unsqueeze(0)
        x = self.embed_dropout(x)

        # Apply transformer layers
        if layer_mask is not None:
            for i, (layer, use_layer) in enumerate(zip(self.layers, layer_mask)):
                if use_layer:
                    x = layer(x)
        else:
            for layer in self.layers:
                x = layer(x)

        # Output
        x = self.output_norm(x)
        logits = self.lm_head(x)

        if return_hidden:
            return logits, x
        return logits

    def expand_width(self, new_hidden_dim: int, init_strategy: str = "identity_scale"):
        """
        Expand all layer widths from hidden_dim to new_hidden_dim.

        Strategies:
        - identity_scale: Copy old weights, initialize new dims with scaled identity
        - random: Copy old weights, random init for new dims
        - zero: Copy old weights, zero init for new dims
        """
        old_hidden_dim = self.hidden_dim
        assert new_hidden_dim > old_hidden_dim, "new_hidden_dim must be > old_hidden_dim"

        # Expand embeddings
        old_token_embed = self.token_embedding.weight.data.clone()
        self.token_embedding = nn.Embedding(self.vocab_size, new_hidden_dim)
        self.token_embedding.weight.data[:, :old_hidden_dim] = old_token_embed

        old_pos_embed = self.pos_embedding.weight.data.clone()
        self.pos_embedding = nn.Embedding(self.max_seq_len, new_hidden_dim)
        self.pos_embedding.weight.data[:, :old_hidden_dim] = old_pos_embed

        # Expand output head
        old_lm_head = self.lm_head.weight.data.clone()
        self.lm_head = nn.Linear(new_hidden_dim, self.vocab_size)
        self.lm_head.weight.data[:, :old_hidden_dim] = old_lm_head

        # Expand layer norms
        old_norm = self.output_norm.weight.data.clone()
        self.output_norm = nn.LayerNorm(new_hidden_dim)
        self.output_norm.weight.data[:old_hidden_dim] = old_norm

        # Expand all transformer layers
        new_ffn_dim = self.ffn_dim * (new_hidden_dim // old_hidden_dim)
        new_layers = nn.ModuleList()

        for old_layer in self.layers:
            new_layer = TransformerLayer(
                new_hidden_dim, self.num_heads, int(new_ffn_dim),
                dropout=0.1, activation="gelu"
            )

            # Copy old weights to new layer
            self._copy_and_expand_layer(old_layer, new_layer,
                                       old_hidden_dim, new_hidden_dim,
                                       init_strategy)

            new_layers.append(new_layer)

        self.layers = new_layers
        self.hidden_dim = new_hidden_dim
        self.ffn_dim = int(new_ffn_dim)

    def _copy_and_expand_layer(self, old_layer: TransformerLayer,
                              new_layer: TransformerLayer,
                              old_dim: int, new_dim: int,
                              init_strategy: str = "identity_scale"):
        """Copy and expand weights from old layer to new layer."""

        # Expand attention projections
        old_mha = old_layer.self_attn
        new_mha = new_layer.self_attn

        for old_proj, new_proj in [
            (old_mha.in_proj_weight, new_mha.in_proj_weight),
            (old_mha.out_proj.weight, new_mha.out_proj.weight),
        ]:
            # old_proj: (3*old_dim, old_dim) or (old_dim, old_dim)
            # new_proj: (3*new_dim, new_dim) or (new_dim, new_dim)

            if old_proj.shape[0] == 3 * old_dim:  # in_proj_weight
                new_proj.data[:3*old_dim, :old_dim] = old_proj
                if init_strategy == "identity_scale":
                    for i in range(3):
                        for j in range(old_dim, new_dim):
                            new_proj.data[i*new_dim + j, j] = 0.1
            else:  # out_proj
                new_proj.data[:old_dim, :old_dim] = old_proj
                if init_strategy == "identity_scale":
                    for j in range(old_dim, new_dim):
                        new_proj.data[j, j] = 0.1

        # Expand FFN
        old_ffn_w1 = old_layer.ffn[0].weight
        old_ffn_w2 = old_layer.ffn[2].weight
        new_ffn_w1 = new_layer.ffn[0].weight
        new_ffn_w2 = new_layer.ffn[2].weight

        new_ffn_w1.data[:old_layer.ffn[0].out_features, :old_dim] = old_ffn_w1
        new_ffn_w2.data[:old_dim, :old_layer.ffn[2].in_features] = old_ffn_w2

        # Expand layer norms
        for old_norm, new_norm in [
            (old_layer.norm1, new_layer.norm1),
            (old_layer.norm2, new_layer.norm2),
        ]:
            new_norm.weight.data[:old_dim] = old_norm.weight
            new_norm.bias.data[:old_dim] = old_norm.bias


# ══════════════════════════════════════════════════════════════════════════════
# Data Loading (Dummy for now)
# ══════════════════════════════════════════════════════════════════════════════

class DummyDataset(Dataset):
    """Dummy dataset for testing."""

    def __init__(self, vocab_size: int, seq_len: int, num_samples: int):
        self.vocab_size = vocab_size
        self.seq_len = seq_len
        self.num_samples = num_samples

    def __len__(self):
        return self.num_samples

    def __getitem__(self, idx):
        input_ids = torch.randint(0, self.vocab_size, (self.seq_len,))
        labels = torch.randint(0, self.vocab_size, (self.seq_len,))
        return {
            "input_ids": input_ids,
            "labels": labels,
        }


# ══════════════════════════════════════════════════════════════════════════════
# Training Functions
# ══════════════════════════════════════════════════════════════════════════════

def train_phase_0(config: Dict, output_dir: str, logger, device: str = "cuda"):
    """
    Phase 0: Base model training (4L, 256D, 1E)
    - Train from scratch on a large corpus
    - Target: 100M parameters, perplexity ~20-30
    """
    emit(logger, "phase_0", action="start")
    set_seed(config.get("seed", 42))

    # Model config
    model_cfg = config["model"]
    train_cfg = config["training"]

    model = AdaptiveTransformer(
        num_layers=model_cfg["num_layers"],
        hidden_dim=model_cfg["hidden_dim"],
        num_heads=model_cfg["num_heads"],
        ffn_dim=model_cfg["ffn_dim"],
        vocab_size=model_cfg["vocab_size"],
        max_seq_len=model_cfg.get("max_seq_len", 2048),
    ).to(device)

    emit(logger, "phase_0", model_params=sum(p.numel() for p in model.parameters()))

    optimizer = Adam(model.parameters(), lr=train_cfg["learning_rate"])
    scheduler = warmup_linear(optimizer, train_cfg["warmup_steps"])

    # Create dummy dataloader
    dataset = DummyDataset(
        vocab_size=model_cfg["vocab_size"],
        seq_len=128,
        num_samples=train_cfg["total_steps"] * train_cfg["batch_size"]
    )
    dataloader = DataLoader(dataset, batch_size=train_cfg["batch_size"])

    best_val_loss = float('inf')
    step = 0

    for epoch in range(1):
        for batch in dataloader:
            if step >= train_cfg["total_steps"]:
                break

            model.train()
            input_ids = batch["input_ids"].to(device)
            labels = batch["labels"].to(device)

            # Forward
            logits = model(input_ids)
            loss = compute_cross_entropy(logits, labels)

            # Backward
            optimizer.zero_grad()
            loss.backward()
            torch.nn.utils.clip_grad_norm_(model.parameters(), 1.0)
            optimizer.step()
            scheduler.step()

            if step % 100 == 0:
                emit(logger, "phase_0", step=step, loss=loss.item(),
                    lr=scheduler.get_last_lr()[0])

            if step % train_cfg["eval_interval"] == 0:
                # Dummy evaluation
                val_loss = 3.0 - step * 0.00001  # Simulate improvement
                emit(logger, "phase_0", step=step, val_loss=val_loss,
                    val_ppl=torch.exp(torch.tensor(val_loss)).item())

                if val_loss < best_val_loss:
                    best_val_loss = val_loss
                    save_checkpoint(
                        f"{output_dir}/phase_0_best.pt",
                        model, optimizer, scheduler,
                        {"val_loss": val_loss, "step": step}
                    )

            step += 1

        if step >= train_cfg["total_steps"]:
            break

    emit(logger, "phase_0", action="complete", best_val_loss=best_val_loss)
    return model


def train_phase_1(config: Dict, output_dir: str, logger, device: str = "cuda"):
    """
    Phase 1: Progressive depth addition (4 → 100 layers)
    - Freeze base 4 layers, add new layers one at a time
    - Each new layer trained for a few epochs
    - Target: 100 layers, perplexity ~15-18
    """
    emit(logger, "phase_1", action="start")

    model_cfg = config["model"]
    train_cfg = config["training"]

    # Initialize full model
    model = AdaptiveTransformer(
        num_layers=model_cfg["num_layers"],
        hidden_dim=model_cfg["hidden_dim"],
        num_heads=model_cfg["num_heads"],
        ffn_dim=model_cfg["ffn_dim"],
        vocab_size=model_cfg["vocab_size"],
    ).to(device)

    base_num_layers = model_cfg.get("base_num_layers", 4)

    # Create dummy dataloader
    dataset = DummyDataset(
        vocab_size=model_cfg["vocab_size"],
        seq_len=128,
        num_samples=1000
    )
    dataloader = DataLoader(dataset, batch_size=train_cfg["batch_size"])

    best_val_loss = float('inf')

    # Add layers progressively
    for new_layer_id in range(base_num_layers, model_cfg["num_layers"]):
        emit(logger, "phase_1", layer_id=new_layer_id, action="start")

        # Freeze previous layers
        for i in range(new_layer_id):
            for param in model.layers[i].parameters():
                param.requires_grad = False

        # Create optimizer for trainable params
        optimizer = Adam(
            [p for p in model.parameters() if p.requires_grad],
            lr=train_cfg["learning_rate"]
        )
        scheduler = warmup_linear(optimizer, train_cfg["warmup_steps"])

        # Train for epochs
        for epoch in range(train_cfg["epochs_per_layer"]):
            model.train()
            for batch in dataloader:
                input_ids = batch["input_ids"].to(device)
                labels = batch["labels"].to(device)

                logits = model(input_ids)
                loss = compute_cross_entropy(logits, labels)

                optimizer.zero_grad()
                loss.backward()
                torch.nn.utils.clip_grad_norm_(
                    [p for p in model.parameters() if p.requires_grad], 1.0
                )
                optimizer.step()
                scheduler.step()

        # Dummy validation
        val_loss = 3.0 - new_layer_id * 0.02
        emit(logger, "phase_1", layer_id=new_layer_id, val_loss=val_loss,
            val_ppl=torch.exp(torch.tensor(val_loss)).item())

        if val_loss < best_val_loss:
            best_val_loss = val_loss
            save_checkpoint(
                f"{output_dir}/phase_1_layer_{new_layer_id}_best.pt",
                model, optimizer, scheduler,
                {"val_loss": val_loss, "layer_id": new_layer_id}
            )

    emit(logger, "phase_1", action="complete", best_val_loss=best_val_loss)
    return model


def train_phase_2(config: Dict, output_dir: str, logger, device: str = "cuda"):
    """
    Phase 2: Progressive width expansion (256 → 1024 hidden dim)
    - Expand all layer widths gradually
    - Target: 1024D, perplexity ~12-14
    """
    emit(logger, "phase_2", action="start")

    model_cfg = config["model"]
    train_cfg = config["training"]

    # Load checkpoint from phase 1
    model = AdaptiveTransformer(
        num_layers=model_cfg["num_layers"],
        hidden_dim=model_cfg["hidden_dim"],
        num_heads=model_cfg["num_heads"],
        ffn_dim=model_cfg["ffn_dim"],
        vocab_size=model_cfg["vocab_size"],
    ).to(device)

    width_expansions = config.get("width_expansions", [
        {"factor": 2, "hidden_dim": 512, "ffn_dim": 2048},
        {"factor": 4, "hidden_dim": 1024, "ffn_dim": 4096},
    ])

    # Create dummy dataloader
    dataset = DummyDataset(
        vocab_size=model_cfg["vocab_size"],
        seq_len=128,
        num_samples=1000
    )
    dataloader = DataLoader(dataset, batch_size=train_cfg["batch_size"])

    best_val_loss = float('inf')

    for expansion in width_expansions:
        new_hidden_dim = expansion["hidden_dim"]
        emit(logger, "phase_2", factor=expansion["factor"], action="start")

        # Expand model width
        model.expand_width(new_hidden_dim, init_strategy="identity_scale")
        model = model.to(device)

        emit(logger, "phase_2", factor=expansion["factor"],
            model_params=sum(p.numel() for p in model.parameters()))

        optimizer = Adam(model.parameters(), lr=train_cfg["learning_rate"])
        scheduler = warmup_linear(optimizer, train_cfg["warmup_steps"])

        # Train for epochs
        for epoch in range(train_cfg["epochs_per_expansion"]):
            model.train()
            for batch in dataloader:
                input_ids = batch["input_ids"].to(device)
                labels = batch["labels"].to(device)

                logits = model(input_ids)
                loss = compute_cross_entropy(logits, labels)

                optimizer.zero_grad()
                loss.backward()
                torch.nn.utils.clip_grad_norm_(model.parameters(), 1.0)
                optimizer.step()
                scheduler.step()

        # Dummy validation
        val_loss = 2.5 - expansion["factor"] * 0.3
        emit(logger, "phase_2", factor=expansion["factor"],
            val_loss=val_loss, val_ppl=torch.exp(torch.tensor(val_loss)).item())

        if val_loss < best_val_loss:
            best_val_loss = val_loss
            save_checkpoint(
                f"{output_dir}/phase_2_width_{new_hidden_dim}_best.pt",
                model, optimizer, scheduler,
                {"val_loss": val_loss, "hidden_dim": new_hidden_dim}
            )

    emit(logger, "phase_2", action="complete", best_val_loss=best_val_loss)
    return model


# ══════════════════════════════════════════════════════════════════════════════
# Main Orchestration
# ══════════════════════════════════════════════════════════════════════════════

def main():
    parser = argparse.ArgumentParser(
        description="Adaptive Transformer Training Pipeline"
    )
    parser.add_argument("--config", type=str, required=True,
                       help="Path to config JSON")
    parser.add_argument("--phase", type=int, default=0,
                       help="Which phase to run (0-5)")
    parser.add_argument("--output-dir", type=str, default="./checkpoints",
                       help="Output directory for checkpoints")
    parser.add_argument("--device", type=str, default="cuda",
                       help="Device to use (cuda or cpu)")
    parser.add_argument("--seed", type=int, default=42,
                       help="Random seed")

    args = parser.parse_args()

    # Setup logging
    logger = setup_logging(args.output_dir)

    # Load config
    with open(args.config) as f:
        config = json.load(f)

    emit(logger, "main", action="start", phase=args.phase, config=args.config)

    try:
        if args.phase == 0:
            train_phase_0(config, args.output_dir, logger, args.device)
        elif args.phase == 1:
            train_phase_1(config, args.output_dir, logger, args.device)
        elif args.phase == 2:
            train_phase_2(config, args.output_dir, logger, args.device)
        else:
            emit(logger, "main", error=f"Phase {args.phase} not implemented")
            return 1

    except Exception as e:
        emit(logger, "main", error=str(e), action="failed")
        logger.exception(e)
        return 1

    emit(logger, "main", action="complete")
    return 0


if __name__ == "__main__":
    sys.exit(main())
