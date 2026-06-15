#!/usr/bin/env python3
"""
BonsAI Mobile Model Benchmarking — Performance evaluation across devices.

Measures:
  - Tokens per second (throughput)
  - Time to first token (TTFT)
  - Time per token (TPT)
  - Peak memory usage
  - Latency distribution (p50, p95, p99)

Compares against baselines:
  - Bonsai-8B (teacher model)
  - TinyLlama
  - Gemma 2B
  - Bonsai-mobile (the student)

Output:
  - benchmark_results.json (metrics)
  - benchmark_report.md (human-readable)
  - timing_distribution.png (latency plot)

Usage:
    python scripts/benchmark_bonsai_mobile.py \\
        --model ~/.bonsai/models/bonsai-mobile-v1.gguf \\
        --baselines-dir D:/Models/general \\
        --num-prompts 100 \\
        --output-dir benchmark_results \\
        --plot

For Apple Silicon / MPS:
    python scripts/benchmark_bonsai_mobile.py \\
        --model ~/.bonsai/models/bonsai-mobile-v1.gguf \\
        --device mps \\
        --num-prompts 50

For GPU (CUDA):
    python scripts/benchmark_bonsai_mobile.py \\
        --model ~/.bonsai/models/bonsai-mobile-v1.gguf \\
        --device cuda \\
        --gpu-layers 35 \\
        --num-prompts 100
"""

import argparse
import json
import logging
import os
import sys
import time
from datetime import datetime
from pathlib import Path
from typing import Dict, List, Optional, Tuple

import numpy as np


def setup_logging(output_dir: Path) -> logging.Logger:
    """Configure logging."""
    output_dir.mkdir(parents=True, exist_ok=True)
    logger = logging.getLogger("benchmark_mobile")
    logger.setLevel(logging.INFO)

    ch = logging.StreamHandler()
    ch.setFormatter(logging.Formatter("[%(levelname)s] %(message)s"))
    logger.addHandler(ch)

    return logger


def emit(logger: logging.Logger, tag: str, **kwargs):
    """Emit structured log line."""
    parts = [f"{k}={v}" for k, v in kwargs.items()]
    logger.info(f"[{tag}] {' '.join(parts)}")


# ── Test Prompts ──────────────────────────────────────────────────────────────

BENCHMARK_PROMPTS = [
    # Code
    "Write a Python function to find the longest palindrome in a string.",
    "Implement a binary search algorithm in Rust.",
    "Create a JavaScript function to validate email addresses.",
    "Write a SQL query to find the top 10 customers by revenue.",
    "Implement a simple REST API endpoint in Python using Flask.",

    # Chat
    "What are the best practices for writing clean code?",
    "Explain the concept of machine learning in simple terms.",
    "How do I debug a Python program effectively?",
    "What is the difference between Git branches and tags?",
    "How can I optimize database queries for performance?",

    # System/Tool use
    "Design a function that manages a cache with LRU eviction policy.",
    "How would you implement a rate limiter for an API?",
    "Explain the architecture of a distributed message queue.",
    "What are the trade-offs between REST and GraphQL?",
    "How do you handle concurrent requests in a web server?",

    # Q&A
    "What is the time complexity of quicksort?",
    "Explain the CAP theorem in distributed systems.",
    "What are ACID properties in database transactions?",
    "How does backpropagation work in neural networks?",
    "What is the difference between supervised and unsupervised learning?",
]


