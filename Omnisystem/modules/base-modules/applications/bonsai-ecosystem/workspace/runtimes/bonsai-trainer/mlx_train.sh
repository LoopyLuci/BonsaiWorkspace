#!/usr/bin/env bash
# BonsAI MLX-native training for Apple Silicon (M1/M2/M3/M4).
#
# Uses Apple's MLX framework via mlx-lm which runs natively on the unified
# memory architecture — no VRAM limit, much faster than PyTorch on Metal.
#
# Prerequisites (one-time, no HuggingFace token needed for Qwen2.5):
#   pip install mlx-lm
#   # Download the base model once (internet required, one-time only):
#   huggingface-cli download Qwen/Qwen2.5-1.5B-Instruct
#   # Or for a larger model:
#   huggingface-cli download Qwen/Qwen2.5-7B-Instruct
#
# Usage:
#   ./mlx_train.sh                              # SFT with defaults
#   ./mlx_train.sh --model Qwen/Qwen2.5-7B-Instruct --iters 1000
#   ./mlx_train.sh --dpo                        # DPO mode
#   ./mlx_train.sh --distill --teacher Qwen/Qwen2.5-14B-Instruct
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

# ── Defaults ──────────────────────────────────────────────────────────────────
MODEL="${BONSAI_TRAIN_MODEL:-Qwen/Qwen2.5-1.5B-Instruct}"
TEACHER_MODEL="${BONSAI_TEACHER_MODEL:-Qwen/Qwen2.5-7B-Instruct}"
EXPORT_DIR="${EXPORT_DIR:-$HOME/.bonsai/training_export}"
ADAPTER_DIR="${ADAPTER_DIR:-$HOME/.bonsai/adapters}"
ITERS="${ITERS:-600}"
BATCH_SIZE="${BATCH_SIZE:-4}"
LR="${LR:-1e-5}"
LORA_RANK="${LORA_RANK:-16}"
MODE="sft"   # sft | dpo | distill

# Parse args
while [[ $# -gt 0 ]]; do
    case "$1" in
        --model)     MODEL="$2";        shift 2 ;;
        --teacher)   TEACHER_MODEL="$2"; shift 2 ;;
        --iters)     ITERS="$2";        shift 2 ;;
        --batch)     BATCH_SIZE="$2";   shift 2 ;;
        --lr)        LR="$2";           shift 2 ;;
        --rank)      LORA_RANK="$2";    shift 2 ;;
        --dpo)       MODE="dpo";        shift ;;
        --distill)   MODE="distill";    shift ;;
        *) echo "Unknown arg: $1"; exit 1 ;;
    esac
done

# ── Check prerequisites ───────────────────────────────────────────────────────
if ! python3 -c "import mlx_lm" 2>/dev/null; then
    echo "ERROR: mlx-lm not installed."
    echo "  Run: pip install mlx-lm"
    exit 1
fi

if ! python3 -c "import mlx" 2>/dev/null; then
    echo "ERROR: mlx not installed or not on Apple Silicon."
    echo "  Run: pip install mlx"
    exit 1
fi

echo "==> BonsAI MLX Training (Apple Silicon)"
echo "    Mode:    $MODE"
echo "    Model:   $MODEL"
echo "    Iters:   $ITERS"
echo "    LR:      $LR"

# ── Export latest training data ───────────────────────────────────────────────
echo "==> Exporting training data..."
bash "$ROOT/scripts/export_training_data.sh" 2>/dev/null || \
    echo "    (export script skipped — using existing data)"

TRAIN_JSONL="$EXPORT_DIR/bonsai_merged_latest.jsonl"
DPO_JSONL="$EXPORT_DIR/bonsai_dpo_latest.jsonl"

if [ ! -f "$TRAIN_JSONL" ]; then
    # Fall back to curated baseline
    TRAIN_JSONL="$ROOT/bonsai-workspace/data/bonsai_core/bonsai_core_train_v2.jsonl"
    echo "    Using curated baseline: $TRAIN_JSONL"
fi

TIMESTAMP="$(date +%Y%m%d_%H%M%S)"
ADAPTER_OUT="$ADAPTER_DIR/bonsai-mlx-$TIMESTAMP"
mkdir -p "$ADAPTER_DIR"

# ── Convert training data to MLX format ───────────────────────────────────────
# mlx-lm expects {"text": "..."} or {"messages": [...]} format — our JSONL
# already uses the messages format, so it's compatible directly.
MLX_TRAIN_DIR="$EXPORT_DIR/mlx_train_$TIMESTAMP"
mkdir -p "$MLX_TRAIN_DIR"
cp "$TRAIN_JSONL" "$MLX_TRAIN_DIR/train.jsonl"

# Validation split
if [ -f "$ROOT/bonsai-workspace/data/bonsai_core/bonsai_core_val.jsonl" ]; then
    cp "$ROOT/bonsai-workspace/data/bonsai_core/bonsai_core_val.jsonl" \
       "$MLX_TRAIN_DIR/valid.jsonl"
else
    # Use last 5% of train as validation
    python3 - <<'PYEOF' "$TRAIN_JSONL" "$MLX_TRAIN_DIR/valid.jsonl"
