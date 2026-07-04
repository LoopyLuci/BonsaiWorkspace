#!/usr/bin/env python3
"""
BonsAI Knowledge Distillation — 100% offline.

Transfers knowledge from a LARGE teacher model (already on disk) to the
SMALL student model (Bonsai-1.7B) using KL-divergence on soft logit targets.

Two teacher modes:
  --teacher-dir   : Load teacher weights directly into PyTorch (needs RAM/VRAM)
  --teacher-api   : Call a running llama-server on --teacher-port (teacher as GGUF)
                    Teacher runs as a sidecar; student trains locally.
                    This is the recommended mode for large teachers (35B+).

Loss = alpha * KL(teacher||student) + (1-alpha) * CrossEntropy(student, hard_labels)

Usage (teacher via llama-server GGUF sidecar — recommended for 7900 XTX / M1):
    # First: start the teacher sidecar
    llama-server -m /path/to/Qwen3-35B-Q4_K_M.gguf -ngl 99 --port 8080
    # Then:
    python distill.py \
        --student-model  ~/.cache/huggingface/hub/.../snapshots/.../  \
        --teacher-api    http://127.0.0.1:8080 \
        --prompts        ~/.bonsai/training_export/distill_prompts.txt \
        --output         ~/.bonsai/adapters/bonsai-distilled-v1 \
        --alpha 0.5

Usage (teacher loaded in-process — needs enough VRAM for both models):
    python distill.py \
        --student-model  /path/to/student-hf-dir \
        --teacher-dir    /path/to/teacher-hf-dir \
        --prompts        prompts.txt \
        --output         ~/adapters/distilled
"""

import os
os.environ["TRANSFORMERS_OFFLINE"]       = "1"
os.environ["HF_HUB_OFFLINE"]            = "1"
os.environ["HF_DATASETS_OFFLINE"]       = "1"
os.environ["HF_HUB_DISABLE_TELEMETRY"] = "1"

import argparse, json, time, math
from pathlib import Path
from typing import Optional

import torch
import torch.nn.functional as F


def emit(tag: str, **kw):
    print(f"[{tag}] " + " ".join(f"{k}={v}" for k, v in kw.items()), flush=True)


def get_device():
    if torch.cuda.is_available():
        emit("device", using="cuda")
        return torch.device("cuda"), torch.float16
    if torch.backends.mps.is_available():
        emit("device", using="mps_apple_silicon")
        return torch.device("mps"), torch.float32
    emit("device", using="cpu")
    return torch.device("cpu"), torch.float32


# ── Teacher via llama-server API ──────────────────────────────────────────────

