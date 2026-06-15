"""
PoC: Test what llama-cpp-python actually exposes for hidden states.

Tests both the completion API (embedding=True on Llama constructor) and the
dedicated embed() / create_embedding() calls, reporting exactly what is and
isn't available — and what that means for a hybrid GPU-forward/CPU-backward
training approach.

Usage:
    python test_vulkan_capture.py <path-to-gguf>
"""
import sys, json
from pathlib import Path

if len(sys.argv) < 2:
    print("Usage: python test_vulkan_capture.py <gguf-path>")
    sys.exit(1)

gguf_path = sys.argv[1]
if not Path(gguf_path).exists():
    print(f"ERROR: model not found: {gguf_path}")
    sys.exit(1)

from llama_cpp import Llama
import numpy as np

print(f"[poc] loading model: {gguf_path}")
print(f"[poc] n_gpu_layers=20  n_ctx=128  embedding=True")

# ── Test 1: embedding=True on completion output ───────────────────────────────
model = Llama(
    model_path=gguf_path,
    n_gpu_layers=20,
    n_ctx=128,
    embedding=True,
    verbose=False,
)

print("\n── Test 1: completion output keys ──────────────────────────────────────")
output = model("Hello world", max_tokens=1)
print(f"  output keys:         {list(output.keys())}")
has_embedding_in_output = output.get("embedding") is not None
print(f"  'embedding' in output: {has_embedding_in_output}")

# ── Test 2: create_embedding() ────────────────────────────────────────────────
print("\n── Test 2: create_embedding() ──────────────────────────────────────────")
try:
    emb_result = model.create_embedding("Hello world")
    embedding_vec = emb_result["data"][0]["embedding"]
    arr = np.array(embedding_vec, dtype=np.float32)
    print(f"  embedding shape:     {arr.shape}")
    print(f"  embedding dtype:     {arr.dtype}")
    print(f"  embedding norm:      {np.linalg.norm(arr):.4f}")
    print(f"  embedding sample:    {arr[:4]}")
    has_create_embedding = True
except Exception as e:
    print(f"  create_embedding() failed: {e}")
    has_create_embedding = False

# ── Test 3: embed() ───────────────────────────────────────────────────────────
print("\n── Test 3: embed() ─────────────────────────────────────────────────────")
try:
    tokens = model.tokenize(b"Hello world")
    raw = model.embed("Hello world")
    arr2 = np.array(raw, dtype=np.float32)
    print(f"  embed() shape:       {arr2.shape}")
    print(f"  tokens:              {len(tokens)} tokens")
    has_embed = True
except Exception as e:
    print(f"  embed() failed: {e}")
    has_embed = False

# ── Test 4: per-token / per-layer access ─────────────────────────────────────
print("\n── Test 4: per-layer hidden states ─────────────────────────────────────")
# llama-cpp-python doesn't expose intermediate layer activations.
# Check what introspection is available.
api_attrs = [a for a in dir(model) if not a.startswith("_")]
gradient_attrs = [a for a in api_attrs if any(k in a.lower()
    for k in ("grad", "layer", "hidden", "activat", "logit", "state", "kv"))]
print(f"  gradient-adjacent attrs: {gradient_attrs or 'none'}")

logits_available = hasattr(model, "eval_logits") or hasattr(model, "scores")
print(f"  logits attr:         {logits_available}")

# ── Summary ───────────────────────────────────────────────────────────────────
print("\n══ SUMMARY ══════════════════════════════════════════════════════════════")
print(f"  Embeddings available (create_embedding): {has_create_embedding}")
print(f"  Embeddings available (embed):            {has_embed}")
print(f"  Per-layer hidden states:                 False")
print(f"  Autograd-compatible tensors:             False")
print()
print("  WHAT THIS MEANS FOR THE HYBRID TRAINER:")
print()
if has_create_embedding or has_embed:
    print("  YES — llama-cpp-python exposes the FINAL hidden state (last layer")
    print("  output before LM head).  This is a fixed-size float32 vector per")
    print("  input sequence.")
    print()
    print("  NO  — this is NOT sufficient for the Phase 1 hybrid scheduler as")
    print("  described.  Reasons:")
    print("    1. The embedding is the RESULT of the forward pass, not a")
    print("       computation graph.  There are no gradients attached.")
    print("    2. LoRA backward requires gradients propagated through ALL")
    print("       28 attention layers, not just the final output vector.")
    print("    3. llama.cpp uses GGML tensors (no autograd) — calling")
    print("       lora_backward_cpu() on a numpy array is not possible.")
    print("    4. The LoRA delta weights (A, B matrices) are separate from")
    print("       the base model — the optimizer needs dL/dA and dL/dB,")
    print("       which require a full differentiable forward pass.")
    print()
    print("  VERDICT: PoC result = embeddings YES, Phase 1 hybrid = NOT viable.")
    print("  Recommend: Fallback plan (Training Monitor UI + checkpoint/resume).")
else:
    print("  No embeddings available. Fallback plan required.")