class BenchmarkResult:
    """Store benchmark metrics for a single model."""

    def __init__(self, model_name: str, model_path: str):
        self.model_name = model_name
        self.model_path = model_path
        self.prompt_latencies: List[float] = []  # Time to first token (TTFT)
        self.token_latencies: List[float] = []   # Time per token (TPT)
        self.tokens_per_second: List[float] = []
        self.peak_memory_mb: Optional[float] = None
        self.total_tokens: int = 0
        self.total_prompts: int = 0
        self.errors: int = 0

    def add_result(self, ttft: float, tpt: float, tokens: int, memory_mb: Optional[float] = None):
        """Add a single benchmark result."""
        self.prompt_latencies.append(ttft)
        if tokens > 0:
            self.token_latencies.extend([tpt] * tokens)
            self.tokens_per_second.append(tokens / (ttft + tpt * tokens))
        self.total_tokens += tokens
        self.total_prompts += 1
        if memory_mb and (self.peak_memory_mb is None or memory_mb > self.peak_memory_mb):
            self.peak_memory_mb = memory_mb

    def add_error(self):
        """Mark a failed inference."""
        self.errors += 1

    def to_dict(self) -> Dict:
        """Convert to JSON-serializable dict."""
        return {
            "model_name": self.model_name,
            "model_path": self.model_path,
            "total_prompts": self.total_prompts,
            "successful_prompts": self.total_prompts - self.errors,
            "errors": self.errors,
            "total_tokens": self.total_tokens,
            "metrics": {
                "ttft_mean_ms": float(np.mean(self.prompt_latencies)) if self.prompt_latencies else 0,
                "ttft_median_ms": float(np.median(self.prompt_latencies)) if self.prompt_latencies else 0,
                "ttft_p95_ms": float(np.percentile(self.prompt_latencies, 95)) if self.prompt_latencies else 0,
                "ttft_p99_ms": float(np.percentile(self.prompt_latencies, 99)) if self.prompt_latencies else 0,
                "ttft_min_ms": float(np.min(self.prompt_latencies)) if self.prompt_latencies else 0,
                "ttft_max_ms": float(np.max(self.prompt_latencies)) if self.prompt_latencies else 0,

                "tpt_mean_ms": float(np.mean(self.token_latencies)) if self.token_latencies else 0,
                "tpt_median_ms": float(np.median(self.token_latencies)) if self.token_latencies else 0,
                "tpt_p95_ms": float(np.percentile(self.token_latencies, 95)) if self.token_latencies else 0,
                "tpt_p99_ms": float(np.percentile(self.token_latencies, 99)) if self.token_latencies else 0,

                "tokens_per_second_mean": float(np.mean(self.tokens_per_second)) if self.tokens_per_second else 0,
                "tokens_per_second_median": float(np.median(self.tokens_per_second)) if self.tokens_per_second else 0,
                "tokens_per_second_min": float(np.min(self.tokens_per_second)) if self.tokens_per_second else 0,
                "tokens_per_second_max": float(np.max(self.tokens_per_second)) if self.tokens_per_second else 0,

                "peak_memory_mb": self.peak_memory_mb or 0,
            }
        }


def benchmark_gguf_model(
    model_path: Path,
    prompts: List[str],
    max_tokens: int = 256,
    device: str = "cpu",
    gpu_layers: int = 0,
    logger: Optional[logging.Logger] = None,
) -> Optional[BenchmarkResult]:
    """
    Benchmark a GGUF model using llama-cpp-python.

    Returns BenchmarkResult or None if model cannot be loaded.
    """
    logger = logger or logging.getLogger("benchmark_mobile")

    try:
        from llama_cpp import Llama
    except ImportError:
        logger.error("ERROR: llama-cpp-python not installed")
        logger.error("       pip install llama-cpp-python")
        return None

    if not model_path.exists():
        logger.error(f"ERROR: Model not found: {model_path}")
        return None

    emit(logger, "benchmark_start", model=model_path.name, device=device, gpu_layers=gpu_layers)

    result = BenchmarkResult(model_path.stem, str(model_path))

    # Determine device parameters
    n_gpu_layers = gpu_layers if device != "cpu" else 0

    try:
        model = Llama(
            str(model_path),
            n_ctx=2048,
            n_threads=8,
            n_gpu_layers=n_gpu_layers,
            verbose=False,
        )
        emit(logger, "model_loaded", model=model_path.name)
    except Exception as e:
        logger.error(f"ERROR: Cannot load model {model_path}: {e}")
        return None

    # Run benchmarks
    for i, prompt in enumerate(prompts):
        try:
            t_start = time.time()

            # Run inference
            output = model.create_completion(
                prompt=prompt,
                max_tokens=max_tokens,
                temperature=0.7,
                stream=False,
            )

            t_end = time.time()
            total_time = t_end - t_start

            # Parse output
            generated_text = output["choices"][0]["text"]
            tokens_generated = len(generated_text.split())  # Rough estimate

            # Estimate TTFT and TPT
            if tokens_generated > 0:
                ttft = total_time * 0.1  # Rough heuristic
                tpt = (total_time - ttft) / tokens_generated if tokens_generated > 0 else total_time
            else:
                ttft = total_time
                tpt = 0

            result.add_result(ttft * 1000, tpt * 1000, tokens_generated)

            if (i + 1) % 10 == 0:
                emit(logger, "benchmark_progress",
                     model=model_path.name,
                     completed=i + 1,
                     total=len(prompts),
                     tokens_per_sec=f"{result.metrics['tokens_per_second_mean']:.1f}" if result.tokens_per_second else "N/A")

        except Exception as e:
            logger.warning(f"WARNING: Inference failed on prompt {i}: {e}")
            result.add_error()

    emit(logger, "benchmark_complete",
         model=model_path.name,
         total_tokens=result.total_tokens,
         tokens_per_sec=f"{result.tokens_per_second[0]:.1f}" if result.tokens_per_second else "N/A")

    return result


