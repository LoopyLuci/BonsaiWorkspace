DB_SCHEMA — Bonsai Workspace

Last updated: 2026-04-26

Overview
- Engine: SQLite (single file DB used by the Tauri backend).
- DB file: `bonsai.db` under the Tauri app data directory (constructed in `src-tauri/src/wal.rs`).
- WAL: several stores enable WAL or use WAL-friendly settings. `memory_store.rs` sets `PRAGMA journal_mode=WAL`. The code uses a write-ahead log table (`wal_events`) for audit/telemetry.
- Migrations: Rust stores either create tables directly at startup or use a `schema_migrations` table and an `apply_migration` pattern. See `assistant_store.rs` and `user_skills.rs` for the migration pattern.

Notes on types/JSON
- Many fields are stored as `TEXT` but contain JSON (convention in this repo). These are noted next to each table where applicable (e.g., `tags_json`, `stats_json`, `payload_json`).

---

Schema (tables, columns, keys, indexes)

1) `schema_migrations`
- Source: created by `assistant_store.rs` and `user_skills.rs` (migration helpers).
- Columns:
  - `version` INTEGER PRIMARY KEY
  - `applied_at` INTEGER NOT NULL
  - `description` TEXT NOT NULL
- Purpose: track applied migration versions; stores add rows after running migration SQL.

2) `wal_events`
- Source: `src-tauri/src/wal.rs`
- Columns:
  - `id` INTEGER PRIMARY KEY AUTOINCREMENT
  - `timestamp` TEXT NOT NULL DEFAULT (datetime('now'))
  - `event_type` TEXT NOT NULL
  - `payload_json` TEXT NOT NULL  -- JSON payload serialized to string
- Indexes:
  - `idx_wal_timestamp` ON `wal_events(timestamp)`
- Purpose: lightweight write-ahead/audit log for events emitted by the Rust side.

3) `memory_records`
- Source: `src-tauri/src/memory_store.rs`
- PRAGMA: `journal_mode=WAL` is set in migration SQL.
- Columns:
  - `id` TEXT PRIMARY KEY
  - `domain` TEXT NOT NULL  -- one of `session`, `working`, `long_term`
  - `session_id` TEXT
  - `profile_id` TEXT NOT NULL
  - `content` TEXT NOT NULL
  - `tags_json` TEXT NOT NULL DEFAULT '[]'  -- JSON array string
  - `score` REAL NOT NULL DEFAULT 0.5
  - `sensitivity` TEXT NOT NULL DEFAULT 'public'
  - `created_at` INTEGER NOT NULL
  - `expires_at` INTEGER
- Indexes:
  - `idx_memory_profile` ON `memory_records(profile_id, domain)`
  - `idx_memory_session` ON `memory_records(session_id)` WHERE session_id IS NOT NULL
  - `idx_memory_expires` ON `memory_records(expires_at)` WHERE expires_at IS NOT NULL
- Auxiliary table:
  - `memory_audit` (id INTEGER PK AUTOINCREMENT, ts INTEGER NOT NULL, action TEXT NOT NULL, record_id TEXT NOT NULL, profile_id TEXT NOT NULL)

4) `assistant_profiles`
- Source: `src-tauri/src/assistant_store.rs`
- Columns:
  - `id` TEXT PRIMARY KEY
  - `name` TEXT NOT NULL DEFAULT 'Bonsai Buddy'
  - `persona_id` TEXT
  - `avatar_id` TEXT
  - `tts_voice` TEXT NOT NULL DEFAULT 'en_US-amy-medium'
  - `tts_speed` REAL NOT NULL DEFAULT 1.0
  - `tts_pitch` REAL NOT NULL DEFAULT 1.0
  - `tts_enabled` INTEGER NOT NULL DEFAULT 1
  - `wake_word` TEXT
  - `tool_permissions` TEXT NOT NULL DEFAULT '{}'  -- typically JSON object as text
  - `system_prompt` TEXT NOT NULL DEFAULT 'You are Bonsai Buddy, a helpful personal AI assistant.'
  - `model_id` TEXT
  - `is_active` INTEGER NOT NULL DEFAULT 0
  - `created_at` INTEGER NOT NULL
  - `updated_at` INTEGER NOT NULL
- Indexes/constraints:
  - `idx_one_active_profile` UNIQUE INDEX ON `assistant_profiles(is_active)` WHERE is_active = 1 (ensures at most one active profile)

5) `avatar_assets`
- Source: `assistant_store.rs`
- Columns:
  - `id` TEXT PRIMARY KEY
  - `name` TEXT NOT NULL
  - `asset_type` TEXT NOT NULL CHECK(asset_type IN ('svg_builtin','svg_custom','photo'))
  - `asset_data` TEXT
  - `file_path` TEXT
  - `thumbnail_svg` TEXT
  - `validated` INTEGER NOT NULL DEFAULT 0
  - `created_at` INTEGER NOT NULL
  - `updated_at` INTEGER NOT NULL

