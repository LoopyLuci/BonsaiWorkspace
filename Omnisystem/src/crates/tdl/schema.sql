-- Bonsai Training Data Library (TDL) Schema
-- Version: 1.0
-- SQLite database schema for versioned training dataset management

-- Table: versions
-- Stores metadata about training dataset versions
CREATE TABLE IF NOT EXISTS versions (
    id TEXT PRIMARY KEY,
    version_string TEXT NOT NULL UNIQUE,
    example_count INTEGER NOT NULL DEFAULT 0,
    total_size_bytes INTEGER NOT NULL DEFAULT 0,
    created_by TEXT NOT NULL,
    description TEXT NOT NULL,
    created_at TEXT NOT NULL,
    tags TEXT NOT NULL DEFAULT '[]',
    avg_quality_score REAL NOT NULL DEFAULT 0.0,
    version_hash TEXT NOT NULL,
    created_at_timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Table: examples
-- Individual training examples with metadata
CREATE TABLE IF NOT EXISTS examples (
    id TEXT PRIMARY KEY,
    version_id TEXT NOT NULL,
    content TEXT NOT NULL,
    metadata TEXT NOT NULL,
    quality_score REAL NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    content_hash TEXT NOT NULL UNIQUE,
    content_size_bytes INTEGER NOT NULL,
    FOREIGN KEY (version_id) REFERENCES versions(id) ON DELETE CASCADE
);

-- Table: version_examples
-- Junction table for many-to-many relationship between versions and examples
CREATE TABLE IF NOT EXISTS version_examples (
    version_id TEXT NOT NULL,
    example_id TEXT NOT NULL,
    PRIMARY KEY (version_id, example_id),
    FOREIGN KEY (version_id) REFERENCES versions(id) ON DELETE CASCADE,
    FOREIGN KEY (example_id) REFERENCES examples(id) ON DELETE CASCADE
);

-- Table: datasets
-- Exported datasets (JSONL, Parquet, etc.)
CREATE TABLE IF NOT EXISTS datasets (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL,
    version_id TEXT NOT NULL,
    format TEXT NOT NULL,
    file_path TEXT NOT NULL,
    created_at TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    checksum TEXT NOT NULL,
    FOREIGN KEY (version_id) REFERENCES versions(id) ON DELETE CASCADE
);

-- Indexes for performance

-- Examples lookup by version
CREATE INDEX IF NOT EXISTS idx_examples_version_id ON examples(version_id);

-- Quality score range queries
CREATE INDEX IF NOT EXISTS idx_examples_quality ON examples(quality_score DESC);

-- Content deduplication
CREATE INDEX IF NOT EXISTS idx_examples_content_hash ON examples(content_hash);

-- Dataset lookups by version
CREATE INDEX IF NOT EXISTS idx_datasets_version_id ON datasets(version_id);

-- Version history sorting
CREATE INDEX IF NOT EXISTS idx_versions_created_at ON versions(created_at DESC);

-- Quality score average tracking
CREATE INDEX IF NOT EXISTS idx_versions_avg_quality ON versions(avg_quality_score DESC);

-- Content size tracking
CREATE INDEX IF NOT EXISTS idx_versions_total_size ON versions(total_size_bytes DESC);
