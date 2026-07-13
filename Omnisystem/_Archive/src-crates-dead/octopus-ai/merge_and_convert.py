#!/usr/bin/env python3
"""
Phase 5: Merge LoRA adapter and convert to GGUF format
Output: server-expert-model-v1.Q4_K_M.gguf
"""

import torch
import logging
import subprocess
import os
from pathlib import Path
from peft import PeftModel
from transformers import AutoModelForCausalLM, AutoTokenizer

logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s [%(levelname)s] %(message)s'
)
logger = logging.getLogger(__name__)

base_model_id = "distilgpt2"
lora_path = "./server-expert-lora"
merged_output = "./server-expert-merged"
gguf_output = "./server-expert-model-v1.Q4_K_M.gguf"

logger.info(f"Base model: {base_model_id}")
logger.info(f"LoRA adapter: {lora_path}")

# Step 1: Merge LoRA into base model
logger.info("Loading base model...")
model = AutoModelForCausalLM.from_pretrained(
    base_model_id,
    device_map="cpu",
    torch_dtype=torch.float32,
)

logger.info("Loading LoRA adapter...")
model = PeftModel.from_pretrained(model, lora_path)

logger.info("Merging LoRA into base model...")
model = model.merge_and_unload()

logger.info(f"Saving merged model to {merged_output}...")
model.save_pretrained(merged_output)

logger.info("Loading tokenizer...")
tokenizer = AutoTokenizer.from_pretrained(base_model_id)
tokenizer.save_pretrained(merged_output)

logger.info("Model merged successfully!")

# Step 2: Convert to GGUF format using llama.cpp
logger.info("Converting to GGUF format...")

llama_cpp_convert = "llama.cpp/convert.py"

if not os.path.exists(llama_cpp_convert):
    logger.warning("llama.cpp not found. Cloning...")
    result = subprocess.run(
        ["git", "clone", "https://github.com/ggerganov/llama.cpp"],
        capture_output=True,
        text=True
    )
    if result.returncode != 0:
        logger.error(f"Failed to clone llama.cpp: {result.stderr}")
        logger.info("Skipping GGUF conversion - llama.cpp not available")
        logger.info("Merged model available at: " + merged_output)
        exit(0)

if os.path.exists(llama_cpp_convert):
    logger.info("Running llama.cpp conversion...")
    convert_cmd = [
        "python",
        llama_cpp_convert,
        merged_output,
        "--outfile", gguf_output,
        "--outtype", "q4_k_m",
    ]

    result = subprocess.run(convert_cmd, capture_output=True, text=True)

    if result.returncode == 0:
        logger.info(result.stdout)
    else:
        logger.warning(f"Conversion had issues: {result.stderr}")
        if result.stdout:
            logger.info(result.stdout)

# Verify output
if os.path.exists(gguf_output):
    size_mb = os.path.getsize(gguf_output) / (1024 * 1024)
    logger.info(f"GGUF model created: {gguf_output} ({size_mb:.1f} MB)")
elif os.path.exists(merged_output):
    logger.info(f"Merged model available: {merged_output}")
    logger.info("GGUF conversion skipped - llama.cpp may not be available")
else:
    logger.error("Neither GGUF nor merged model found!")
    exit(1)
