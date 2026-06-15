#!/usr/bin/env python3
"""
Export Training Data for Mobile Distillation — prepare JSONL for train_bonsai_mobile.py.

This script aggregates data from multiple sources:
  - Survival system fixes (crashes, patches)
  - Code examples (from datasets and generated)
  - Chat conversations (eternal training loop)
  - Tool-calling examples (MCP integration)
  - Q&A pairs (Academy, documentation)

Output: ~/.bonsai/training_export/combined_mobile_training.jsonl
  Each line: {"text": "...", "domain": "code|survival|chat|tool_use|qa", "quality": 0.0-1.0}

Usage:
    python scripts/export_mobile_training_data.py \\
        --output ~/.bonsai/training_export/combined_mobile_training.jsonl \\
        --max-examples 100000 \\
        --min-quality 0.70 \\
        --domain-weights code:0.4,survival:0.2,tool_use:0.2,chat:0.1,qa:0.1
"""

import argparse
import json
import logging
import sys
from datetime import datetime, timedelta
from pathlib import Path
from typing import Dict, List, Optional


def setup_logging() -> logging.Logger:
    """Configure logging."""
    logger = logging.getLogger("export_mobile")
    logger.setLevel(logging.INFO)
    ch = logging.StreamHandler()
    ch.setFormatter(logging.Formatter("[%(levelname)s] %(message)s"))
    logger.addHandler(ch)
    return logger


def emit(logger: logging.Logger, tag: str, **kwargs):
    """Emit structured log line."""
    parts = [f"{k}={v}" for k, v in kwargs.items()]
    logger.info(f"[{tag}] {' '.join(parts)}")


# ── Data Source Loaders ──────────────────────────────────────────────────────

def load_jsonl(path: Path, domain: str, min_quality: float = 0.0, limit: Optional[int] = None) -> List[Dict]:
    """Load examples from JSONL file."""
    if not path.exists():
        return []

    examples = []
    try:
        with open(path, "r", encoding="utf-8") as f:
            for line in f:
                if limit and len(examples) >= limit:
                    break

                try:
                    record = json.loads(line)
                except json.JSONDecodeError:
                    continue

                # Extract text
                text = record.get("text") or record.get("prompt") or record.get("instruction") or ""
                if not text or not text.strip():
                    continue

                quality = record.get("quality", 1.0)
                if quality < min_quality:
                    continue

                examples.append({
                    "text": text,
                    "domain": domain,
                    "quality": quality,
                    "source": path.name,
                })
    except Exception as e:
        print(f"WARNING: Error loading {path}: {e}")

    return examples


def load_survival_fixes(logger: logging.Logger, limit: Optional[int] = None) -> List[Dict]:
    """Load from survival system database."""
    examples = []
    db_path = Path.home() / ".bonsai" / "survival_kb.db"

    if not db_path.exists():
        emit(logger, "skip_survival", reason="database_not_found")
        return examples

    try:
        import sqlite3
        conn = sqlite3.connect(db_path)
        cursor = conn.cursor()

        # Query: recent survival fixes with quality score
        query = """
        SELECT fix_description, confidence FROM survival_entries
        WHERE fixed_at > datetime('now', '-90 days')
        ORDER BY confidence DESC
        """
        cursor.execute(query)

        count = 0
        for row in cursor.fetchall():
            if limit and count >= limit:
                break

            fix_desc, confidence = row
            if fix_desc and fix_desc.strip():
                examples.append({
                    "text": fix_desc,
                    "domain": "system_repair",
                    "quality": min(1.0, confidence),
                    "source": "survival_kb",
                })
                count += 1

        conn.close()
        emit(logger, "load_survival", count=len(examples))

    except Exception as e:
        emit(logger, "skip_survival", error=str(e))

    return examples