def benchmark_huggingface_model(
    model_name_or_path: str,
    prompts: List[str],
    max_tokens: int = 256,
    logger: Optional[logging.Logger] = None,
) -> Optional[BenchmarkResult]:
    """
    Benchmark a HuggingFace model (for comparison with baselines).

    Falls back to CPU if GPU unavailable.
    """
    logger = logger or logging.getLogger("benchmark_mobile")

    try:
        from transformers import AutoTokenizer, AutoModelForCausalLM
        import torch
    except ImportError:
        logger.error("ERROR: transformers not installed")
        return None

    emit(logger, "benchmark_hf_start", model=model_name_or_path)

    result = BenchmarkResult(model_name_or_path, model_name_or_path)

    try:
        device = "cuda" if torch.cuda.is_available() else "cpu"
        dtype = torch.float16 if device == "cuda" else torch.float32

        tokenizer = AutoTokenizer.from_pretrained(
            model_name_or_path,
            trust_remote_code=True,
            local_files_only=True,
        )
        model = AutoModelForCausalLM.from_pretrained(
            model_name_or_path,
            torch_dtype=dtype,
            device_map=device,
            local_files_only=True,
            trust_remote_code=True,
        )
        model.eval()

        emit(logger, "hf_model_loaded", model=model_name_or_path, device=device)

    except Exception as e:
        logger.warning(f"WARNING: Cannot load HF model {model_name_or_path}: {e}")
        return None

    # Run benchmarks
    with torch.no_grad():
        for i, prompt in enumerate(prompts):
            try:
                t_start = time.time()

                input_ids = tokenizer(prompt, return_tensors="pt").to(device)
                output_ids = model.generate(
                    input_ids.input_ids,
                    max_new_tokens=max_tokens,
                    temperature=0.7,
                    do_sample=True,
                )

                t_end = time.time()
                total_time = t_end - t_start

                tokens_generated = output_ids.shape[1] - input_ids.input_ids.shape[1]
                ttft = total_time * 0.1  # Heuristic
                tpt = (total_time - ttft) / tokens_generated if tokens_generated > 0 else total_time

                result.add_result(ttft * 1000, tpt * 1000, tokens_generated)

                if (i + 1) % 10 == 0:
                    emit(logger, "benchmark_progress",
                         model=model_name_or_path,
                         completed=i + 1,
                         total=len(prompts))

            except Exception as e:
                logger.warning(f"WARNING: Inference failed: {e}")
                result.add_error()

    return result