import sys, random
lines = [l for l in open(sys.argv[1]) if l.strip()]
val_n = max(5, len(lines)//20)
random.shuffle(lines)
with open(sys.argv[2],'w') as f:
    for l in lines[:val_n]: f.write(l)
PYEOF
fi

# ── SFT training ──────────────────────────────────────────────────────────────
if [ "$MODE" = "sft" ]; then
    echo "==> Starting SFT with mlx-lm..."
    python3 -m mlx_lm.lora \
        --model "$MODEL" \
        --train \
        --data "$MLX_TRAIN_DIR" \
        --iters "$ITERS" \
        --batch-size "$BATCH_SIZE" \
        --learning-rate "$LR" \
        --lora-rank "$LORA_RANK" \
        --val-batches 5 \
        --steps-per-report 20 \
        --steps-per-eval 100 \
        --adapter-path "$ADAPTER_OUT"

# ── DPO training ─────────────────────────────────────────────────────────────
elif [ "$MODE" = "dpo" ]; then
    if [ ! -f "$DPO_JSONL" ]; then
        echo "ERROR: No DPO data found at $DPO_JSONL"
        echo "  Run: bash scripts/export_training_data.sh"
        exit 1
    fi
    echo "==> Starting DPO with mlx-lm..."
    # mlx-lm DPO requires {"prompt","chosen","rejected"} format
    python3 - <<'PYEOF' "$DPO_JSONL" "$MLX_TRAIN_DIR/dpo_train.jsonl"
import sys, json
with open(sys.argv[1]) as fi, open(sys.argv[2],'w') as fo:
    for ln in fi:
        ln=ln.strip()
        if not ln: continue
        obj=json.loads(ln)
        if 'chosen' in obj and 'rejected' in obj:
            fo.write(json.dumps({'prompt':obj.get('prompt',''),'chosen':obj['chosen'],'rejected':obj['rejected']})+'\n')
PYEOF
    python3 -m mlx_lm.lora \
        --model "$MODEL" \
        --train \
        --data "$MLX_TRAIN_DIR/dpo_train.jsonl" \
        --iters "$ITERS" \
        --batch-size "$BATCH_SIZE" \
        --learning-rate "$LR" \
        --lora-rank "$LORA_RANK" \
        --adapter-path "$ADAPTER_OUT" \
        --dpo

# ── Distillation via teacher ──────────────────────────────────────────────────
elif [ "$MODE" = "distill" ]; then
    echo "==> Distillation: generating teacher responses with $TEACHER_MODEL..."
    DISTILL_JSONL="$MLX_TRAIN_DIR/distill_train.jsonl"
    PROMPTS="$EXPORT_DIR/distill_prompts.txt"

    if [ ! -f "$PROMPTS" ]; then
        # Extract prompts from training data
        python3 -c "
import json, sys
with open('$TRAIN_JSONL') as f:
    for ln in f:
        try:
            msgs=json.loads(ln).get('messages',[])
            for m in msgs:
                if m.get('role')=='user': print(m['content']); break
        except: pass
" > "$PROMPTS"
    fi

    # Generate teacher completions with mlx-lm generate
    python3 - <<'PYEOF' "$PROMPTS" "$DISTILL_JSONL" "$TEACHER_MODEL"
import sys, json, subprocess
prompts_path, out_path, model = sys.argv[1], sys.argv[2], sys.argv[3]
prompts = [l.strip() for l in open(prompts_path) if l.strip()][:2000]
print(f"Generating teacher completions for {len(prompts)} prompts...")
with open(out_path, 'w') as fout:
    for i, p in enumerate(prompts):
        try:
            result = subprocess.run(
                ["python3","-m","mlx_lm.generate","--model",model,
                 "--prompt",p,"--max-tokens","256","--temp","0.3"],
                capture_output=True, text=True, timeout=60
            )
            resp = result.stdout.strip()
            if resp:
                msgs=[{"role":"user","content":p},{"role":"assistant","content":resp}]
                fout.write(json.dumps({"messages":msgs,"source":"distillation"})+"\n")
                if i%100==0: print(f"  {i}/{len(prompts)} done")
        except Exception as e:
            print(f"  skip ({e})")
print(f"Wrote {i+1} distillation examples")
PYEOF

    echo "==> Fine-tuning student on teacher completions..."
    python3 -m mlx_lm.lora \
        --model "$MODEL" \
        --train \
        --data "$DISTILL_JSONL" \
        --iters "$ITERS" \
        --batch-size "$BATCH_SIZE" \
        --learning-rate "$LR" \
        --lora-rank "$LORA_RANK" \
        --adapter-path "$ADAPTER_OUT"
fi

# ── Fuse and convert to GGUF ──────────────────────────────────────────────────
echo "==> Fusing LoRA into base model..."
FUSED_DIR="$ADAPTER_DIR/bonsai-fused-$TIMESTAMP"
python3 -m mlx_lm.fuse \
    --model "$MODEL" \
    --adapter-path "$ADAPTER_OUT" \
    --save-path "$FUSED_DIR" \
    --de-quantize 2>/dev/null || \
python3 -m mlx_lm.fuse \
    --model "$MODEL" \
    --adapter-path "$ADAPTER_OUT" \
    --save-path "$FUSED_DIR"

# Convert to GGUF if llama.cpp convert tool is available
CONVERT_PY="$(find /usr/local /opt /home -name 'convert_hf_to_gguf.py' 2>/dev/null | head -1)"
if [ -n "$CONVERT_PY" ]; then
    GGUF_OUT="$HOME/.bonsai/models/bonsai-latest.gguf"
    echo "==> Converting to GGUF: $GGUF_OUT"
    python3 "$CONVERT_PY" "$FUSED_DIR" --outfile "$GGUF_OUT" --outtype q4_k_m
    echo "    GGUF written to: $GGUF_OUT"
    echo "    Restart Bonsai Workspace to use the new model."
else
    echo "    GGUF conversion skipped (llama.cpp convert_hf_to_gguf.py not found)."
    echo "    Fused HF model at: $FUSED_DIR"
    echo "    To convert: python3 /path/to/llama.cpp/convert_hf_to_gguf.py $FUSED_DIR --outtype q4_k_m"
fi

echo ""
echo "Training complete."
echo "  Adapter:    $ADAPTER_OUT"
echo "  Fused model:$FUSED_DIR"
