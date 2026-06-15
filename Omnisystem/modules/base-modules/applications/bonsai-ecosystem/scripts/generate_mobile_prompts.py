#!/usr/bin/env python3
import json
import random
from pathlib import Path

output_dir = Path("training_data")
output_dir.mkdir(exist_ok=True)

prompts = []

# Chat prompts (5000)
chat_templates = [
    "Explain {0} in simple terms.",
    "How do I use {0} in Python?",
    "What is {0} and why is it important?",
    "Write a brief explanation of {0}.",
    "Break down {0} into steps.",
    "Give an example of {0}.",
]

topics = [
    "recursion", "async/await", "threads", "memory management",
    "the Bonsai Ecosystem", "machine learning", "neural networks",
    "transformers", "LLMs", "Rust", "Python", "TypeScript",
    "databases", "caching", "API design", "microservices"
]

for _ in range(5000):
    topic = random.choice(topics)
    template = random.choice(chat_templates)
    prompts.append({
        "role": "user",
        "content": template.format(topic)
    })

# Tool-use prompts (2000)
tool_templates = [
    "Read the file {0} and tell me its purpose.",
    "Run {0} and explain what it does.",
    "Search for '{0}' in the codebase and show results.",
    "List all functions in {0}.",
    "Check if {0} compiles without errors.",
]

files = ["Cargo.toml", "main.rs", "lib.rs", "build.rs", "Makefile"]
tools = ["cargo check", "cargo build", "cargo test", "cargo fmt", "cargo clippy"]

for _ in range(2000):
    if random.random() < 0.5:
        template = random.choice(tool_templates)
        file_or_tool = random.choice(files + tools)
        prompts.append({
            "role": "user",
            "content": template.format(file_or_tool)
        })
    else:
        prompts.append({
            "role": "user",
            "content": random.choice([
                "Debug this Rust error: [E0502] cannot borrow as mutable",
                "I'm getting an OOM error during training. How do I fix it?",
                "My GPU memory is full. What should I do?",
                "How do I profile my code for performance?",
                "What's the difference between Arc and Rc?",
            ])
        })

# Survival/fix prompts (1000)
survival_prompts = [
    "I encountered error E0502. How do I fix it?",
    "My training crashed with OOM. What should I do?",
    "The model is not converging. What can I try?",
    "How do I reduce memory usage?",
    "The inference is too slow. How do I optimize it?",
    "I'm getting NaN loss. How do I debug?",
    "The model generated gibberish. What went wrong?",
    "How do I check if the model is overfitting?",
]

for _ in range(1000):
    prompts.append({
        "role": "user",
        "content": random.choice(survival_prompts)
    })

# Write JSONL
output_path = output_dir / "mobile_distill_prompts.jsonl"
with open(output_path, "w") as f:
    for prompt in prompts:
        f.write(json.dumps({"messages": [prompt, {"role": "assistant", "content": ""}]}) + "\n")

print(f"Generated {len(prompts)} prompts → {output_path}")
