#!/usr/bin/env python3
"""Generate DPO preference pairs for the Training Agent.

Uses the local teacher (Qwen3-35B via llama-server) to produce
(chosen = correct training decision, rejected = plausible-but-wrong decision)
for 200+ training scenarios.

Requirements:
  - llama-server running: llama-server -m D:/Models/general/Qwen3-35B-A22B-Q4_K_M.gguf --port 8080
  - ~6 GB VRAM available (teacher inference)

Usage:
    python scripts/generate_training_agent_dpo.py \
        --output ~/.bonsai/training_agent/tier3_dpo_pairs.jsonl \
        --count 10000 \
        --teacher-url http://127.0.0.1:8080
"""
import argparse
import json
import os
import random
import time
from pathlib import Path

os.environ.setdefault("TRANSFORMERS_OFFLINE", "1")
os.environ.setdefault("HF_HUB_OFFLINE", "1")
os.environ.setdefault("HF_DATASETS_OFFLINE", "1")
os.environ.setdefault("HF_HUB_DISABLE_TELEMETRY", "1")

try:
    import requests
    HAS_REQUESTS = True
except ImportError:
    HAS_REQUESTS = False

# ── Hardware context injected into every scenario ─────────────────────────────

HW_CONTEXT = """
Hardware: Windows 10 Pro, AMD Ryzen 5900X (CPU training only), 64 GB RAM.
AMD RX 7900 XTX 24 GB VRAM — inference only via llama-server GGUF. DirectML backward pass crashes.
Python 3.12, torch 2.4.1+cpu, peft 0.19.1.
Critical constraint: max_length must be ≤ 256 on CPU with two model copies; default to 128.
Training scripts: dpo_train.py (DPO), finetune_sft.py (SFT), distill.py (distillation).
All HF env vars must be set to offline before any import.
Do not run training from Git Bash — use PowerShell System.Diagnostics.Process.
"""

# ── Scenario templates ─────────────────────────────────────────────────────────
# Each tuple: (scenario_description, correct_direction_hint, wrong_direction_hint)
# The teacher will elaborate on each based on the correct/wrong hints.

