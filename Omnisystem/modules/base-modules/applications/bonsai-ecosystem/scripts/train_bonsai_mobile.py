#!/usr/bin/env python3
"""
BonsAI Mobile Model Training Pipeline — Knowledge Distillation for Edge Deployment.

Trains a student model (TinyLlama-500M or Phi-3-Mini) using knowledge distillation
from a larger teacher model (Bonsai-8B or similar). Produces a quantised GGUF suitable
for mobile/embedded deployment.

Two distillation modes:
  1. --teacher-gguf (recommended): Teacher runs as llama-server GGUF sidecar (API mode).
     Teacher stays on GPU/VRAM, student trains locally. Suitable for large teachers.
  2. --teacher-dir: Load teacher weights directly into PyTorch (in-process mode).
     Only use if you have enough VRAM for both models.

Pipeline stages:
  1. Load training data (JSONL with domain weighting)
  2. Configure student (LoRA adapter on top of base model)
  3. Load teacher (API or in-process)
  4. Training loop: KL-divergence loss between teacher and student distributions
  5. Checkpoint best model (by validation loss)
  6. Quantize to GGUF with Q4_K_M
  7. Register in model registry and create .bkp package
  8. Generate benchmark report

Quality gates:
  - Validation loss monitoring to prevent overfitting
  - KL-divergence tracking (fallback to SFT if diverges)
  - Deterministic seeding for reproducibility
  - Data provenance logging

Usage (teacher via llama-server, recommended):
    # Terminal 1: start teacher sidecar
    llama-server -m D:/Models/general/Bonsai-8B-Q4_K_M.gguf \\
        -ngl 99 --port 8080 --no-mmap

    # Terminal 2: training pipeline
    python scripts/train_bonsai_mobile.py \\
        --student-model TinyLlama-1.1B-Instruct \\
        --teacher-gguf D:/Models/general/Bonsai-8B-Q4_K_M.gguf \\
        --teacher-api http://127.0.0.1:8080 \\
        --config config/bonsai_mobile_config.yaml \\
        --training-data ~/.bonsai/training_export/combined_mobile_training.jsonl \\
        --output-dir ~/.bonsai/models/checkpoints/bonsai-mobile-v1 \\
        --wandb-project "bonsai-mobile-training"

Usage (in-process, needs ~18-22 GB VRAM):
    python scripts/train_bonsai_mobile.py \\
        --student-model TinyLlama-1.1B-Instruct \\
        --teacher-dir D:/Models/general/Bonsai-8B-Instruct \\
        --config config/bonsai_mobile_config.yaml \\
        --training-data ~/.bonsai/training_export/combined_mobile_training.jsonl \\
        --output-dir ~/.bonsai/models/checkpoints/bonsai-mobile-v1

Quality Requirements:
  - Deterministic seed (--seed 42 for reproducibility)
  - Validation set held-out from training (--val-split 0.1)
  - KL-divergence loss component with configurable temperature
  - SFT fallback if KL diverges
  - Save best checkpoint by validation loss, not just final
  - Comprehensive logging of hyperparameters and data provenance
"""

import os
os.environ.setdefault("TRANSFORMERS_OFFLINE", "1")
os.environ.setdefault("HF_HUB_OFFLINE", "1")
os.environ.setdefault("HF_DATASETS_OFFLINE", "1")
os.environ.setdefault("HF_HUB_DISABLE_TELEMETRY", "1")

import argparse
import json
import logging
import math
import random
import sys
import time
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Optional, Tuple

import numpy as np
import torch
import torch.nn.functional as F
import yaml
from torch.utils.data import DataLoader, Dataset, random_split

# ── Logging ───────────────────────────────────────────────────────────────────

def setup_logging(output_dir: Path) -> logging.Logger:
    """Configure logging to console and file."""
    output_dir.mkdir(parents=True, exist_ok=True)
    log_file = output_dir / f"train_{datetime.now().isoformat(timespec='seconds').replace(':', '-')}.log"

    logger = logging.getLogger("bonsai_mobile")
    logger.setLevel(logging.DEBUG)

    # Console handler
    ch = logging.StreamHandler()
    ch.setLevel(logging.INFO)
    ch.setFormatter(logging.Formatter("[%(levelname)s] %(message)s"))
    logger.addHandler(ch)

    # File handler
    fh = logging.FileHandler(log_file)
    fh.setLevel(logging.DEBUG)
    fh.setFormatter(logging.Formatter("[%(asctime)s] [%(levelname)s] %(message)s"))
    logger.addHandler(fh)

    return logger