6) `assistant_sessions`
- Source: `assistant_store.rs`
- Columns:
  - `id` TEXT PRIMARY KEY
  - `profile_id` TEXT
  - `title` TEXT NOT NULL DEFAULT 'New conversation'
  - `created_at` INTEGER NOT NULL
  - `updated_at` INTEGER NOT NULL
- Constraints:
  - `FOREIGN KEY(profile_id) REFERENCES assistant_profiles(id) ON DELETE CASCADE`
- Indexes:
  - `idx_asst_sessions_profile` ON `assistant_sessions(profile_id, updated_at DESC)`

7) `assistant_messages`
- Source: `assistant_store.rs`
- Columns:
  - `id` TEXT PRIMARY KEY
  - `session_id` TEXT NOT NULL
  - `role` TEXT NOT NULL CHECK(role IN ('user','assistant','tool'))
  - `content` TEXT NOT NULL
  - `tool_name` TEXT
  - `tool_result` TEXT
  - `tts_synthesized` INTEGER NOT NULL DEFAULT 0
  - `created_at` INTEGER NOT NULL
  - (migration adds) `tool_call_id` TEXT  -- may exist via ALTER TABLE migration
- Constraints:
  - `FOREIGN KEY(session_id) REFERENCES assistant_sessions(id) ON DELETE CASCADE`
- Indexes:
  - `idx_asst_msgs_session` ON `assistant_messages(session_id, created_at)`

8) `backup_registry`
- Source: `assistant_store.rs`
- Columns:
  - `id` TEXT PRIMARY KEY
  - `filename` TEXT NOT NULL
  - `file_path` TEXT NOT NULL
  - `size_bytes` INTEGER NOT NULL
  - `includes` TEXT NOT NULL
  - `checksum` TEXT
  - `encrypted` INTEGER NOT NULL DEFAULT 0
  - `created_at` INTEGER NOT NULL

9) `mcp_servers`
- Source: `assistant_store.rs`
- Columns:
  - `id` TEXT PRIMARY KEY
  - `name` TEXT NOT NULL
  - `command` TEXT NOT NULL
  - `args` TEXT NOT NULL DEFAULT '[]'  -- JSON array string
  - `namespace` TEXT NOT NULL
  - `enabled` INTEGER NOT NULL DEFAULT 1
  - `created_at` INTEGER NOT NULL
  - `updated_at` INTEGER NOT NULL

10) `chat_sessions`
- Source: `src-tauri/src/chat_sessions.rs`
- Columns:
  - `id` TEXT PRIMARY KEY
  - `title` TEXT NOT NULL DEFAULT 'New chat'
  - `workspace_path` TEXT
  - `created_at` INTEGER NOT NULL
  - `updated_at` INTEGER NOT NULL
- Indexes:
  - `idx_sessions_updated` ON `chat_sessions(updated_at DESC)`

11) `session_messages`
- Source: `chat_sessions.rs` (also extended by `agent_store.rs` to add `agent_id`)
- Columns:
  - `id` TEXT PRIMARY KEY
  - `session_id` TEXT NOT NULL REFERENCES chat_sessions(id) ON DELETE CASCADE
  - `role` TEXT NOT NULL
  - `content` TEXT NOT NULL
  - `stats_json` TEXT  -- token stats or similar (JSON as text)
  - `tools_used_json` TEXT
  - `agent_id` TEXT
  - `agent_label` TEXT
  - `agent_color` TEXT
  - `agent_icon` TEXT
  - `agent_slot` INTEGER
  - `created_at` INTEGER NOT NULL
- Indexes:
  - `idx_msgs_session` ON `session_messages(session_id, created_at)`
  - `idx_msgs_agent` created by `agent_store.rs` on `session_messages(agent_id)`

12) `chat_session_groups`
- Source: `chat_sessions.rs`
- Columns:
  - `id` TEXT PRIMARY KEY
  - `title` TEXT NOT NULL DEFAULT 'New session'
  - `tags_json` TEXT NOT NULL DEFAULT '[]'
  - `is_locked` INTEGER NOT NULL DEFAULT 0
  - `is_favorite` INTEGER NOT NULL DEFAULT 0
  - `is_deleted` INTEGER NOT NULL DEFAULT 0
  - `created_at` INTEGER NOT NULL
  - `updated_at` INTEGER NOT NULL
- Indexes:
  - `idx_chat_session_groups_updated` ON `chat_session_groups(updated_at DESC)`

