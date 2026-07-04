#!/usr/bin/env python3
"""
Import historical errors from JSONL or git history into the Survival KB.

Usage:
    python scripts/import_historical_errors.py
    python scripts/import_historical_errors.py --db ~/.bonsai/survival_kb.db
    python scripts/import_historical_errors.py --from-git   # scan git log for fix: commits
"""
import argparse
import json
import os
import re
import sqlite3
import subprocess
import sys
from datetime import datetime
from pathlib import Path

DB_DEFAULT = Path.home() / ".bonsai" / "survival_kb.db"

SCHEMA = """
PRAGMA journal_mode=WAL;
CREATE TABLE IF NOT EXISTS fixes (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    error_pattern   TEXT    NOT NULL,
    solution_type   TEXT    NOT NULL DEFAULT 'rule',
    solution_script TEXT    NOT NULL,
    confidence      REAL    NOT NULL DEFAULT 0.5,
    usage_count     INTEGER NOT NULL DEFAULT 0,
    success_count   INTEGER NOT NULL DEFAULT 0,
    created_by      TEXT    NOT NULL DEFAULT 'system',
    verified        INTEGER NOT NULL DEFAULT 0,
    category        TEXT    NOT NULL DEFAULT 'other',
    tags            TEXT    NOT NULL DEFAULT '',
    created_at      DATETIME DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX IF NOT EXISTS idx_fixes_pattern  ON fixes(error_pattern);
CREATE INDEX IF NOT EXISTS idx_fixes_category ON fixes(category);
"""

HISTORICAL = [
    # Svelte / TypeScript
    ("Type cast 'as' not valid in Svelte template",
     "Move TypeScript 'as' cast to a helper function in <script lang=\"ts\">.",
     "svelte", "svelte,typescript,type-assertion", 1.0),
    ("onMount return type Promise not assignable to void",
     "Remove async from onMount or cast: (onMount as any)(async () => { ...; return cleanup; })",
     "svelte", "svelte,async,lifecycle", 1.0),
    ("e.target as HTMLInputElement in Svelte template",
     "Define helper: function inputValue(e: Event): string { return (e.target as HTMLInputElement)?.value ?? ''; }",
     "svelte", "svelte,dom,type-assertion", 1.0),
    ("as Record<string, string> in template expression",
     "Move cast to a helper function in <script>: function lookup(m: Record<string,string>, k: string) { return m[k]; }",
     "svelte", "svelte,typescript,template", 1.0),
    ("JSDoc cast Expected } parse error in each block",
     "JSDoc comments not supported in Svelte template. Move to typed const in <script>.",
     "svelte", "svelte,jsdoc,syntax", 1.0),
    ("A11y div with click handler must have ARIA role",
     "Add role=\"button\" tabindex=\"0\" on:keydown to clickable div, or use <button>.",
     "svelte", "svelte,a11y,accessibility", 1.0),
    # Rust
    ("raw string terminates at backslash-quote",
     "In Rust raw strings, backslash has no special meaning. Use r#\"...\"# to include double-quotes, or use [\"'] instead of [\\'\\\"]. Example: change r\"['\\\"]\" to r#\"[\"']\"#",
     "rust", "rust,regex,raw-string", 1.0),
    ("error[E0432]: unresolved import log",
     "Replace log::* with tracing::*. Add tracing = \"0.1\" to Cargo.toml.",
     "rust", "rust,logging,tracing", 1.0),
    ("error[E0252]: name defined multiple times",
     "Remove duplicate 'use' statement. Check for glob imports that may re-import the same symbol.",
     "rust", "rust,imports,duplicate", 1.0),
    ("error[E0308]: expected Option found Result try_state",
     "app_handle.try_state::<T>() returns Option<State<T>>, not Result. Use if let Some(s) = try_state::<T>().",
     "rust", "rust,tauri,option-result", 1.0),
    ("error[E0433]: cannot find module dirs",
     "Add dirs = \"5\" to Cargo.toml [dependencies].",
     "rust", "rust,dependencies,dirs", 1.0),
    ("non-exhaustive patterns in match SystemEvent",
     "Add all new SystemEvent variants to the match. Add missing variants to the non-recording arm.",
     "rust", "rust,match,exhaustive", 1.0),
    ("error[E0004] non-exhaustive patterns",
     "Add the missing enum variants to the match expression, or add a catch-all arm: _ => {} .",
     "rust", "rust,match,exhaustive", 1.0),
    ("cannot find macro log in this scope",
     "Replace log::info!/warn!/error! with tracing::info!/warn!/error!",
     "rust", "rust,logging,macro", 1.0),
    ("unsafe raw pointer mutation of Arc struct field",
     "Use Arc<RwLock<T>> for mutable shared state instead of raw pointer casting.",
     "rust", "rust,unsafe,arc", 0.95),
    ("max_agents field not found in SwarmSpec",
     "SwarmSpec uses max_workers (u32) not max_agents. Check struct definition before using field names suggested by AI agents.",
     "rust", "rust,struct,field-name", 1.0),
    # Build / Environment
    ("cargo not recognized in bash",
     "Add $HOME/.cargo/bin to PATH: export PATH=\"$PATH:$HOME/.cargo/bin\"",
     "build", "build,cargo,path", 1.0),
    ("LLAMA_CPP_PATH not set bonsai-native",
     "Set LLAMA_CPP_PATH env var to the llama.cpp build dir, or set BONSAI_NATIVE_SKIP=1.",
     "build", "build,native,llama", 0.95),
    ("rusqlite 0.31 conflicts with sqlx libsqlite3-sys",
     "Use rusqlite = { version = \"0.32\", features = [\"bundled\"] } to avoid the conflict.",
     "build", "build,rusqlite,sqlx", 1.0),
    ("PowerShell += not recognized in bash tool",
     "Use bash syntax in Bash tool: export PATH=\"$PATH:...\". For PowerShell, use the PowerShell tool.",
     "build", "build,powershell,shell", 1.0),
    # Runtime / Training
    ("STATUS_ACCESS_VIOLATION dpo_train.py Windows",
     "Use --max-length 128 (or 256). Windows CPU allocator cannot handle large tensor sizes.",
     "training", "training,windows,pytorch,oom", 1.0),
    ("segfault exit 139 after pairs loaded dpo_train.py",
     "Use --max-length 128 for CPU training. Two model copies × max_length=512 exceeds allocatable memory.",
     "training", "training,segfault,pytorch", 1.0),
    ("HF hub connection timeout",
     "Set HF_HUB_OFFLINE=1, TRANSFORMERS_OFFLINE=1. Pass local_files_only=True to from_pretrained().",
     "training", "training,huggingface,network", 1.0),
    ("GPU OOM during training",
     "Reduce n_gpu_layers, batch_size, and max_length. Use gradient_checkpointing=True.",
     "training", "training,gpu,oom", 1.0),
    ("port 11369 already in use",
     "Kill zombie daemon: pkill bonsai-workspace (Linux) or Stop-Process -Name bonsai-workspace (PowerShell).",
     "runtime", "runtime,port,daemon", 1.0),
    ("panicked at index out of bounds",
     "Use .get(idx) instead of [idx] and handle None. Add input validation before indexing.",
     "runtime", "runtime,rust,panic,bounds", 1.0),
    # Universe / Time Travel
    ("SystemEvent match non-exhaustive in universe_hooks",
     "Add new SystemEvent variants to the non-recording arm in universe_hooks::convert_system_event().",
     "rust", "rust,universe,match", 1.0),
    ("tokio-rusqlite Error mapping",
     "Map rusqlite errors inside call() closures with .map_err(|e| tokio_rusqlite::Error::Rusqlite(e)).",
     "rust", "rust,sqlite,tokio-rusqlite", 1.0),
    # Survival System
    ("AI-generated script rejected by safety gate",
     "Review the safety gate allowlist in survival.rs::is_safe_script(). Add specific patterns after human review.",
     "runtime", "runtime,survival,safety", 0.9),
]


