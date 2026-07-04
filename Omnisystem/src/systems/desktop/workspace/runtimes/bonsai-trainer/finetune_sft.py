#!/usr/bin/env python3
"""Standard SFT (Supervised Fine-Tuning) with QLoRA for Bonsai.

Unlike dpo_train.py (which needs chosen/rejected pairs), this script trains on
single-response examples using cross-entropy (teacher forcing). It accepts the
same JSONL formats used elsewhere in the pipeline:

  - {"messages": [{"role": "user", "content": "..."}, {"role": "assistant", "content": "..."}]}
  - {"prompt": "...", "chosen": "..."}          # uses chosen as target
  - {"instruction": "...", "response": "..."}
  - {"text": "..."}                              # raw text, trained verbatim

Training is CPU-only by default (DirectML cannot do backward passes; GPU is used
for teacher inference only). Quantised loading with BitsAndBytes is skipped on
CPU — LoRA is applied in fp32 instead.

Usage:
    python finetune_sft.py \
        --base-model ~/.cache/huggingface/hub/models--Qwen--Qwen2.5-1.5B-Instruct/snapshots/<hash> \
        --data ~/.bonsai/training_export/bonsai_combined_final.jsonl \
        --output ~/.bonsai/adapters/bonsai-sft-$(date +%Y%m%d) \
        --device cpu --epochs 3
"""
import argparse
import json
import os

# ── Offline enforcement (must be before any HF import) ───────────────────────
os.environ.setdefault("TRANSFORMERS_OFFLINE", "1")
os.environ.setdefault("HF_HUB_OFFLINE", "1")
os.environ.setdefault("HF_DATASETS_OFFLINE", "1")
os.environ.setdefault("HF_HUB_DISABLE_TELEMETRY", "1")

import torch
from transformers import (
    AutoModelForCausalLM,
    AutoTokenizer,
    TrainingArguments,
    Trainer,
    DataCollatorForLanguageModeling,
)
from peft import LoraConfig, get_peft_model, prepare_model_for_kbit_training
from datasets import Dataset


# ── Helpers ───────────────────────────────────────────────────────────────────

def load_jsonl(path: str) -> list[dict]:
    data = []
    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line:
                data.append(json.loads(line))
    return data


def format_example(example: dict) -> str:
    """Convert any supported record format to a single training string."""
    if "messages" in example:
        parts = []
        for msg in example["messages"]:
            role = msg.get("role", "user")
            content = msg.get("content", "")
            if role == "system":
                parts.append(f"<|system|>\n{content}")
            elif role == "user":
                parts.append(f"<|user|>\n{content}")
            elif role == "assistant":
                parts.append(f"<|assistant|>\n{content}")
        return "\n".join(parts)
    if "chosen" in example:
        prompt = example.get("prompt", "")
        chosen = example["chosen"]
        if prompt:
            return f"<|user|>\n{prompt}\n<|assistant|>\n{chosen}"
        return chosen
    if "instruction" in example:
        instruction = example["instruction"]
        response = example.get("response", example.get("output", ""))
        return f"<|user|>\n{instruction}\n<|assistant|>\n{response}"
    if "text" in example:
        return example["text"]
    # last resort: stringify everything
    return json.dumps(example)