13) `chat_group_links`
- Source: `chat_sessions.rs`
- Columns:
  - `group_id` TEXT NOT NULL REFERENCES chat_session_groups(id) ON DELETE CASCADE
  - `chat_id` TEXT NOT NULL REFERENCES chat_sessions(id) ON DELETE CASCADE
  - `linked_at` INTEGER NOT NULL
  - PRIMARY KEY (group_id, chat_id)
- Indexes:
  - `idx_chat_group_links_chat` ON `chat_group_links(chat_id)`

14) `user_skills`
- Source: `src-tauri/src/user_skills.rs` (created under migration version 100)
- Columns:
  - `id` TEXT PRIMARY KEY
  - `name` TEXT NOT NULL UNIQUE
  - `description` TEXT NOT NULL
  - `kind` TEXT NOT NULL CHECK(kind IN ('shell','sequence'))
  - `body` TEXT NOT NULL
  - `tags` TEXT NOT NULL DEFAULT '[]'  -- JSON array stored as text
  - `enabled` INTEGER NOT NULL DEFAULT 1
  - `created_at` INTEGER NOT NULL
  - `updated_at` INTEGER NOT NULL

15) `personas`, `agent_configs`, `swarm_runs`, `swarm_agent_results`
- Source: `src-tauri/src/agent_store.rs`

`personas`
- Columns:
  - `id` TEXT PRIMARY KEY
  - `name` TEXT NOT NULL
  - `system_prompt` TEXT NOT NULL
  - `model_id` TEXT
  - `color` TEXT NOT NULL DEFAULT '#4a9eff'
  - `icon_emoji` TEXT NOT NULL DEFAULT '🤖'
  - `created_at` INTEGER NOT NULL
  - `updated_at` INTEGER NOT NULL

`agent_configs`
- Columns:
  - `id` TEXT PRIMARY KEY
  - `slot_index` INTEGER NOT NULL UNIQUE
  - `label` TEXT NOT NULL
  - `persona_id` TEXT REFERENCES personas(id) ON DELETE SET NULL
  - `model_id` TEXT
  - `color` TEXT NOT NULL DEFAULT '#4a9eff'
  - `icon_emoji` TEXT NOT NULL DEFAULT '🤖'
  - `enabled` INTEGER NOT NULL DEFAULT 1
  - `max_tokens` INTEGER NOT NULL DEFAULT 4096
  - `created_at` INTEGER NOT NULL
  - `updated_at` INTEGER NOT NULL
- Indexes:
  - `idx_agent_configs_slot` ON `agent_configs(slot_index)`

`swarm_runs`
- Columns:
  - `id` TEXT PRIMARY KEY
  - `session_id` TEXT
  - `user_prompt` TEXT NOT NULL
  - `leader_plan` TEXT
  - `status` TEXT NOT NULL DEFAULT 'running'
  - `started_at` INTEGER NOT NULL
  - `completed_at` INTEGER

`swarm_agent_results`
- Columns:
  - `id` TEXT PRIMARY KEY
  - `swarm_run_id` TEXT NOT NULL REFERENCES swarm_runs(id) ON DELETE CASCADE
  - `agent_id` TEXT NOT NULL
  - `agent_slot` INTEGER NOT NULL
  - `subtask` TEXT NOT NULL
  - `result` TEXT
  - `stats_json` TEXT
  - `started_at` INTEGER NOT NULL
  - `completed_at` INTEGER

---

Migrations handling (summary)
- The codebase uses two approaches:
  1) Inline `CREATE TABLE IF NOT EXISTS` calls executed at startup for stores that do not track explicit migrations (e.g., `agent_store.rs` does direct CREATE TABLE and CREATE INDEX).
  2) A migration-tracking pattern that creates a `schema_migrations` table and runs `apply_migration(version, description, &[sql_statements])` (see `assistant_store.rs` and `user_skills.rs`). After executing migration statements, code inserts a row into `schema_migrations` to mark it applied.
- Many upgrades use `ALTER TABLE ... ADD COLUMN ...` wrapped with ignoring failures so the migration is idempotent (e.g., adding `tool_call_id` to `assistant_messages` or extending `session_messages`).

References (primary Rust sources)
- `bonsai-workspace/src-tauri/src/assistant_store.rs`
- `bonsai-workspace/src-tauri/src/chat_sessions.rs`
- `bonsai-workspace/src-tauri/src/memory_store.rs`
- `bonsai-workspace/src-tauri/src/agent_store.rs`
- `bonsai-workspace/src-tauri/src/user_skills.rs`
- `bonsai-workspace/src-tauri/src/wal.rs`

If you need a schema file suitable for `sqlite3` shell imports (DDL-only), I can render the CREATE TABLE statements verbatim into a .sql file next.