def ensure_schema(conn: sqlite3.Connection) -> None:
    conn.executescript(SCHEMA)
    conn.commit()


def import_entry(conn: sqlite3.Connection, pattern: str, script: str, category: str, tags: str, confidence: float) -> bool:
    existing = conn.execute(
        "SELECT COUNT(*) FROM fixes WHERE error_pattern = ?", (pattern,)
    ).fetchone()[0]
    if existing:
        return False
    conn.execute(
        "INSERT INTO fixes (error_pattern, solution_type, solution_script, confidence, created_by, category, tags, verified) "
        "VALUES (?, 'historical', ?, ?, 'import_script', ?, ?, 1)",
        (pattern, script, confidence, category, tags),
    )
    return True


def import_from_git(conn: sqlite3.Connection, workspace_root: str) -> int:
    """Scan git log for fix: commits and import error→fix pairs."""
    try:
        result = subprocess.run(
            ["git", "log", "--all", "--format=%H %s", "--grep=^fix:"],
            cwd=workspace_root, capture_output=True, text=True, timeout=30
        )
    except (subprocess.TimeoutExpired, FileNotFoundError):
        print("  Warning: git not available or timed out.")
        return 0

    added = 0
    for line in result.stdout.strip().splitlines():
        parts = line.split(" ", 1)
        if len(parts) < 2:
            continue
        commit_sha, subject = parts
        pattern = re.sub(r"^fix:\s*", "", subject).strip()
        if not pattern or len(pattern) < 10:
            continue
        body_result = subprocess.run(
            ["git", "log", "-1", "--format=%b", commit_sha],
            cwd=workspace_root, capture_output=True, text=True, timeout=10
        )
        script = (body_result.stdout.strip().split("\n")[0] if body_result.stdout.strip() else f"See commit {commit_sha}")
        if import_entry(conn, pattern[:200], script[:500], "git_history", "git,fix,historical", 0.8):
            added += 1

    return added


def main():
    parser = argparse.ArgumentParser(description="Import historical errors into Survival KB")
    parser.add_argument("--db", default=str(DB_DEFAULT), help="Path to survival_kb.db")
    parser.add_argument("--from-git", action="store_true", help="Also import from git log")
    parser.add_argument("--workspace", default=".", help="Workspace root for git log")
    parser.add_argument("--dry-run", action="store_true", help="Show what would be imported without writing")
    args = parser.parse_args()

    db_path = Path(args.db).expanduser()
    db_path.parent.mkdir(parents=True, exist_ok=True)

    if args.dry_run:
        print(f"[dry-run] Would import {len(HISTORICAL)} hardcoded entries into {db_path}")
        for pattern, *_ in HISTORICAL:
            print(f"  - {pattern[:80]}")
        return

    conn = sqlite3.connect(str(db_path))
    ensure_schema(conn)

    # Import hardcoded historical entries
    added = 0
    skipped = 0
    for pattern, script, category, tags, confidence in HISTORICAL:
        if import_entry(conn, pattern, script, category, tags, confidence):
            added += 1
        else:
            skipped += 1
    conn.commit()
    print(f"Hardcoded entries: {added} added, {skipped} already present.")

    # Import from git log
    if args.from_git:
        git_added = import_from_git(conn, args.workspace)
        conn.commit()
        print(f"Git history: {git_added} entries added.")

    total = conn.execute("SELECT COUNT(*) FROM fixes").fetchone()[0]
    print(f"\nTotal rules in KB: {total}")
    conn.close()


if __name__ == "__main__":
    main()
