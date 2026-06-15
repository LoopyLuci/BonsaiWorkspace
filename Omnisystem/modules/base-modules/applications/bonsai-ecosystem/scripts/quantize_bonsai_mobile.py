#!/usr/bin/env python3
"""
BonsAI Mobile Model Quantization — Convert trained HuggingFace model to GGUF.

Takes the final trained model from train_bonsai_mobile.py and:
  1. Merges LoRA adapter into base weights
  2. Quantizes to GGUF format (Q4_K_M by default)
  3. Validates output integrity
  4. Generates metadata and model card

Usage:
    python scripts/quantize_bonsai_mobile.py \\
        --final-model ~/.bonsai/models/checkpoints/bonsai-mobile-v1/final_model \\
        --output-dir ~/.bonsai/models/releases \\
        --quantization Q4_K_M \\
        --validate

This script is typically called automatically by the training pipeline.
"""

import argparse
import json
import logging
import os
import shutil
import sys
import subprocess
from datetime import datetime
from pathlib import Path
from typing import Dict, Optional

# Offline enforcement
os.environ.setdefault("TRANSFORMERS_OFFLINE", "1")
os.environ.setdefault("HF_HUB_OFFLINE", "1")


def setup_logging(output_dir: Path) -> logging.Logger:
    """Configure logging."""
    output_dir.mkdir(parents=True, exist_ok=True)
    logger = logging.getLogger("quantize_mobile")
    logger.setLevel(logging.INFO)

    ch = logging.StreamHandler()
    ch.setFormatter(logging.Formatter("[%(levelname)s] %(message)s"))
    logger.addHandler(ch)

    return logger


def emit(logger: logging.Logger, tag: str, **kwargs):
    """Emit structured log line."""
    parts = [f"{k}={v}" for k, v in kwargs.items()]
    logger.info(f"[{tag}] {' '.join(parts)}")


def get_model_size(model_path: Path) -> int:
    """Get total size of model files in bytes."""
    total = 0
    for f in model_path.rglob("*"):
        if f.is_file():
            total += f.stat().st_size
    return total


def merge_lora_adapter(
    final_model_dir: Path,
    output_dir: Path,
    logger: logging.Logger,
) -> Path:
    """
    Merge LoRA adapter (if present) into base model.
    Returns path to fused model.
    """
    from transformers import AutoModelForCausalLM, AutoTokenizer
    from peft import PeftModel
    import torch

    fused_dir = output_dir / "fused_model"
    fused_dir.mkdir(parents=True, exist_ok=True)

    student_model_dir = final_model_dir / "student_model"
    if not student_model_dir.exists():
        logger.warning(f"student_model dir not found, using {final_model_dir} directly")
        student_model_dir = final_model_dir

    emit(logger, "merge_start", model_dir=str(student_model_dir))

    try:
        # Load base model
        model = AutoModelForCausalLM.from_pretrained(
            str(student_model_dir),
            torch_dtype=torch.float32,
            device_map={"": "cpu"},
            local_files_only=True,
        )
        tokenizer = AutoTokenizer.from_pretrained(
            str(student_model_dir),
            local_files_only=True,
        )

        # Check if LoRA adapter is present
        adapter_config = student_model_dir / "adapter_config.json"
        if adapter_config.exists():
            emit(logger, "lora_detected", config=str(adapter_config))
            model = PeftModel.from_pretrained(model, str(student_model_dir))
            model = model.merge_and_unload()
            emit(logger, "lora_merged")
        else:
            emit(logger, "no_lora", using_base_model=True)

        # Save fused model
        model.save_pretrained(str(fused_dir))
        tokenizer.save_pretrained(str(fused_dir))
        emit(logger, "fused_model_saved", path=str(fused_dir))

        return fused_dir

    except Exception as e:
        logger.error(f"ERROR during merge: {e}")
        raise


