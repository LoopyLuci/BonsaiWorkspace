/// Survival Engine — runtime self-repair for the main Tauri process.
///
/// Exposes Tauri commands:
///   - `repair_error`                 — tries KB rules then records outcome
///   - `report_fix`                   — saves a user/AI fix to the KB
///   - `ai_repair_error`              — routes error text to OmniAI for diagnosis
///   - `list_fixes`                   — returns current KB for the UI
///   - `export_survival_training_data`— dumps KB→JSONL for fine-tuning
///   - `sync_watchdog_kb`             — merges fixes from the watchdog's separate DB
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use tauri::command;
use tracing::{info, warn};

// ── State ─────────────────────────────────────────────────────────────────────

pub struct SurvivalState {
    pub pool: Arc<SqlitePool>,
}

impl SurvivalState {
    pub fn new(db_path: &str) -> Self {
        let url = format!("sqlite://{db_path}?mode=rwc");
        let pool = tauri::async_runtime::block_on(async {
            let p = SqlitePool::connect(&url).await.unwrap_or_else(|_| {
                tauri::async_runtime::block_on(SqlitePool::connect("sqlite::memory:"))
                    .expect("in-memory DB failed")
            });
            sqlx::query(
                "PRAGMA journal_mode = WAL;
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
                     created_at      DATETIME DEFAULT CURRENT_TIMESTAMP
                 );
                 CREATE INDEX IF NOT EXISTS idx_survival_pattern ON fixes(error_pattern);",
            )
            .execute(&p)
            .await
            .ok();
            p
        });

        let state = Self {
            pool: Arc::new(pool),
        };
        tauri::async_runtime::block_on(seed_builtin_rules(&state.pool));
        state
    }
}