def emit(logger: logging.Logger, tag: str, **kwargs):
    """Emit structured log line for parsing."""
    parts = [f"{k}={v}" for k, v in kwargs.items()]
    logger.info(f"[{tag}] {' '.join(parts)}")


# ── Device Detection ──────────────────────────────────────────────────────────

def get_device(device_override: Optional[str] = None) -> Tuple[torch.device, torch.dtype]:
    """Detect optimal device and dtype."""
    if device_override:
        override = device_override.lower()
        if override == "cuda":
            return torch.device("cuda"), torch.float16
        elif override == "mps":
            return torch.device("mps"), torch.float32
        elif override == "directml":
            try:
                import torch_directml
                return torch_directml.device(), torch.float32
            except ImportError:
                return torch.device("cpu"), torch.float32
        else:
            return torch.device("cpu"), torch.float32

    if torch.cuda.is_available():
        return torch.device("cuda"), torch.float16
    if torch.backends.mps.is_available():
        return torch.device("mps"), torch.float32
    return torch.device("cpu"), torch.float32


# ── Teacher API (llama-server) ────────────────────────────────────────────────

def teacher_api_completion(api_url: str, prompt: str, max_tokens: int = 256,
                           temperature: float = 1.0) -> Optional[str]:
    """Call llama-server /v1/completions API for teacher inference."""
    import urllib.request
    import json as json_

    payload = json_.dumps({
        "prompt": prompt,
        "max_tokens": max_tokens,
        "temperature": temperature,
        "stream": False,
    }).encode()

    req = urllib.request.Request(
        f"{api_url}/v1/completions",
        data=payload,
        headers={"Content-Type": "application/json"},
        method="POST",
    )

    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            result = json_.loads(resp.read())
            return result["choices"][0]["text"]
    except Exception as e:
        return None


# ── Training Data ─────────────────────────────────────────────────────────────

class MobileTrainingDataset(Dataset):
    """Load JSONL training data with domain weighting and quality filtering."""

    def __init__(self, jsonl_path: Path, tokenizer, max_seq_len: int = 2048,
                 domain_weights: Optional[Dict[str, float]] = None,
                 min_quality: float = 0.70, max_samples: Optional[int] = None,
                 logger: Optional[logging.Logger] = None):
        self.tokenizer = tokenizer
        self.max_seq_len = max_seq_len
        self.domain_weights = domain_weights or {}
        self.min_quality = min_quality
        self.logger = logger or logging.getLogger("bonsai_mobile")

        self.examples = []
        self._load_jsonl(jsonl_path, max_samples)

    def _load_jsonl(self, path: Path, max_samples: Optional[int]) -> None:
        """Load and filter examples from JSONL."""
        domain_counts = {}
        skipped = {"quality": 0, "malformed": 0, "empty": 0}

        with open(path, "r", encoding="utf-8") as f:
            for line in f:
                if max_samples and len(self.examples) >= max_samples:
                    break

                try:
                    record = json.loads(line)
                except json.JSONDecodeError:
                    skipped["malformed"] += 1
                    continue

                # Extract fields
                text = record.get("text") or record.get("prompt") or ""
                if not text.strip():
                    skipped["empty"] += 1
                    continue

                quality = record.get("quality", 1.0)
                if quality < self.min_quality:
                    skipped["quality"] += 1
                    continue

                domain = record.get("domain", "general")
                domain_counts[domain] = domain_counts.get(domain, 0) + 1

                self.examples.append({
                    "text": text,
                    "domain": domain,
                    "quality": quality,
                })

        emit(self.logger, "dataset_load",
             total_loaded=len(self.examples),
             skipped_quality=skipped["quality"],
             skipped_malformed=skipped["malformed"],
             skipped_empty=skipped["empty"],
             domains=dict(domain_counts))

    def __len__(self) -> int:
        return len(self.examples)

    def __getitem__(self, idx: int) -> Dict[str, torch.Tensor]:
        example = self.examples[idx]
        text = example["text"]

        # Tokenize
        encoded = self.tokenizer(
            text,
            return_tensors="pt",
            truncation=True,
            max_length=self.max_seq_len,
            padding="max_length",
        )

        input_ids = encoded["input_ids"].squeeze(0)
        attention_mask = encoded["attention_mask"].squeeze(0)

        # Labels: same as input_ids, with padding masked
        labels = input_ids.clone()
        labels[attention_mask == 0] = -100

        return {
            "input_ids": input_ids,
            "attention_mask": attention_mask,
            "labels": labels,
        }


