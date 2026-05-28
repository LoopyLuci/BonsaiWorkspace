/// Survival Engine — runtime self-repair for the main Tauri process.
///
/// Exposes Tauri commands:
///   - `repair_error`                 — tries KB rules then records outcome
///   - `report_fix`                   — saves a user/AI fix to the KB
///   - `ai_repair_error`              — routes error text to BonsAI for diagnosis
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
            let p = SqlitePool::connect(&url)
                .await
                .unwrap_or_else(|_| {
                    tauri::async_runtime::block_on(
                        SqlitePool::connect("sqlite::memory:")
                    ).expect("in-memory DB failed")
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

        let state = Self { pool: Arc::new(pool) };
        tauri::async_runtime::block_on(seed_builtin_rules(&state.pool));
        state
    }
}

async fn seed_builtin_rules(pool: &SqlitePool) {
    let seeds: &[(&str, &str, &str)] = &[
        ("EADDRINUSE",                       "rule", "lsof -ti:11369 2>/dev/null | xargs -r kill -9 ; sleep 1"),
        ("address already in use",           "rule", "lsof -ti:11369 2>/dev/null | xargs -r kill -9 ; sleep 1"),
        ("Failed to bind socket",            "rule", "lsof -ti:11369 2>/dev/null | xargs -r kill -9"),
        ("Cannot find module",               "rule", "npm install --prefix bonsai-workspace"),
        ("toml parse error",                 "rule", "rm -f ~/.bonsai/bonsai-config.json"),
        ("TOML parse error",                 "rule", "rm -f ~/.bonsai/bonsai-config.json"),
        ("database disk image is malformed", "rule", "rm -f ~/.bonsai/bonsai.db"),
        ("GPU: out of memory",               "rule", "echo CPU_FALLBACK"),
        ("llama-server: exited",             "rule", "echo SIDECAR_RESTART"),
        ("Failed to create CAS",             "rule", "mkdir -p ~/.bonsai/cas_blobs"),
        ("no space left on device",          "rule", "find /tmp -name 'bonsai_*' -mmin +60 -delete"),
    ];
    for (pattern, stype, script) in seeds {
        let exists: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM fixes WHERE error_pattern = ?"
        )
        .bind(pattern)
        .fetch_one(pool)
        .await
        .unwrap_or(0);
        if exists == 0 {
            sqlx::query(
                "INSERT INTO fixes (error_pattern, solution_type, solution_script, confidence, created_by)
                 VALUES (?, ?, ?, 0.9, 'system')"
            )
            .bind(pattern).bind(stype).bind(script)
            .execute(pool)
            .await
            .ok();
        }
    }
}

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct FixEntry {
    pub id:              i64,
    pub error_pattern:   String,
    pub solution_type:   String,
    pub solution_script: String,
    pub confidence:      f64,
    pub usage_count:     i64,
    pub success_count:   i64,
    pub created_by:      String,
    pub verified:        bool,
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
    solution:      String,
    created_by:    Option<String>,
    state:         tauri::State<'_, SurvivalState>,
) -> Result<i64, String> {
    let who = created_by.as_deref().unwrap_or("user");
    let result = sqlx::query(
        "INSERT INTO fixes (error_pattern, solution_type, solution_script, confidence, created_by)
         VALUES (?, 'user', ?, 0.6, ?)"
    )
    .bind(&error_pattern).bind(&solution).bind(who)
    .execute(state.pool.as_ref())
    .await
    .map_err(|e| e.to_string())?;
    let id = result.last_insert_rowid();
    info!("[survival] fix #{id} recorded by {who}");
    Ok(id)
}

/// Ask BonsAI to diagnose `error` and suggest a repair script.
#[command]
pub async fn ai_repair_error(
    error: String,
    state: tauri::State<'_, SurvivalState>,
) -> Result<String, String> {
    let suggestion = ai_diagnose(&error).await?;
    if suggestion == "NOT_FIXABLE" || suggestion.is_empty() {
        return Ok("BonsAI could not determine a fix for this error.".into());
    }
    let forbidden = ["rm -rf /", "mkfs", ":(){ :|:& };:", "format c:"];
    if forbidden.iter().any(|f| suggestion.contains(f)) {
        warn!("[survival] AI script rejected (safety gate)");
        return Err("AI suggestion rejected by safety filter".into());
    }
    let pattern = &error[..error.len().min(200)];
    sqlx::query(
        "INSERT INTO fixes (error_pattern, solution_type, solution_script, confidence, created_by)
         VALUES (?, 'ai', ?, 0.7, 'bonsai')"
    )
    .bind(pattern).bind(&suggestion)
    .execute(state.pool.as_ref())
    .await
    .ok();
    Ok(suggestion)
}

