#!/usr/bin/env python3
"""Fuse a LoRA adapter into the base model and convert to GGUF.

Steps:
  1. Load base model (fp32, CPU).
  2. Load LoRA adapter with PEFT.
  3. merge_and_unload() → fused fp32 model.
  4. Save fused model to a temporary directory.
  5. Call llama.cpp convert_hf_to_gguf.py to produce a quantised GGUF.

The GGUF ends up at --output. If --output is ~/.bonsai/models/bonsai-latest.gguf
the hot-reload watcher (hot_reload.rs) will pick it up automatically within 2 s.

Usage:
    python scripts/convert_to_gguf.py \
        --base-model  C:/Users/limpi/.cache/huggingface/hub/models--Qwen--Qwen2.5-1.5B-Instruct/snapshots/<hash> \
        --adapter     C:/Users/limpi/.bonsai/adapters/bonsai-final-v1 \
        --output      C:/Users/limpi/.bonsai/models/bonsai-latest.gguf \
        --llama-cpp-dir C:/path/to/llama.cpp

Offline-safe: no HuggingFace calls.
"""
import argparse
import os
import pathlib
import shutil
import subprocess
import sys

# ── Offline enforcement ──────────────────────────────────────────────────────
os.environ.setdefault("TRANSFORMERS_OFFLINE", "1")
os.environ.setdefault("HF_HUB_OFFLINE", "1")
os.environ.setdefault("HF_DATASETS_OFFLINE", "1")
os.environ.setdefault("HF_HUB_DISABLE_TELEMETRY", "1")


def fuse_adapter(base_model: str, adapter: str, tmp_dir: pathlib.Path) -> None:
    print(f"[convert] Loading base model from {base_model}")
    from transformers import AutoModelForCausalLM, AutoTokenizer
    from peft import PeftModel
    import torch

    model = AutoModelForCausalLM.from_pretrained(
        base_model,
        device_map={"": "cpu"},
        torch_dtype=torch.float32,
        trust_remote_code=True,
    )
    tokenizer = AutoTokenizer.from_pretrained(base_model, trust_remote_code=True)

    print(f"[convert] Loading LoRA adapter from {adapter}")
    model = PeftModel.from_pretrained(model, adapter)

    print("[convert] Merging adapter into base weights…")
    model = model.merge_and_unload()

    print(f"[convert] Saving fused model to {tmp_dir}")
    model.save_pretrained(tmp_dir)
    tokenizer.save_pretrained(tmp_dir)


def convert_to_gguf(fused_dir: pathlib.Path, output: str, llama_cpp_dir: str, quant_type: str) -> None:
    convert_script = pathlib.Path(llama_cpp_dir) / "convert_hf_to_gguf.py"
    if not convert_script.exists():
        # Older llama.cpp used a different name
        convert_script = pathlib.Path(llama_cpp_dir) / "convert.py"
    if not convert_script.exists():
        print(f"[convert] ERROR: Could not find convert_hf_to_gguf.py in {llama_cpp_dir}")
        print("          Download llama.cpp: git clone https://github.com/ggerganov/llama.cpp")
        sys.exit(1)

    pathlib.Path(output).parent.mkdir(parents=True, exist_ok=True)

    cmd = [
        sys.executable, str(convert_script),
        str(fused_dir),
        "--outfile", output,
        "--outtype", quant_type,
    ]
    print(f"[convert] Running: {' '.join(cmd)}")
    subprocess.run(cmd, check=True)
    print(f"[convert] GGUF written to {output}")


def main():
    parser = argparse.ArgumentParser(description="Fuse LoRA + convert to GGUF")
    parser.add_argument("--base-model",     required=True, help="Path to base HF model directory")
    parser.add_argument("--adapter",        required=True, help="Path to LoRA adapter directory")
    parser.add_argument("--output",         required=True, help="Output .gguf file path")
    parser.add_argument("--llama-cpp-dir",  default=str(pathlib.Path.home() / "llama.cpp"),
                        help="Path to llama.cpp checkout (needs convert_hf_to_gguf.py)")
    parser.add_argument("--quant-type",     default="q4_k_m",
                        choices=["f16", "f32", "q4_k_m", "q5_k_m", "q8_0"],
                        help="GGUF quantisation type")
    parser.add_argument("--keep-fused",     action="store_true",
                        help="Keep the intermediate fused HF model directory")
    args = parser.parse_args()

    tmp_dir = pathlib.Path("tmp_fused_model")
    if tmp_dir.exists():
        print(f"[convert] Removing stale {tmp_dir}")
        shutil.rmtree(tmp_dir)
    tmp_dir.mkdir()

    try:
        fuse_adapter(args.base_model, args.adapter, tmp_dir)
        convert_to_gguf(tmp_dir, args.output, args.llama_cpp_dir, args.quant_type)
    finally:
        if not args.keep_fused and tmp_dir.exists():
            shutil.rmtree(tmp_dir)
            print(f"[convert] Cleaned up {tmp_dir}")

    print(f"\n[convert] Done — {args.output}")
    print("          The hot-reload watcher will pick it up within ~2 s if the app is running.")


if __name__ == "__main__":
    main()