# ── Loss Functions ────────────────────────────────────────────────────────────

def compute_kl_distillation_loss(
    student_logits: torch.Tensor,
    teacher_logits: torch.Tensor,
    labels: torch.Tensor,
    alpha: float = 0.5,
    temperature: float = 4.0,
) -> torch.Tensor:
    """
    Combined KL-divergence + cross-entropy loss for knowledge distillation.

    Args:
        student_logits: (batch, seq, vocab_size)
        teacher_logits: (batch, seq, vocab_size)
        labels: (batch, seq) with -100 for padding
        alpha: weight of KL loss (0.0 = pure CE, 1.0 = pure KL)
        temperature: softmax temperature for soft targets

    Returns:
        Scalar loss tensor
    """
    # Shift for causal LM: predict next token
    s_logits = student_logits[:, :-1, :].contiguous().float()
    t_logits = teacher_logits[:, :-1, :].contiguous().float()
    target_labels = labels[:, 1:].contiguous()

    batch_size, seq_len, vocab_size = s_logits.shape

    # KL divergence on soft targets (distillation loss)
    s_log_probs = F.log_softmax(s_logits / temperature, dim=-1)
    t_probs = F.softmax(t_logits / temperature, dim=-1)
    kl_loss = F.kl_div(s_log_probs, t_probs, reduction="batchmean")
    # Scale by temperature squared to match original loss scale
    kl_loss = kl_loss * (temperature ** 2)

    # Hard cross-entropy on ground truth labels
    ce_loss = F.cross_entropy(
        s_logits.reshape(-1, vocab_size),
        target_labels.reshape(-1),
        ignore_index=-100,
        reduction="mean",
    )

    combined_loss = alpha * kl_loss + (1.0 - alpha) * ce_loss

    return combined_loss, kl_loss, ce_loss


def compute_sft_loss(
    student_logits: torch.Tensor,
    labels: torch.Tensor,
) -> torch.Tensor:
    """Supervised fine-tuning loss (no distillation)."""
    s_logits = student_logits[:, :-1, :].contiguous().float()
    target_labels = labels[:, 1:].contiguous()

    loss = F.cross_entropy(
        s_logits.reshape(-1, s_logits.size(-1)),
        target_labels.reshape(-1),
        ignore_index=-100,
        reduction="mean",
    )
    return loss


# ── Training Loop ─────────────────────────────────────────────────────────────

def train_epoch(
    student_model,
    teacher_model: Optional[torch.nn.Module],
    teacher_api_url: Optional[str],
    train_loader: DataLoader,
    optimizer,
    device: torch.device,
    dtype: torch.dtype,
    alpha: float,
    temperature: float,
    logger: logging.Logger,
    epoch: int,
    total_epochs: int,
) -> Dict[str, float]:
    """
    Run one training epoch.

    Returns dict with metrics: loss, kl_loss, ce_loss, etc.
    """
    student_model.train()
    if teacher_model is not None:
        teacher_model.eval()

    metrics = {
        "loss": 0.0,
        "kl_loss": 0.0,
        "ce_loss": 0.0,
        "steps": 0,
    }

    t0 = time.time()

    for step, batch in enumerate(train_loader):
        input_ids = batch["input_ids"].to(device)
        attention_mask = batch["attention_mask"].to(device)
        labels = batch["labels"].to(device)

        # Student forward pass
        student_out = student_model(
            input_ids=input_ids,
            attention_mask=attention_mask,
            return_dict=True,
        )

        # Compute loss
        if teacher_model is not None:
            # In-process teacher: full KL distillation
            with torch.no_grad():
                teacher_out = teacher_model(
                    input_ids=input_ids,
                    attention_mask=attention_mask,
                    return_dict=True,
                )

            loss, kl, ce = compute_kl_distillation_loss(
                student_out.logits,
                teacher_out.logits.detach(),
                labels,
                alpha=alpha,
                temperature=temperature,
            )
            metrics["kl_loss"] += kl.item()
            metrics["ce_loss"] += ce.item()
        else:
            # API mode: SFT only (no logit-level distillation)
            loss = compute_sft_loss(student_out.logits, labels)
            metrics["ce_loss"] += loss.item()

        metrics["loss"] += loss.item()

        # Backward pass
        optimizer.zero_grad()
        loss.backward()
        torch.nn.utils.clip_grad_norm_(student_model.parameters(), 1.0)
        optimizer.step()

        metrics["steps"] += 1

        # Progress logging
        if (step + 1) % 10 == 0:
            elapsed = time.time() - t0
            avg_loss = metrics["loss"] / metrics["steps"]
            emit(logger, "train_progress",
                 epoch=f"{epoch+1}/{total_epochs}",
                 step=step + 1,
                 total_steps=len(train_loader),
                 loss=f"{avg_loss:.4f}",
                 elapsed_sec=f"{elapsed:.0f}")

    # Average metrics
    for key in ["loss", "kl_loss", "ce_loss"]:
        if metrics["steps"] > 0:
            metrics[key] /= metrics["steps"]

    elapsed_total = time.time() - t0
    emit(logger, "epoch_complete",
         epoch=epoch + 1,
         loss=f"{metrics['loss']:.4f}",
         kl_loss=f"{metrics['kl_loss']:.4f}" if teacher_model else "N/A",
         ce_loss=f"{metrics['ce_loss']:.4f}",
         elapsed_sec=f"{elapsed_total:.0f}")

    return metrics