/// Seed rules as (pattern, unix_script, windows_script). `run_script` execs
/// via `cmd /C` on Windows and `sh -c` elsewhere (see below), so a script
/// written in POSIX syntax (`lsof`/`xargs`/`rm -f`) silently no-ops on
/// Windows — `cmd /C` doesn't understand any of those. Since this app's
/// actual target/dev platform is Windows 10, every rule below has a real,
/// tested-syntax Windows counterpart using real app paths (`%APPDATA%\
/// com.omnisystem.workspace\...`, matching `config::config_path`/CAS's
/// `app_data_dir().join("cas_blobs")` — not the previous `~/.workspace/...`
/// guesses, which pointed at a directory this app never actually writes to).
///
/// The two purely-informational entries that used to exist here
/// ("GPU: out of memory" → `echo CPU_FALLBACK`, "llama-server: exited" →
/// `echo SIDECAR_RESTART`) were removed: `echo` always exits 0, so
/// `repair_error` would record them as a successful fix despite doing
/// nothing — misleading telemetry for a problem `model_orchestrator.rs`
/// already recovers from internally (GPU-unsafe-quant detection / crash-retry
/// to CPU). A shell script can't reach into that in-process state, so there
/// is no honest KB rule to write for these two patterns.
async fn seed_builtin_rules(pool: &SqlitePool) {
    let seeds: &[(&str, &str, &str)] = &[
        (
            "EADDRINUSE",
            "lsof -ti:47100 2>/dev/null | xargs -r kill -9 ; sleep 1",
            r#"powershell -NoProfile -Command "Get-NetTCPConnection -LocalPort 47100 -ErrorAction SilentlyContinue | ForEach-Object { Stop-Process -Id $_.OwningProcess -Force -ErrorAction SilentlyContinue }""#,
        ),
        (
            "address already in use",
            "lsof -ti:47100 2>/dev/null | xargs -r kill -9 ; sleep 1",
            r#"powershell -NoProfile -Command "Get-NetTCPConnection -LocalPort 47100 -ErrorAction SilentlyContinue | ForEach-Object { Stop-Process -Id $_.OwningProcess -Force -ErrorAction SilentlyContinue }""#,
        ),
        (
            "Failed to bind socket",
            "lsof -ti:47100 2>/dev/null | xargs -r kill -9",
            r#"powershell -NoProfile -Command "Get-NetTCPConnection -LocalPort 47100 -ErrorAction SilentlyContinue | ForEach-Object { Stop-Process -Id $_.OwningProcess -Force -ErrorAction SilentlyContinue }""#,
        ),
        (
            "Cannot find module",
            "npm install --prefix workspace",
            "npm install --prefix workspace",
        ),
        (
            "toml parse error",
            "rm -f ~/.workspace/workspace-config.json",
            r#"if exist "%APPDATA%\com.omnisystem.workspace\workspace-config.json" del /F /Q "%APPDATA%\com.omnisystem.workspace\workspace-config.json""#,
        ),
        (
            "TOML parse error",
            "rm -f ~/.workspace/workspace-config.json",
            r#"if exist "%APPDATA%\com.omnisystem.workspace\workspace-config.json" del /F /Q "%APPDATA%\com.omnisystem.workspace\workspace-config.json""#,
        ),
        (
            "database disk image is malformed",
            "rm -f ~/.workspace/telemetry.db",
            r#"if exist "%USERPROFILE%\.workspace\telemetry.db" del /F /Q "%USERPROFILE%\.workspace\telemetry.db""#,
        ),
        (
            "Failed to create CAS",
            "mkdir -p ~/.workspace/cas_blobs",
            r#"if not exist "%APPDATA%\com.omnisystem.workspace\cas_blobs" mkdir "%APPDATA%\com.omnisystem.workspace\cas_blobs""#,
        ),
        (
            "no space left on device",
            "find /tmp -name 'workspace_*' -mmin +60 -delete",
            r#"powershell -NoProfile -Command "Get-ChildItem $env:TEMP -Filter 'workspace_*' -ErrorAction SilentlyContinue | Where-Object { $_.LastWriteTime -lt (Get-Date).AddMinutes(-60) } | Remove-Item -Recurse -Force -ErrorAction SilentlyContinue""#,
        ),
    ];
    for (pattern, unix_script, windows_script) in seeds {
        let script = if cfg!(target_os = "windows") { windows_script } else { unix_script };
        let exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM fixes WHERE error_pattern = ?")
            .bind(pattern)
            .fetch_one(pool)
            .await
            .unwrap_or(0);
        if exists == 0 {
            sqlx::query(
                "INSERT INTO fixes (error_pattern, solution_type, solution_script, confidence, created_by)
                 VALUES (?, 'rule', ?, 0.9, 'system')"
            )
            .bind(pattern).bind(script)
            .execute(pool)
            .await
            .ok();
        } else {
            // A prior app version may have already persisted an old/wrong
            // script for this pattern (e.g. POSIX syntax on Windows) into the
            // on-disk KB — inserting only on absence would leave it stuck
            // forever. Safe to overwrite only rows the system authored and
            // that have never actually been run/verified yet (usage_count=0);
            // anything a user has customized or that already has a track
            // record is left alone.
            sqlx::query(
                "UPDATE fixes SET solution_script = ?
                 WHERE error_pattern = ? AND created_by = 'system' AND usage_count = 0"
            )
            .bind(script).bind(pattern)
            .execute(pool)
            .await
            .ok();
        }
    }
}

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct FixEntry {
    pub id: i64,
    pub error_pattern: String,
    pub solution_type: String,
    pub solution_script: String,
    pub confidence: f64,
    pub usage_count: i64,
    pub success_count: i64,
    pub created_by: String,
    pub verified: bool,
}

// ── Commands ──────────────────────────────────────────────────────────────────

/// Try rule-based KB fixes for `error_description`. Returns true if one succeeded.
#[command]
pub async fn repair_error(
    error_description: String,
    state: tauri::State<'_, SurvivalState>,
) -> Result<bool, String> {
    let fixes = fetch_matching(&state.pool, &error_description).await;
    for fix in &fixes {
        if run_script(&fix.solution_script) {
            sqlx::query(
                "UPDATE fixes SET usage_count=usage_count+1, success_count=success_count+1, verified=1 WHERE id=?"
            )
            .bind(fix.id)
            .execute(state.pool.as_ref())
            .await
            .ok();
            info!("[survival] rule #{} applied", fix.id);
            return Ok(true);
        }
        sqlx::query("UPDATE fixes SET usage_count=usage_count+1 WHERE id=?")
            .bind(fix.id)
            .execute(state.pool.as_ref())
            .await
            .ok();
    }
    Ok(false)
}