SCENARIOS = [
    # Loss curve issues
    ("DPO training. After epoch 1, loss was 0.45. After epoch 2, loss jumped to 2.3 and kept rising.",
     "gradient explosion due to high beta or high LR; reduce both, add gradient clipping",
     "learning rate is too low, increase it 10x"),
    ("SFT training. Loss dropped fast for 50 steps then plateaued at 0.9 for 400 more steps.",
     "learning rate too high initially, causing fast initial drop then getting stuck in flat region; implement cosine decay or ReduceLROnPlateau",
     "the model has converged, stop training"),
    ("DPO training. Loss = NaN from step 1.",
     "tokenizer pad_token_id not set, causing all-pad labels which produce NaN log-probs; set tokenizer.pad_token = tokenizer.eos_token",
     "float16 precision issue, switch to bfloat16"),
    ("SFT training. Loss at epoch 3 is lower than epoch 1, but validation loss is 3x higher.",
     "overfitting; increase dropout, reduce epochs, add weight decay, consider data augmentation",
     "validation set is too small, ignore it"),
    ("Distillation training. Student loss is not decreasing after 100 steps.",
     "alpha parameter too high (all KL, no hard labels), or teacher temperature too high making distribution too flat; try alpha=0.5 and temperature=1.0",
     "student model is too large, reduce LoRA rank"),
    # OOM / hardware issues
    ("DPO training on CPU crashed with ACCESS_VIOLATION (exit code 0xC0000005) after data loading.",
     "max_length=512 with two model copies (policy + reference) exceeds Windows CPU allocator. Use max_length=128",
     "increase RAM by closing other processes"),
    ("Training script crashed with exit 139 (segfault) when run from Git Bash.",
     "Git Bash signal handling on Windows is unreliable for Python ML processes. Run via PowerShell System.Diagnostics.Process",
     "install WSL2 and run from there"),
    ("torch.OutOfMemoryError during forward pass with 7B model and batch_size=4.",
     "reduce batch_size to 1 with gradient_accumulation_steps=8, and enable gradient checkpointing",
     "upgrade to 16B model which is more memory efficient"),
    # Data issues
    ("Training data has 90% of examples from one category and 10% from another. Model ignores the minority class.",
     "upsample minority class or use weighted sampling; alternatively apply class-balanced loss weighting",
     "add more majority class examples to make the distribution more extreme"),
    ("SFT training. Some examples have response length of 2000 tokens, most are 50 tokens.",
     "the long examples dominate gradient updates; truncate to max_length=512 and use pack-padding; or filter/split long examples",
     "increase max_length to 3000 to accommodate all examples"),
    ("DPO training. All chosen and rejected responses are nearly identical (differ by 1-2 words).",
     "DPO requires meaningful preference differences; generate new dataset with clearly distinct chosen/rejected pairs differing in quality, safety, or helpfulness",
     "reduce beta to 0.01 to handle near-identical pairs"),
    # Hyperparameter tuning
    ("Planning a DPO run on Qwen2.5-0.5B with 50 preference pairs on CPU. What config should I use?",
     "beta=0.1-0.15, lr=5e-5, epochs=3, max_length=128, lora_rank=16, batch_size=1, grad_accum=1; expect ~420s, final loss ~0.001",
     "beta=0.5, lr=1e-3, epochs=10, max_length=512"),
    ("Planning SFT on Qwen2.5-7B with 10,000 examples on CPU. Estimate config and duration.",
     "lora_r=64, lr=2e-4, epochs=3, max_length=128, batch_size=2, grad_accum=16 (effective batch=32); expect 48-72h; monitor for OOM at max_length>128",
     "same config as 0.5B, just with lora_r=16"),
    ("Which LoRA target modules should I use for Qwen2.5 models?",
     "q_proj, k_proj, v_proj, o_proj, gate_proj, up_proj, down_proj — this covers attention and MLP, maximizing expressiveness",
     "only q_proj and v_proj — this is the minimal set used in the original LoRA paper"),
    # Pipeline sequencing
    ("Should I run SFT or DPO first when starting from a base model?",
     "always SFT first: the base model needs to learn the task format before preference tuning makes sense; DPO on untrained base produces unstable gradients",
     "DPO first, since it's more efficient and combines instruction following with alignment"),
    ("After DPO, should I merge the LoRA adapter before GGUF conversion?",
     "yes: merge adapter weights into base model first using peft merge_and_unload(), then convert to GGUF; converting a LoRA adapter directly to GGUF loses the adapter structure",
     "convert directly to GGUF with the adapter separate, llama.cpp handles it automatically"),
    # Distillation specific
    ("Setting up distillation with a 35B teacher and 1.5B student. What alpha value?",
     "alpha=0.5-0.7 for general distillation (balances KL and CE); alpha=0.9 if the teacher is much stronger and you want maximum knowledge transfer; alpha=0.3 if the dataset is high quality and hard labels are reliable",
     "always use alpha=1.0 to get pure KL distillation"),
    ("Teacher API is at localhost:8080. Sometimes it returns HTTP 503 during distillation.",
     "add retry logic with exponential backoff (max 3 retries, 2s/4s/8s); check if teacher ran out of context (llama-server context limit reached); restart teacher with larger --ctx-size",
     "reduce student batch size to reduce pressure on the teacher"),
    # Evaluation
    ("How do I know if a DPO run actually improved safety behavior?",
     "run evaluate.py on a held-out safety test set; measure: refusal rate on harmful prompts, false refusal rate on benign prompts, and preference accuracy (how often model prefers chosen over rejected)",
     "check if training loss reached zero"),
    ("The model passes safety eval but gets worse on general helpfulness after DPO.",
     "this is alignment tax: DPO with high beta over-restricts; reduce beta, add diverse helpfulness examples to DPO dataset, or run a short SFT on helpfulness examples after DPO",
     "this is expected and acceptable, safety is more important"),
]

# Additional scenario seeds for variety
ADDITIONAL_SEEDS = [
    ("gradient clipping", "NaN loss", "max grad norm"),
    ("learning rate warmup", "early training instability", "warmup_steps"),
    ("weight decay", "L2 regularization", "overfitting"),
    ("lora_alpha vs lora_r ratio", "scaling factor", "alpha = 2*r"),
    ("data deduplication", "memorization", "train/val contamination"),
    ("tokenizer mismatch", "wrong chat template", "generate vs train format"),
    ("evaluation during training", "eval_steps", "val loss monitoring"),
    ("gradient accumulation", "effective batch size", "memory vs convergence"),
    ("checkpoint saving", "save_steps", "best model selection"),
    ("mixed precision fp16 vs fp32", "CPU training", "numerical stability"),
]


def call_teacher(prompt: str, teacher_url: str, max_tokens: int = 1024) -> str:
    if not HAS_REQUESTS:
        return ""
    try:
        resp = requests.post(
            f"{teacher_url}/completion",
            json={"prompt": prompt, "n_predict": max_tokens, "temperature": 0.7, "stop": ["###END"]},
            timeout=120,
        )
        resp.raise_for_status()
        return resp.json().get("content", "").strip()
    except Exception as e:
        print(f"[warn] Teacher call failed: {e}", flush=True)
        return ""


