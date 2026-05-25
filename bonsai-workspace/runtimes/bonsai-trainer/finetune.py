#!/usr/bin/env python3
"""
BonsAI-Core LoRA fine-tune — 100% offline, zero network calls.

How local-only training works
------------------------------
GGUF is a quantized inference format; gradient descent requires float32
weights.  There is no llama.cpp-native LoRA training binary in this build
(the upstream finetune example was removed after llama.cpp v0.0.1000).

Instead we use a two-step resolve:
  1. Parse the GGUF header to identify the base architecture.
  2. Locate the matching base model in the local HF cache (models that were
     ALREADY downloaded by the user).  If none is found, print a clear error
     with manual-download instructions and exit 1.

Network access is blocked by setting TRANSFORMERS_OFFLINE / HF_HUB_OFFLINE /
HF_DATASETS_OFFLINE before any Hugging Face imports.  Any accidental network
call will raise an error rather than silently downloading.

Device priority: CUDA -> DirectML fp32 (AMD/Intel Windows) -> CPU.
DirectML note: fp32 matmul/linear ops work; the full transformer backward
graph crashes torch_directml due to unsupported ops — fall back to CPU.

Usage (offline, GGUF model on disk):
    py finetune.py \
      --gguf "D:/Models/general/Bonsai-1.7B-Q2_K/Bonsai-1.7B-Q2_K.gguf" \
      --data data/bonsai_core/bonsai_core_train_v2.jsonl \
      --output ~/.bonsai/adapters/bonsai-core-v3

Usage (explicit local HF model dir, already on disk):
    py finetune.py \
      --base-model C:/path/to/Qwen2.5-1.5B-Instruct \
      --data data/bonsai_core/bonsai_core_train_v2.jsonl \
      --output ~/.bonsai/adapters/bonsai-core-v3
"""

# ── OFFLINE LOCK — must be the very first executable lines ───────────────────
# These three env vars tell every Hugging Face library component to refuse any
# network access.  Setting them here (before any HF imports) ensures they take
# effect even if subprocesses or lazy importers fire later.
import os
os.environ["TRANSFORMERS_OFFLINE"]  = "1"
os.environ["HF_HUB_OFFLINE"]        = "1"
os.environ["HF_DATASETS_OFFLINE"]   = "1"
os.environ["HF_HUB_DISABLE_TELEMETRY"] = "1"
# ── END OFFLINE LOCK ──────────────────────────────────────────────────────────

import argparse, json, shutil, struct, time
from pathlib import Path

import torch

# ── Progress helpers ──────────────────────────────────────────────────────────

_TRAIN_START: float = 0.0

def emit(tag: str, **kw):
    """Structured progress line for UI parsing: [tag] key=val ..."""
    print(f"[{tag}] " + " ".join(f"{k}={v}" for k, v in kw.items()), flush=True)


# ── GGUF header parser (pure stdlib, no network) ──────────────────────────────

GGUF_MAGIC    = b"GGUF"
_ARCH_KEYS    = {"general.architecture", "general.name"}
# Fixed-byte sizes for each GGUF value type
_VTYPE_SIZES  = {0:1, 1:1, 2:2, 3:2, 4:4, 5:4, 6:4, 7:8, 9:8, 10:1, 11:8}

# Architecture string (from GGUF header) → candidate HF cache folder names.
# Only entries that are ALREADY on this machine need to be here.
_ARCH_HF_SLUG: dict[str, list[str]] = {
    "qwen2":   ["models--Qwen--Qwen2.5-1.5B-Instruct",
                "models--Qwen--Qwen2.5-0.5B-Instruct"],
    "qwen3":   ["models--Qwen--Qwen2.5-1.5B-Instruct",
                "models--Qwen--Qwen2.5-0.5B-Instruct"],
    "llama":   ["models--meta-llama--Llama-3.2-1B-Instruct"],
    "mistral": ["models--mistralai--Mistral-7B-Instruct-v0.3"],
    "gemma":   ["models--google--gemma-2-2b-it"],
    "gemma2":  ["models--google--gemma-2-2b-it"],
}


def _read_str(f) -> str:
    n = struct.unpack("<Q", f.read(8))[0]
    return f.read(n).decode("utf-8", errors="replace")