/// Save a user or agent-supplied fix to the KB.
#[command]
pub async fn report_fix(
    error_pattern: String,
    solution: String,
    created_by: Option<String>,
    state: tauri::State<'_, SurvivalState>,
) -> Result<i64, String> {
    let who = created_by.as_deref().unwrap_or("user");
    let result = sqlx::query(
        "INSERT INTO fixes (error_pattern, solution_type, solution_script, confidence, created_by)
         VALUES (?, 'user', ?, 0.6, ?)",
    )
    .bind(&error_pattern)
    .bind(&solution)
    .bind(who)
    .execute(state.pool.as_ref())
    .await
    .map_err(|e| e.to_string())?;
    let id = result.last_insert_rowid();
    info!("[survival] fix #{id} recorded by {who}");
    prune_kb(&state.pool).await;
    Ok(id)
}

/// Ask OmniAI to diagnose `error` and suggest a repair script.
#[command]
pub async fn ai_repair_error(
    error: String,
    state: tauri::State<'_, SurvivalState>,
) -> Result<String, String> {
    let suggestion = ai_diagnose(&error).await?;
    if suggestion == "NOT_FIXABLE" || suggestion.is_empty() {
        return Ok("OmniAI could not determine a fix for this error.".into());
    }
    let forbidden = ["rm -rf /", "mkfs", ":(){ :|:& };:", "format c:"];
    if forbidden.iter().any(|f| suggestion.contains(f)) {
        warn!("[survival] AI script rejected (safety gate)");
        return Err("AI suggestion rejected by safety filter".into());
    }
    let pattern = &error[..error.len().min(200)];
    sqlx::query(
        "INSERT INTO fixes (error_pattern, solution_type, solution_script, confidence, created_by)
         VALUES (?, 'ai', ?, 0.7, 'workspace')",
    )
    .bind(pattern)
    .bind(&suggestion)
    .execute(state.pool.as_ref())
    .await
    .ok();
    prune_kb(&state.pool).await;
    Ok(suggestion)
}

/// Return the full KB for the SurvivalPanel UI.
#[command]
pub async fn list_fixes(state: tauri::State<'_, SurvivalState>) -> Result<Vec<FixEntry>, String> {
    fetch_all(&state.pool).await.map_err(|e| e.to_string())
}

/// Dump KB→JSONL for fine-tuning the survival model.
#[command]
pub async fn export_survival_training_data(
    output_path: String,
    state: tauri::State<'_, SurvivalState>,
) -> Result<usize, String> {
    let entries = fetch_all(&state.pool).await.map_err(|e| e.to_string())?;
    let examples: Vec<serde_json::Value> = entries
        .into_iter()
        .filter(|e| e.success_count > 0)
        .map(|e| serde_json::json!({
            "messages": [
                {"role": "system",    "content": "You are an expert at fixing the Workspace AI application. Given an error log, output a single shell command to fix it. Output NOT_FIXABLE if you cannot."},
                {"role": "user",      "content": e.error_pattern},
                {"role": "assistant", "content": e.solution_script},
            ]
        }))
        .collect();
    let count = examples.len();
    let jsonl = examples
        .iter()
        .map(|e| e.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&output_path, jsonl).map_err(|e| e.to_string())?;
    info!("[survival] exported {count} training examples → {output_path}");
    Ok(count)
}

/// Merge successful fixes from the watchdog's SQLite KB into the app KB.
/// Called at startup to absorb repairs the watchdog discovered during recovery.
#[command]
pub async fn sync_watchdog_kb(state: tauri::State<'_, SurvivalState>) -> Result<usize, String> {
    let wdb_path = dirs::home_dir()
        .unwrap_or_default()
        .join(".workspace/survival_kb.db");
    if !wdb_path.exists() {
        return Ok(0);
    }

    let url = format!("sqlite://{}?mode=ro", wdb_path.display());
    let wpool = SqlitePool::connect(&url).await.map_err(|e| e.to_string())?;

    let rows = sqlx::query(
        "SELECT error_pattern, solution_type, solution_script, confidence, created_by
         FROM fixes WHERE success_count > 0",
    )
    .fetch_all(&wpool)
    .await
    .map_err(|e| e.to_string())?;

    let mut merged = 0usize;
    for row in &rows {
        let pattern: String = row.try_get(0).unwrap_or_default();
        let stype: String = row.try_get(1).unwrap_or_default();
        let script: String = row.try_get(2).unwrap_or_default();
        let conf: f64 = row.try_get(3).unwrap_or(0.5);
        let who: String = row.try_get(4).unwrap_or_default();

        let exists: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM fixes WHERE error_pattern=? AND solution_script=?",
        )
        .bind(&pattern)
        .bind(&script)
        .fetch_one(state.pool.as_ref())
        .await
        .unwrap_or(1); // default to 1 (exists) on error, so we don't double-insert

        if exists == 0 {
            sqlx::query(
                "INSERT INTO fixes (error_pattern, solution_type, solution_script, confidence, created_by)
                 VALUES (?, ?, ?, ?, ?)"
            )
            .bind(&pattern).bind(&stype).bind(&script).bind(conf).bind(&who)
            .execute(state.pool.as_ref())
            .await
            .ok();
            merged += 1;
        }
    }

    if merged > 0 {
        info!("[survival] merged {merged} fixes from watchdog KB");
        prune_kb(&state.pool).await;
    }
    Ok(merged)
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Cap on non-system KB rows. A long-running personal install would
/// otherwise accumulate `fixes` rows forever — every `report_fix`,
/// `ai_repair_error`, and `sync_watchdog_kb` merge adds one and nothing ever
/// removed one. Built-in system rules (`seed_builtin_rules`, a small fixed
/// set) are always kept regardless of this cap; only user/AI-contributed
/// rows are pruned, ranked by actual track record so the ones that have
/// never worked are the first to go.
const MAX_KB_ENTRIES: i64 = 500;

