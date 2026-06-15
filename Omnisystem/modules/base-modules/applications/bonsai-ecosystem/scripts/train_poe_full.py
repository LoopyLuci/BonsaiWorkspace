#!/usr/bin/env python3
"""
Poe AI Empathy Model — Complete 7-Stage Training Pipeline.
Integrates with Bonsai Model Trainer and produces a .bkp package.
"""
import argparse
import json
import subprocess
import sys
from pathlib import Path

BONSAI_TRAINER = Path("./target/release/bonsai-trainer")

def run_stage(name, cmd):
    print(f"\n{'='*60}")
    print(f"🧬 Stage: {name}")
    print(f"{'='*60}")
    result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
    if result.returncode != 0:
        print(f"❌ Stage {name} failed:\n{result.stderr}")
        sys.exit(1)
    print(f"✅ Stage {name} complete")
    if result.stdout:
        print(result.stdout[-500:])

def stage1_distillation(args):
    """Distill from a large teacher model into the BAT minimal configuration."""
    cmd = f"""
    python -m bonsai_trainer.distill \
        --teacher-model {args.teacher_model} \
        --student-config poe-empathy-bat-minimal.json \
        --data "tdl://ds-empathetic-dialogues-v1" \
        --data "tdl://ds-biometric-affect-v1" \
        --output {args.checkpoint_dir}/stage1-distilled \
        --epochs 3 \
        --batch-size 64 \
        --learning-rate 2e-4
    """
    run_stage("1 - Distillation", cmd)

def stage2_progressive_scaling(args):
    """Train depth, width, and expert scaling dimensions."""
    for dim in ["depth", "width", "experts"]:
        cmd = f"""
        python -m bonsai_trainer.train_scaling \
            --base-checkpoint {args.checkpoint_dir}/stage1-distilled \
            --dimension {dim} \
            --data "tdl://ds-empathetic-dialogues-v1" \
            --output {args.checkpoint_dir}/stage2-{dim} \
            --steps 10000
        """
        run_stage(f"2 - Progressive Scaling ({dim})", cmd)

def stage3_lora_adapters(args):
    """Train 100 LoRA adapters for fine-grained personality control."""
    cmd = f"""
    python -m bonsai_trainer.train_adaptive_lora \
        --base-model {args.checkpoint_dir}/stage2-depth \
        --num-adapters 100 \
        --dataset "tdl://ds-empathetic-dialogues-v1" \
        --dataset "tdl://ds-ac-poe-style-v1" \
        --output {args.checkpoint_dir}/stage3-lora \
        --r 16 --alpha 32
    """
    run_stage("3 - LoRA Adapter Stacking", cmd)

def stage4_constitutional_dpo(args):
    """Constitutional DPO training for ethical alignment (Guardrail)."""
    cmd = f"""
    python -m bonsai_trainer.dpo_train \
        --base-model {args.checkpoint_dir}/stage3-lora \
        --dataset "tdl://ds-constitutional-safety-v1" \
        --output {args.checkpoint_dir}/stage4-constitutional \
        --beta 0.1 \
        --epochs 2
    """
    run_stage("4 - Constitutional DPO", cmd)

def stage5_refusal_classifier(args):
    """Train the refusal classifier for Guardrail safety layer."""
    cmd = f"""
    python -m bonsai_trainer.train_classifier \
        --dataset "tdl://ds-constitutional-safety-v1" \
        --output {args.checkpoint_dir}/stage5-classifier.onnx \
        --model-size 50M
    """
    run_stage("5 - Refusal Classifier", cmd)

def stage6_kdb_integration(args):
    """Fine-tune cross-attention layers for KDB knowledge retrieval."""
    cmd = f"""
    python -m bonsai_trainer.finetune_kdb \
        --base-model {args.checkpoint_dir}/stage4-constitutional \
        --kdb-modules ac-poe-style.kmod \
        --output {args.checkpoint_dir}/stage6-kdb \
        --epochs 1
    """
    run_stage("6 - KDB Integration", cmd)

def stage7_joint_finetune(args):
    """Final joint fine-tuning with all components unfrozen."""
    cmd = f"""
    python -m bonsai_trainer.joint_finetune \
        --checkpoint {args.checkpoint_dir}/stage6-kdb \
        --random-scale true \
        --output {args.checkpoint_dir}/stage7-final \
        --epochs 1
    """
    run_stage("7 - Joint Fine-tuning", cmd)

def package_model(args):
    """Package the trained model into a signed .bkp file."""
    cmd = f"""
    cargo run --release -p bonsai-package -- create \
        --checkpoint {args.checkpoint_dir}/stage7-final \
        --safety-layers {args.checkpoint_dir}/stage5-classifier.onnx \
        --kdb-modules ac-poe-style.kmod \
        --output {args.output_dir}/poe-empathy-v1.bkp \
        --sign
    """
    run_stage("Package", cmd)

    if args.encrypt_flowers:
        cmd = f"""
        cargo run --release -p bonsai-package -- encrypt \
            --input {args.output_dir}/poe-empathy-v1.bkp \
            --tpm-seal \
            --output {args.output_dir}/poe-flowers-v1.bkp.enc
        """
        run_stage("Package (Flowers encryption)", cmd)

def main():
    parser = argparse.ArgumentParser(description="Train Poe AI Empathy Model")
    parser.add_argument("--teacher-model", default="bonsai://models/llama-3-8b.bkp")
    parser.add_argument("--checkpoint-dir", default="./checkpoints/poe")
    parser.add_argument("--output-dir", default="./releases")
    parser.add_argument("--encrypt-flowers", action="store_true")
    parser.add_argument("--stages", default="1-7", help="Stages to run (e.g., '1-7' or '1,3,5')")
    args = parser.parse_args()

    Path(args.checkpoint_dir).mkdir(parents=True, exist_ok=True)
    Path(args.output_dir).mkdir(parents=True, exist_ok=True)

    stages_to_run = set()
    for part in args.stages.split(","):
        if "-" in part:
            start, end = part.split("-")
            stages_to_run.update(range(int(start), int(end)+1))
        else:
            stages_to_run.add(int(part))

    stage_fns = {
        1: stage1_distillation,
        2: stage2_progressive_scaling,
        3: stage3_lora_adapters,
        4: stage4_constitutional_dpo,
        5: stage5_refusal_classifier,
        6: stage6_kdb_integration,
        7: stage7_joint_finetune,
    }

    for stage in sorted(stages_to_run):
        if stage in stage_fns:
            stage_fns[stage](args)

    if 7 in stages_to_run:
        package_model(args)

    print("\n🎉 Poe AI training complete!")

if __name__ == "__main__":
    main()
