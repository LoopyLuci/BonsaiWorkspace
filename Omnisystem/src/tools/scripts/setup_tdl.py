#!/usr/bin/env python3
"""Initialize the Training Data Library for Poe AI empathy training."""
import sqlite3
import os
from pathlib import Path

TDL_PATH = Path(os.environ.get("POE_TDL_PATH", "./poe_tdl.db"))

def setup_tdl():
    conn = sqlite3.connect(TDL_PATH)
    cursor = conn.cursor()

    cursor.executescript("""
        CREATE TABLE IF NOT EXISTS datasets (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            version TEXT NOT NULL,
            description TEXT,
            num_examples INTEGER DEFAULT 0,
            quality_score REAL DEFAULT 0.0,
            created_at TEXT DEFAULT (datetime('now')),
            tags TEXT
        );

        CREATE TABLE IF NOT EXISTS examples (
            id TEXT PRIMARY KEY,
            dataset_id TEXT NOT NULL,
            prompt TEXT NOT NULL,
            response TEXT NOT NULL,
            quality_score REAL DEFAULT 0.8,
            metadata TEXT,
            created_at TEXT DEFAULT (datetime('now')),
            FOREIGN KEY (dataset_id) REFERENCES datasets(id)
        );

        CREATE TABLE IF NOT EXISTS training_runs (
            id TEXT PRIMARY KEY,
            stage TEXT NOT NULL,
            config TEXT,
            started_at TEXT DEFAULT (datetime('now')),
            completed_at TEXT,
            status TEXT DEFAULT 'pending',
            metrics TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_examples_dataset ON examples(dataset_id);
        CREATE INDEX IF NOT EXISTS idx_examples_quality ON examples(quality_score);
        CREATE INDEX IF NOT EXISTS idx_training_runs_status ON training_runs(status);
    """)

    # Register base datasets
    datasets = [
        ("ds-empathetic-dialogues-v1", "Empathetic Dialogues", "1.0",
         "500k empathetic conversation pairs from therapy and companionship contexts", 0.85,
         "empathy,dialogue,companionship"),
        ("ds-biometric-affect-v1", "Biometric Affect Mappings", "1.0",
         "200k pairs of biometric telemetry → appropriate emotional response", 0.82,
         "biometric,affect,telemetry"),
        ("ds-constitutional-safety-v1", "Constitutional Safety Examples", "1.0",
         "50k safety principle demonstrations with chosen/rejected pairs", 0.95,
         "safety,constitutional,dpo"),
        ("ds-ac-poe-style-v1", "AC Poe Style Corpus", "1.0",
         "10k in-character dialogues from Altered Carbon analysis", 0.78,
         "ac-poe,style,gothic"),
        ("ds-fallback-scenarios-v1", "Fallback & Degradation Scenarios", "1.0",
         "25k scenarios of network loss, jamming, low power, hardware faults", 0.80,
         "fallback,degradation,survival"),
    ]

    for ds in datasets:
        cursor.execute(
            "INSERT OR IGNORE INTO datasets (id, name, version, description, quality_score, tags) VALUES (?,?,?,?,?,?)",
            ds
        )

    conn.commit()
    conn.close()
    print(f"✅ TDL initialized at {TDL_PATH}")

if __name__ == "__main__":
    setup_tdl()
