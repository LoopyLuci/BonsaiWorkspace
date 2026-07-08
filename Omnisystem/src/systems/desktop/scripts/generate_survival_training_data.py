"""
generate_survival_training_data.py

Exports the Bonsai Survival Knowledge Base to JSONL format suitable for
fine-tuning a language model (llama.cpp format: instruction-following pairs).

Usage:
    python scripts/generate_survival_training_data.py \
        [--db ~/.bonsai/survival_kb.db] \
        [--output survival_training.jsonl] \
        [--min-success 1]

The output can be fed directly into the existing BonsAI training pipeline:
    python scripts/finetune_lora.py --data survival_training.jsonl
"""

import argparse
import json
import os
import sqlite3
import sys


SYSTEM_PROMPT = (
    "You are an expert at diagnosing and fixing the Bonsai AI application. "
    "Given an error log or symptom description, output a single shell command "
    "that will fix the problem. If you cannot determine a safe fix, output "
    "NOT_FIXABLE. Never suggest destructive commands like 'rm -rf /'."
)


def export_training_data(
    db_path: str,
    output_file: str,
    min_success: int = 1,
    include_unverified: bool = False,
) -> int:
    if not os.path.exists(db_path):
        print(f"[warn] KB not found at {db_path} — no training data exported", file=sys.stderr)
        return 0

    conn = sqlite3.connect(db_path)
    cursor = conn.cursor()

    query = "SELECT error_pattern, solution_script, solution_type, success_count, confidence FROM fixes"
    conditions = []
    if min_success > 0:
        conditions.append(f"success_count >= {min_success}")
    if not include_unverified:
        conditions.append("(verified = 1 OR solution_type = 'rule')")
    if conditions:
        query += " WHERE " + " AND ".join(conditions)
    query += " ORDER BY success_count DESC, confidence DESC"

    cursor.execute(query)
    rows = cursor.fetchall()
    conn.close()

    examples = []
    for error_pattern, solution_script, sol_type, success, confidence in rows:
        entry = {
            "messages": [
                {"role": "system",    "content": SYSTEM_PROMPT},
                {"role": "user",      "content": error_pattern},
                {"role": "assistant", "content": solution_script},
            ],
            # Metadata fields (stripped before sending to trainer, kept for inspection)
            "_meta": {
                "type":       sol_type,
                "success":    success,
                "confidence": confidence,
            },
        }
        examples.append(entry)

    with open(output_file, "w", encoding="utf-8") as f:
        for ex in examples:
            f.write(json.dumps(ex, ensure_ascii=False) + "\n")

    print(f"[survival-export] Exported {len(examples)} examples → {output_file}")
    return len(examples)


def main():
    default_db = os.path.join(os.path.expanduser("~"), ".bonsai", "survival_kb.db")

    parser = argparse.ArgumentParser(description="Export Bonsai Survival KB to JSONL")
    parser.add_argument("--db",          default=default_db,           help="Path to survival_kb.db")
    parser.add_argument("--output",      default="survival_training.jsonl", help="Output JSONL path")
    parser.add_argument("--min-success", type=int, default=1,          help="Min successful uses to include")
    parser.add_argument("--include-unverified", action="store_true",   help="Include unverified AI fixes")
    args = parser.parse_args()

    count = export_training_data(
        db_path=args.db,
        output_file=args.output,
        min_success=args.min_success,
        include_unverified=args.include_unverified,
    )
    sys.exit(0 if count >= 0 else 1)


if __name__ == "__main__":
    main()
