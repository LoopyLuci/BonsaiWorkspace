/// Persisted crash/error reporting.
///
/// The panic hook in `lib.rs::run()` used to overwrite a single
/// `%TEMP%\workspace_crash.txt` file with just the panic message — no
/// backtrace, no timestamp, no history (each new panic destroyed evidence of
/// the last one), and nothing captured from the frontend at all. That made
/// exactly the kind of multi-layered bug we just spent a session tracking
/// down (GPU crash → stale config → hardcoded default → GGUF parsing bug)
/// far harder to diagnose after the fact, since only the *last* crash's
/// one-line message ever survived a restart.
///
/// This module writes one JSON file per crash/error under
/// `<app_local_data_dir>/crash_reports/`, keeps the most recent
/// `MAX_REPORTS` and prunes older ones, and exposes both backend panics and
/// frontend JS errors/rejections through the same store so there is a single
/// place to look after something goes wrong.
use serde::Serialize;
use std::path::PathBuf;

const MAX_REPORTS: usize = 200;

#[derive(Serialize)]
struct CrashReport {
    timestamp: String,
    kind: String,
    message: String,
    location: Option<String>,
    backtrace: Option<String>,
    details: Option<serde_json::Value>,
}

fn crash_dir() -> PathBuf {
    let base = dirs::data_local_dir().unwrap_or_else(std::env::temp_dir);
    base.join("com.omnisystem.workspace").join("crash_reports")
}

fn write_report(report: &CrashReport) {
    let dir = crash_dir();
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let fname = format!(
        "{}_{}.json",
        report.timestamp.replace([':', '.'], "-"),
        report.kind
    );
    if let Ok(json) = serde_json::to_vec_pretty(report) {
        let _ = std::fs::write(dir.join(fname), json);
    }
    prune(&dir);
}

fn prune(dir: &std::path::Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut files: Vec<_> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "json").unwrap_or(false))
        .collect();
    if files.len() <= MAX_REPORTS {
        return;
    }
    files.sort_by_key(|e| e.file_name());
    let excess = files.len() - MAX_REPORTS;
    for entry in files.into_iter().take(excess) {
        let _ = std::fs::remove_file(entry.path());
    }
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Called from the `std::panic::set_hook` closure in `lib.rs`.
pub fn record_panic(info: &std::panic::PanicHookInfo) {
    let message = info
        .payload()
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| info.payload().downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic payload".into());
    let location = info.location().map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()));
    let backtrace = std::backtrace::Backtrace::force_capture().to_string();

    let report = CrashReport {
        timestamp: now_iso(),
        kind: "backend_panic".into(),
        message,
        location,
        backtrace: Some(backtrace),
        details: None,
    };
    write_report(&report);
}

/// Called from the `report_frontend_error` Tauri command.
pub fn record_frontend_error(kind: &str, message: String, details: Option<serde_json::Value>) {
    let report = CrashReport {
        timestamp: now_iso(),
        kind: format!("frontend_{kind}"),
        message,
        location: None,
        backtrace: None,
        details,
    };
    write_report(&report);
}

#[tauri::command]
pub fn report_frontend_error(
    kind: String,
    message: String,
    details: Option<serde_json::Value>,
) -> Result<(), String> {
    record_frontend_error(&kind, message, details);
    Ok(())
}

/// The most recent persisted backend panic, if any. Used by
/// `crash_recovery::check_and_recover` to feed the crash message into
/// Survival's KB-based auto-repair after an unclean shutdown is detected.
pub fn most_recent_backend_panic() -> Option<serde_json::Value> {
    let dir = crash_dir();
    let entries = std::fs::read_dir(&dir).ok()?;
    let mut files: Vec<_> = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_str()
                .map(|n| n.contains("_backend_panic.json"))
                .unwrap_or(false)
        })
        .collect();
    files.sort_by_key(|e| std::cmp::Reverse(e.file_name()));
    files
        .into_iter()
        .next()
        .and_then(|entry| std::fs::read_to_string(entry.path()).ok())
        .and_then(|contents| serde_json::from_str(&contents).ok())
}

#[tauri::command]
pub fn list_crash_reports() -> Result<Vec<serde_json::Value>, String> {
    let dir = crash_dir();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Ok(Vec::new());
    };
    let mut files: Vec<_> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "json").unwrap_or(false))
        .collect();
    files.sort_by_key(|e| std::cmp::Reverse(e.file_name()));
    let reports = files
        .into_iter()
        .filter_map(|entry| std::fs::read_to_string(entry.path()).ok())
        .filter_map(|contents| serde_json::from_str(&contents).ok())
        .collect();
    Ok(reports)
}

#[tauri::command]
pub fn clear_crash_reports() -> Result<(), String> {
    let dir = crash_dir();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
    Ok(())
}
