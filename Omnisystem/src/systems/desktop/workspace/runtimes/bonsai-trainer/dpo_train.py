#!/usr/bin/env python3
"""
BonsAI DPO (Direct Preference Optimisation) fine-tuning — 100% offline.

DPO trains the model to prefer chosen responses over rejected ones without
needing a separate reward model, making it much faster than full RLHF.

Input format (one JSON object per line):
    {"system": "...", "prompt": "...", "chosen": "...", "rejected": "..."}

Usage:
    python dpo_train.py \
        --base-model /path/to/hf-model-dir \
        --data ~/.bonsai/training_export/bonsai_dpo_latest.jsonl \
        --output ~/.bonsai/adapters/bonsai-dpo-v1 \
        --beta 0.1

Apple Silicon (M1/M2/M3) — uses MPS automatically.
AMD 7900 XTX — uses CPU (DirectML crashes on backward; ROCm needs Linux).
NVIDIA — uses CUDA automatically.
"""

import os
os.environ["TRANSFORMERS_OFFLINE"]       = "1"
os.environ["HF_HUB_OFFLINE"]            = "1"
os.environ["HF_DATASETS_OFFLINE"]       = "1"
os.environ["HF_HUB_DISABLE_TELEMETRY"] = "1"

import argparse, json, time
from pathlib import Path

import torch
import torch.nn.functional as F


def emit(tag: str, **kw):
    print(f"[{tag}] " + " ".join(f"{k}={v}" for k, v in kw.items()), flush=True)


# ── Device ────────────────────────────────────────────────────────────────────

def get_device():
    if torch.cuda.is_available():
        emit("device", using="cuda")
        return torch.device("cuda"), torch.float16
    if torch.backends.mps.is_available():
        emit("device", using="mps_apple_silicon")
        return torch.device("mps"), torch.float32
    emit("device", using="cpu")
    return torch.device("cpu"), torch.float32


# ── DPO loss ──────────────────────────────────────────────────────────────────

def dpo_loss(
    policy_chosen_logps: torch.Tensor,
    policy_rejected_logps: torch.Tensor,
    ref_chosen_logps: torch.Tensor,
    ref_rejected_logps: torch.Tensor,
    beta: float,
) -> torch.Tensor:
    """
    Compute the DPO loss (Rafailov et al., 2023).
    All log-prob tensors have shape (batch,).
    """
    chosen_rewards   = beta * (policy_chosen_logps   - ref_chosen_logps)
    rejected_rewards = beta * (policy_rejected_logps - ref_rejected_logps)
    loss = -F.logsigmoid(chosen_rewards - rejected_rewards).mean()
    return loss


def gather_log_probs(logits: torch.Tensor, labels: torch.Tensor) -> torch.Tensor:
    """Sum per-token log-probs for each sequence in the batch."""
    log_probs  = F.log_softmax(logits[:, :-1, :], dim=-1)
    raw_target = labels[:, 1:]
    # Clamp -100 (ignore_index) to 0 before gather so indices stay in-bounds;
    # the mask below zeroes out those positions in the final sum.
    target     = raw_target.clamp(min=0).unsqueeze(-1)
    token_lp   = log_probs.gather(-1, target).squeeze(-1)
    mask       = (raw_target != -100).float()
    return (token_lp * mask).sum(-1)


# ── Tokenise a preference pair ────────────────────────────────────────────────

def make_text(system: str, prompt: str, response: str) -> str:
    parts = []
    if system:
        parts.append(f"<|system|>\n{system}")
    parts.append(f"<|user|>\n{prompt}")
    parts.append(f"<|assistant|>\n{response}")
    return "\n".join(parts)


def tokenize_pair(tokenizer, system: str, prompt: str, chosen: str, rejected: str,
                  max_len: int = 512):
    chosen_text   = make_text(system, prompt, chosen)
    rejected_text = make_text(system, prompt, rejected)

    def enc(text):
        ids = tokenizer(text, truncation=True, max_length=max_len,
                        padding="max_length", return_tensors="pt")
        labels = ids["input_ids"].clone()
        labels[labels == tokenizer.pad_token_id] = -100
        return ids["input_ids"], labels

    chosen_ids,   chosen_labels   = enc(chosen_text)
    rejected_ids, rejected_labels = enc(rejected_text)
    return chosen_ids, chosen_labels, rejected_ids, rejected_labels


