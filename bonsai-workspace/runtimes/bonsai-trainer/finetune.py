#!/usr/bin/env python3
"""
BonsAI-Core LoRA fine-tune with Unsloth.

Usage:
    py finetune.py --data data/bonsai_core/bonsai_core_train.jsonl \
                   --output %USERPROFILE%/.bonsai/adapters/bonsai-core-v1
"""
import argparse, json, shutil
from pathlib import Path

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--data", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--base_model", default="unsloth/Phi-3-mini-4k-instruct")
    parser.add_argument("--epochs", type=int, default=3)
    parser.add_argument("--batch_size", type=int, default=4)
    parser.add_argument("--lr", type=float, default=2e-4)
    parser.add_argument("--max_seq_length", type=int, default=2048)
    args = parser.parse_args()

    try:
        from unsloth import FastLanguageModel
        from trl import SFTTrainer
        from transformers import TrainingArguments
        from datasets import Dataset
    except ImportError:
        print("Install: pip install unsloth trl transformers datasets")
        raise

    # Load data
    with open(args.data, encoding="utf-8") as f:
        records = [json.loads(l) for l in f if l.strip()]
    texts = []
    for r in records:
        msgs = r["messages"]
        text = "\n".join(f"<|{m['role']}|>\n{m['content']}" for m in msgs) + "\n<|end|>"
        texts.append({"text": text})
    dataset = Dataset.from_list(texts)

    # Load base model with 4-bit quantisation
    model, tokenizer = FastLanguageModel.from_pretrained(
        args.base_model,
        max_seq_length=args.max_seq_length,
        load_in_4bit=True,
    )
    model = FastLanguageModel.get_peft_model(
        model,
        r=16,
        target_modules=["q_proj", "v_proj"],
        lora_alpha=16,
        lora_dropout=0.05,
        bias="none",
        use_gradient_checkpointing=True,
    )

    trainer = SFTTrainer(
        model=model,
        tokenizer=tokenizer,
        train_dataset=dataset,
        dataset_text_field="text",
        max_seq_length=args.max_seq_length,
        args=TrainingArguments(
            output_dir=args.output,
            num_train_epochs=args.epochs,
            per_device_train_batch_size=args.batch_size,
            learning_rate=args.lr,
            fp16=True,
            logging_steps=10,
            save_strategy="epoch",
        ),
    )
    trainer.train()

    out = Path(args.output)
    out.mkdir(parents=True, exist_ok=True)
    model.save_pretrained(str(out))
    tokenizer.save_pretrained(str(out))

    # Copy prompt template alongside adapter
    tmpl = Path(__file__).parent / "prompt_template.txt"
    if tmpl.exists():
        shutil.copy(tmpl, out / "prompt_template.txt")
        print(f"Prompt template -> {out / 'prompt_template.txt'}")

    print(f"Adapter saved to {out}")

if __name__ == "__main__":
    main()