# ── Main ──────────────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(description="QLoRA SFT for Bonsai student model")
    parser.add_argument("--base-model",  required=True, help="Path to HF model directory")
    parser.add_argument("--data",        required=True, help="Path to training JSONL file")
    parser.add_argument("--output",      required=True, help="Output directory for LoRA adapter")
    parser.add_argument("--device",      default="cpu",   help="cpu | cuda | mps | directml")
    parser.add_argument("--epochs",      type=int,   default=3)
    parser.add_argument("--batch-size",  type=int,   default=2,    help="Per-device batch size (keep low on CPU)")
    parser.add_argument("--grad-accum",  type=int,   default=8,    help="Gradient accumulation steps")
    parser.add_argument("--lr",          type=float, default=2e-4)
    parser.add_argument("--max-length",  type=int,   default=1024)
    parser.add_argument("--lora-r",      type=int,   default=64)
    parser.add_argument("--lora-alpha",  type=int,   default=16)
    args = parser.parse_args()

    print(f"[sft] device={args.device}  epochs={args.epochs}  lr={args.lr}  lora_r={args.lora_r}")

    # ── Device handling ───────────────────────────────────────────────────────
    if args.device == "directml":
        print("[sft] DirectML cannot be used for training (backward pass crash). Falling back to CPU.")
        args.device = "cpu"

    use_4bit = args.device in ("cuda",)  # 4-bit only makes sense on CUDA

    # ── Model ─────────────────────────────────────────────────────────────────
    load_kwargs: dict = dict(
        trust_remote_code=True,
        torch_dtype=torch.float32,
    )
    if use_4bit:
        from transformers import BitsAndBytesConfig
        bnb_config = BitsAndBytesConfig(
            load_in_4bit=True,
            bnb_4bit_use_double_quant=True,
            bnb_4bit_quant_type="nf4",
            bnb_4bit_compute_dtype=torch.bfloat16,
        )
        load_kwargs["quantization_config"] = bnb_config
        load_kwargs["device_map"] = "auto"
    else:
        load_kwargs["device_map"] = {"": args.device}

    print(f"[sft] Loading model from {args.base_model}")
    model = AutoModelForCausalLM.from_pretrained(args.base_model, **load_kwargs)
    tokenizer = AutoTokenizer.from_pretrained(args.base_model, trust_remote_code=True)
    if tokenizer.pad_token is None:
        tokenizer.pad_token = tokenizer.eos_token

    if use_4bit:
        model = prepare_model_for_kbit_training(model)

    # ── LoRA ──────────────────────────────────────────────────────────────────
    lora_config = LoraConfig(
        r=args.lora_r,
        lora_alpha=args.lora_alpha,
        target_modules=["q_proj", "k_proj", "v_proj", "o_proj",
                        "gate_proj", "up_proj", "down_proj"],
        lora_dropout=0.05,
        bias="none",
        task_type="CAUSAL_LM",
    )
    model = get_peft_model(model, lora_config)
    model.print_trainable_parameters()

    # ── Dataset ───────────────────────────────────────────────────────────────
    print(f"[sft] Loading data from {args.data}")
    raw = load_jsonl(args.data)
    print(f"[sft] {len(raw)} examples loaded")

    formatted = [{"text": format_example(ex)} for ex in raw]
    dataset = Dataset.from_list(formatted)

    def tokenize(batch):
        out = tokenizer(
            batch["text"],
            truncation=True,
            max_length=args.max_length,
            padding=False,
        )
        out["labels"] = out["input_ids"].copy()
        return out

    dataset = dataset.map(tokenize, batched=True, remove_columns=["text"])

    # ── Training ──────────────────────────────────────────────────────────────
    training_args = TrainingArguments(
        output_dir=args.output,
        num_train_epochs=args.epochs,
        per_device_train_batch_size=args.batch_size,
        gradient_accumulation_steps=args.grad_accum,
        learning_rate=args.lr,
        fp16=False,
        bf16=False,
        save_steps=500,
        logging_steps=50,
        save_total_limit=2,
        remove_unused_columns=True,
        dataloader_pin_memory=False,
        report_to="none",
        warmup_ratio=0.05,
        lr_scheduler_type="cosine",
    )

    trainer = Trainer(
        model=model,
        args=training_args,
        train_dataset=dataset,
        data_collator=DataCollatorForLanguageModeling(tokenizer, mlm=False),
    )

    print("[sft] Starting training…")
    trainer.train()

    print(f"[sft] Saving adapter to {args.output}")
    model.save_pretrained(args.output)
    tokenizer.save_pretrained(args.output)
    print("[sft] Done.")


if __name__ == "__main__":
    main()
