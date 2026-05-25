#!/usr/bin/env python3
"""
BonsAI-Core LoRA fine-tune.

Auto-detects: CUDA → DirectML (AMD/Intel on Windows) → MPS → CPU.

Usage:
    py finetune.py --data data/bonsai_core/bonsai_core_train.jsonl \
                   --output %USERPROFILE%/.bonsai/adapters/bonsai-core-v2
"""
import argparse, json, os, shutil
from pathlib import Path

import torch
from datasets import Dataset
from transformers import AutoModelForCausalLM, AutoTokenizer, TrainingArguments, Trainer
from peft import LoraConfig, get_peft_model


def get_device():
    if torch.cuda.is_available():
        return torch.device("cuda"), torch.float16
    try:
        import torch_directml
        return torch_directml.device(), torch.float16
    except ImportError:
        pass
    if torch.backends.mps.is_available():
        return torch.device("mps"), torch.float32
    return torch.device("cpu"), torch.float32


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--base_model", default="Qwen/Qwen2.5-1.5B-Instruct")
    parser.add_argument("--data", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--epochs", type=int, default=3)
    parser.add_argument("--batch_size", type=int, default=1)
    parser.add_argument("--grad_accum", type=int, default=8)
    parser.add_argument("--lr", type=float, default=2e-4)
    args = parser.parse_args()

    dev, dtype = get_device()
    print(f"[finetune] device={dev}, dtype={dtype}")

    model = AutoModelForCausalLM.from_pretrained(args.base_model, torch_dtype=dtype)
    if str(dev) != "cpu":
        model = model.to(dev)
    tokenizer = AutoTokenizer.from_pretrained(args.base_model)
    tokenizer.pad_token = tokenizer.eos_token

    lora_config = LoraConfig(
        r=16,
        lora_alpha=32,
        target_modules=["q_proj", "k_proj", "v_proj", "o_proj"],
        lora_dropout=0.05,
        bias="none",
        task_type="CAUSAL_LM",
    )
    model = get_peft_model(model, lora_config)
    model.print_trainable_parameters()

    with open(args.data, encoding="utf-8") as f:
        examples = [json.loads(line) for line in f if line.strip()]
    dataset = Dataset.from_list([{"text": json.dumps(ex["messages"])} for ex in examples])

    use_fp16 = (dtype == torch.float16) and str(dev) != "cpu"
    training_args = TrainingArguments(
        output_dir=args.output,
        num_train_epochs=args.epochs,
        per_device_train_batch_size=args.batch_size,
        gradient_accumulation_steps=args.grad_accum,
        learning_rate=args.lr,
        fp16=use_fp16,
        logging_steps=10,
        save_strategy="epoch",
        remove_unused_columns=False,
    )
    trainer = Trainer(
        model=model,
        args=training_args,
        train_dataset=dataset,
        tokenizer=tokenizer,
    )
    trainer.train()

    out = Path(args.output)
    out.mkdir(parents=True, exist_ok=True)
    model.save_pretrained(str(out))
    tokenizer.save_pretrained(str(out))

    tmpl = Path(__file__).parent / "prompt_template.txt"
    if tmpl.exists():
        shutil.copy(tmpl, out / "prompt_template.txt")

    print(f"[finetune] Adapter saved to {out}")


if __name__ == "__main__":
    main()