def teacher_api_logprobs(api_url: str, prompt: str, temperature: float = 1.0) -> Optional[str]:
    """
    Call llama-server /v1/completions and return the full generated text.
    We use this as a soft-target "chosen" response — the student learns to
    mimic the teacher's output distribution.
    """
    import urllib.request
    payload = json.dumps({
        "prompt": prompt,
        "max_tokens": 256,
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
        with urllib.request.urlopen(req, timeout=60) as resp:
            result = json.loads(resp.read())
            return result["choices"][0]["text"]
    except Exception as e:
        emit("teacher_api", error=str(e))
        return None


# ── KL distillation loss ──────────────────────────────────────────────────────

def distill_loss(
    student_logits: torch.Tensor,
    teacher_logits: torch.Tensor,
    labels: torch.Tensor,
    alpha: float,
    temperature: float,
) -> torch.Tensor:
    """
    Combined KL + cross-entropy loss.
    student_logits, teacher_logits: (batch, seq, vocab)
    labels: (batch, seq) with -100 for padding
    """
    # Shift for causal LM
    s_logits = student_logits[:, :-1, :].float()
    t_logits = teacher_logits[:, :-1, :].float()
    lbl      = labels[:, 1:]

    # KL divergence (soft targets)
    s_log_prob = F.log_softmax(s_logits / temperature, dim=-1)
    t_prob     = F.softmax(t_logits / temperature, dim=-1)
    kl = F.kl_div(s_log_prob, t_prob, reduction="batchmean") * (temperature ** 2)

    # Hard cross-entropy
    ce = F.cross_entropy(
        s_logits.reshape(-1, s_logits.size(-1)),
        lbl.reshape(-1),
        ignore_index=-100,
    )

    return alpha * kl + (1.0 - alpha) * ce


# ── Main ──────────────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(description="BonsAI knowledge distillation")
    parser.add_argument("--student-model", required=True,
        help="Local HF model directory for the student.")
    parser.add_argument("--teacher-dir",   default=None,
        help="Local HF model directory for the teacher (in-process).")
    parser.add_argument("--teacher-api",   default=None,
        help="URL of a running llama-server (e.g. http://127.0.0.1:8080).")
    parser.add_argument("--prompts",       required=True,
        help="Text file with one distillation prompt per line.")
    parser.add_argument("--output",        required=True,
        help="Output directory for the distilled LoRA adapter.")
    parser.add_argument("--alpha",   type=float, default=0.5,
        help="KL loss weight (0=pure CE, 1=pure distillation).")
    parser.add_argument("--temperature", type=float, default=2.0,
        help="Softmax temperature for soft targets (higher = softer).")
    parser.add_argument("--epochs",  type=int,   default=1)
    parser.add_argument("--lr",      type=float, default=1e-4)
    parser.add_argument("--max-prompts", type=int, default=5000)
    parser.add_argument("--lora-rank",   type=int, default=16)
    parser.add_argument("--device", default=None,
        help="Force device: cpu | cuda | mps | directml. Default: auto-detect.")
    args = parser.parse_args()

    if not args.teacher_dir and not args.teacher_api:
        raise SystemExit("[distill] ERROR: Supply --teacher-dir or --teacher-api")

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
    emit("distill", alpha=args.alpha, temperature=args.temperature,
         teacher="api" if args.teacher_api else "local")

    # ── Load student ─────────────────────────────────────────────────────────
    from transformers import AutoModelForCausalLM, AutoTokenizer
    from peft import LoraConfig, get_peft_model

    emit("load", model="student", path=args.student_model)
    tokenizer = AutoTokenizer.from_pretrained(args.student_model, local_files_only=True)
    tokenizer.pad_token = tokenizer.eos_token

    student = AutoModelForCausalLM.from_pretrained(
        args.student_model, torch_dtype=dtype, local_files_only=True)
    lora_cfg = LoraConfig(
        r=args.lora_rank, lora_alpha=args.lora_rank * 2,
        target_modules=["q_proj", "k_proj", "v_proj", "o_proj"],
        lora_dropout=0.05, bias="none", task_type="CAUSAL_LM",
    )
    student = get_peft_model(student, lora_cfg)
    student.to(dev); student.train()
    student.print_trainable_parameters()

    # ── Load teacher (in-process mode) ────────────────────────────────────────
    teacher = None
    if args.teacher_dir:
        emit("load", model="teacher", path=args.teacher_dir)
        teacher = AutoModelForCausalLM.from_pretrained(
            args.teacher_dir, torch_dtype=dtype, local_files_only=True)
        teacher.to(dev); teacher.eval()
        for p in teacher.parameters(): p.requires_grad_(False)
        emit("load", status="teacher_ready")

    # ── Load prompts ──────────────────────────────────────────────────────────
    prompts = []
    with open(args.prompts, encoding="utf-8") as f:
        for ln in f:
            ln = ln.strip()
            if ln:
                prompts.append(ln)

    if len(prompts) > args.max_prompts:
        import random; random.shuffle(prompts); prompts = prompts[:args.max_prompts]
    emit("data", prompts=len(prompts))

    # ── Training ──────────────────────────────────────────────────────────────
    optimiser = torch.optim.AdamW(
        [p for p in student.parameters() if p.requires_grad],
        lr=args.lr, weight_decay=0.01,
    )

    t0 = time.time()
    MAX_LEN = 256
    total = len(prompts) * args.epochs
    step = 0

    for epoch in range(args.epochs):
        import random as _r; _r.shuffle(prompts)
        epoch_loss = 0.0

        for i, prompt in enumerate(prompts):
            # ── Get teacher response ─────────────────────────────────────────
            if args.teacher_api:
                teacher_text = teacher_api_logprobs(args.teacher_api, prompt,
                                                     args.temperature)
                if teacher_text is None:
                    continue
                full_text = prompt + teacher_text
            else:
                # Tokenise prompt for teacher inference
                p_enc = tokenizer(prompt, return_tensors="pt",
                                  truncation=True, max_length=MAX_LEN).to(dev)
                with torch.no_grad():
                    t_out = teacher(**p_enc)
                # Build full text from teacher greedy decode
                teacher_ids = t_out.logits.argmax(-1)
                full_text = tokenizer.decode(teacher_ids[0], skip_special_tokens=True)

            # ── Tokenise for student training ────────────────────────────────
            enc = tokenizer(full_text, return_tensors="pt",
                            truncation=True, max_length=MAX_LEN,
                            padding="max_length")
            input_ids = enc["input_ids"].to(dev)
            labels    = input_ids.clone()
            labels[labels == tokenizer.pad_token_id] = -100

            # ── Forward pass ─────────────────────────────────────────────────
            s_out = student(input_ids=input_ids)

            if teacher is not None:
                with torch.no_grad():
                    t_out = teacher(input_ids=input_ids)
                t_logits = t_out.logits.detach()
                loss = distill_loss(s_out.logits, t_logits, labels,
                                    args.alpha, args.temperature)
            else:
                # API mode: use cross-entropy only (no logit-level KL possible)
                loss = F.cross_entropy(
                    s_out.logits[:, :-1, :].reshape(-1, s_out.logits.size(-1)),
                    labels[:, 1:].reshape(-1),
                    ignore_index=-100,
                )

            optimiser.zero_grad()
            loss.backward()
            torch.nn.utils.clip_grad_norm_(student.parameters(), 1.0)
            optimiser.step()

            epoch_loss += loss.item()
            step += 1

            if step % 50 == 0:
                elapsed = time.time() - t0
                eta = elapsed / step * (total - step) if step < total else 0
                emit("progress", step=step, total=total,
                     epoch=f"{epoch+1}/{args.epochs}",
                     loss=f"{loss.item():.4f}",
                     elapsed=f"{elapsed:.0f}s", eta=f"{eta:.0f}s")

        emit("epoch", epoch=epoch+1,
             avg_loss=f"{epoch_loss/max(1,len(prompts)):.4f}")

    emit("train", status="complete", steps=step,
         elapsed=f"{time.time()-t0:.0f}s")

    out = Path(args.output)
    out.mkdir(parents=True, exist_ok=True)
    student.save_pretrained(str(out))
    tokenizer.save_pretrained(str(out))
    emit("save", path=str(out))


if __name__ == "__main__":
    main()