def parse_gguf_arch(gguf_path: str) -> str:
    """Extract the base architecture name from a GGUF file header."""
    try:
        with open(gguf_path, "rb") as f:
            if f.read(4) != GGUF_MAGIC:
                return "unknown"
            f.read(4)                              # version
            f.read(8)                              # n_tensors
            n_kv = struct.unpack("<Q", f.read(8))[0]
            for _ in range(min(n_kv, 512)):
                key   = _read_str(f)
                vtype = struct.unpack("<I", f.read(4))[0]
                if vtype == 8:                     # GGUF_TYPE_STRING
                    val = _read_str(f)
                    if key in _ARCH_KEYS:
                        return val.lower().split("-")[0].split("_")[0]
                elif vtype in _VTYPE_SIZES:
                    f.read(_VTYPE_SIZES[vtype])
                elif vtype == 12:                  # array — stop; too complex
                    break
                else:
                    break
    except Exception as exc:
        emit("gguf", warning=f"header_parse_error:{exc}")
    return "unknown"


def _find_snapshot(slug_dir: Path) -> str | None:
    """Return the newest complete snapshot path under a cache slug dir."""
    for config in sorted(slug_dir.glob("snapshots/*/config.json"),
                         key=lambda p: p.stat().st_mtime, reverse=True):
        return str(config.parent)
    return None


def resolve_base_model(gguf_path: str) -> str:
    """
    Given a local GGUF path, find the matching base model already on disk.

    Search order:
      1. Canonical HF cache  (~/.cache/huggingface/hub/)
      2. $HF_HOME/hub/
      3. Common alternate locations (~/models/, ~/huggingface/)

    Raises SystemExit(1) with download instructions if nothing is found.
    No network access is performed.
    """
    arch = parse_gguf_arch(gguf_path)
    emit("gguf", path=gguf_path, detected_arch=arch)

    hf_home = Path(os.environ.get("HF_HOME",
                   Path.home() / ".cache" / "huggingface"))
    hub_dir = hf_home / "hub"

    slugs = _ARCH_HF_SLUG.get(arch, []) or list(_ARCH_HF_SLUG.values())[0]

    for slug in slugs:
        snap = _find_snapshot(hub_dir / slug)
        if snap:
            emit("gguf", resolved=slug.replace("models--", "").replace("--", "/"),
                 cache=snap)
            return snap

    # Last resort: check if any Qwen2.5 snapshot exists at all
    for candidate in hub_dir.glob("models--Qwen--*"):
        snap = _find_snapshot(candidate)
        if snap:
            emit("gguf", resolved_fallback=candidate.name, cache=snap)
            return snap

    # Nothing found — give the user exact instructions
    candidates_str = "\n  ".join(
        f"huggingface-cli download {s.replace('models--','').replace('--','/',1)}"
        for slugs_for_arch in _ARCH_HF_SLUG.values()
        for s in slugs_for_arch[:1]
    )
    raise SystemExit(
        f"\n[gguf] ERROR: No local base model found for GGUF arch='{arch}'.\n"
        f"\n"
        f"  GGUF files are quantized for inference only.  LoRA training\n"
        f"  requires float32 weights from the original base model.\n"
        f"\n"
        f"  To fix: run ONE of the following (requires internet, one-time):\n"
        f"  {candidates_str}\n"
        f"\n"
        f"  Or pass --base-model with a local HF model directory:\n"
        f"    finetune.py --base-model /path/to/model-dir ...\n"
    )


# ── Device selection ──────────────────────────────────────────────────────────

def get_device(backend: str, precision: str):
    dtype = torch.float16 if precision == "fp16" else torch.float32

    if backend == "cuda" or (backend == "auto" and torch.cuda.is_available()):
        emit("device", using="cuda", dtype=str(dtype))
        return torch.device("cuda"), dtype

    if backend in ("directml", "auto"):
        try:
            import torch_directml
            dev = torch_directml.device()
            # Quick smoke-test — crashes if Vulkan init fails
            _ = (torch.ones(4, 4, dtype=torch.float32).to(dev) @
                 torch.ones(4, 4, dtype=torch.float32).to(dev))
            emit("device", using="privateuseone:0_directml", dtype="float32")
            # Force CPU anyway: torch_directml crashes on transformer backward
            # (unsupported aten ops in attention/layernorm backward path).
            # Leaving this wired so it becomes usable when DML adds support.
            if backend == "directml":
                raise RuntimeError(
                    "DirectML is available but crashes on transformer backward. "
                    "Use --backend cpu (or wait for DML op coverage to improve)."
                )
        except RuntimeError as e:
            if backend == "directml":
                raise SystemExit(f"[device] {e}")
        except ImportError:
            if backend == "directml":
                raise SystemExit("[device] torch-directml not installed")

    if backend in ("mps", "auto") and torch.backends.mps.is_available():
        emit("device", using="mps", dtype="float32")
        return torch.device("mps"), torch.float32

    emit("device", using="cpu", dtype="float32")
    return torch.device("cpu"), torch.float32