# ── Main ──────────────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(description="BonsAI DPO training — offline only")
    parser.add_argument("--base-model", required=True,
        help="Local HF model directory (must already be on disk).")
    parser.add_argument("--data",        required=True,
        help="JSONL file with {system, prompt, chosen, rejected} rows.")
    parser.add_argument("--output",      required=True,
        help="Output directory for the LoRA adapter.")
    parser.add_argument("--beta",        type=float, default=0.1,
        help="DPO temperature (lower = tighter alignment).")
    parser.add_argument("--epochs",      type=int,   default=1)
    parser.add_argument("--lr",          type=float, default=5e-5)
    parser.add_argument("--max-pairs",   type=int,   default=2000,
        help="Cap training pairs to prevent OOM.")
    parser.add_argument("--lora-rank",   type=int,   default=16)
    parser.add_argument("--device", default=None,
        help="Force device: cpu | cuda | mps | directml. Default: auto-detect.")
    parser.add_argument("--max-length", type=int, default=256,
        help="Max token length per sequence. Lower = less RAM. Default: 256.")
    args = parser.parse_args()

    base = Path(args.base_model)
    if not (base / "config.json").exists():
        raise SystemExit(f"[model] ERROR: No config.json at {base}. "
                         f"Pass a valid local HF snapshot directory.")

    if args.device:
        _forced = args.device.lower()
        if _forced == "directml":
            try:
                import torch_directml
                dev = torch_directml.device()
                dtype = torch.float32
                emit("device", using="directml_forced")
            except ImportError:
                emit("device", warning="torch-directml not installed, falling back to CPU")
                dev, dtype = torch.device("cpu"), torch.float32
        elif _forced == "cpu":
            dev, dtype = torch.device("cpu"), torch.float32
            emit("device", using="cpu_forced")
        elif _forced == "cuda":
            dev, dtype = torch.device("cuda"), torch.float16
            emit("device", using="cuda_forced")
        elif _forced == "mps":
            dev, dtype = torch.device("mps"), torch.float32
            emit("device", using="mps_forced")
        else:
            dev, dtype = get_device()
    else:
        dev, dtype = get_device()
    emit("dpo", beta=args.beta, epochs=args.epochs, lr=args.lr)

    # ── Load base model + LoRA ───────────────────────────────────────────────
    from transformers import AutoModelForCausalLM, AutoTokenizer
    from peft import LoraConfig, get_peft_model, PeftModel

    emit("load", status="loading", path=str(base))
    tokenizer = AutoTokenizer.from_pretrained(str(base), local_files_only=True)
    tokenizer.pad_token = tokenizer.eos_token

    # Policy model (trained)
    policy = AutoModelForCausalLM.from_pretrained(
        str(base), torch_dtype=dtype, local_files_only=True)
    lora_cfg = LoraConfig(
        r=args.lora_rank, lora_alpha=args.lora_rank * 2,
        target_modules=["q_proj", "k_proj", "v_proj", "o_proj"],
        lora_dropout=0.05, bias="none", task_type="CAUSAL_LM",
    )
    policy = get_peft_model(policy, lora_cfg)
    policy.to(dev)
    policy.train()

    # Reference model (frozen — same weights, no LoRA)
    reference = AutoModelForCausalLM.from_pretrained(
        str(base), torch_dtype=dtype, local_files_only=True)
    reference.to(dev)
    reference.eval()
    for p in reference.parameters():
        p.requires_grad_(False)

    emit("load", status="done")
    policy.print_trainable_parameters()

    # ── Load data ────────────────────────────────────────────────────────────
    pairs = []
    with open(args.data, encoding="utf-8") as f:
        for ln in f:
            ln = ln.strip()
            if not ln:
                continue
            try:
                obj = json.loads(ln)
                if "prompt" in obj and "chosen" in obj and "rejected" in obj:
                    pairs.append(obj)
            except Exception:
                pass

    if not pairs:
        raise SystemExit("[data] ERROR: No valid preference pairs found. "
                         "Each row needs {prompt, chosen, rejected}.")

    if len(pairs) > args.max_pairs:
        import random; random.shuffle(pairs)
        pairs = pairs[:args.max_pairs]

    emit("data", pairs=len(pairs))

    # ── Training loop ────────────────────────────────────────────────────────
    optimiser = torch.optim.AdamW(
        [p for p in policy.parameters() if p.requires_grad],
        lr=args.lr, weight_decay=0.01,
    )

    t0 = time.time()
    global_step = 0
    total_steps = len(pairs) * args.epochs

    for epoch in range(args.epochs):
        import random as _r; _r.shuffle(pairs)
        epoch_loss = 0.0

        for i, pair in enumerate(pairs):
            system   = pair.get("system", "")
            prompt   = pair["prompt"]
            chosen   = pair["chosen"]
            rejected = pair["rejected"]

            c_ids, c_labels, r_ids, r_labels = tokenize_pair(
                tokenizer, system, prompt, chosen, rejected, max_len=args.max_length)
            c_ids   = c_ids.to(dev);   c_labels   = c_labels.to(dev)
            r_ids   = r_ids.to(dev);   r_labels   = r_labels.to(dev)

            # Policy log-probs
            policy_c_logits = policy(input_ids=c_ids).logits
            policy_r_logits = policy(input_ids=r_ids).logits
            policy_c_lp     = gather_log_probs(policy_c_logits, c_labels)
            policy_r_lp     = gather_log_probs(policy_r_logits, r_labels)

            # Reference log-probs (no grad)
            with torch.no_grad():
                ref_c_logits = reference(input_ids=c_ids).logits
                ref_r_logits = reference(input_ids=r_ids).logits
                ref_c_lp     = gather_log_probs(ref_c_logits, c_labels)
                ref_r_lp     = gather_log_probs(ref_r_logits, r_labels)

            loss = dpo_loss(policy_c_lp, policy_r_lp, ref_c_lp, ref_r_lp, args.beta)

            optimiser.zero_grad()
            loss.backward()
            torch.nn.utils.clip_grad_norm_(policy.parameters(), 1.0)
            optimiser.step()

            epoch_loss += loss.item()
            global_step += 1

            if global_step % 20 == 0:
                elapsed = time.time() - t0
                eta = elapsed / global_step * (total_steps - global_step)
                emit("progress",
                     step=global_step, total=total_steps,
                     epoch=f"{epoch+1}/{args.epochs}",
                     loss=f"{loss.item():.4f}",
                     avg_loss=f"{epoch_loss/(i+1):.4f}",
                     elapsed=f"{elapsed:.0f}s", eta=f"{eta:.0f}s")

        emit("epoch", epoch=epoch+1, avg_loss=f"{epoch_loss/len(pairs):.4f}")

    elapsed = time.time() - t0
    emit("train", status="complete", steps=global_step, elapsed=f"{elapsed:.0f}s")

    # ── Save ─────────────────────────────────────────────────────────────────
    out = Path(args.output)
    out.mkdir(parents=True, exist_ok=True)
    policy.save_pretrained(str(out))
    tokenizer.save_pretrained(str(out))
    emit("save", path=str(out))


if __name__ == "__main__":
    main()
