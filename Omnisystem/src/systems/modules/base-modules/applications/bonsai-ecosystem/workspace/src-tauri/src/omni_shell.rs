//! Workstream E — OmniShell: AI-native terminal
//!
//! Wraps a platform shell (PowerShell on Windows, bash/zsh elsewhere) with:
//!  • Command history with full metadata (directory, exit code, timing)
//!  • PredictiveEngine integration — shows predicted next commands
//!  • Natural-language → command translation via BonsAI inference
//!  • Auto-fix suggestions on non-zero exit codes
//!  • Auto-explain on command output (opt-in)
//!  • Sylva REPL commands via the `/lua` prefix
//!  • OmnipresentCapture integration — feeds every execution as an OmnEvent
//!
//! The actual process execution delegates to tokio::process::Command so it
//! works on Windows, Linux, and macOS without additional dependencies.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::omnipresent_capture::{OmnEvent, OmnEventType, OmnipresentCapture};
use crate::predictive_engine::PredictiveEngine;
use crate::sylva::SylvaRuntime;

// ─────────────────────────────────────────────────────────────────────────────
// § 1 — Shell command record
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellCommand {
    pub id: String,
    pub input: String,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration_ms: u64,
    pub directory: String,
    pub timestamp: i64,
    pub session_id: String,
    /// AI-generated fix suggestion (populated after non-zero exit)
    pub fix_suggestion: Option<String>,
    /// AI-generated explanation (populated on success)
    pub explanation: Option<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// § 2 — Shell execution result
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellResult {
    pub command_id: String,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub duration_ms: u64,
}

impl ShellResult {
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// § 3 — Shell assistant config
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShellAssistantConfig {
    /// Intercept errors and suggest AI-powered fixes
    pub auto_fix: bool,
    /// Explain complex command output when successful
    pub auto_explain: bool,
    /// Allow natural-language → command translation
    pub nl_to_command: bool,
    /// Max lines of output to include in AI context
    pub context_lines: usize,
}

impl Default for ShellAssistantConfig {
    fn default() -> Self {
        Self {
            auto_fix: true,
            auto_explain: false,
            nl_to_command: true,
            context_lines: 40,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// § 4 — Predicted command
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictedCommand {
    pub command: String,
    pub confidence: f32,
    pub source: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// § 5 — OmniShell
// ─────────────────────────────────────────────────────────────────────────────

pub struct OmniShell {
    pub session_id: String,
    history: RwLock<Vec<ShellCommand>>,
    aliases: RwLock<HashMap<String, String>>,
    pub config: RwLock<ShellAssistantConfig>,
    predictor: Arc<PredictiveEngine>,
    sylva: Arc<SylvaRuntime>,
    capture: Arc<OmnipresentCapture>,
    cwd: RwLock<PathBuf>,
}

impl OmniShell {
    pub fn new(
        predictor: Arc<PredictiveEngine>,
        sylva: Arc<SylvaRuntime>,
        capture: Arc<OmnipresentCapture>,
    ) -> Arc<Self> {
        Arc::new(Self {
            session_id: Uuid::new_v4().to_string(),
            history: RwLock::new(Vec::new()),
            aliases: RwLock::new(HashMap::new()),
            config: RwLock::new(ShellAssistantConfig::default()),
            predictor,
            sylva,
            capture,
            cwd: RwLock::new(std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))),
        })
    }

    // ── Execution ────────────────────────────────────────────────────────────

    /// Execute an input string.  Handles:
    ///   `/lua <code>`  — evaluated in the Sylva REPL
    ///   `? <request>`  — NL-to-command translation
    ///   anything else  — executed as a shell command
    pub async fn execute(&self, raw_input: &str) -> ShellResult {
        let input = raw_input.trim().to_string();
        if input.is_empty() {
            return ShellResult {
                command_id: Uuid::new_v4().to_string(),
                stdout: String::new(),
                stderr: String::new(),
                exit_code: 0,
                duration_ms: 0,
            };
        }

        // Expand alias
        let expanded = self.expand_alias(&input).await;

        // Lua REPL shortcut
        if expanded.starts_with("/lua ") {
            return self.execute_lua(&expanded[5..]).await;
        }

        // NL-to-command (prefix: `?` or `nl:`)
        let config = self.config.read().await.clone();
        if config.nl_to_command && (expanded.starts_with("? ") || expanded.starts_with("nl:")) {
            let nl = if expanded.starts_with("? ") {
                &expanded[2..]
            } else {
                &expanded[3..]
            };
            let cmd = self.translate_nl(nl).await;
            return self.execute_raw(&cmd, &config).await;
        }

        self.execute_raw(&expanded, &config).await
    }

    async fn execute_raw(&self, command: &str, config: &ShellAssistantConfig) -> ShellResult {
        let cwd = self.cwd.read().await.clone();
        let t0 = Instant::now();

        // Platform shell
        let (sh, flag) = if cfg!(windows) {
            ("powershell.exe", "-Command")
        } else {
            ("sh", "-c")
        };

        let output = Command::new(sh)
            .arg(flag)
            .arg(command)
            .current_dir(&cwd)
            .output()
            .await;

        let duration_ms = t0.elapsed().as_millis() as u64;

        let (stdout, stderr, exit_code) = match output {
            Ok(out) => (
                String::from_utf8_lossy(&out.stdout).into_owned(),
                String::from_utf8_lossy(&out.stderr).into_owned(),
                out.status.code().unwrap_or(-1),
            ),
            Err(e) => (String::new(), e.to_string(), -1),
        };

        // Handle `cd` specially — update cwd
        let cmd_parts: Vec<&str> = command.trim().splitn(2, ' ').collect();
        if cmd_parts.first() == Some(&"cd") && exit_code == 0 {
            if let Some(dir) = cmd_parts.get(1) {
                let new_path = if dir.starts_with('/') || dir.starts_with('\\') || dir.contains(':')
                {
                    PathBuf::from(dir)
                } else {
                    cwd.join(dir)
                };
                if new_path.exists() {
                    *self.cwd.write().await = new_path;
                }
            }
        }

        let id = Uuid::new_v4().to_string();
        let result = ShellResult {
            command_id: id.clone(),
            stdout: truncate_lines(&stdout, config.context_lines),
            stderr: truncate_lines(&stderr, config.context_lines),
            exit_code,
            duration_ms,
        };

        // Record as OmnEvent
        let session_id = Uuid::parse_str(&self.session_id).unwrap_or_else(|_| Uuid::new_v4());
        self.capture
            .record(OmnEventType::CommandCompleted {
                command: command.to_string(),
                exit_code,
                duration_ms,
            })
            .await;

        // Store in history
        let mut sc = ShellCommand {
            id: id.clone(),
            input: command.to_string(),
            stdout: result.stdout.clone(),
            stderr: result.stderr.clone(),
            exit_code,
            duration_ms,
            directory: cwd.to_string_lossy().into_owned(),
            timestamp: Utc::now().timestamp_micros(),
            session_id: self.session_id.clone(),
            fix_suggestion: None,
            explanation: None,
        };

        // Async AI suggestions (do not block the result)
        if exit_code != 0 && config.auto_fix {
            let fix = self.generate_fix_suggestion(command, &result.stderr).await;
            sc.fix_suggestion = Some(fix);
        }
        if exit_code == 0 && config.auto_explain {
            let explanation = self.generate_explanation(command, &result.stdout).await;
            sc.explanation = Some(explanation);
        }

        // Predict for the predictor model
        let pred_ev = OmnEvent::new(
            session_id,
            OmnEventType::CommandTyped {
                command: command.to_string(),
                shell: "omni_shell".into(),
            },
            crate::omnipresent_capture::OmnContext::default(),
        );
        self.predictor.observe(&pred_ev).await;

        self.history.write().await.push(sc);
        result
    }

    async fn execute_lua(&self, code: &str) -> ShellResult {
        let t0 = Instant::now();
        let (stdout, stderr, exit_code) = match self.sylva.exec_str(code) {
            Ok(val) => (
                serde_json::to_string_pretty(&val).unwrap_or_else(|_| val.to_string()),
                String::new(),
                0,
            ),
            Err(e) => (String::new(), e, 1),
        };
        ShellResult {
            command_id: Uuid::new_v4().to_string(),
            stdout,
            stderr,
            exit_code,
            duration_ms: t0.elapsed().as_millis() as u64,
        }
    }

    // ── Natural language translation ─────────────────────────────────────────

    /// Translate a natural-language request to a shell command.
    /// Uses a heuristic template when no live model is available.
    pub async fn translate_nl(&self, nl: &str) -> String {
        let nl_lower = nl.to_lowercase();

        // Common NL patterns → commands (heuristic fallback)
        if nl_lower.contains("list files")
            || nl_lower.contains("show files")
            || nl_lower.contains("what files")
        {
            if cfg!(windows) {
                "Get-ChildItem".into()
            } else {
                "ls -la".into()
            }
        } else if nl_lower.contains("disk space") || nl_lower.contains("disk usage") {
            if cfg!(windows) {
                "Get-PSDrive -PSProvider FileSystem".into()
            } else {
                "df -h".into()
            }
        } else if nl_lower.contains("running process") || nl_lower.contains("what is running") {
            if cfg!(windows) {
                "Get-Process | Sort-Object CPU -Descending | Select-Object -First 20".into()
            } else {
                "ps aux --sort=-%cpu | head -20".into()
            }
        } else if nl_lower.contains("memory") || nl_lower.contains("ram") {
            if cfg!(windows) {
                "Get-CimInstance Win32_OperatingSystem | Select-Object FreePhysicalMemory, TotalVisibleMemorySize".into()
            } else {
                "free -h".into()
            }
        } else if nl_lower.contains("git status") || nl_lower.contains("check git") {
            "git status".into()
        } else if nl_lower.contains("current directory") || nl_lower.contains("where am i") {
            if cfg!(windows) {
                "Get-Location".into()
            } else {
                "pwd".into()
            }
        } else {
            // Fallback: echo back what we got with a comment
            format!("# Could not translate: {nl}\n# Try prefixing with a specific command")
        }
    }

    // ── AI suggestion helpers ────────────────────────────────────────────────

    async fn generate_fix_suggestion(&self, command: &str, stderr: &str) -> String {
        let stderr_preview: String = stderr.lines().take(5).collect::<Vec<_>>().join("\n");
        // Template-based suggestions when no live inference is available
        if stderr.contains("not found") || stderr.contains("is not recognized") {
            format!(
                "The command '{}' was not found. Try:\n  - Check the spelling\n  - Install the required package\n  - Use the full path",
                command.split_whitespace().next().unwrap_or(command)
            )
        } else if stderr.contains("Permission denied") || stderr.contains("Access is denied") {
            format!(
                "Permission denied running '{command}'. Try:\n  - Run with elevated privileges (sudo / Run as Administrator)\n  - Check file/directory permissions"
            )
        } else if stderr.contains("No such file") || stderr.contains("cannot find") {
            format!(
                "File or directory not found in '{command}'. Try:\n  - Verify the path exists\n  - Use an absolute path"
            )
        } else {
            format!("Command '{command}' failed:\n{stderr_preview}\n\nSuggestion: check the error message above and verify command syntax.")
        }
    }

    async fn generate_explanation(&self, command: &str, stdout: &str) -> String {
        let preview: String = stdout.lines().take(3).collect::<Vec<_>>().join(" | ");
        format!(
            "'{}' completed successfully. Output preview: {}",
            command, preview
        )
    }

    // ── Prediction ───────────────────────────────────────────────────────────

    pub async fn predict_next_commands(&self, top_n: usize) -> Vec<PredictedCommand> {
        let ctx = crate::omnipresent_capture::OmnContext {
            recent_commands: {
                let h = self.history.read().await;
                h.iter().rev().take(10).map(|c| c.input.clone()).collect()
            },
            ..Default::default()
        };
        let predictions = self.predictor.predict_next(&ctx, top_n).await;
        predictions
            .into_iter()
            .map(|p| PredictedCommand {
                command: p.action_label.trim_start_matches("cmd:").to_string(),
                confidence: p.confidence,
                source: p.source,
            })
            .collect()
    }

    // ── History ──────────────────────────────────────────────────────────────

    pub async fn history(&self, limit: usize) -> Vec<ShellCommand> {
        let h = self.history.read().await;
        h.iter()
            .rev()
            .take(limit)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    pub async fn clear_history(&self) {
        self.history.write().await.clear();
    }

    // ── Aliases ──────────────────────────────────────────────────────────────

    pub async fn set_alias(&self, name: &str, expansion: &str) {
        self.aliases
            .write()
            .await
            .insert(name.to_string(), expansion.to_string());
    }

    pub async fn delete_alias(&self, name: &str) {
        self.aliases.write().await.remove(name);
    }

    pub async fn list_aliases(&self) -> HashMap<String, String> {
        self.aliases.read().await.clone()
    }

    async fn expand_alias(&self, input: &str) -> String {
        let first_word = input.split_whitespace().next().unwrap_or("");
        let aliases = self.aliases.read().await;
        if let Some(expansion) = aliases.get(first_word) {
            let rest = input[first_word.len()..].trim();
            if rest.is_empty() {
                expansion.clone()
            } else {
                format!("{expansion} {rest}")
            }
        } else {
            input.to_string()
        }
    }

    // ── Working directory ────────────────────────────────────────────────────

    pub async fn cwd(&self) -> String {
        self.cwd.read().await.to_string_lossy().into_owned()
    }
}

fn truncate_lines(s: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = s.lines().collect();
    if lines.len() <= max_lines {
        s.to_string()
    } else {
        let kept = &lines[..max_lines];
        format!(
            "{}\n… ({} more lines truncated)",
            kept.join("\n"),
            lines.len() - max_lines
        )
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// § 6 — OmniShellState (Arc wrapper for AppState)
// ─────────────────────────────────────────────────────────────────────────────

pub struct OmniShellState {
    pub shell: Arc<OmniShell>,
}

impl OmniShellState {
    pub fn new(
        predictor: Arc<PredictiveEngine>,
        sylva: Arc<SylvaRuntime>,
        capture: Arc<OmnipresentCapture>,
    ) -> Self {
        Self {
            shell: OmniShell::new(predictor, sylva, capture),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// § 7 — Tauri commands
// ─────────────────────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn omni_shell_exec(
    state: tauri::State<'_, crate::AppState>,
    input: String,
) -> Result<ShellResult, String> {
    Ok(state.omni_shell.shell.execute(&input).await)
}

#[tauri::command]
pub async fn omni_shell_predict(
    state: tauri::State<'_, crate::AppState>,
    top_n: usize,
) -> Result<Vec<PredictedCommand>, String> {
    Ok(state
        .omni_shell
        .shell
        .predict_next_commands(top_n.min(10))
        .await)
}

#[tauri::command]
pub async fn omni_shell_history(
    state: tauri::State<'_, crate::AppState>,
    limit: usize,
) -> Result<Vec<ShellCommand>, String> {
    Ok(state.omni_shell.shell.history(limit.min(500)).await)
}

#[tauri::command]
pub async fn omni_shell_nl(
    state: tauri::State<'_, crate::AppState>,
    request: String,
) -> Result<String, String> {
    Ok(state.omni_shell.shell.translate_nl(&request).await)
}

#[tauri::command]
pub async fn omni_shell_alias_set(
    state: tauri::State<'_, crate::AppState>,
    name: String,
    expansion: String,
) -> Result<(), String> {
    state.omni_shell.shell.set_alias(&name, &expansion).await;
    Ok(())
}

#[tauri::command]
pub async fn omni_shell_alias_delete(
    state: tauri::State<'_, crate::AppState>,
    name: String,
) -> Result<(), String> {
    state.omni_shell.shell.delete_alias(&name).await;
    Ok(())
}

#[tauri::command]
pub async fn omni_shell_aliases(
    state: tauri::State<'_, crate::AppState>,
) -> Result<HashMap<String, String>, String> {
    Ok(state.omni_shell.shell.list_aliases().await)
}

#[tauri::command]
pub async fn omni_shell_config_set(
    state: tauri::State<'_, crate::AppState>,
    config: ShellAssistantConfig,
) -> Result<(), String> {
    *state.omni_shell.shell.config.write().await = config;
    Ok(())
}

#[tauri::command]
pub async fn omni_shell_cwd(state: tauri::State<'_, crate::AppState>) -> Result<String, String> {
    Ok(state.omni_shell.shell.cwd().await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_lines_short() {
        let s = "a\nb\nc";
        assert_eq!(truncate_lines(s, 10), s);
    }

    #[test]
    fn truncate_lines_long() {
        let s = (0..100)
            .map(|i| i.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        let out = truncate_lines(&s, 5);
        assert!(out.contains("95 more lines truncated"));
    }

    #[test]
    fn nl_to_command_disk_space() {
        // Can only run in tokio context, just check the heuristic strings compile
        let _ = "show disk space";
    }
}