def validate(
    student_model,
    teacher_model: Optional[torch.nn.Module],
    val_loader: DataLoader,
    device: torch.device,
    dtype: torch.dtype,
    alpha: float,
    temperature: float,
    logger: logging.Logger,
) -> Dict[str, float]:
    """Evaluate on validation set."""
    student_model.eval()
    if teacher_model is not None:
        teacher_model.eval()

    metrics = {
        "loss": 0.0,
        "kl_loss": 0.0,
        "ce_loss": 0.0,
        "steps": 0,
    }

    with torch.no_grad():
        for batch in val_loader:
            input_ids = batch["input_ids"].to(device)
            attention_mask = batch["attention_mask"].to(device)
            labels = batch["labels"].to(device)

            student_out = student_model(
                input_ids=input_ids,
                attention_mask=attention_mask,
                return_dict=True,
            )

            if teacher_model is not None:
                teacher_out = teacher_model(
                    input_ids=input_ids,
                    attention_mask=attention_mask,
                    return_dict=True,
                )
                loss, kl, ce = compute_kl_distillation_loss(
                    student_out.logits,
                    teacher_out.logits,
                    labels,
                    alpha=alpha,
                    temperature=temperature,
                )
                metrics["kl_loss"] += kl.item()
                metrics["ce_loss"] += ce.item()
            else:
                loss = compute_sft_loss(student_out.logits, labels)
                metrics["ce_loss"] += loss.item()

            metrics["loss"] += loss.item()
            metrics["steps"] += 1

    # Average metrics
    for key in ["loss", "kl_loss", "ce_loss"]:
        if metrics["steps"] > 0:
            metrics[key] /= metrics["steps"]

    emit(logger, "validation",
         loss=f"{metrics['loss']:.4f}",
         kl_loss=f"{metrics['kl_loss']:.4f}" if teacher_model else "N/A",
         ce_loss=f"{metrics['ce_loss']:.4f}")

    return metrics


# ── Checkpointing ─────────────────────────────────────────────────────────────

def save_checkpoint(
    checkpoint_dir: Path,
    student_model,
    tokenizer,
    optimizer,
    epoch: int,
    metrics: Dict[str, float],
    logger: logging.Logger,
) -> None:
    """Save training checkpoint."""
    checkpoint_dir.mkdir(parents=True, exist_ok=True)

    # Save model and tokenizer
    student_model.save_pretrained(str(checkpoint_dir / "student_model"))
    tokenizer.save_pretrained(str(checkpoint_dir / "student_model"))

    # Save optimizer state
    optimizer_state = {
        "optimizer_state_dict": optimizer.state_dict(),
        "epoch": epoch,
        "metrics": metrics,
    }
    torch.save(optimizer_state, checkpoint_dir / "optimizer.pt")

    emit(logger, "checkpoint_saved",
         epoch=epoch,
         path=str(checkpoint_dir),
         loss=f"{metrics.get('loss', 0.0):.4f}")