def convert_to_gguf(
    fused_model_dir: Path,
    output_gguf: Path,
    quantization: str,
    llama_cpp_dir: Optional[Path],
    logger: logging.Logger,
) -> bool:
    """
    Convert HuggingFace model to GGUF using llama.cpp.

    Looks for convert_hf_to_gguf.py in llama_cpp_dir.
    """
    if llama_cpp_dir is None:
        llama_cpp_dir = Path.home() / "llama.cpp"

    llama_cpp_dir = Path(llama_cpp_dir)
    if not llama_cpp_dir.exists():
        logger.error(f"ERROR: llama.cpp directory not found: {llama_cpp_dir}")
        logger.error("       Clone it: git clone https://github.com/ggerganov/llama.cpp")
        return False

    # Find conversion script
    convert_script = llama_cpp_dir / "convert_hf_to_gguf.py"
    if not convert_script.exists():
        convert_script = llama_cpp_dir / "convert.py"
    if not convert_script.exists():
        logger.error(f"ERROR: Cannot find convert_hf_to_gguf.py in {llama_cpp_dir}")
        return False

    output_gguf.parent.mkdir(parents=True, exist_ok=True)

    # Build command
    cmd = [
        sys.executable,
        str(convert_script),
        str(fused_model_dir),
        "--outfile", str(output_gguf),
        "--outtype", quantization.lower().replace("-", "_"),
    ]

    emit(logger, "convert_start", quantization=quantization, output=str(output_gguf))
    logger.info(f"Running: {' '.join(cmd)}")

    try:
        result = subprocess.run(cmd, check=False, capture_output=False)
        if result.returncode != 0:
            logger.error(f"ERROR: Conversion failed with exit code {result.returncode}")
            return False

        if not output_gguf.exists():
            logger.error(f"ERROR: Output GGUF not created: {output_gguf}")
            return False

        size_mb = output_gguf.stat().st_size / (1024 ** 2)
        emit(logger, "convert_complete", output=str(output_gguf), size_mb=f"{size_mb:.1f}")
        return True

    except Exception as e:
        logger.error(f"ERROR during conversion: {e}")
        return False


def validate_gguf(gguf_path: Path, logger: logging.Logger) -> bool:
    """
    Validate GGUF file integrity and try to load it.

    This is a basic check — a full validation would run inference.
    """
    if not gguf_path.exists():
        logger.error(f"ERROR: GGUF file not found: {gguf_path}")
        return False

    size_mb = gguf_path.stat().st_size / (1024 ** 2)
    emit(logger, "gguf_validate", file=gguf_path.name, size_mb=f"{size_mb:.1f}")

    # Check magic number (GGUF files start with "GGUF")
    try:
        with open(gguf_path, "rb") as f:
            magic = f.read(4)
            if magic != b"GGUF":
                logger.error(f"ERROR: Invalid GGUF magic number: {magic}")
                return False
        emit(logger, "gguf_magic_valid")
    except Exception as e:
        logger.error(f"ERROR reading GGUF header: {e}")
        return False

    # Try to load with llama-cpp-python (optional)
    try:
        from llama_cpp import Llama
        model = Llama(str(gguf_path), n_ctx=128, n_threads=1, verbose=False)
        emit(logger, "gguf_load_test", status="success")
        return True
    except ImportError:
        emit(logger, "gguf_load_test", status="skipped_no_llama_cpp")
        return True  # llama-cpp-python not installed, but file seems valid
    except Exception as e:
        logger.error(f"ERROR loading GGUF with llama-cpp: {e}")
        return False