def load_code_examples(logger: logging.Logger, limit: Optional[int] = None) -> List[Dict]:
    """Load code examples from datasets and generated data."""
    examples = []
    code_dir = Path.home() / ".bonsai" / "training_export"

    if not code_dir.exists():
        emit(logger, "skip_code", reason="directory_not_found")
        return examples

    code_patterns = ["code_*.jsonl", "*code*.jsonl", "*examples*.jsonl"]
    count = 0

    for pattern in code_patterns:
        for path in code_dir.glob(pattern):
            file_examples = load_jsonl(path, "code", min_quality=0.70, limit=limit - count if limit else None)
            examples.extend(file_examples)
            count += len(file_examples)
            if limit and count >= limit:
                break

    emit(logger, "load_code", count=len(examples))
    return examples


def load_chat_conversations(logger: logging.Logger, limit: Optional[int] = None) -> List[Dict]:
    """Load chat conversations from database."""
    examples = []
    db_path = Path.home() / ".bonsai" / "chat_sessions.db"

    if not db_path.exists():
        emit(logger, "skip_chat", reason="database_not_found")
        return examples

    try:
        import sqlite3
        conn = sqlite3.connect(db_path)
        cursor = conn.cursor()

        # Query: recent chat exchanges
        query = """
        SELECT user_message, assistant_response, quality_score
        FROM chat_history
        WHERE created_at > datetime('now', '-30 days')
        AND quality_score >= 0.70
        ORDER BY quality_score DESC
        LIMIT ?
        """
        cursor.execute(query, (limit or 10000,))

        count = 0
        for row in cursor.fetchall():
            user_msg, asst_msg, quality = row
            if user_msg and asst_msg:
                text = f"{user_msg}\n\n{asst_msg}"
                examples.append({
                    "text": text,
                    "domain": "chat",
                    "quality": min(1.0, quality),
                    "source": "chat_sessions",
                })
                count += 1

        conn.close()
        emit(logger, "load_chat", count=len(examples))

    except Exception as e:
        emit(logger, "skip_chat", error=str(e))

    return examples


def load_tool_use_examples(logger: logging.Logger, limit: Optional[int] = None) -> List[Dict]:
    """Load tool-calling and MCP integration examples."""
    examples = []
    export_dir = Path.home() / ".bonsai" / "training_export"

    if not export_dir.exists():
        emit(logger, "skip_tool_use", reason="directory_not_found")
        return examples

    tool_files = list(export_dir.glob("*tool*.jsonl"))
    tool_files.extend(export_dir.glob("*mcp*.jsonl"))
    tool_files.extend(export_dir.glob("*function*.jsonl"))

    count = 0
    for path in tool_files:
        file_examples = load_jsonl(path, "tool_use", min_quality=0.70, limit=limit - count if limit else None)
        examples.extend(file_examples)
        count += len(file_examples)
        if limit and count >= limit:
            break

    emit(logger, "load_tool_use", count=len(examples))
    return examples


def load_qa_pairs(logger: logging.Logger, limit: Optional[int] = None) -> List[Dict]:
    """Load Q&A pairs from Academy and documentation."""
    examples = []
    qa_dir = Path.home() / ".bonsai" / "training_export"

    if not qa_dir.exists():
        emit(logger, "skip_qa", reason="directory_not_found")
        return examples

    qa_files = list(qa_dir.glob("*qa*.jsonl"))
    qa_files.extend(qa_dir.glob("*academy*.jsonl"))
    qa_files.extend(qa_dir.glob("*faq*.jsonl"))

    count = 0
    for path in qa_files:
        file_examples = load_jsonl(path, "qa", min_quality=0.70, limit=limit - count if limit else None)
        examples.extend(file_examples)
        count += len(file_examples)
        if limit and count >= limit:
            break

    emit(logger, "load_qa", count=len(examples))
    return examples


# ── Domain Weighting ─────────────────────────────────────────────────────────

