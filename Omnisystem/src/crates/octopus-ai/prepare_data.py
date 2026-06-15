#!/usr/bin/env python3
"""
🐙 Octopus AI — Fast Synthetic Data Generation
Generates training data for server management.
"""

import json
import random
from pathlib import Path
import logging

logging.basicConfig(level=logging.INFO, format='%(asctime)s [%(levelname)s] %(message)s')
logger = logging.getLogger(__name__)

TEMPLATES = {
    "monitoring": [
        ("How do I check if a service is running?", "Use `systemctl status <service>` or `ps aux | grep <service>`"),
        ("What command shows memory usage?", "Use `free -h` for total memory or `top` for per-process usage"),
        ("How do I check disk space?", "Use `df -h` for filesystem usage or `du -sh /path` for directory size"),
    ],
    "containers": [
        ("How do I list running containers?", "Use `docker ps` for running or `docker ps -a` for all containers"),
        ("How do I check container logs?", "Use `docker logs <container_id>` or `docker logs -f` to follow"),
        ("How do I resource-limit a container?", "Use `-m <memory>` and `--cpus <count>` in docker run"),
    ],
    "security": [
        ("How do I check open ports?", "Use `ss -tuln` or `netstat -tuln` to list listening ports"),
        ("How do I set up a firewall?", "Use ufw on Ubuntu: `ufw enable` then `ufw allow <port>`"),
        ("How do I generate SSH keys?", "Use `ssh-keygen -t rsa -b 4096` to generate public/private key pair"),
    ],
}

def generate_training_data(output_dir: str, num_examples: int = 50000):
    """Generate synthetic training data fast."""
    output_path = Path(output_dir)
    output_path.mkdir(parents=True, exist_ok=True)

    logger.info(f"Generating {num_examples:,} synthetic training examples...")

    # Flatten all Q&A pairs
    all_pairs = []
    for category, pairs in TEMPLATES.items():
        all_pairs.extend(pairs)

    logger.info(f"Base Q&A pairs: {len(all_pairs)}")

    examples = []
    random.seed(42)

    # Simple approach: repeat and vary
    while len(examples) < num_examples:
        for q, a in all_pairs:
            if len(examples) >= num_examples:
                break

            # Add original
            examples.append({
                "instruction": q,
                "input": "",
                "output": a,
            })

            if len(examples) >= num_examples:
                break

            # Add slight variations
            examples.append({
                "instruction": q + "?",
                "input": "",
                "output": a,
            })

    examples = examples[:num_examples]
    logger.info(f"Generated {len(examples):,} training examples")

    # Save train/val split
    split_idx = int(len(examples) * 0.9)

    train_file = output_path / "train.jsonl"
    with open(train_file, 'w') as f:
        for ex in examples[:split_idx]:
            f.write(json.dumps(ex) + '\n')

    val_file = output_path / "validation.jsonl"
    with open(val_file, 'w') as f:
        for ex in examples[split_idx:]:
            f.write(json.dumps(ex) + '\n')

    logger.info(f"Train set: {split_idx:,} examples")
    logger.info(f"Validation set: {len(examples)-split_idx:,} examples")
    logger.info(f"Data preparation complete!")

if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", default="./training-data", help="Output directory")
    parser.add_argument("--num-examples", type=int, default=50000, help="Number of examples")
    args = parser.parse_args()

    generate_training_data(args.output, args.num_examples)