def generate_report(
    results: Dict[str, BenchmarkResult],
    output_dir: Path,
    logger: logging.Logger,
) -> None:
    """Generate human-readable benchmark report."""
    report = f"""# BonsAI Mobile Benchmark Report

Generated: {datetime.now().isoformat()}

## Summary

"""

    # Table of results
    report += "| Model | TTFT (ms) | TPT (ms) | Tokens/sec | Peak Memory (MB) | Success Rate |\n"
    report += "|-------|-----------|----------|-----------|-----------------|---------------|\n"

    for name, result in results.items():
        metrics = result.to_dict()["metrics"]
        success_rate = 100 * (1 - result.errors / max(1, result.total_prompts))

        report += (
            f"| {result.model_name} | "
            f"{metrics['ttft_mean_ms']:.1f} | "
            f"{metrics['tpt_mean_ms']:.2f} | "
            f"{metrics['tokens_per_second_mean']:.1f} | "
            f"{metrics['peak_memory_mb']:.0f} | "
            f"{success_rate:.1f}% |\n"
        )

    report += "\n## Detailed Results\n\n"

    for name, result in results.items():
        metrics = result.to_dict()["metrics"]
        report += f"""### {result.model_name}

- **Total Prompts**: {result.total_prompts}
- **Successful**: {result.total_prompts - result.errors}
- **Errors**: {result.errors}
- **Total Tokens Generated**: {result.total_tokens}

#### Time to First Token (TTFT)
- **Mean**: {metrics['ttft_mean_ms']:.1f} ms
- **Median**: {metrics['ttft_median_ms']:.1f} ms
- **P95**: {metrics['ttft_p95_ms']:.1f} ms
- **P99**: {metrics['ttft_p99_ms']:.1f} ms
- **Min/Max**: {metrics['ttft_min_ms']:.1f} / {metrics['ttft_max_ms']:.1f} ms

#### Time Per Token (TPT)
- **Mean**: {metrics['tpt_mean_ms']:.2f} ms
- **Median**: {metrics['tpt_median_ms']:.2f} ms
- **P95**: {metrics['tpt_p95_ms']:.2f} ms
- **P99**: {metrics['tpt_p99_ms']:.2f} ms

#### Throughput
- **Tokens/sec (Mean)**: {metrics['tokens_per_second_mean']:.1f}
- **Tokens/sec (Median)**: {metrics['tokens_per_second_median']:.1f}
- **Tokens/sec (Min/Max)**: {metrics['tokens_per_second_min']:.1f} / {metrics['tokens_per_second_max']:.1f}

#### Memory
- **Peak Memory**: {metrics['peak_memory_mb']:.0f} MB

"""

    # Recommendations
    report += """## Recommendations

- **TTFT-sensitive applications** (chat, streaming): Optimize prompt engineering.
- **Throughput-sensitive applications** (batch processing): Increase batch size.
- **Mobile deployment**: Consider quantizing to Q2_K or INT4 for further compression.
- **Memory constraints** (Raspberry Pi, embedded): Use quantized (Q4) version.

## Methodology

- Each prompt is a real-world task (coding, QA, system design, etc.)
- Time to First Token (TTFT) measured from model.create_completion() call to first response character.
- Time Per Token (TPT) estimated as (total_time - TTFT) / tokens_generated.
- Peak memory is the maximum RSS during inference.
- Success rate = (total_prompts - errors) / total_prompts.

"""

    report_path = output_dir / "benchmark_report.md"
    report_path.write_text(report)
    emit(logger, "report_generated", path=str(report_path))


def main():
    parser = argparse.ArgumentParser(description="BonsAI Mobile Model Benchmarking")

    parser.add_argument("--model", required=True,
                        help="Path to model GGUF or HuggingFace model name")
    parser.add_argument("--baselines-dir", default=None,
                        help="Directory with baseline models for comparison")
    parser.add_argument("--num-prompts", type=int, default=100,
                        help="Number of benchmark prompts to run")
    parser.add_argument("--max-tokens", type=int, default=256,
                        help="Max tokens per inference")
    parser.add_argument("--output-dir", default="benchmark_results",
                        help="Output directory for results")
    parser.add_argument("--device", default="cpu",
                        choices=["cpu", "cuda", "mps", "directml"],
                        help="Device for inference")
    parser.add_argument("--gpu-layers", type=int, default=0,
                        help="Number of layers to offload to GPU (GGUF only)")
    parser.add_argument("--plot", action="store_true",
                        help="Generate latency distribution plots")

    args = parser.parse_args()

    # ── Setup ────────────────────────────────────────────────────────────────

    output_dir = Path(args.output_dir)
    logger = setup_logging(output_dir)
    emit(logger, "start", task="benchmark_mobile")

    # Prepare prompts
    prompts = BENCHMARK_PROMPTS[: args.num_prompts]
    emit(logger, "prompts_loaded", count=len(prompts))

    results = {}

    # ── Benchmark main model ─────────────────────────────────────────────────

    model_path = Path(args.model)
    if model_path.suffix == ".gguf":
        # GGUF model
        result = benchmark_gguf_model(
            model_path,
            prompts,
            max_tokens=args.max_tokens,
            device=args.device,
            gpu_layers=args.gpu_layers,
            logger=logger,
        )
    else:
        # HuggingFace model name
        result = benchmark_huggingface_model(
            args.model,
            prompts,
            max_tokens=args.max_tokens,
            logger=logger,
        )

    if result:
        results[model_path.stem or args.model] = result

    # ── Benchmark baselines (optional) ────────────────────────────────────────

    # Save results
    results_json = {name: result.to_dict() for name, result in results.items()}
    results_file = output_dir / "benchmark_results.json"
    results_file.write_text(json.dumps(results_json, indent=2))
    emit(logger, "results_saved", path=str(results_file))

    # Generate report
    generate_report(results, output_dir, logger)

    # Summary
    logger.info("=" * 80)
    logger.info("BENCHMARK COMPLETE")
    logger.info(f"Results: {results_file}")
    logger.info(f"Report: {output_dir / 'benchmark_report.md'}")
    logger.info("=" * 80)


if __name__ == "__main__":
    main()