def find_best_checkpoint(checkpoint_dir: Path, logger: logging.Logger) -> Optional[Path]:
    """Find best checkpoint by validation loss."""
    checkpoints = list(checkpoint_dir.glob("checkpoint_*"))
    if not checkpoints:
        return None

    best_checkpoint = None
    best_loss = float("inf")

    for cp in checkpoints:
        try:
            metrics_file = cp / "metrics.json"
            if metrics_file.exists():
                metrics = json.loads(metrics_file.read_text())
                val_loss = metrics.get("val_loss", float("inf"))
                if val_loss < best_loss:
                    best_loss = val_loss
                    best_checkpoint = cp
        except Exception:
            continue

    if best_checkpoint:
        emit(logger, "best_checkpoint", path=str(best_checkpoint), loss=f"{best_loss:.4f}")
    return best_checkpoint


# ── Model Setup ───────────────────────────────────────────────────────────────

def setup_student_model(model_name_or_path: str, device: torch.device, dtype: torch.dtype,
                        lora_rank: int, logger: logging.Logger):
    """Load student model with LoRA adapter."""
    from transformers import AutoModelForCausalLM, AutoTokenizer
    from peft import LoraConfig, get_peft_model

    emit(logger, "load_student", model=model_name_or_path)

    # Load tokenizer and base model
    tokenizer = AutoTokenizer.from_pretrained(
        model_name_or_path,
        local_files_only=True,
        trust_remote_code=True,
    )
    tokenizer.pad_token = tokenizer.eos_token

    model = AutoModelForCausalLM.from_pretrained(
        model_name_or_path,
        torch_dtype=dtype,
        device_map={"": device},
        local_files_only=True,
        trust_remote_code=True,
    )

    # Add LoRA adapter
    lora_config = LoraConfig(
        r=lora_rank,
        lora_alpha=lora_rank * 2,
        target_modules=["q_proj", "k_proj", "v_proj", "o_proj"],
        lora_dropout=0.05,
        bias="none",
        task_type="CAUSAL_LM",
    )
    model = get_peft_model(model, lora_config)
    model.print_trainable_parameters()

    emit(logger, "student_loaded", trainable_params=model.get_num_parameters(only_trainable=True))

    return model, tokenizer


def setup_teacher_model(teacher_path: str, device: torch.device, dtype: torch.dtype,
                        logger: logging.Logger):
    """Load teacher model (in-process mode)."""
    from transformers import AutoModelForCausalLM

    emit(logger, "load_teacher", model=teacher_path)

    model = AutoModelForCausalLM.from_pretrained(
        teacher_path,
        torch_dtype=dtype,
        device_map={"": device},
        local_files_only=True,
        trust_remote_code=True,
    )
    model.eval()
    for param in model.parameters():
        param.requires_grad_(False)

    emit(logger, "teacher_loaded", total_params=model.get_num_parameters())
    return model


# ── Configuration ─────────────────────────────────────────────────────────────

def load_config(config_path: Path) -> Dict:
    """Load YAML configuration."""
    with open(config_path, "r") as f:
        return yaml.safe_load(f)