def generate_model_card(
    model_name: str,
    student_model: str,
    teacher_model: str,
    quantization: str,
    gguf_path: Path,
    training_summary: Optional[Dict],
    output_dir: Path,
    logger: logging.Logger,
) -> None:
    """Generate model card (README) for the quantized model."""
    size_mb = gguf_path.stat().st_size / (1024 ** 2) if gguf_path.exists() else 0

    card = f"""# {model_name}

Quantized mobile model trained via knowledge distillation from {teacher_model}.

## Model Details

- **Base Model**: {student_model}
- **Quantization**: {quantization}
- **Size**: {size_mb:.1f} MB
- **Created**: {datetime.now().isoformat()}

## Training

- **Teacher**: {teacher_model}
- **Method**: Knowledge Distillation (KL-divergence loss)
- **Framework**: PyTorch + PEFT (LoRA)
"""

    if training_summary:
        card += f"""
### Training Configuration
- **Epochs**: {training_summary.get('epochs', 'N/A')}
- **Batch Size**: {training_summary.get('batch_size', 'N/A')}
- **Learning Rate**: {training_summary.get('learning_rate', 'N/A')}
- **Best Validation Loss**: {training_summary.get('best_val_loss', 'N/A'):.4f}
- **Training Time**: {training_summary.get('training_time_sec', 0) / 3600:.1f} hours

### Data
- **Domains**: Code, Survival, Tool Use, Chat, Q&A
- **Total Training Examples**: (see training_summary.json)
"""

    card += f"""
## Usage

### With llama-cpp-python
```python
from llama_cpp import Llama

model = Llama(
    model_path="{gguf_path.name}",
    n_ctx=2048,
    n_threads=8,
)

response = model.create_completion(
    prompt="Write a function to sort a list in Python",
    max_tokens=256,
)
print(response['choices'][0]['text'])
```

### With llama-server (OpenAI-compatible)
```bash
llama-server -m {gguf_path.name} -ngl 35 -cb
curl http://localhost:8000/v1/completions \\
  -H "Content-Type: application/json" \\
  -d '{{
    "prompt": "Write a function to sort a list in Python",
    "max_tokens": 256
  }}'
```

## Performance

Mobile-optimized model suitable for:
- iOS/Android app inference
- Edge devices (Raspberry Pi, Jetson)
- CPU-only inference
- Latency-sensitive applications

Expected throughput: 10-50 tokens/second (depends on device)

## License

See original model licenses for {student_model} and {teacher_model}.

## Distillation Pipeline

This model was created with the BonsAI Mobile Training Pipeline:
https://github.com/your-org/bonsai/scripts/train_bonsai_mobile.py
"""

    card_path = output_dir / "README.md"
    card_path.write_text(card)
    emit(logger, "model_card_generated", path=str(card_path))


def create_metadata(
    model_name: str,
    student_model: str,
    teacher_model: str,
    quantization: str,
    gguf_path: Path,
    training_summary: Optional[Dict],
    output_dir: Path,
    logger: logging.Logger,
) -> None:
    """Create metadata JSON for model registry."""
    import hashlib

    # Compute file hash
    sha256_hash = hashlib.sha256()
    with open(gguf_path, "rb") as f:
        for chunk in iter(lambda: f.read(4096), b""):
            sha256_hash.update(chunk)

    metadata = {
        "name": model_name,
        "student_model": student_model,
        "teacher_model": teacher_model,
        "quantization": quantization,
        "file": gguf_path.name,
        "size_bytes": gguf_path.stat().st_size,
        "sha256": sha256_hash.hexdigest(),
        "created": datetime.now().isoformat(),
        "format": "GGUF",
        "context_length": training_summary.get("max_seq_len", 2048) if training_summary else 2048,
        "domains": ["coding", "system_repair", "tool_use", "chat", "qa"],
        "role": "mobile_student",
        "parameter_estimate": "500M-1.7B",  # Adjust based on student model
    }

    if training_summary:
        metadata["training"] = {
            "epochs": training_summary.get("epochs"),
            "batch_size": training_summary.get("batch_size"),
            "learning_rate": training_summary.get("learning_rate"),
            "alpha": training_summary.get("alpha"),
            "temperature": training_summary.get("temperature"),
            "best_val_loss": training_summary.get("best_val_loss"),
            "training_time_sec": training_summary.get("training_time_sec"),
        }

    metadata_path = output_dir / f"{gguf_path.stem}.metadata.json"
    metadata_path.write_text(json.dumps(metadata, indent=2))
    emit(logger, "metadata_created", path=str(metadata_path))


def create_bkp_package(
    gguf_path: Path,
    metadata_path: Path,
    output_dir: Path,
    logger: logging.Logger,
) -> None:
    """
    Create .bkp package (tar.gz with model, metadata, and model card).

    BKP format is BonsAI Knowledge Package.
    """
    import tarfile

    package_name = gguf_path.stem + ".bkp"
    package_path = output_dir / package_name

    emit(logger, "bkp_create_start", package=package_name)

    try:
        with tarfile.open(str(package_path), "w:gz") as tar:
            # Add GGUF
            tar.add(str(gguf_path), arcname=gguf_path.name)
            # Add metadata
            if metadata_path.exists():
                tar.add(str(metadata_path), arcname=metadata_path.name)
            # Add model card
            card_path = gguf_path.parent / "README.md"
            if card_path.exists():
                tar.add(str(card_path), arcname="README.md")

        size_mb = package_path.stat().st_size / (1024 ** 2)
        emit(logger, "bkp_created", package=str(package_path), size_mb=f"{size_mb:.1f}")

    except Exception as e:
        logger.error(f"ERROR creating .bkp package: {e}")