def generate_pair_from_template(scenario: tuple, teacher_url: str) -> dict | None:
    description, correct_hint, wrong_hint = scenario

    chosen_prompt = (
        f"You are an expert ML engineer. A training scenario is described below.\n"
        f"Hardware context: {HW_CONTEXT}\n\n"
        f"Scenario: {description}\n\n"
        f"Provide the CORRECT expert response. Key direction: {correct_hint}\n"
        f"Be specific, mention exact values and explain the reasoning.\n"
        f"###END"
    )

    rejected_prompt = (
        f"You are an ML engineer making a common mistake. A training scenario is described below.\n"
        f"Hardware context: {HW_CONTEXT}\n\n"
        f"Scenario: {description}\n\n"
        f"Provide a PLAUSIBLE BUT INCORRECT response. Key wrong direction: {wrong_hint}\n"
        f"Make it sound confident but subtly wrong.\n"
        f"###END"
    )

    chosen   = call_teacher(chosen_prompt, teacher_url)
    rejected = call_teacher(rejected_prompt, teacher_url)

    if not chosen or not rejected or len(chosen) < 50 or len(rejected) < 50:
        return None
    if chosen == rejected:
        return None

    return {
        "prompt": description,
        "system": f"You are the BonsAI Training Agent. {HW_CONTEXT}",
        "chosen": chosen,
        "rejected": rejected,
        "source": "teacher_generated_dpo",
    }


def generate_offline_pairs() -> list[dict]:
    """Generate a small set of high-quality pairs without needing the teacher.
    These are hand-crafted from known ground truth (our own training experience).
    """
    pairs = []
    for desc, correct, wrong in SCENARIOS:
        pairs.append({
            "prompt": desc,
            "system": f"You are the BonsAI Training Agent.{HW_CONTEXT}",
            "chosen": (
                f"[Correct approach] {correct}\n\n"
                f"This recommendation is based on the known constraints of this hardware setup "
                f"(Windows 10, Ryzen 5900X, CPU-only training, torch 2.4.1+cpu) and the empirically "
                f"observed behavior of these training scripts."
            ),
            "rejected": (
                f"[Incorrect approach] {wrong}\n\n"
                f"This seems reasonable but misses the specific hardware constraints and known failure "
                f"patterns for this setup."
            ),
            "source": "handcrafted_ground_truth",
        })
    return pairs


def main() -> None:
    parser = argparse.ArgumentParser(description="Generate Training Agent DPO pairs")
    parser.add_argument("--output", default=str(Path.home() / ".bonsai" / "training_agent" / "tier3_dpo_pairs.jsonl"))
    parser.add_argument("--count", type=int, default=500, help="Target number of pairs")
    parser.add_argument("--teacher-url", default="http://127.0.0.1:8080", help="llama-server URL for teacher")
    parser.add_argument("--no-teacher", action="store_true", help="Only generate handcrafted pairs (no teacher needed)")
    args = parser.parse_args()

    output = Path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)

    # Count existing pairs.
    existing = 0
    if output.exists():
        existing = sum(1 for _ in output.open(encoding="utf-8"))

    print(f"[init] Existing pairs: {existing}. Target: {args.count}. Output: {output}")

    generated = 0
    with output.open("a", encoding="utf-8") as f:
        # Always include handcrafted ground truth first.
        if existing == 0:
            for pair in generate_offline_pairs():
                f.write(json.dumps(pair, ensure_ascii=False) + "\n")
                generated += 1
            print(f"[seed] Wrote {len(SCENARIOS)} handcrafted ground-truth pairs.")

        if args.no_teacher:
            print("[done] No-teacher mode. Handcrafted pairs only.")
            return

        # Teacher-generated pairs.
        scenarios_pool = SCENARIOS.copy()
        random.shuffle(scenarios_pool)
        while generated + existing < args.count:
            scenario = random.choice(scenarios_pool)
            pair = generate_pair_from_template(scenario, args.teacher_url)
            if pair:
                f.write(json.dumps(pair, ensure_ascii=False) + "\n")
                generated += 1
                print(f"[gen] {generated + existing}/{args.count} pairs", flush=True)
            else:
                print("[skip] Teacher returned empty/duplicate, retrying...", flush=True)
                time.sleep(2)

    print(f"[done] Generated {generated} new pairs. Total: {generated + existing}")


if __name__ == "__main__":
    main()