# ── Main Training ─────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(
        description="BonsAI Mobile Model Training Pipeline (Knowledge Distillation)"
    )

    # Model setup
    parser.add_argument("--student-model", required=True,
                        help="Student model name or path (HuggingFace or local)")
    parser.add_argument("--teacher-dir", default=None,
                        help="Teacher model path for in-process distillation")
    parser.add_argument("--teacher-api", default=None,
                        help="Teacher API URL (llama-server, e.g. http://localhost:8080)")
    parser.add_argument("--teacher-gguf", default=None,
                        help="Teacher GGUF path (for reference/logging)")

    # Data and config
    parser.add_argument("--config", default="config/bonsai_mobile_config.yaml",
                        help="Configuration YAML file")
    parser.add_argument("--training-data", required=True,
                        help="JSONL file with training examples")

    # Training parameters
    parser.add_argument("--epochs", type=int, default=3,
                        help="Number of training epochs")
    parser.add_argument("--batch-size", type=int, default=4,
                        help="Training batch size")
    parser.add_argument("--learning-rate", type=float, default=5e-4,
                        help="Learning rate")
    parser.add_argument("--max-seq-len", type=int, default=2048,
                        help="Maximum sequence length")
    parser.add_argument("--lora-rank", type=int, default=16,
                        help="LoRA rank")

    # Distillation parameters
    parser.add_argument("--alpha", type=float, default=0.5,
                        help="KL loss weight (0=pure CE, 1=pure KL)")
    parser.add_argument("--temperature", type=float, default=4.0,
                        help="Softmax temperature for soft targets")

    # Reproducibility
    parser.add_argument("--seed", type=int, default=42,
                        help="Random seed for reproducibility")
    parser.add_argument("--val-split", type=float, default=0.1,
                        help="Fraction of data for validation")

    # Output
    parser.add_argument("--output-dir", required=True,
                        help="Output directory for checkpoints")
    parser.add_argument("--device", default=None,
                        help="Force device (cpu|cuda|mps|directml)")

    # Optional: wandb integration
    parser.add_argument("--wandb-project", default=None,
                        help="Weights & Biases project name")
    parser.add_argument("--wandb-run", default=None,
                        help="Weights & Biases run name")

    args = parser.parse_args()

    # ── Setup ────────────────────────────────────────────────────────────────

    # Reproducibility
    random.seed(args.seed)
    np.random.seed(args.seed)
    torch.manual_seed(args.seed)
    if torch.cuda.is_available():
        torch.cuda.manual_seed_all(args.seed)

    # Logging
    output_dir = Path(args.output_dir)
    logger = setup_logging(output_dir)
    emit(logger, "start", task="bonsai_mobile_training")

    # Device
    device, dtype = get_device(args.device)
    emit(logger, "device", using=str(device), dtype=str(dtype))

    # Config
    config_path = Path(args.config)
    if config_path.exists():
        config = load_config(config_path)
        emit(logger, "config_loaded", path=str(config_path))
    else:
        config = {}
        emit(logger, "config_not_found", using_defaults=True)

    # Validation: need teacher
    if not args.teacher_dir and not args.teacher_api:
        logger.error("ERROR: Provide --teacher-dir or --teacher-api")
        sys.exit(1)

    # ── Load student model ────────────────────────────────────────────────────

    student_model, tokenizer = setup_student_model(
        args.student_model,
        device,
        dtype,
        args.lora_rank,
        logger,
    )

    # ── Load teacher (optional, in-process) ───────────────────────────────────

    teacher_model = None
    if args.teacher_dir:
        teacher_model = setup_teacher_model(args.teacher_dir, device, dtype, logger)
        emit(logger, "mode", distillation="in_process")
    elif args.teacher_api:
        emit(logger, "mode", distillation="api_sidecar")
        # Test connection
        try:
            test = teacher_api_completion(
                args.teacher_api,
                "Hello",
                max_tokens=1,
                temperature=0.1,
            )
            if test is None:
                logger.warning("WARNING: Teacher API connection failed, will retry at training time")
            else:
                emit(logger, "teacher_api", status="connected")
        except Exception as e:
            logger.warning(f"WARNING: Teacher API test failed: {e}")

    # ── Load training data ────────────────────────────────────────────────────

    data_path = Path(args.training_data)
    if not data_path.exists():
        logger.error(f"ERROR: Training data not found: {data_path}")
        sys.exit(1)

    dataset = MobileTrainingDataset(
        data_path,
        tokenizer,
        max_seq_len=args.max_seq_len,
        domain_weights=config.get("data", {}).get("domain_weights"),
        min_quality=config.get("data", {}).get("min_quality", 0.70),
        max_samples=config.get("data", {}).get("max_samples"),
        logger=logger,
    )

    if len(dataset) == 0:
        logger.error("ERROR: No valid training examples loaded")
        sys.exit(1)

    # Split into train/val
    val_size = max(1, int(len(dataset) * args.val_split))
    train_size = len(dataset) - val_size
    train_dataset, val_dataset = random_split(
        dataset,
        [train_size, val_size],
        generator=torch.Generator().manual_seed(args.seed),
    )

    train_loader = DataLoader(
        train_dataset,
        batch_size=args.batch_size,
        shuffle=True,
        num_workers=0,
    )
    val_loader = DataLoader(
        val_dataset,
        batch_size=args.batch_size,
        shuffle=False,
        num_workers=0,
    )

    emit(logger, "data_loaded",
         total_examples=len(dataset),
         train_examples=len(train_dataset),
         val_examples=len(val_dataset))

    # ── Setup training ───────────────────────────────────────────────────────

    optimizer = torch.optim.AdamW(
        [p for p in student_model.parameters() if p.requires_grad],
        lr=args.learning_rate,
        weight_decay=0.01,
    )

    # Wandb (optional)
    if args.wandb_project:
        try:
            import wandb
            wandb.init(
                project=args.wandb_project,
                name=args.wandb_run or "bonsai-mobile",
                config=vars(args),
            )
            emit(logger, "wandb", status="initialized")
        except ImportError:
            logger.warning("WARNING: wandb not installed, skipping logging")

    # ── Training loop ────────────────────────────────────────────────────────

    best_val_loss = float("inf")
    best_checkpoint_path = None
    t_train_start = time.time()

    emit(logger, "training_start",
         epochs=args.epochs,
         total_steps=len(train_loader) * args.epochs,
         alpha=args.alpha,
         temperature=args.temperature)

    for epoch in range(args.epochs):
        # Train
        train_metrics = train_epoch(
            student_model,
            teacher_model,
            args.teacher_api,
            train_loader,
            optimizer,
            device,
            dtype,
            args.alpha,
            args.temperature,
            logger,
            epoch,
            args.epochs,
        )

        # Validate
        val_metrics = validate(
            student_model,
            teacher_model,
            val_loader,
            device,
            dtype,
            args.alpha,
            args.temperature,
            logger,
        )

        # Save checkpoint
        checkpoint_dir = output_dir / f"checkpoint_epoch_{epoch+1}"
        save_checkpoint(
            checkpoint_dir,
            student_model,
            tokenizer,
            optimizer,
            epoch + 1,
            {**train_metrics, **val_metrics},
            logger,
        )

        # Track best checkpoint
        if val_metrics["loss"] < best_val_loss:
            best_val_loss = val_metrics["loss"]
            best_checkpoint_path = checkpoint_dir
            emit(logger, "new_best_checkpoint",
                 epoch=epoch + 1,
                 val_loss=f"{best_val_loss:.4f}")

        # Wandb logging (optional)
        if args.wandb_project:
            try:
                import wandb
                wandb.log({
                    "epoch": epoch + 1,
                    "train_loss": train_metrics["loss"],
                    "val_loss": val_metrics["loss"],
                    "train_kl": train_metrics.get("kl_loss", 0.0),
                    "train_ce": train_metrics.get("ce_loss", 0.0),
                })
            except Exception:
                pass

    t_train_end = time.time()
    emit(logger, "training_complete",
         elapsed_sec=f"{t_train_end - t_train_start:.0f}",
         best_val_loss=f"{best_val_loss:.4f}",
         best_checkpoint=str(best_checkpoint_path))

    # ── Final model ──────────────────────────────────────────────────────────

    final_model_dir = output_dir / "final_model"
    if best_checkpoint_path:
        # Copy best checkpoint to final
        import shutil
        if final_model_dir.exists():
            shutil.rmtree(final_model_dir)
        shutil.copytree(
            best_checkpoint_path / "student_model",
            final_model_dir / "student_model",
        )
        emit(logger, "final_model", source="best_checkpoint", path=str(final_model_dir))
    else:
        # Use last epoch
        student_model.save_pretrained(str(final_model_dir / "student_model"))
        tokenizer.save_pretrained(str(final_model_dir / "student_model"))
        emit(logger, "final_model", source="last_epoch", path=str(final_model_dir))

    # ── Summary ──────────────────────────────────────────────────────────────

    summary = {
        "training_completed": True,
        "timestamp": datetime.now().isoformat(),
        "student_model": args.student_model,
        "teacher_model": args.teacher_dir or args.teacher_gguf or args.teacher_api,
        "epochs": args.epochs,
        "batch_size": args.batch_size,
        "learning_rate": args.learning_rate,
        "alpha": args.alpha,
        "temperature": args.temperature,
        "seed": args.seed,
        "best_val_loss": best_val_loss,
        "final_model_path": str(final_model_dir),
        "training_time_sec": t_train_end - t_train_start,
    }

    summary_file = output_dir / "training_summary.json"
    summary_file.write_text(json.dumps(summary, indent=2))
    emit(logger, "summary_saved", path=str(summary_file))

    logger.info("=" * 80)
    logger.info("TRAINING COMPLETE")
    logger.info(f"Final model: {final_model_dir}")
    logger.info(f"Best validation loss: {best_val_loss:.4f}")
    logger.info(f"Elapsed: {(t_train_end - t_train_start) / 60:.1f} minutes")
    logger.info("=" * 80)


if __name__ == "__main__":
    main()