# ── Live-progress trainer callback ───────────────────────────────────────────

try:
    from transformers import TrainerCallback as _TC

    class LiveProgressCallback(_TC):
        def on_log(self, args, state, control, logs=None, **kw):
            if not logs:
                return
            elapsed = time.time() - _TRAIN_START
            loss    = logs.get("loss", "?")
            step    = state.global_step
            total   = state.max_steps
            pct     = step / total * 100 if total else 0
            eta     = elapsed / step * (total - step) if step > 0 else 0
            emit("progress",
                 step=step, total=total,
                 epoch=f"{state.epoch or 0:.2f}",
                 loss=f"{loss:.4f}" if isinstance(loss, float) else loss,
                 pct=f"{pct:.1f}", elapsed=f"{elapsed:.0f}s",
                 eta=f"{eta:.0f}s")

except ImportError:
    LiveProgressCallback = None  # type: ignore


# ── Main ──────────────────────────────────────────────────────────────────────

def main():
    global _TRAIN_START

    parser = argparse.ArgumentParser(
        description="BonsAI LoRA fine-tune — offline only, no HF downloads")
    parser.add_argument("--gguf", type=str, default=None,
        help="Local GGUF model path.  Resolves to an already-cached base model "
             "by architecture.  No network calls.")
    parser.add_argument("--base-model", "--base_model", dest="base_model",
        default=None,
        help="Explicit local HF model directory (must already be on disk).")
    parser.add_argument("--data",       required=True,
        help="Primary training JSONL file")
    parser.add_argument("--output",     required=True,
        help="Output directory for the LoRA adapter")
    parser.add_argument("--epochs",     type=int,   default=3)
    parser.add_argument("--batch-size", "--batch_size",
        type=int, default=1, dest="batch_size")
    parser.add_argument("--grad-accum", "--grad_accum",
        type=int, default=8, dest="grad_accum")
    parser.add_argument("--lr",         type=float, default=2e-4)
    parser.add_argument("--backend",    default="auto",
        choices=["auto", "cuda", "directml", "mps", "cpu"])
    parser.add_argument("--precision",  default="fp32", choices=["fp32", "fp16"])
    parser.add_argument("--extra-data", "--extra_data",
        dest="extra_data", type=str, default=None,
        help="Extra JSONL of curated live examples to merge")
    parser.add_argument("--local-only", action="store_true", default=True,
        help="(Default: on) Enforce offline mode. Exits with error if the "
             "base model is not already on disk.")
    args = parser.parse_args()

    # This flag is informational — the offline env vars set at module load
    # already guarantee no network access regardless of this flag's value.
    if args.local_only:
        emit("offline", enforced=True,
             TRANSFORMERS_OFFLINE=os.environ.get("TRANSFORMERS_OFFLINE"),
             HF_HUB_OFFLINE=os.environ.get("HF_HUB_OFFLINE"))

    # ── Resolve model (local only) ────────────────────────────────────────────
    if args.gguf:
        if not Path(args.gguf).exists():
            raise SystemExit(f"[gguf] ERROR: GGUF file not found: {args.gguf}")
        base_model_path = resolve_base_model(args.gguf)
    elif args.base_model:
        p = Path(args.base_model)
        # Handle HF cache layout: user may pass the slug dir rather than snapshot
        if (p / "config.json").exists():
            base_model_path = str(p)
        else:
            snap = _find_snapshot(p)
            if snap:
                base_model_path = snap
            else:
                raise SystemExit(
                    f"[model] ERROR: No config.json found at {args.base_model}\n"
                    f"  Pass the snapshot directory directly, or use --gguf."
                )
        emit("model", resolved="explicit_local", path=base_model_path)
    else:
        raise SystemExit(
            "[model] ERROR: Supply --gguf or --base-model.\n"
            "  Example: --gguf D:/Models/Bonsai-1.7B-Q2_K/Bonsai-1.7B-Q2_K.gguf"
        )

    # Confirm config.json is present — final safety check before loading
    if not (Path(base_model_path) / "config.json").exists():
        raise SystemExit(
            f"[model] ERROR: {base_model_path} is missing config.json.\n"
            f"  The model directory may be incomplete."
        )

    # ── Device ───────────────────────────────────────────────────────────────
    dev, dtype = get_device(args.backend, args.precision)
    dev_str    = str(dev)

    # ── Load (local_files_only=True enforces offline at the HF layer too) ────
    from transformers import AutoModelForCausalLM, AutoTokenizer, TrainingArguments, Trainer
    from peft import LoraConfig, get_peft_model

    emit("load", status="loading_model", path=base_model_path)
    model = AutoModelForCausalLM.from_pretrained(
        base_model_path,
        torch_dtype=dtype,
        local_files_only=True,   # belt-and-suspenders: refuse download even if
                                  # TRANSFORMERS_OFFLINE were somehow unset
    )
    tokenizer = AutoTokenizer.from_pretrained(
        base_model_path,
        local_files_only=True,
    )
    tokenizer.pad_token = tokenizer.eos_token
    emit("load", status="model_loaded")

    lora_cfg = LoraConfig(
        r=16, lora_alpha=32,
        target_modules=["q_proj", "k_proj", "v_proj", "o_proj"],
        lora_dropout=0.05, bias="none", task_type="CAUSAL_LM",
    )
    model = get_peft_model(model, lora_cfg)
    model.print_trainable_parameters()

    # ── Data ──────────────────────────────────────────────────────────────────
    with open(args.data, encoding="utf-8") as f:
        examples = [json.loads(ln) for ln in f if ln.strip()]
    emit("data", synthetic=len(examples))

    curated_count = 0
    if args.extra_data and Path(args.extra_data).exists():
        with open(args.extra_data, encoding="utf-8") as f:
            for ln in f:
                ln = ln.strip()
                if not ln:
                    continue
                try:
                    examples.append(json.loads(ln))
                    curated_count += 1
                except Exception:
                    pass
        emit("data", curated=curated_count, total=len(examples))
    else:
        emit("data", curated=0)

    texts = [ex["text"] if "text" in ex else json.dumps(ex["messages"])
             for ex in examples]

    # ── Tokenise ──────────────────────────────────────────────────────────────
    MAX_LEN = 512
    from datasets import Dataset

    def tokenize(batch):
        enc = tokenizer(batch["text"], truncation=True,
                        max_length=MAX_LEN, padding="max_length")
        enc["labels"] = enc["input_ids"].copy()
        return enc

    raw     = Dataset.from_list([{"text": t} for t in texts])
    dataset = raw.map(tokenize, batched=True, remove_columns=["text"])
    dataset.set_format("torch")
    emit("data", tokenized=len(dataset))

    # ── Train ─────────────────────────────────────────────────────────────────
    training_args = TrainingArguments(
        output_dir=args.output,
        num_train_epochs=args.epochs,
        per_device_train_batch_size=args.batch_size,
        gradient_accumulation_steps=args.grad_accum,
        learning_rate=args.lr,
        fp16=False,
        bf16=False,
        logging_steps=5,
        save_strategy="epoch",
        report_to="none",
        remove_unused_columns=False,
        use_cpu=(dev_str == "cpu"),
        no_cuda=(dev_str == "cpu"),
        push_to_hub=False,                 # explicit: never push to HF Hub
        hub_token=None,
    )

    callbacks = [LiveProgressCallback()] if LiveProgressCallback else []
    trainer = Trainer(
        model=model,
        args=training_args,
        train_dataset=dataset,
        processing_class=tokenizer,
        callbacks=callbacks,
    )

    emit("train", status="starting", examples=len(dataset),
         epochs=args.epochs, backend=dev_str)
    _TRAIN_START = time.time()
    result       = trainer.train()
    elapsed      = time.time() - _TRAIN_START

    emit("train", status="complete",
         loss=f"{result.training_loss:.4f}",
         elapsed=f"{elapsed:.0f}s",
         steps=result.global_step)

    # ── Save ──────────────────────────────────────────────────────────────────
    out = Path(args.output)
    out.mkdir(parents=True, exist_ok=True)
    model.save_pretrained(str(out))
    tokenizer.save_pretrained(str(out))

    tmpl = Path(__file__).parent / "prompt_template.txt"
    if tmpl.exists():
        shutil.copy(tmpl, out / "prompt_template.txt")

    adapter_bytes = sum(f.stat().st_size for f in out.glob("*.safetensors"))
    emit("save", path=str(out),
         adapter_mb=f"{adapter_bytes / 1024 / 1024:.1f}",
         curated_merged=curated_count,
         synthetic_examples=len(dataset) - curated_count)


if __name__ == "__main__":
    main()