async fn prune_kb(pool: &SqlitePool) {
    let _ = sqlx::query(
        "DELETE FROM fixes WHERE created_by != 'system' AND id NOT IN (
            SELECT id FROM fixes WHERE created_by != 'system'
            ORDER BY success_count DESC, confidence DESC, usage_count DESC
            LIMIT ?
        )",
    )
    .bind(MAX_KB_ENTRIES)
    .execute(pool)
    .await;
}

async fn fetch_matching(pool: &SqlitePool, log: &str) -> Vec<FixEntry> {
    fetch_all(pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|f| log.contains(&f.error_pattern))
        .collect()
}

async fn fetch_all(pool: &SqlitePool) -> sqlx::Result<Vec<FixEntry>> {
    let rows = sqlx::query(
        "SELECT id, error_pattern, solution_type, solution_script, confidence,
                usage_count, success_count, created_by, verified
         FROM fixes ORDER BY success_count DESC, confidence DESC",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| FixEntry {
            id: r.try_get(0).unwrap_or(0),
            error_pattern: r.try_get(1).unwrap_or_default(),
            solution_type: r.try_get(2).unwrap_or_default(),
            solution_script: r.try_get(3).unwrap_or_default(),
            confidence: r.try_get(4).unwrap_or(0.0),
            usage_count: r.try_get(5).unwrap_or(0),
            success_count: r.try_get(6).unwrap_or(0),
            created_by: r.try_get(7).unwrap_or_default(),
            verified: r.try_get::<i64, _>(8).unwrap_or(0) != 0,
        })
        .collect())
}

fn run_script(script: &str) -> bool {
    let result = if cfg!(target_os = "windows") {
        std::process::Command::new("cmd")
            .args(["/C", script])
            .output()
    } else {
        std::process::Command::new("sh")
            .args(["-c", script])
            .output()
    };
    result.map(|o| o.status.success()).unwrap_or(false)
}

