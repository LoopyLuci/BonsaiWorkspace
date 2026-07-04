"""Record a completed training phase into ~/.bonsai/brain_metadata.json.

Called automatically after DPO/SFT training completes.

Usage:
    python scripts/record_training_phase.py --phase safety
    python scripts/record_training_phase.py --phase tool_use
"""
import argparse
import json
from datetime import datetime, timezone
from pathlib import Path

META_PATH = Path.home() / ".bonsai" / "brain_metadata.json"

VALID_PHASES = [
    "safety", "survival", "tool_use", "code",
    "chat", "reason", "final", "convert",
]


def load_meta() -> dict:
    if META_PATH.exists():
        try:
            return json.loads(META_PATH.read_text(encoding="utf-8"))
        except Exception:
            pass
    return {"lessons_completed": 0, "phases_done": [], "last_training": None}


def save_meta(meta: dict) -> None:
    META_PATH.parent.mkdir(parents=True, exist_ok=True)
    META_PATH.write_text(json.dumps(meta, indent=2), encoding="utf-8")


def main() -> None:
    parser = argparse.ArgumentParser(description="Record a completed BonsAI training phase")
    parser.add_argument("--phase", required=True, choices=VALID_PHASES,
                        help="Phase key that was just completed")
    args = parser.parse_args()

    meta = load_meta()
    if args.phase not in meta.get("phases_done", []):
        meta.setdefault("phases_done", []).append(args.phase)
        meta["lessons_completed"] = meta.get("lessons_completed", 0) + 1
    meta["last_training"] = datetime.now(timezone.utc).isoformat()
    save_meta(meta)

    phase_count = len(meta["phases_done"])
    print(f"[brain] Phase '{args.phase}' recorded. "
          f"Total phases done: {phase_count}/8  "
          f"Lessons: {meta['lessons_completed']}")
    print(f"[brain] Metadata saved to {META_PATH}")


if __name__ == "__main__":
    main()
