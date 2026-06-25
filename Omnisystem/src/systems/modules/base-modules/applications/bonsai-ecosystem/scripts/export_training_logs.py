#!/usr/bin/env python3
"""Convert completed BonsAI training run logs into Training Agent training examples.

Reads:
  - ~/.bonsai/brain_metadata.json     (completed phases)
  - ~/.bonsai/training_export/*.log   (raw training logs, if present)
  - Structured output from dpo_train.py / finetune_sft.py / distill.py

Appends to:
  ~/.bonsai/training_agent/bonsai_logs.jsonl

Run after every training session to keep the Training Agent's dataset current.
"""
import json
import re
import sys
from datetime import datetime, timezone
from pathlib import Path

# ── Offline enforcement ───────────────────────────────────────────────────────
import os
os.environ.setdefault("TRANSFORMERS_OFFLINE", "1")
os.environ.setdefault("HF_HUB_OFFLINE", "1")
os.environ.setdefault("HF_DATASETS_OFFLINE", "1")
os.environ.setdefault("HF_HUB_DISABLE_TELEMETRY", "1")

BONSAI_DIR  = Path.home() / ".bonsai"
OUTPUT_FILE = BONSAI_DIR / "training_agent" / "bonsai_logs.jsonl"
META_FILE   = BONSAI_DIR / "brain_metadata.json"

# Known Windows/CPU constraints — always include in context so the agent learns them.
BONSAI_CONSTRAINTS = {
    "os": "Windows 10 Pro",
    "cpu": "AMD Ryzen 5900X (12-core, 24-thread)",
    "ram_gb": 64,
    "gpu": "AMD RX 7900 XTX 24GB VRAM",
    "gpu_training": "NOT USABLE (DirectML backward pass crashes on Windows)",
    "gpu_inference": "USABLE via llama-server GGUF sidecar (ROCm not available on Windows)",
    "python": "C:/Users/limpi/AppData/Local/Programs/Python/Python312/python.exe",
    "peft_version": "0.19.1",
    "torch_version": "2.4.1+cpu",
    "critical_rule_max_length": "max_length MUST be ≤ 256 on CPU with two model copies; 128 is safest for 0.5B-1.5B models",
    "critical_rule_offline": "TRANSFORMERS_OFFLINE=1 MUST be set before any HF import",
    "critical_rule_bash": "Do NOT run training scripts from Git Bash on Windows — causes segfault (exit 139). Use PowerShell System.Diagnostics.Process instead.",
}

# Canonical phase descriptions for annotation.
PHASE_META = {
    "safety":         {"script": "dpo_train.py",    "method": "DPO",            "data": "safety_dpo.jsonl"},
    "survival":       {"script": "distill.py",       "method": "Distillation",   "data": "survival_distill_prompts.txt"},
    "tool_use":       {"script": "dpo_train.py",    "method": "DPO",            "data": "tool_use_dpo.jsonl"},
    "code":           {"script": "distill.py",       "method": "Distillation",   "data": "code_distill_prompts.txt"},
    "chat":           {"script": "finetune_sft.py",  "method": "SFT",            "data": "chat_sft.jsonl"},
    "reason":         {"script": "distill.py",       "method": "Distillation",   "data": "reason_distill_prompts.txt"},
    "final":          {"script": "finetune_sft.py",  "method": "SFT (merge)",    "data": "combined_final.jsonl"},
    "convert":        {"script": "convert_hf_to_gguf.py", "method": "Conversion", "data": "N/A"},
    "training_agent": {"script": "finetune_sft.py + dpo_train.py", "method": "SFT+DPO", "data": "combined_sft.jsonl + tier3_dpo_pairs.jsonl"},
}


def load_meta() -> dict:
    if META_FILE.exists():
        try:
            return json.loads(META_FILE.read_text(encoding="utf-8"))
        except Exception:
            pass
    return {"lessons_completed": 0, "phases_done": [], "last_training": None}


def parse_log_snippet(log_text: str) -> dict:
    """Extract structured info from training log output."""
    result = {
        "epochs_completed": 0,
        "final_loss": None,
        "steps_total": None,
        "elapsed_seconds": None,
        "converged": False,
        "errors": [],
    }

    # Parse epoch lines: [epoch] epoch=3 avg_loss=0.0022
    epoch_matches = re.findall(r"\[epoch\] epoch=(\d+) avg_loss=([\d.]+)", log_text)
    if epoch_matches:
        result["epochs_completed"] = max(int(e[0]) for e in epoch_matches)
        result["final_loss"] = float(epoch_matches[-1][1])
        result["converged"] = result["final_loss"] < 0.05

    # Parse train complete line: [train] status=complete steps=150 elapsed=418s
    m = re.search(r"\[train\] status=complete steps=(\d+) elapsed=(\d+)s", log_text)
    if m:
        result["steps_total"]      = int(m.group(1))
        result["elapsed_seconds"]  = int(m.group(2))

    # Check for errors
    if "Traceback" in log_text or "Error:" in log_text or "NaN" in log_text:
        lines = log_text.split("\n")
        result["errors"] = [l for l in lines if any(k in l for k in ("Error", "NaN", "Traceback", "Segmentation"))]

    return result


