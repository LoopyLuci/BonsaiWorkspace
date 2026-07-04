/// Knowledge base — SQLite-backed store for issue→fix mappings.
///
/// Every entry contains:
/// - one or more symptom patterns (substrings / regexes matched against logs)
/// - a repair script (shell command or structured action)
/// - metadata: confidence, usage, category, tags, who created it
///
/// The KB doubles as a universal AI/ML training dataset.
/// Export formats: SFT (chat), DPO (chosen/rejected pairs), instruction-tuning.

use anyhow::Result;
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixEntry {
    pub id:              i64,
    pub error_pattern:   String,
    pub solution_type:   String,   // "rule" | "ai" | "user" | "historical"
    pub solution_script: String,
    pub confidence:      f64,
    pub usage_count:     i64,
    pub success_count:   i64,
    pub created_by:      String,   // "bonsai" | "user" | "agent" | "system"
    pub verified:        bool,
    pub category:        String,   // "rust" | "svelte" | "build" | "training" | "runtime" | "network"
    pub tags:            String,   // comma-separated
}

// ─── Historical fixes seeded on first run ─────────────────────────────────────

pub const SEEDED_FIXES: &[(&str, &str, &str, &str)] = &[
    // (error_pattern, solution_script, category, tags)

    // ── Svelte / TypeScript template errors ──────────────────────────────────
    ("Type cast 'as' is not valid in Svelte template attribute",
     "Move the TypeScript 'as' cast out of the template expression into a helper function in <script lang=\"ts\">. E.g. function setTab(t: string) { activeTab = t as Tab; } then call setTab(tab) in the template.",
     "svelte", "svelte,typescript,type-assertion"),

    ("Unexpected token TypeScript cast in Svelte template",
     "Svelte does not support 'as T' type assertions inside template expressions (HTML part). Move the cast to a helper function in <script lang=\"ts\">.",
     "svelte", "svelte,typescript,type-assertion"),

    ("onMount return type Promise is not assignable to void",
     "onMount expects a synchronous cleanup function or void. Either (a) remove async from the onMount callback and use .then(), or (b) cast: (onMount as any)(async () => { ...; return cleanup; })",
     "svelte", "svelte,async,onMount"),

    ("e.target as HTMLInputElement is not valid in Svelte template",
     "Use e.currentTarget instead of (e.target as HTMLInputElement) inside Svelte template event handlers. Or define a helper: function inputValue(e: Event): string { return (e.target as HTMLInputElement)?.value ?? ''; }",
     "svelte", "svelte,typescript,dom"),

    ("Property 'test' does not exist on type 'UserConfigExport'",
     "Add '/// <reference types=\"vitest\" />' at the top of vite.config.ts, or move Vitest config to a separate vitest.config.ts file.",
     "svelte", "svelte,vitest,vite"),

    ("invoke returns unknown not assignable to any[]",
     "Use explicit type parameter: invoke<any[]>('command_name', args). Tauri's invoke() returns Promise<unknown> by default.",
     "svelte", "svelte,tauri,typescript"),

    ("No overload matches createEventDispatcher",
     "Add all dispatched event names to createEventDispatcher generic: createEventDispatcher<{ close: void; submit: string }>().",
     "svelte", "svelte,typescript,events"),

    ("Property dataset does not exist on type Element",
     "Cast to HTMLElement: (el as HTMLElement).dataset. The dataset property exists on HTMLElement but not on the base Element type.",
     "svelte", "svelte,typescript,dom"),

    ("description[..n] Python-style slice",
     "Replace Python-style slice 'str[..n]' with JavaScript 'str.slice(0, n)' or 'str.substring(0, n)'.",
     "svelte", "svelte,javascript,syntax"),

    ("JSDoc cast Expected } parse error in each block",
     "JSDoc comments in Svelte {#each} block expressions cause parse errors. Define a typed constant array in <script> instead.",
     "svelte", "svelte,jsdoc,syntax"),

    ("Module has no default export",
     "Svelte components export by default. If the error persists, check that svelte-check and TypeScript configs point to the correct tsconfig.json path. Run svelte-check from the src/ directory.",
     "svelte", "svelte,typescript,imports"),

    ("A11y: div with click handler must have an ARIA role",
     "Add role=\"button\" tabindex=\"0\" and on:keydown to clickable div elements. Or replace the div with a <button> element.",
     "svelte", "svelte,a11y,accessibility"),

    ("A11y: visible non-interactive elements with on:click must have keyboard handler",
     "Add on:keydown={(e) => { if (e.key === 'Enter' || e.key === ' ') handler(); }} to clickable non-interactive elements, or use <button> instead of <div>.",
     "svelte", "svelte,a11y,keyboard"),

    ("as Record<string, string> in Svelte template",
     "Type assertions using 'as' are not supported in Svelte template expressions. Extract to a helper function: function lookup(map: Record<string,string>, key: string): string { return map[key] ?? ''; }",
     "svelte", "svelte,typescript,type-assertion"),

    // ── Rust compilation errors ───────────────────────────────────────────────
    ("raw string terminates at backslash-quote inside r\"",
     "In Rust raw strings r\"...\", backslash has no special meaning, so '\\\"' is two characters: backslash and double-quote, and the double-quote terminates the string. Use r#\"...\"# to include literal double-quotes, or rewrite regex as [\"'] instead of [\\'\\\"]. Example: change r\"['\\\"]\" to r#\"[\"']\"#",
     "rust", "rust,regex,raw-string"),

    ("r#\"...\"# raw string not closed",
     "Raw string r#\"...\"# is terminated by a matching #. Ensure the closing \"# has exactly the same number of # symbols as the opening r#. Do not nest raw strings.",
     "rust", "rust,syntax,raw-string"),

    ("error[E0432]: unresolved import log",
     "The workspace uses tracing, not log. Replace log::info!, log::warn!, log::error!, log::debug! with tracing::info!, tracing::warn!, tracing::error!, tracing::debug!",
     "rust", "rust,logging,tracing"),

    ("use of unresolved module or unlinked crate log",
     "Replace log::* macros with tracing::* macros. Add tracing = \"0.1\" to the crate's Cargo.toml [dependencies].",
     "rust", "rust,logging,tracing"),

    ("error[E0252]: the name is defined multiple times",
     "A type or value is imported twice in the same scope. Remove the duplicate 'use' statement. Check both explicit imports and glob imports (use foo::*).",
     "rust", "rust,imports,duplicate"),

    ("error[E0308]: mismatched types Option not Result",
     "app_handle.try_state::<T>() returns Option<tauri::State<T>>, not Result. Use if let Some(state) = app_handle.try_state::<T>() { ... } instead of if let Ok(state) = ...",
     "rust", "rust,tauri,option-result"),

    ("error[E0308]: expected Option found Result",
     "try_state() returns Option, not Result. Use if let Some(x) = try_state::<T>() pattern.",
     "rust", "rust,tauri,option-result"),

    ("error[E0433]: cannot find module or crate dirs",
     "Add dirs = \"5\" to [dependencies] in Cargo.toml. The dirs crate provides platform-specific directory paths.",
     "rust", "rust,dependencies,dirs"),

    ("cannot find crate error-types",
     "Check that error-types is listed in the workspace Cargo.toml members and add error-types = { path = \"../error-types\" } to the crate's Cargo.toml dependencies.",
     "rust", "rust,dependencies,workspace"),

    ("field timeout_secs not in SwarmSpec",
     "Check the SwarmSpec struct definition. Field names may differ from what the DeepSeek agent suggested: max_agents is max_workers, timeout_secs exists.",
     "rust", "rust,struct,field-name"),

    ("non-exhaustive patterns SystemEvent",
     "All SystemEvent enum variants must be matched. Add the missing variants (e.g. TestStarted, DreamCycleStarted, CheckpointRequested) to the match arm that returns None/default.",
     "rust", "rust,match,exhaustive"),

    ("error[E0433]: cannot find macro log in this scope",
     "The project uses tracing, not log. Replace log::info! with tracing::info!, etc. Do not add the log crate as a dependency.",
     "rust", "rust,logging,macro"),

    ("cannot find value CATEGORY_ICONS in this scope",
     "CATEGORY_ICONS is defined as a const Record in TypeScript, not accessible from Rust. Ensure the Svelte component imports the constant from the correct location.",
     "svelte", "svelte,typescript,const"),

    ("error[E0004]: non-exhaustive patterns in match",
     "A match expression does not cover all variants. Add the missing variants or add a catch-all arm: _ => { /* handle */ } or _ => return None.",
     "rust", "rust,match,exhaustive"),

    // ── Build / environment errors ────────────────────────────────────────────
    ("cargo: The term 'cargo' is not recognized",
     "Cargo binary is not in PATH. Add $HOME/.cargo/bin to PATH: export PATH=\"$PATH:$HOME/.cargo/bin\" in bash, or $env:PATH += \";$env:USERPROFILE\\.cargo\\bin\" in PowerShell.",
     "build", "build,cargo,path"),

    ("error: failed to run custom build command for bonsai-native",
     "Set LLAMA_CPP_PATH environment variable to the directory containing the llama.cpp build. Or set BONSAI_NATIVE_SKIP=1 to build without GPU acceleration.",
     "build", "build,native,llama"),

    ("LLAMA_CPP_PATH not set; bonsai-native will not link",
     "This is a warning, not an error. Set LLAMA_CPP_PATH to the llama.cpp build directory for GPU acceleration. The crate will compile without it (CPU-only mode).",
     "build", "build,native,llama,warning"),

    ("Module not found Can't resolve @tauri-apps/api",
     "Run 'npm install' in the bonsai-workspace directory. Also check that @tauri-apps/api is listed in package.json dependencies.",
     "build", "build,npm,tauri,frontend"),

    ("duplicate features key in Cargo.toml",
     "A Cargo.toml features section has a duplicate key. Check for repeated feature names and remove the duplicate. Run 'cargo check' to verify.",
     "build", "build,cargo,toml"),

    ("sqlx version 0.7 not compatible",
     "Update sqlx to version 0.8 in Cargo.toml: sqlx = { version = \"0.8\", features = [\"runtime-tokio\", \"sqlite\"] }",
     "build", "build,sqlx,version"),

    ("rusqlite 0.31 conflicts with sqlx libsqlite3-sys 0.30",
     "bonsai-native uses rusqlite 0.31 which conflicts with sqlx's libsqlite3-sys. Use rusqlite = { version = \"0.32\", features = [\"bundled\"] } to avoid the conflict, or keep bonsai-native excluded from the workspace.",
     "build", "build,rusqlite,sqlx,conflict"),

    ("sccache not configured as rustc-wrapper",
     "Add to .cargo/config.toml: [build]\nrustc-wrapper = \"sccache\"  # path to sccache binary",
     "build", "build,sccache,cache"),

    // ── Runtime / crash errors ────────────────────────────────────────────────
    ("STATUS_ACCESS_VIOLATION exit code -1073741819",
     "Windows ACCESS_VIOLATION in PyTorch DPO training is caused by tensors too large for the CPU allocator. Fix: pass --max-length 128 (or 256) to dpo_train.py. Default is now 256.",
     "runtime", "runtime,training,windows,pytorch"),

    ("dpo_train.py segfault exit 139 after pairs loaded",
     "Segfault after '[data] pairs=N' in dpo_train.py on Windows/CPU is a tensor size issue. Two model copies x max_length=512 exceeds allocatable contiguous memory. Use --max-length 128 for CPU training.",
     "runtime", "runtime,training,segfault,memory"),

    ("port 11369 already in use",
     "A previous Bonsai daemon process is still running. Kill it: pkill bonsai-workspace (Linux/macOS) or Stop-Process -Name bonsai-workspace (PowerShell). Or change the port in config.",
     "runtime", "runtime,port,daemon"),

    ("WebSocket connection refused localhost",
     "The Bonsai daemon is not running or has not started yet. Start it with 'cargo run --release' or via the installer. Check daemon.log for startup errors.",
     "runtime", "runtime,websocket,daemon"),

    ("GPU OOM during training",
     "Reduce GPU layers in config/training.yaml: set n_gpu_layers to a lower value, or reduce batch_size and max_length. Use --device cpu as fallback.",
     "runtime", "runtime,training,gpu,oom"),

    ("GPU OOM out of memory CUDA",
     "Reduce batch size, sequence length, or number of GPU layers. Consider using gradient checkpointing (gradient_checkpointing=True in training config). Or switch to CPU training.",
     "runtime", "runtime,gpu,cuda,oom"),

    ("panicked at index out of bounds",
     "An array/vec index is out of range. Add bounds checking before indexing: use .get(idx) instead of [idx] and handle None. Check that inputs are validated before use.",
     "runtime", "runtime,rust,panic,bounds"),

    ("panicked at called Option::unwrap on a None value",
     "An unwrap() call failed. Replace .unwrap() with .expect(\"meaningful message\") for debugging, then fix the underlying cause. Use .ok()?, .unwrap_or_default(), or proper error propagation.",
     "runtime", "runtime,rust,panic,unwrap"),

    ("UnicodeDecodeError in Python training script",
     "Force UTF-8 encoding: open(file, encoding='utf-8', errors='replace'). Also ensure training data JSONL files are saved as UTF-8.",
     "runtime", "runtime,training,python,encoding"),

    ("HF hub connection timeout",
     "Set environment variables: HF_HUB_OFFLINE=1 and TRANSFORMERS_OFFLINE=1. Pass local_files_only=True to all from_pretrained() calls. Run huggingface-cli download <model-name> first.",
     "runtime", "runtime,training,huggingface,network"),

    ("No student HF snapshot found",
     "Run: huggingface-cli download Qwen/Qwen2.5-1.5B-Instruct --local-dir ~/.bonsai/models/base/qwen2.5-1.5b-instruct. Then set the model path in config/training.yaml.",
     "runtime", "runtime,training,huggingface,model"),

    // ── Training / data pipeline errors ──────────────────────────────────────
    ("PyTorch DirectML backward pass crash",
     "Fall back to CPU training: pass --device cpu to all training scripts. DirectML does not support all PyTorch operations. Consider using CUDA (Nvidia) or ROCm (AMD) instead.",
     "training", "training,directml,windows,pytorch"),

    ("Loss is NaN during training",
     "NaN loss indicates numerical instability. Reduce learning rate by 10x, add gradient clipping (max_norm=1.0), check for NaN in training data, and ensure labels are correctly shifted.",
     "training", "training,loss,nan,stability"),

    ("GGUF conversion failed",
     "Ensure llama.cpp convert_hf_to_gguf.py can find all model files. Run: python convert_hf_to_gguf.py --model-dir <path> --outtype q8_0 --outfile <output.gguf>. Check that the model config.json is present.",
     "training", "training,gguf,conversion"),

    // ── Survival system specific ──────────────────────────────────────────────
    ("SurvivalOverlay appears too often",
     "Increase the auto_retry_count threshold in survival.rs or add a cooldown period between overlay appearances. Check if the triggering error is being permanently fixed vs. recurring.",
     "runtime", "runtime,survival,overlay"),

    ("AI-generated script rejected by safety gate",
     "The script contains patterns blocked by the safety gate (network calls, file deletion, eval, etc.). Review the allowlist in survival.rs::is_safe_script(). Add specific patterns to the allowlist after human review.",
     "runtime", "runtime,survival,safety"),

    ("Watchdog not restarting daemon",
     "Ensure bonsai-watchdog binary is in PATH and the systemd/launchd/task-scheduler service is running. On Windows: check Task Scheduler for the BonsaiWatchdog task. On Linux: check systemctl status bonsai-watchdog.",
     "runtime", "runtime,watchdog,daemon"),

    // ── Universe / Time Travel ───────────────────────────────────────────────
    ("universe_hooks SystemEvent match non-exhaustive",
     "The universe_hooks convert_system_event() function must match all SystemEvent variants. Add any new variants to the non-recording arm: SystemEvent::NewVariant { .. } => return None.",
     "rust", "rust,universe,match"),

    ("tokio-rusqlite Error mapping",
     "tokio-rusqlite errors wrap rusqlite errors. Map with: .map_err(|e| tokio_rusqlite::Error::Rusqlite(e)) for rusqlite errors inside call() closures.",
     "rust", "rust,sqlite,tokio-rusqlite"),

    // ── General best practices captured from this project ────────────────────
    ("AppState not managed when command is called",
     "Tauri commands that use State<AppState> require the state to be registered with app.manage(AppState { ... }) in the setup closure BEFORE any commands are invoked. Check the order in lib.rs setup.",
     "rust", "rust,tauri,state"),

    ("gethostname crate vs hostname crate",
     "The workspace uses gethostname = \"0.4\". Use gethostname::gethostname().to_string_lossy().to_string() to get the hostname. Do not add the 'hostname' crate.",
     "rust", "rust,dependencies,hostname"),

    ("bonsai-ci PathBuf re-import duplicate",
     "When adding new use statements to a file, check for existing identical imports. Rust reports 'the name X is defined multiple times' for duplicate imports.",
     "rust", "rust,imports,duplicate"),

    ("unsafe raw pointer mutation of Arc struct field",
     "Do not use unsafe raw pointer casting to mutate fields of Arc<T>. Instead, use Arc<RwLock<T>> for mutable shared state, or restructure initialization to avoid post-construction mutation.",
     "rust", "rust,unsafe,arc,mutation"),

    ("PowerShell += operator not recognized in Bash tool",
     "The Bash tool uses bash, not PowerShell. Use export PATH=\"$PATH:$HOME/.cargo/bin\" syntax in Bash. For PowerShell, use the PowerShell tool instead.",
     "build", "build,powershell,bash,path"),
];