def apply_domain_weights(
    all_examples: List[Dict],
    domain_weights: Dict[str, float],
    max_total: int,
    logger: logging.Logger,
) -> List[Dict]:
    """
    Apply domain weighting to create balanced dataset.

    domain_weights: {domain_name: fraction}  (must sum to 1.0)
    """
    # Group by domain
    by_domain = {}
    for ex in all_examples:
        domain = ex.get("domain", "general")
        if domain not in by_domain:
            by_domain[domain] = []
        by_domain[domain].append(ex)

    # Compute target sizes per domain
    target_per_domain = {}
    for domain, weight in domain_weights.items():
        target = int(max_total * weight)
        target_per_domain[domain] = target
        if domain not in by_domain:
            by_domain[domain] = []

    # Sample from each domain
    weighted_examples = []
    for domain, target in target_per_domain.items():
        candidates = by_domain.get(domain, [])
        sampled = candidates[: min(len(candidates), target)]
        weighted_examples.extend(sampled)
        emit(logger, "domain_sample", domain=domain, target=target, actual=len(sampled))

    return weighted_examples


# ── Main ──────────────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(description="Export training data for mobile distillation")

    parser.add_argument("--output", required=True,
                        help="Output JSONL file path")
    parser.add_argument("--max-examples", type=int, default=100_000,
                        help="Maximum examples to export")
    parser.add_argument("--min-quality", type=float, default=0.70,
                        help="Minimum quality score (0.0-1.0)")
    parser.add_argument("--domain-weights", default="code:0.4,survival:0.2,tool_use:0.2,chat:0.1,qa:0.1",
                        help="Domain weight distribution (comma-separated, e.g. code:0.4,survival:0.2)")

    args = parser.parse_args()

    # ── Setup ────────────────────────────────────────────────────────────────

    logger = setup_logging()
    emit(logger, "start", task="export_mobile_training_data")

    output_path = Path(args.output)
    output_path.parent.mkdir(parents=True, exist_ok=True)

    # Parse domain weights
    domain_weights = {}
    total_weight = 0.0
    for spec in args.domain_weights.split(","):
        domain, weight = spec.strip().split(":")
        domain_weights[domain.strip()] = float(weight)
        total_weight += float(weight)

    if abs(total_weight - 1.0) > 0.01:
        logger.warning(f"WARNING: Domain weights sum to {total_weight}, not 1.0")

    emit(logger, "config",
         max_examples=args.max_examples,
         min_quality=args.min_quality,
         domains=",".join(domain_weights.keys()))

    # ── Load data ────────────────────────────────────────────────────────────

    all_examples = []

    # Load each domain
    all_examples.extend(load_survival_fixes(logger))
    all_examples.extend(load_code_examples(logger))
    all_examples.extend(load_chat_conversations(logger))
    all_examples.extend(load_tool_use_examples(logger))
    all_examples.extend(load_qa_pairs(logger))

    emit(logger, "data_loaded_total", count=len(all_examples))

    if len(all_examples) == 0:
        logger.error("ERROR: No training examples loaded!")
        sys.exit(1)

    # ── Apply domain weighting ───────────────────────────────────────────────

    weighted_examples = apply_domain_weights(
        all_examples,
        domain_weights,
        args.max_examples,
        logger,
    )

    # Shuffle for better convergence
    import random
    random.seed(42)
    random.shuffle(weighted_examples)

    emit(logger, "data_weighted", count=len(weighted_examples))

    # ── Write output ─────────────────────────────────────────────────────────

    with open(output_path, "w", encoding="utf-8") as f:
        for ex in weighted_examples:
            f.write(json.dumps(ex) + "\n")

    emit(logger, "export_complete",
         output=str(output_path),
         total_examples=len(weighted_examples),
         size_mb=f"{output_path.stat().st_size / (1024**2):.1f}")

    # ── Summary ──────────────────────────────────────────────────────────────

    domain_counts = {}
    for ex in weighted_examples:
        domain = ex.get("domain", "unknown")
        domain_counts[domain] = domain_counts.get(domain, 0) + 1

    logger.info("=" * 80)
    logger.info("EXPORT COMPLETE")
    logger.info(f"Output: {output_path}")
    logger.info(f"Total examples: {len(weighted_examples)}")
    logger.info("Domain breakdown:")
    for domain, count in sorted(domain_counts.items()):
        pct = 100 * count / len(weighted_examples)
        logger.info(f"  {domain:15s} {count:7d} ({pct:5.1f}%)")
    logger.info("=" * 80)


if __name__ == "__main__":
    main()