def build_example(phase: str, config: dict, log_text: str, outcome: str, diagnosis: str) -> dict:
    """Build a Training Agent training example in instruction-response format."""
    meta = PHASE_META.get(phase, {"script": "unknown", "method": "unknown"})
    context = {
        "phase": phase,
        "method": meta["method"],
        "script": meta["script"],
        "config": config,
        "hardware": BONSAI_CONSTRAINTS,
        "timestamp": datetime.now(timezone.utc).isoformat(),
    }

    instruction = (
        f"Training phase '{phase}' ({meta['method']}) completed on {BONSAI_CONSTRAINTS['cpu']}. "
        f"Script: {meta['script']}. "
        f"Config: {json.dumps(config)}. "
        f"Logs:\n{log_text}\n\n"
        f"Diagnose what happened and describe the key lessons for future training runs on this hardware."
    )

    return {
        "instruction": instruction,
        "response": f"Outcome: {outcome}\n\n{diagnosis}",
        "context": context,
        "source": "bonsai_training_log",
    }


# ── Hardcoded records for completed runs (retroactively logged) ────────────────

COMPLETED_RUNS = [
    {
        "phase": "safety",
        "config": {"beta": 0.15, "epochs": 3, "lr": 5e-5, "max_length": 128,
                   "model": "Qwen2.5-0.5B-Instruct", "pairs": 50, "lora_rank": 16},
        "log": (
            "[device] using=cpu_forced\n"
            "[dpo] beta=0.15 epochs=3 lr=5e-05\n"
            "[load] status=done\n"
            "trainable params: 2,162,688 || all params: 496,195,456 || trainable%: 0.4359\n"
            "[data] pairs=50\n"
            "[epoch] epoch=1 avg_loss=0.0450\n"
            "[epoch] epoch=2 avg_loss=0.0001\n"
            "[epoch] epoch=3 avg_loss=0.0001\n"
            "[train] status=complete steps=150 elapsed=418s\n"
            "[save] path=C:\\Users\\limpi\\.bonsai\\adapters\\bonsai-safety-v1\n"
        ),
        "outcome": "SUCCESS. Loss converged cleanly from 0.0450 to 0.0001 over 3 epochs.",
        "diagnosis": (
            "The safety DPO phase succeeded after fixing two critical issues:\n"
            "1. SEGFAULT FIX: The original default max_length=512 caused Windows ACCESS_VIOLATION "
            "(exit code 0xC0000005) because two model copies (policy + reference) at 512 tokens each "
            "exceeded the Windows CPU allocator's contiguous memory limit. Reducing to max_length=128 "
            "resolved this immediately. RULE: For CPU training on Windows with two model copies, always "
            "use max_length ≤ 256. max_length=128 is the safe default for 0.5B models.\n"
            "2. BASH SEGFAULT FIX: Running dpo_train.py from Git Bash (exit 139) is unreliable on "
            "Windows. Always use PowerShell System.Diagnostics.Process to launch the Python training "
            "process. This eliminates signal handling issues.\n"
            "3. GATHER FIX: PyTorch .gather() cannot handle index=-100 (HuggingFace ignore_index). "
            "Must clamp labels before gather: raw_target.clamp(min=0).unsqueeze(-1), then apply mask "
            "(raw_target != -100).float() after. This fix is already in dpo_train.py.\n"
            "Expected performance on this hardware: ~420s for 50 pairs × 3 epochs at max_length=128."
        ),
    },
]


def main() -> None:
    OUTPUT_FILE.parent.mkdir(parents=True, exist_ok=True)

    # Load already-exported run IDs to avoid duplicates.
    existing_instructions = set()
    if OUTPUT_FILE.exists():
        for line in OUTPUT_FILE.read_text(encoding="utf-8").splitlines():
            try:
                obj = json.loads(line)
                existing_instructions.add(obj.get("instruction", "")[:80])
            except Exception:
                pass

    appended = 0
    with OUTPUT_FILE.open("a", encoding="utf-8") as f:
        for run in COMPLETED_RUNS:
            example = build_example(
                run["phase"], run["config"], run["log"], run["outcome"], run["diagnosis"])
            key = example["instruction"][:80]
            if key in existing_instructions:
                print(f"[skip] Already exported: phase={run['phase']}")
                continue
            f.write(json.dumps(example, ensure_ascii=False) + "\n")
            existing_instructions.add(key)
            appended += 1
            print(f"[export] Phase '{run['phase']}' -> {OUTPUT_FILE}")

    print(f"[done] Appended {appended} new training examples. "
          f"Total file: {OUTPUT_FILE}")
    print(f"[tip] Add new runs to COMPLETED_RUNS list in this script after each training session.")


if __name__ == "__main__":
    main()
