-- ETL Feedback Events Table
CREATE TABLE IF NOT EXISTS feedback_events (
    event_id TEXT PRIMARY KEY,
    event_type TEXT NOT NULL,
    rule_id TEXT NOT NULL,
    file TEXT NOT NULL,
    line INTEGER NOT NULL,
    timestamp DATETIME NOT NULL,
    user_id TEXT NOT NULL,
    action TEXT,
    outcome TEXT,
    explanation TEXT,
    dismissal_count INTEGER
);

CREATE INDEX IF NOT EXISTS idx_feedback_timestamp ON feedback_events(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_feedback_rule_id ON feedback_events(rule_id);
CREATE INDEX IF NOT EXISTS idx_feedback_user_id ON feedback_events(user_id);

-- Rule Metrics Table
CREATE TABLE IF NOT EXISTS rule_metrics (
    rule_id TEXT PRIMARY KEY,
    true_positives INTEGER NOT NULL DEFAULT 0,
    false_positives INTEGER NOT NULL DEFAULT 0,
    dismissed_count INTEGER NOT NULL DEFAULT 0,
    applied_fixes INTEGER NOT NULL DEFAULT 0,
    fix_success_rate REAL NOT NULL DEFAULT 1.0,
    last_updated DATETIME NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_metrics_last_updated ON rule_metrics(last_updated DESC);

-- ETL Cycle History Table
CREATE TABLE IF NOT EXISTS etl_cycles (
    cycle_id TEXT PRIMARY KEY,
    started_at DATETIME NOT NULL,
    completed_at DATETIME,
    feedback_events_processed INTEGER,
    rules_analyzed INTEGER,
    confidence_updates_applied INTEGER,
    refinement_proposals INTEGER,
    duration_ms INTEGER,
    status TEXT NOT NULL DEFAULT 'running'
);

CREATE INDEX IF NOT EXISTS idx_cycles_completed_at ON etl_cycles(completed_at DESC);