def main():
    parser = argparse.ArgumentParser(
        description="BonsAI Mobile Model Quantization"
    )

    parser.add_argument("--final-model", required=True,
                        help="Path to final trained model directory")
    parser.add_argument("--output-dir", required=True,
                        help="Output directory for GGUF and artifacts")
    parser.add_argument("--model-name", default="bonsai-mobile-v1",
                        help="Name for the quantized model")
    parser.add_argument("--student-model", default="TinyLlama-1.1B",
                        help="Student model name (for metadata)")
    parser.add_argument("--teacher-model", default="Bonsai-8B",
                        help="Teacher model name (for metadata)")
    parser.add_argument("--quantization", default="Q4_K_M",
                        choices=["Q4_K_M", "Q5_K_M", "Q8_0", "F16"],
                        help="GGUF quantization type")
    parser.add_argument("--llama-cpp-dir", default=None,
                        help="Path to llama.cpp directory")
    parser.add_argument("--validate", action="store_true",
                        help="Validate GGUF after quantization")
    parser.add_argument("--create-bkp", action="store_true", default=True,
                        help="Create .bkp package")
    parser.add_argument("--training-summary", default=None,
                        help="Path to training_summary.json")

    args = parser.parse_args()

    # ── Setup ────────────────────────────────────────────────────────────────

    output_dir = Path(args.output_dir)
    logger = setup_logging(output_dir)
    emit(logger, "start", task="quantize_bonsai_mobile")

    final_model_dir = Path(args.final_model)
    if not final_model_dir.exists():
        logger.error(f"ERROR: Final model directory not found: {final_model_dir}")
        sys.exit(1)

    # Load training summary if provided
    training_summary = None
    if args.training_summary:
        summary_path = Path(args.training_summary)
        if summary_path.exists():
            training_summary = json.loads(summary_path.read_text())
            emit(logger, "training_summary_loaded", path=str(summary_path))

    # ── Merge LoRA ────────────────────────────────────────────────────────────

    fused_dir = merge_lora_adapter(final_model_dir, output_dir, logger)

    # ── Convert to GGUF ──────────────────────────────────────────────────────

    gguf_path = output_dir / f"{args.model_name}.gguf"
    success = convert_to_gguf(
        fused_dir,
        gguf_path,
        args.quantization,
        Path(args.llama_cpp_dir) if args.llama_cpp_dir else None,
        logger,
    )

    if not success:
        logger.error("ERROR: GGUF conversion failed")
        sys.exit(1)

    # ── Validate ──────────────────────────────────────────────────────────────

    if args.validate:
        is_valid = validate_gguf(gguf_path, logger)
        if not is_valid:
            logger.error("ERROR: GGUF validation failed")
            sys.exit(1)

    # ── Generate artifacts ────────────────────────────────────────────────────

    metadata_path = output_dir / f"{gguf_path.stem}.metadata.json"
    create_metadata(
        args.model_name,
        args.student_model,
        args.teacher_model,
        args.quantization,
        gguf_path,
        training_summary,
        output_dir,
        logger,
    )

    generate_model_card(
        args.model_name,
        args.student_model,
        args.teacher_model,
        args.quantization,
        gguf_path,
        training_summary,
        output_dir,
        logger,
    )

    if args.create_bkp:
        create_bkp_package(gguf_path, metadata_path, output_dir, logger)

    # ── Summary ───────────────────────────────────────────────────────────────

    emit(logger, "quantization_complete",
         gguf=str(gguf_path),
         size_mb=f"{gguf_path.stat().st_size / (1024**2):.1f}",
         quantization=args.quantization)

    logger.info("=" * 80)
    logger.info("QUANTIZATION COMPLETE")
    logger.info(f"GGUF: {gguf_path}")
    logger.info(f"Size: {gguf_path.stat().st_size / (1024**2):.1f} MB")
    logger.info(f"Output directory: {output_dir}")
    logger.info("=" * 80)


if __name__ == "__main__":
    main()
