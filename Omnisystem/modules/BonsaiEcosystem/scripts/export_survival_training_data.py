#!/usr/bin/env python3
"""
Export the Survival KB as AI/ML training data.

Formats:
  --format sft       Chat-format JSONL for supervised fine-tuning (default)
  --format dpo       DPO pairs: chosen=correct fix, rejected=no-fix response
  --format instruct  Instruction-tuning JSONL
  --format all       Export all three formats

Usage:
    python scripts/export_survival_training_data.py
    python scripts/export_survival_training_data.py --format dpo --out training/
    python scripts/export_survival_training_data.py --category rust --min-confidence 0.8
"""
import argparse
import json
import sqlite3
from datetime import datetime
from pathlib import Path

DB_DEFAULT = Path.home() / ".bonsai" / "survival_kb.db"
OUT_DEFAULT = Path.home() / ".bonsai" / "training_export"

SYSTEM_PROMPT = (
    "You are BonsAI, an expert debugging and software engineering assistant for the "
    "Bonsai Ecosystem (Rust + Tauri + Svelte). Given an error message, log output, "
    "or problem description, provide the exact fix or solution. Be concise and precise."
)


def load_entries(conn: sqlite3.Connection, category: str | None, min_confidence: float) -> list[dict]:
    query = "SELECT error_pattern, solution_script, category, tags, confidence, success_count FROM fixes WHERE confidence >= ?"
    params = [min_confidence]
    if category:
        query += " AND category = ?"
        params.append(category)
    query += " ORDER BY success_count DESC, confidence DESC"
    rows = conn.execute(query, params).fetchall()
    return [
        {"pattern": r[0], "script": r[1], "category": r[2],
         "tags": r[3], "confidence": r[4], "success_count": r[5]}
        for r in rows
    ]


def to_sft(entry: dict) -> dict:
    return {
        "messages": [
            {"role": "system", "content": f"{SYSTEM_PROMPT} Category: {entry['category']}."},
            {"role": "user", "content": entry["pattern"]},
            {"role": "assistant", "content": entry["script"]},
        ],
        "metadata": {
            "category": entry["category"],
            "tags": entry["tags"],
            "confidence": entry["confidence"],
            "success_count": entry["success_count"],
            "source": "bonsai_survival_kb",
        }
    }


def to_dpo(entry: dict) -> dict:
    return {
        "prompt": f"Error: {entry['pattern']}\n\nWhat is the fix?",
        "chosen": entry["script"],
        "rejected": "I cannot determine the fix. Please consult the documentation.",
        "metadata": {
            "category": entry["category"],
            "confidence": entry["confidence"],
            "source": "bonsai_survival_kb",
        }
    }


def to_instruct(entry: dict) -> dict:
    return {
        "instruction": f"Fix the following {entry['category']} error in the Bonsai Ecosystem:",
        "input": entry["pattern"],
        "output": entry["script"],
        "metadata": {
            "category": entry["category"],
            "tags": entry["tags"],
        }
    }


def export_format(entries: list[dict], fmt: str, out_dir: Path) -> Path:
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    out_path = out_dir / f"survival_kb_{fmt}_{timestamp}.jsonl"
    out_dir.mkdir(parents=True, exist_ok=True)

    converters = {"sft": to_sft, "dpo": to_dpo, "instruct": to_instruct}
    converter = converters[fmt]

    with open(out_path, "w", encoding="utf-8") as f:
        for entry in entries:
            f.write(json.dumps(converter(entry), ensure_ascii=False) + "\n")

    return out_path


def main():
    parser = argparse.ArgumentParser(description="Export Survival KB as training data")
    parser.add_argument("--db", default=str(DB_DEFAULT), help="Path to survival_kb.db")
    parser.add_argument("--out", default=str(OUT_DEFAULT), help="Output directory")
    parser.add_argument("--format", default="sft", choices=["sft", "dpo", "instruct", "all"])
    parser.add_argument("--category", default=None, help="Filter by category (rust/svelte/build/training/runtime)")
    parser.add_argument("--min-confidence", type=float, default=0.5, help="Minimum confidence threshold")
    parser.add_argument("--stats", action="store_true", help="Print statistics only")
    args = parser.parse_args()

    db_path = Path(args.db).expanduser()
    if not db_path.exists():
        print(f"DB not found: {db_path}. Run: python scripts/import_historical_errors.py first.")
        return

    conn = sqlite3.connect(str(db_path))
    entries = load_entries(conn, args.category, args.min_confidence)

    if args.stats:
        cats = {}
        for e in entries:
            cats[e["category"]] = cats.get(e["category"], 0) + 1
        print(f"Total entries: {len(entries)}")
        for cat, count in sorted(cats.items(), key=lambda x: -x[1]):
            print(f"  {cat:20s}: {count}")
        conn.close()
        return

    out_dir = Path(args.out).expanduser()
    formats = ["sft", "dpo", "instruct"] if args.format == "all" else [args.format]
    exported = []
    for fmt in formats:
        path = export_format(entries, fmt, out_dir)
        exported.append((fmt, path))
        print(f"Exported {len(entries)} entries ({fmt}) → {path}")

    conn.close()

    print(f"\nReady for training:")
    for fmt, path in exported:
        if fmt == "dpo":
            print(f"  DPO:     python scripts/dpo_train.py --dataset {path}")
        elif fmt == "sft":
            print(f"  SFT:     python scripts/finetune_sft.py --dataset {path}")
        elif fmt == "instruct":
            print(f"  Instruct: python scripts/finetune_sft.py --dataset {path} --format instruct")


if __name__ == "__main__":
    main()
