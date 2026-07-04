"""Import survival fixes registry into the survival knowledge base.

Reads scripts/survival_fixes_registry.json and inserts each entry into
~/.bonsai/survival_kb.db (the same database the Bonsai watchdog uses).

Idempotent: skips entries whose error_pattern already exists in the DB.

Usage:
    python scripts/import_survival_fixes.py [--db PATH]
"""

import argparse
import json
import sqlite3
from pathlib import Path


SCHEMA = """
CREATE TABLE IF NOT EXISTS fixes (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    error_pattern   TEXT    NOT NULL,
    solution_type   TEXT    NOT NULL DEFAULT 'rule',
    solution_script TEXT    NOT NULL,
    confidence      REAL    NOT NULL DEFAULT 0.95,
    usage_count     INTEGER NOT NULL DEFAULT 0,
    success_count   INTEGER NOT NULL DEFAULT 0,
    created_by      TEXT    NOT NULL DEFAULT 'system',
    verified        INTEGER NOT NULL DEFAULT 1
);
"""

REGISTRY = Path(__file__).parent / "survival_fixes_registry.json"
DEFAULT_DB = Path.home() / ".bonsai" / "survival_kb.db"


def import_fixes(db_path: Path) -> None:
    db_path.parent.mkdir(parents=True, exist_ok=True)

    with sqlite3.connect(db_path) as conn:
        conn.executescript(SCHEMA)

        fixes = json.loads(REGISTRY.read_text(encoding="utf-8"))
        inserted = 0
        skipped = 0

        for fix in fixes:
            pattern = fix["error_pattern"]
            existing = conn.execute(
                "SELECT id FROM fixes WHERE error_pattern = ?", (pattern,)
            ).fetchone()

            if existing:
                skipped += 1
                continue

            conn.execute(
                """INSERT INTO fixes
                   (error_pattern, solution_type, solution_script,
                    confidence, created_by, verified)
                   VALUES (?, ?, ?, 0.95, 'system', 1)""",
                (pattern, fix.get("solution_type", "rule"), fix["solution_script"]),
            )
            inserted += 1

        conn.commit()

    print(f"Survival KB updated: {inserted} inserted, {skipped} already present.")
    print(f"Database: {db_path}")


def main() -> None:
    parser = argparse.ArgumentParser(description="Import survival fixes into BonsAI KB")
    parser.add_argument("--db", default=str(DEFAULT_DB), help="Path to survival_kb.db")
    args = parser.parse_args()
    import_fixes(Path(args.db))


if __name__ == "__main__":
    main()