async fn ai_diagnose(log: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let payload = serde_json::json!({
        "model": "workspace",
        "messages": [
            {"role": "system", "content":
                "You are an expert at fixing the Workspace AI application. \
                 Given an error log, output a single shell command that fixes the problem. \
                 Output NOT_FIXABLE if you cannot determine a safe fix."},
            {"role": "user", "content": &log[..log.len().min(4000)]},
        ],
        "max_tokens": 200,
        "temperature": 0.05,
    });

    for port in [crate::config::BUDDY_API_PORT, 8080] {
        let url = format!("http://127.0.0.1:{port}/v1/chat/completions");
        if let Ok(resp) = client.post(&url).json(&payload).send().await {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(s) = json["choices"][0]["message"]["content"].as_str() {
                    return Ok(s.trim().to_string());
                }
            }
        }
    }
    Err("AI model unreachable".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn memory_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query(
            "CREATE TABLE fixes (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                error_pattern   TEXT    NOT NULL,
                solution_type   TEXT    NOT NULL DEFAULT 'rule',
                solution_script TEXT    NOT NULL,
                confidence      REAL    NOT NULL DEFAULT 0.5,
                usage_count     INTEGER NOT NULL DEFAULT 0,
                success_count   INTEGER NOT NULL DEFAULT 0,
                created_by      TEXT    NOT NULL DEFAULT 'system',
                verified        INTEGER NOT NULL DEFAULT 0,
                created_at      DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool
    }

    #[tokio::test]
    async fn seeded_rules_are_windows_correct_on_this_platform() {
        let pool = memory_pool().await;
        seed_builtin_rules(&pool).await;

        let matches = fetch_matching(&pool, "EADDRINUSE: address already in use").await;
        assert!(!matches.is_empty());
        let script = &matches[0].solution_script;
        if cfg!(target_os = "windows") {
            // Must be real Windows-executable syntax, not POSIX lsof/xargs
            // that silently no-ops under `cmd /C` (the bug this fixed).
            assert!(script.contains("powershell") || script.contains("Get-NetTCPConnection"));
            assert!(!script.contains("lsof"));
        } else {
            assert!(script.contains("lsof"));
        }
    }

    #[tokio::test]
    async fn seeding_is_idempotent() {
        let pool = memory_pool().await;
        seed_builtin_rules(&pool).await;
        let first_count = fetch_all(&pool).await.unwrap().len();
        seed_builtin_rules(&pool).await;
        let second_count = fetch_all(&pool).await.unwrap().len();
        assert_eq!(first_count, second_count, "re-seeding must not duplicate rows");
    }

    #[tokio::test]
    async fn stale_unverified_system_script_self_heals_but_verified_one_does_not() {
        let pool = memory_pool().await;
        sqlx::query(
            "INSERT INTO fixes (error_pattern, solution_type, solution_script, confidence, created_by, usage_count)
             VALUES ('EADDRINUSE', 'rule', 'lsof -ti:47100 | xargs kill -9', 0.9, 'system', 0)",
        )
        .execute(&pool)
        .await
        .unwrap();

        seed_builtin_rules(&pool).await;
        let matches = fetch_matching(&pool, "EADDRINUSE").await;
        // usage_count=0 (never actually run) — safe to overwrite with the
        // current, OS-correct script.
        assert!(!matches[0].solution_script.contains("lsof"));

        // A rule with usage_count > 0 (has actually been tried) must NOT be
        // silently rewritten out from under whatever track record it has.
        sqlx::query("UPDATE fixes SET usage_count = 3 WHERE error_pattern = 'EADDRINUSE'")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "UPDATE fixes SET solution_script = 'a-previously-used-script' WHERE error_pattern = 'EADDRINUSE'",
        )
        .execute(&pool)
        .await
        .unwrap();
        seed_builtin_rules(&pool).await;
        let matches = fetch_matching(&pool, "EADDRINUSE").await;
        assert_eq!(matches[0].solution_script, "a-previously-used-script");
    }

    #[tokio::test]
    async fn prune_kb_keeps_system_rows_and_top_performers_only() {
        let pool = memory_pool().await;
        sqlx::query(
            "INSERT INTO fixes (error_pattern, solution_script, created_by, success_count) VALUES (?, 'x', 'system', 0)",
        )
        .bind("system-rule")
        .execute(&pool)
        .await
        .unwrap();

        // Insert far more user rows than the cap, with distinguishable
        // success_count so we can assert the *best* ones survive.
        for i in 0..(MAX_KB_ENTRIES + 50) {
            sqlx::query(
                "INSERT INTO fixes (error_pattern, solution_script, created_by, success_count) VALUES (?, 'x', 'user', ?)",
            )
            .bind(format!("pattern-{i}"))
            .bind(i)
            .execute(&pool)
            .await
            .unwrap();
        }

        prune_kb(&pool).await;

        let all = fetch_all(&pool).await.unwrap();
        let system_rows = all.iter().filter(|f| f.created_by == "system").count();
        let user_rows = all.iter().filter(|f| f.created_by == "user").count();
        assert_eq!(system_rows, 1, "system rows must never be pruned");
        assert_eq!(user_rows as i64, MAX_KB_ENTRIES, "user rows must be capped exactly at the limit");

        // The highest-success_count rows must be the ones that survived.
        let best_survived = all.iter().any(|f| f.error_pattern == format!("pattern-{}", MAX_KB_ENTRIES + 49));
        let worst_pruned = !all.iter().any(|f| f.error_pattern == "pattern-0");
        assert!(best_survived, "highest-performing row should survive pruning");
        assert!(worst_pruned, "lowest-performing row should be pruned");
    }
}