/// Return the full KB for the SurvivalPanel UI.
#[command]
pub async fn list_fixes(
    state: tauri::State<'_, SurvivalState>,
) -> Result<Vec<FixEntry>, String> {
    fetch_all(&state.pool).await.map_err(|e| e.to_string())
}

/// Dump KB→JSONL for fine-tuning the survival model.
#[command]
pub async fn export_survival_training_data(
    output_path: String,
    state:       tauri::State<'_, SurvivalState>,
) -> Result<usize, String> {
    let entries = fetch_all(&state.pool).await.map_err(|e| e.to_string())?;
    let examples: Vec<serde_json::Value> = entries
        .into_iter()
        .filter(|e| e.success_count > 0)
        .map(|e| serde_json::json!({
            "messages": [
                {"role": "system",    "content": "You are an expert at fixing the Bonsai AI application. Given an error log, output a single shell command to fix it. Output NOT_FIXABLE if you cannot."},
                {"role": "user",      "content": e.error_pattern},
                {"role": "assistant", "content": e.solution_script},
            ]
        }))
        .collect();
    let count = examples.len();
    let jsonl = examples.iter().map(|e| e.to_string()).collect::<Vec<_>>().join("\n");
    std::fs::write(&output_path, jsonl).map_err(|e| e.to_string())?;
    info!("[survival] exported {count} training examples → {output_path}");
    Ok(count)
}

/// Merge successful fixes from the watchdog's SQLite KB into the app KB.
/// Called at startup to absorb repairs the watchdog discovered during recovery.
#[command]
pub async fn sync_watchdog_kb(
    state: tauri::State<'_, SurvivalState>,
) -> Result<usize, String> {
    let wdb_path = dirs::home_dir()
        .unwrap_or_default()
        .join(".bonsai/survival_kb.db");
    if !wdb_path.exists() {
        return Ok(0);
    }

    let url = format!("sqlite://{}?mode=ro", wdb_path.display());
    let wpool = SqlitePool::connect(&url).await.map_err(|e| e.to_string())?;

    let rows = sqlx::query(
        "SELECT error_pattern, solution_type, solution_script, confidence, created_by
         FROM fixes WHERE success_count > 0"
    )
    .fetch_all(&wpool)
    .await
    .map_err(|e| e.to_string())?;

    let mut merged = 0usize;
    for row in &rows {
        let pattern: String  = row.try_get(0).unwrap_or_default();
        let stype:   String  = row.try_get(1).unwrap_or_default();
        let script:  String  = row.try_get(2).unwrap_or_default();
        let conf:    f64     = row.try_get(3).unwrap_or(0.5);
        let who:     String  = row.try_get(4).unwrap_or_default();

        let exists: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM fixes WHERE error_pattern=? AND solution_script=?"
        )
        .bind(&pattern).bind(&script)
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
    }
    Ok(merged)
}

// ── Internal helpers ──────────────────────────────────────────────────────────

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
         FROM fixes ORDER BY success_count DESC, confidence DESC"
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| FixEntry {
        id:              r.try_get(0).unwrap_or(0),
        error_pattern:   r.try_get(1).unwrap_or_default(),
        solution_type:   r.try_get(2).unwrap_or_default(),
        solution_script: r.try_get(3).unwrap_or_default(),
        confidence:      r.try_get(4).unwrap_or(0.0),
        usage_count:     r.try_get(5).unwrap_or(0),
        success_count:   r.try_get(6).unwrap_or(0),
        created_by:      r.try_get(7).unwrap_or_default(),
        verified:        r.try_get::<i64, _>(8).unwrap_or(0) != 0,
    }).collect())
}

fn run_script(script: &str) -> bool {
    let result = if cfg!(target_os = "windows") {
        std::process::Command::new("cmd").args(["/C", script]).output()
    } else {
        std::process::Command::new("sh").args(["-c", script]).output()
    };
    result.map(|o| o.status.success()).unwrap_or(false)
}

async fn ai_diagnose(log: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let payload = serde_json::json!({
        "model": "bonsai",
        "messages": [
            {"role": "system", "content":
                "You are an expert at fixing the Bonsai AI application. \
                 Given an error log, output a single shell command that fixes the problem. \
                 Output NOT_FIXABLE if you cannot determine a safe fix."},
            {"role": "user", "content": &log[..log.len().min(4000)]},
        ],
        "max_tokens": 200,
        "temperature": 0.05,
    });

    for port in [11420u16, 8080] {
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