// ─── KnowledgeBase implementation ─────────────────────────────────────────────

pub struct KnowledgeBase {
    conn: Connection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingEntry {
    pub prompt:   String,
    pub chosen:   String,
    pub rejected: String,
    pub category: String,
    pub source:   String,
}

impl KnowledgeBase {
    #[cfg(test)]
    pub fn conn(&self) -> &Connection { &self.conn }
}

impl KnowledgeBase {
    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("
            PRAGMA journal_mode = WAL;
            CREATE TABLE IF NOT EXISTS fixes (
                id             INTEGER PRIMARY KEY AUTOINCREMENT,
                error_pattern  TEXT    NOT NULL,
                solution_type  TEXT    NOT NULL DEFAULT 'rule',
                solution_script TEXT   NOT NULL,
                confidence     REAL    NOT NULL DEFAULT 0.5,
                usage_count    INTEGER NOT NULL DEFAULT 0,
                success_count  INTEGER NOT NULL DEFAULT 0,
                created_by     TEXT    NOT NULL DEFAULT 'system',
                verified       INTEGER NOT NULL DEFAULT 0,
                category       TEXT    NOT NULL DEFAULT 'other',
                tags           TEXT    NOT NULL DEFAULT '',
                created_at     DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX IF NOT EXISTS idx_fixes_pattern  ON fixes(error_pattern);
            CREATE INDEX IF NOT EXISTS idx_fixes_category ON fixes(category);
            CREATE INDEX IF NOT EXISTS idx_fixes_success  ON fixes(success_count DESC);
        ")?;
        Ok(Self { conn })
    }

    /// Find fixes whose error_pattern appears as a substring of `log`.
    pub fn find_matching(&self, log: &str) -> Vec<FixEntry> {
        let mut stmt = self.conn.prepare(
            "SELECT id, error_pattern, solution_type, solution_script, confidence,
                    usage_count, success_count, created_by, verified, category, tags
             FROM fixes
             ORDER BY success_count DESC, confidence DESC"
        ).unwrap();
        stmt.query_map([], |row| {
            Ok(FixEntry {
                id:              row.get(0)?,
                error_pattern:   row.get(1)?,
                solution_type:   row.get(2)?,
                solution_script: row.get(3)?,
                confidence:      row.get(4)?,
                usage_count:     row.get(5)?,
                success_count:   row.get(6)?,
                created_by:      row.get(7)?,
                verified:        row.get::<_, i64>(8)? != 0,
                category:        row.get(9).unwrap_or_default(),
                tags:            row.get(10).unwrap_or_default(),
            })
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .filter(|f| log.contains(&f.error_pattern))
        .collect()
    }

    /// Record a new fix (returns its row id).
    pub fn insert_fix(
        &self,
        pattern:    &str,
        stype:      &str,
        script:     &str,
        confidence: f64,
        created_by: &str,
    ) -> Result<i64> {
        self.insert_fix_full(pattern, stype, script, confidence, created_by, "other", "")
    }

    pub fn insert_fix_full(
        &self,
        pattern:    &str,
        stype:      &str,
        script:     &str,
        confidence: f64,
        created_by: &str,
        category:   &str,
        tags:       &str,
    ) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO fixes (error_pattern, solution_type, solution_script, confidence, created_by, category, tags)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![pattern, stype, script, confidence, created_by, category, tags],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Increment usage counter; increment success counter if `success` is true.
    pub fn record_outcome(&self, id: i64, success: bool) -> Result<()> {
        if success {
            self.conn.execute(
                "UPDATE fixes SET usage_count = usage_count + 1, success_count = success_count + 1 WHERE id = ?1",
                params![id],
            )?;
        } else {
            self.conn.execute(
                "UPDATE fixes SET usage_count = usage_count + 1 WHERE id = ?1",
                params![id],
            )?;
        }
        Ok(())
    }

    pub fn seed_defaults(&self) -> Result<()> {
        for (pattern, script, category, tags) in SEEDED_FIXES {
            let exists: bool = self.conn.query_row(
                "SELECT COUNT(*) FROM fixes WHERE error_pattern = ?1",
                params![pattern],
                |row| row.get::<_, i64>(0),
            ).unwrap_or(0) > 0;
            if !exists {
                let _ = self.insert_fix_full(pattern, "historical", script, 0.95, "system", category, tags);
            }
        }
        Ok(())
    }

    pub fn total_count(&self) -> i64 {
        self.conn.query_row("SELECT COUNT(*) FROM fixes", [], |r| r.get(0)).unwrap_or(0)
    }

    pub fn category_counts(&self) -> Vec<(String, i64)> {
        let mut stmt = self.conn.prepare(
            "SELECT category, COUNT(*) FROM fixes GROUP BY category ORDER BY COUNT(*) DESC"
        ).unwrap();
        stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect()
    }

    /// Export as SFT (chat format) JSONL — all entries.
    pub fn export_sft(&self) -> Vec<serde_json::Value> {
        let mut stmt = self.conn.prepare(
            "SELECT error_pattern, solution_script, category FROM fixes ORDER BY success_count DESC"
        ).unwrap();
        stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .map(|(pattern, script, category)| serde_json::json!({
            "messages": [
                {"role": "system", "content": format!(
                    "You are BonsAI, an expert debugging assistant for the Bonsai Ecosystem. \
                     Category: {}. Given an error log, output the exact fix.",
                    category
                )},
                {"role": "user", "content": &pattern},
                {"role": "assistant", "content": &script},
            ],
            "metadata": {"category": category, "source": "survival_kb"}
        }))
        .collect()
    }

    /// Export as DPO pairs (chosen=correct fix, rejected=do-nothing).
    pub fn export_dpo(&self) -> Vec<serde_json::Value> {
        let mut stmt = self.conn.prepare(
            "SELECT error_pattern, solution_script FROM fixes WHERE success_count > 0"
        ).unwrap();
        stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .map(|(pattern, script)| serde_json::json!({
            "prompt": format!("Error: {}", pattern),
            "chosen": script,
            "rejected": "I cannot determine the fix for this error.",
            "metadata": {"source": "survival_kb"}
        }))
        .collect()
    }

    /// Export as instruction-tuning JSONL.
    pub fn export_instruct(&self) -> Vec<serde_json::Value> {
        let mut stmt = self.conn.prepare(
            "SELECT error_pattern, solution_script, category FROM fixes"
        ).unwrap();
        stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .map(|(pattern, script, category)| serde_json::json!({
            "instruction": format!("Fix the following {} error in the Bonsai Ecosystem:", category),
            "input": pattern,
            "output": script,
        }))
        .collect()
    }

    /// Legacy export (backwards compatible).
    pub fn export_jsonl(&self) -> Vec<serde_json::Value> {
        self.export_sft()
    }
}
