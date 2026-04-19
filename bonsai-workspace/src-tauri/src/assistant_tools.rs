/// Assistant tool implementations — each tool is an independent struct
/// implementing the `Tool` trait from `tool_core`.
///
/// Adding a new tool: create a struct + impl Tool, then register it in
/// `register_all()`. No other file needs to change.
use serde_json::{json, Value};
use crate::tool_core::{
    Tool, ToolContext, ToolError, ToolOutput, ToolPolicyHint, ToolRegistry,
    ToolResult, RetryPolicy, SideEffectProfile,
};

// ── Registration ──────────────────────────────────────────────────────────────

/// Register all built-in assistant tools into a new registry.
pub fn build_registry() -> ToolRegistry {
    let mut r = ToolRegistry::new();
    r.register(GetDatetime);
    r.register(GetSystemStats);
    r.register(GetWeather);
    r.register(FetchUrl);
    r.register(FindFiles);
    r.register(ReadFileAssistant);
    r.register(WriteFileAssistant);
    r.register(OpenUrl);
    r.register(RenderChart);
    r.register(SendEmail);
    r.register(RunCommand);
    r
}

// ─────────────────────────────────────────────────────────────────────────────
// get_datetime
// ─────────────────────────────────────────────────────────────────────────────

pub struct GetDatetime;

#[async_trait::async_trait]
impl Tool for GetDatetime {
    fn name(&self)        -> &'static str { "get_datetime" }
    fn description(&self) -> &'static str { "Returns the current local date and time in ISO-8601 format." }
    fn tags(&self)        -> &'static [&'static str] { &["time", "date", "clock", "now", "current"] }
    fn side_effects(&self) -> SideEffectProfile { SideEffectProfile::None }
    fn policy_hint(&self)  -> ToolPolicyHint    { ToolPolicyHint::safe() }
    fn cache_ttl_secs(&self) -> Option<u64>     { None } // always real-time

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "format": { "type": "string", "description": "Optional strftime-style format hint (informational only)." }
            }
        })
    }

    async fn execute(&self, _args: &Value, _ctx: &ToolContext) -> ToolResult {
        use std::time::{SystemTime, UNIX_EPOCH};
        let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        let dt = format_unix_ts(secs);
        Ok(ToolOutput::Complete(json!({ "datetime": dt, "unix_timestamp": secs })))
    }
}

fn format_unix_ts(secs: u64) -> String {
    let mut r = secs;
    let s = r % 60; r /= 60;
    let m = r % 60; r /= 60;
    let h = r % 24; r /= 24;
    let (y, mo, d) = days_to_ymd(r);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{m:02}:{s:02}Z")
}

fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    let mut year = 1970u64;
    loop {
        let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
        let dy = if leap { 366 } else { 365 };
        if days < dy { break; }
        days -= dy; year += 1;
    }
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let months = [31u64, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 1u64;
    for &dm in &months { if days < dm { break; } days -= dm; month += 1; }
    (year, month, days + 1)
}

// ─────────────────────────────────────────────────────────────────────────────
// get_system_stats
// ─────────────────────────────────────────────────────────────────────────────

pub struct GetSystemStats;

#[async_trait::async_trait]
impl Tool for GetSystemStats {
    fn name(&self)        -> &'static str { "get_system_stats" }
    fn description(&self) -> &'static str { "Returns CPU usage, RAM usage, and disk info for the local machine." }
    fn tags(&self)        -> &'static [&'static str] { &["system", "cpu", "ram", "memory", "disk", "stats", "hardware"] }
    fn side_effects(&self) -> SideEffectProfile { SideEffectProfile::None }
    fn policy_hint(&self)  -> ToolPolicyHint    { ToolPolicyHint::safe() }
    fn cache_ttl_secs(&self) -> Option<u64>     { Some(30) }

    fn schema(&self) -> Value {
        json!({ "type": "object", "properties": {} })
    }

    async fn execute(&self, _args: &Value, _ctx: &ToolContext) -> ToolResult {
        use sysinfo::System;
        let mut sys = System::new_all();
        sys.refresh_all();
        let total = sys.total_memory();
        let used  = sys.used_memory();
        let cpu   = sys.global_cpu_info().cpu_usage();
        Ok(ToolOutput::Complete(json!({
            "cpu_usage_pct":    (cpu * 10.0).round() / 10.0,
            "memory_total_mb":  total / 1024 / 1024,
            "memory_used_mb":   used  / 1024 / 1024,
            "memory_used_pct":  if total > 0 { (used as f64 / total as f64 * 1000.0).round() / 10.0 } else { 0.0 },
        })))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// get_weather
// ─────────────────────────────────────────────────────────────────────────────

pub struct GetWeather;

#[async_trait::async_trait]
impl Tool for GetWeather {
    fn name(&self)        -> &'static str { "get_weather" }
    fn description(&self) -> &'static str { "Returns current weather conditions for a location. No API key required." }
    fn tags(&self)        -> &'static [&'static str] { &["weather", "temperature", "forecast", "rain", "wind", "humidity", "climate"] }
    fn side_effects(&self) -> SideEffectProfile { SideEffectProfile::None }
    fn policy_hint(&self)  -> ToolPolicyHint    { ToolPolicyHint::network() }
    fn cache_ttl_secs(&self) -> Option<u64>     { Some(600) } // 10 min
    fn retry_policy(&self) -> RetryPolicy       { RetryPolicy::network() }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "location": { "type": "string", "description": "City name or 'lat,lon' coordinates. Omit for auto-detect." }
            }
        })
    }

    async fn execute(&self, args: &Value, _ctx: &ToolContext) -> ToolResult {
        let location = args.get("location").and_then(|v| v.as_str()).unwrap_or("auto");
        let (lat, lon, city) = if location == "auto" || location.is_empty() {
            (47.3769f64, 8.5417f64, "Zürich (default)".to_string())
        } else if let Some((la, lo)) = try_parse_latlon(location) {
            (la, lo, location.to_string())
        } else {
            geocode_location(location).await
                .ok_or_else(|| ToolError::NotFound { resource: format!("location: {location}") })?
        };

        let url = format!(
            "https://api.open-meteo.com/v1/forecast?latitude={lat}&longitude={lon}\
             &current=temperature_2m,weathercode,windspeed_10m,relative_humidity_2m\
             &temperature_unit=celsius&wind_speed_unit=kmh&forecast_days=1"
        );
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| ToolError::Internal { message: e.to_string() })?;

        let resp: Value = client.get(&url).send().await
            .map_err(|e| ToolError::Transient { message: format!("weather fetch: {e}"), retry_after_ms: None })?
            .json().await
            .map_err(|e| ToolError::Transient { message: format!("weather parse: {e}"), retry_after_ms: None })?;

        let cur = resp.get("current")
            .ok_or_else(|| ToolError::Internal { message: "no current field".into() })?;

        Ok(ToolOutput::Complete(json!({
            "location":      city,
            "temperature_c": cur.get("temperature_2m").and_then(|v| v.as_f64()).unwrap_or(0.0),
            "condition":     wmo_code_to_desc(cur.get("weathercode").and_then(|v| v.as_i64()).unwrap_or(0)),
            "wind_kmh":      cur.get("windspeed_10m").and_then(|v| v.as_f64()).unwrap_or(0.0),
            "humidity_pct":  cur.get("relative_humidity_2m").and_then(|v| v.as_f64()).unwrap_or(0.0),
        })))
    }
}

fn try_parse_latlon(s: &str) -> Option<(f64, f64)> {
    let p: Vec<&str> = s.splitn(2, ',').collect();
    if p.len() == 2 {
        if let (Ok(la), Ok(lo)) = (p[0].trim().parse::<f64>(), p[1].trim().parse::<f64>()) {
            return Some((la, lo));
        }
    }
    None
}

async fn geocode_location(location: &str) -> Option<(f64, f64, String)> {
    let url = format!(
        "https://geocoding-api.open-meteo.com/v1/search?name={}&count=1&language=en&format=json",
        urlenc(location)
    );
    let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(8)).build().ok()?;
    let resp: Value = client.get(&url).send().await.ok()?.json().await.ok()?;
    let first = resp.get("results")?.as_array()?.first()?;
    Some((
        first.get("latitude")?.as_f64()?,
        first.get("longitude")?.as_f64()?,
        first.get("name").and_then(|v| v.as_str()).unwrap_or(location).to_string(),
    ))
}

fn urlenc(s: &str) -> String {
    s.chars().map(|c| {
        if c.is_alphanumeric() || matches!(c, '-' | '_' | '.') { c.to_string() }
        else { format!("%{:02X}", c as u32) }
    }).collect()
}

fn wmo_code_to_desc(code: i64) -> &'static str {
    match code {
        0 => "Clear sky", 1..=3 => "Partly cloudy", 45 | 48 => "Foggy",
        51..=57 => "Drizzle", 61..=67 => "Rain", 71..=77 => "Snow",
        80..=82 => "Rain showers", 85 | 86 => "Snow showers",
        95 => "Thunderstorm", 96 | 99 => "Thunderstorm with hail", _ => "Unknown",
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// fetch_url
// ─────────────────────────────────────────────────────────────────────────────

pub struct FetchUrl;

#[async_trait::async_trait]
impl Tool for FetchUrl {
    fn name(&self)        -> &'static str { "fetch_url" }
    fn description(&self) -> &'static str { "Fetches a URL and returns its text content (HTML stripped by default)." }
    fn tags(&self)        -> &'static [&'static str] { &["web", "url", "http", "fetch", "scrape", "browse", "internet", "page"] }
    fn side_effects(&self) -> SideEffectProfile { SideEffectProfile::None }
    fn policy_hint(&self)  -> ToolPolicyHint    { ToolPolicyHint::network() }
    fn cache_ttl_secs(&self) -> Option<u64>     { Some(300) }
    fn retry_policy(&self) -> RetryPolicy       { RetryPolicy::network() }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["url"],
            "properties": {
                "url":        { "type": "string", "description": "Full URL including scheme." },
                "strip_html": { "type": "boolean", "description": "Strip HTML tags (default true)." },
                "max_bytes":  { "type": "integer", "description": "Max response size in bytes (default 8192, max 65536)." }
            }
        })
    }

    async fn execute(&self, args: &Value, _ctx: &ToolContext) -> ToolResult {
        let url = args.get("url").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ValidationFailed { field: "url".into(), reason: "required".into() })?;
        let strip = args.get("strip_html").and_then(|v| v.as_bool()).unwrap_or(true);
        let max_bytes = args.get("max_bytes").and_then(|v| v.as_u64())
            .unwrap_or(8192).min(65536) as usize;

        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(ToolError::ValidationFailed {
                field: "url".into(), reason: "must start with http:// or https://".into()
            });
        }

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("BonsaiAssistant/1.0")
            .build()
            .map_err(|e| ToolError::Internal { message: e.to_string() })?;

        let resp = client.get(url).send().await
            .map_err(|e| ToolError::Transient { message: format!("fetch: {e}"), retry_after_ms: None })?;
        let status = resp.status().as_u16();
        let text = resp.text().await
            .map_err(|e| ToolError::Transient { message: format!("read body: {e}"), retry_after_ms: None })?;

        let capped = if text.len() > max_bytes { &text[..max_bytes] } else { &text };
        let content = if strip { strip_html_tags(capped) } else { capped.to_string() };

        Ok(ToolOutput::Complete(json!({
            "url": url, "status": status,
            "text": content,
            "truncated": text.len() > max_bytes,
        })))
    }
}

fn strip_html_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        match ch { '<' => in_tag = true, '>' => in_tag = false, _ if !in_tag => out.push(ch), _ => {} }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

// ─────────────────────────────────────────────────────────────────────────────
// find_files
// ─────────────────────────────────────────────────────────────────────────────

pub struct FindFiles;

#[async_trait::async_trait]
impl Tool for FindFiles {
    fn name(&self)        -> &'static str { "find_files" }
    fn description(&self) -> &'static str { "Searches for files matching a glob pattern under a directory." }
    fn tags(&self)        -> &'static [&'static str] { &["files", "search", "find", "glob", "filesystem", "directory", "list"] }
    fn side_effects(&self) -> SideEffectProfile { SideEffectProfile::Read }
    fn policy_hint(&self)  -> ToolPolicyHint    { ToolPolicyHint::filesystem_read() }
    fn cache_ttl_secs(&self) -> Option<u64>     { Some(60) }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["path", "pattern"],
            "properties": {
                "path":        { "type": "string", "description": "Root directory to search." },
                "pattern":     { "type": "string", "description": "Glob pattern, e.g. '**/*.rs' or '*.json'." },
                "max_results": { "type": "integer", "description": "Max files to return (default 50, max 500)." }
            }
        })
    }

    async fn execute(&self, args: &Value, _ctx: &ToolContext) -> ToolResult {
        let root    = args.get("path").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ValidationFailed { field: "path".into(), reason: "required".into() })?;
        let pattern = args.get("pattern").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ValidationFailed { field: "pattern".into(), reason: "required".into() })?;
        let max     = args.get("max_results").and_then(|v| v.as_u64()).unwrap_or(50).min(500) as usize;

        if root.contains("..") {
            return Err(ToolError::Permission { message: "path traversal not allowed".into() });
        }

        let mut results = Vec::new();
        find_recursive(std::path::Path::new(root), pattern, &mut results, max);
        let count = results.len();
        Ok(ToolOutput::Complete(json!({ "files": results, "count": count })))
    }
}

fn find_recursive(dir: &std::path::Path, pattern: &str, results: &mut Vec<String>, max: usize) {
    if results.len() >= max { return; }
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        if results.len() >= max { return; }
        let path = entry.path();
        if path.is_dir() {
            find_recursive(&path, pattern, results, max);
        } else if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if glob_match(pattern, name) || glob_match_full(pattern, &path.display().to_string()) {
                results.push(path.display().to_string());
            }
        }
    }
}

fn glob_match(pattern: &str, name: &str) -> bool {
    if pattern == "*" || pattern == "**" { return true; }
    if let Some(ext) = pattern.strip_prefix("*.") { return name.ends_with(&format!(".{ext}")); }
    if let Some(pre) = pattern.strip_suffix("*")  { return name.starts_with(pre); }
    pattern == name
}

fn glob_match_full(pattern: &str, path: &str) -> bool {
    // Handle **/*.ext patterns
    if let Some(rest) = pattern.strip_prefix("**/") {
        return glob_match(rest, std::path::Path::new(path)
            .file_name().and_then(|n| n.to_str()).unwrap_or(""));
    }
    glob_match(pattern, path)
}

// ─────────────────────────────────────────────────────────────────────────────
// read_file_assistant
// ─────────────────────────────────────────────────────────────────────────────

pub struct ReadFileAssistant;

#[async_trait::async_trait]
impl Tool for ReadFileAssistant {
    fn name(&self)        -> &'static str { "read_file_assistant" }
    fn description(&self) -> &'static str { "Reads a text file from disk and returns its contents (up to 64KB)." }
    fn tags(&self)        -> &'static [&'static str] { &["file", "read", "open", "content", "text", "source"] }
    fn side_effects(&self) -> SideEffectProfile { SideEffectProfile::Read }
    fn policy_hint(&self)  -> ToolPolicyHint    { ToolPolicyHint::filesystem_read() }
    fn cache_ttl_secs(&self) -> Option<u64>     { Some(30) }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["path"],
            "properties": {
                "path":       { "type": "string" },
                "max_bytes":  { "type": "integer", "description": "Max bytes to return (default 65536)." }
            }
        })
    }

    async fn execute(&self, args: &Value, _ctx: &ToolContext) -> ToolResult {
        let path = args.get("path").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ValidationFailed { field: "path".into(), reason: "required".into() })?;
        let max  = args.get("max_bytes").and_then(|v| v.as_u64()).unwrap_or(65536).min(524288) as usize;

        if path.contains("..") {
            return Err(ToolError::Permission { message: "path traversal not allowed".into() });
        }
        let content = std::fs::read_to_string(path)
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound       => ToolError::NotFound { resource: path.into() },
                std::io::ErrorKind::PermissionDenied => ToolError::Permission { message: format!("cannot read {path}") },
                _ => ToolError::Internal { message: e.to_string() },
            })?;
        let truncated = content.len() > max;
        let content   = if truncated { &content[..max] } else { &content };
        Ok(ToolOutput::Complete(json!({ "path": path, "content": content, "truncated": truncated })))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// write_file_assistant
// ─────────────────────────────────────────────────────────────────────────────

pub struct WriteFileAssistant;

#[async_trait::async_trait]
impl Tool for WriteFileAssistant {
    fn name(&self)        -> &'static str { "write_file_assistant" }
    fn description(&self) -> &'static str { "Writes text content to a file on disk. Requires user confirmation." }
    fn tags(&self)        -> &'static [&'static str] { &["file", "write", "save", "create", "edit", "modify"] }
    fn side_effects(&self) -> SideEffectProfile { SideEffectProfile::Write }
    fn policy_hint(&self)  -> ToolPolicyHint    { ToolPolicyHint::filesystem_write() }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["path", "content"],
            "properties": {
                "path":    { "type": "string" },
                "content": { "type": "string" }
            }
        })
    }

    async fn execute(&self, args: &Value, _ctx: &ToolContext) -> ToolResult {
        let path    = args.get("path").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ValidationFailed { field: "path".into(), reason: "required".into() })?;
        let content = args.get("content").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ValidationFailed { field: "content".into(), reason: "required".into() })?;
        if path.contains("..") {
            return Err(ToolError::Permission { message: "path traversal not allowed".into() });
        }
        // Create parent dirs if needed
        if let Some(parent) = std::path::Path::new(path).parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ToolError::Permission { message: e.to_string() })?;
        }
        std::fs::write(path, content)
            .map_err(|e| ToolError::Permission { message: format!("write {path}: {e}") })?;
        Ok(ToolOutput::Complete(json!({ "path": path, "bytes_written": content.len() })))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// open_url
// ─────────────────────────────────────────────────────────────────────────────

pub struct OpenUrl;

#[async_trait::async_trait]
impl Tool for OpenUrl {
    fn name(&self)        -> &'static str { "open_url" }
    fn description(&self) -> &'static str { "Opens a URL in the system default browser." }
    fn tags(&self)        -> &'static [&'static str] { &["browser", "open", "url", "web", "link", "navigate"] }
    fn side_effects(&self) -> SideEffectProfile { SideEffectProfile::External }
    fn policy_hint(&self)  -> ToolPolicyHint    { ToolPolicyHint::network() }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["url"],
            "properties": {
                "url": { "type": "string" }
            }
        })
    }

    async fn execute(&self, args: &Value, _ctx: &ToolContext) -> ToolResult {
        let url = args.get("url").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ValidationFailed { field: "url".into(), reason: "required".into() })?;
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(ToolError::ValidationFailed { field: "url".into(), reason: "must start with http:// or https://".into() });
        }
        #[cfg(target_os = "windows")]
        let _ = std::process::Command::new("cmd").args(["/c", "start", url]).spawn();
        #[cfg(target_os = "macos")]
        let _ = std::process::Command::new("open").arg(url).spawn();
        #[cfg(target_os = "linux")]
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
        #[cfg(target_os = "android")]
        let _ = (url,); // no-op on Android
        Ok(ToolOutput::Complete(json!({ "url": url, "opened": true })))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// render_chart
// ─────────────────────────────────────────────────────────────────────────────

pub struct RenderChart;

#[async_trait::async_trait]
impl Tool for RenderChart {
    fn name(&self)        -> &'static str { "render_chart" }
    fn description(&self) -> &'static str { "Renders a bar, line, or pie chart as inline SVG from JSON data." }
    fn tags(&self)        -> &'static [&'static str] { &["chart", "graph", "plot", "visualize", "bar", "line", "pie", "data"] }
    fn side_effects(&self) -> SideEffectProfile { SideEffectProfile::None }
    fn policy_hint(&self)  -> ToolPolicyHint    { ToolPolicyHint::safe() }
    fn cache_ttl_secs(&self) -> Option<u64>     { None } // keyed on args, session-scoped cache is enough

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["chart_type", "data_json"],
            "properties": {
                "chart_type": { "type": "string", "enum": ["bar", "line", "pie"], "description": "Chart type." },
                "data_json":  { "type": "string", "description": "JSON array of {label, value} objects." },
                "title":      { "type": "string" }
            }
        })
    }

    async fn execute(&self, args: &Value, _ctx: &ToolContext) -> ToolResult {
        let chart_type = args.get("chart_type").and_then(|v| v.as_str()).unwrap_or("bar");
        let data_str   = args.get("data_json").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ValidationFailed { field: "data_json".into(), reason: "required".into() })?;
        let title      = args.get("title").and_then(|v| v.as_str()).unwrap_or("");

        let data: Vec<Value> = serde_json::from_str(data_str)
            .map_err(|e| ToolError::ValidationFailed { field: "data_json".into(), reason: e.to_string() })?;

        let svg = match chart_type {
            "bar"  => render_bar(&data, title),
            "line" => render_line(&data, title),
            "pie"  => render_pie(&data, title),
            other  => return Err(ToolError::ValidationFailed {
                field: "chart_type".into(), reason: format!("unknown type '{other}'")
            }),
        };
        Ok(ToolOutput::Complete(json!({ "svg": svg, "chart_type": chart_type })))
    }
}

fn render_bar(data: &[Value], title: &str) -> String {
    let (w, h, pad) = (400usize, 260usize, 40usize);
    let cw = w - pad * 2; let ch = h - pad * 2 - 20;
    let max: f64 = data.iter().filter_map(|d| d.get("value")?.as_f64()).fold(0.0f64, f64::max).max(1.0);
    let n = data.len().max(1);
    let bw = (cw / n).saturating_sub(4).max(1);
    let mut bars = String::new(); let mut labels = String::new();
    for (i, item) in data.iter().enumerate() {
        let v  = item.get("value").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let lb = item.get("label").and_then(|v| v.as_str()).unwrap_or("");
        let bh = ((v / max) * ch as f64) as usize;
        let x  = pad + i * cw / n;
        let y  = pad + ch - bh;
        bars.push_str(&format!("<rect x=\"{x}\" y=\"{y}\" width=\"{bw}\" height=\"{bh}\" fill=\"#5ca4ea\" rx=\"2\"/>"));
        labels.push_str(&format!("<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-size=\"10\" fill=\"#888\">{lb}</text>", x + bw / 2, h - 8));
    }
    format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\" style=\"background:#1e1e1e;border-radius:8px\"><text x=\"{}\" y=\"20\" text-anchor=\"middle\" font-size=\"13\" fill=\"#ccc\">{title}</text>{bars}{labels}</svg>", w/2)
}

fn render_line(data: &[Value], title: &str) -> String {
    let (w, h, pad) = (400usize, 260usize, 40usize);
    let cw = w - pad * 2; let ch = h - pad * 2 - 20;
    let max: f64 = data.iter().filter_map(|d| d.get("value")?.as_f64()).fold(0.0f64, f64::max).max(1.0);
    let n = data.len().max(1);
    let pts: Vec<String> = data.iter().enumerate().map(|(i, item)| {
        let v = item.get("value").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let x = pad + i * cw / (n - 1).max(1);
        let y = pad + ch - ((v / max) * ch as f64) as usize;
        format!("{x},{y}")
    }).collect();
    format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\" style=\"background:#1e1e1e;border-radius:8px\"><text x=\"{}\" y=\"20\" text-anchor=\"middle\" font-size=\"13\" fill=\"#ccc\">{title}</text><polyline points=\"{}\" fill=\"none\" stroke=\"#5ca4ea\" stroke-width=\"2\"/></svg>", w/2, pts.join(" "))
}

fn render_pie(data: &[Value], title: &str) -> String {
    let (w, h) = (300usize, 300usize);
    let (cx, cy, r) = (w / 2, h / 2 + 10, 100usize);
    let total: f64 = data.iter().filter_map(|d| d.get("value")?.as_f64()).sum::<f64>().max(1.0);
    let colors = ["#5ca4ea","#e05260","#50c878","#f5a623","#b68bf7","#40c4e0"];
    let mut slices = String::new();
    let mut angle = -std::f64::consts::FRAC_PI_2;
    for (i, item) in data.iter().enumerate() {
        let v = item.get("value").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let sweep = v / total * 2.0 * std::f64::consts::PI;
        let x1 = cx as f64 + r as f64 * angle.cos();
        let y1 = cy as f64 + r as f64 * angle.sin();
        angle += sweep;
        let x2 = cx as f64 + r as f64 * angle.cos();
        let y2 = cy as f64 + r as f64 * angle.sin();
        let large = if sweep > std::f64::consts::PI { 1 } else { 0 };
        slices.push_str(&format!("<path d=\"M{cx},{cy} L{x1:.1},{y1:.1} A{r},{r} 0 {large} 1 {x2:.1},{y2:.1} Z\" fill=\"{}\"/>", colors[i % colors.len()]));
    }
    format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"0 0 {w} {h}\" style=\"background:#1e1e1e;border-radius:8px\"><text x=\"{cx}\" y=\"18\" text-anchor=\"middle\" font-size=\"13\" fill=\"#ccc\">{title}</text>{slices}</svg>")
}

// ─────────────────────────────────────────────────────────────────────────────
// send_email
// ─────────────────────────────────────────────────────────────────────────────

pub struct SendEmail;

#[async_trait::async_trait]
impl Tool for SendEmail {
    fn name(&self)        -> &'static str { "send_email" }
    fn description(&self) -> &'static str { "Sends an email via configured SMTP. Requires user confirmation." }
    fn tags(&self)        -> &'static [&'static str] { &["email", "mail", "send", "smtp", "message", "compose"] }
    fn side_effects(&self) -> SideEffectProfile { SideEffectProfile::External }
    fn policy_hint(&self)  -> ToolPolicyHint    { ToolPolicyHint::external() }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["to", "subject", "body"],
            "properties": {
                "to":      { "type": "string", "description": "Recipient email address." },
                "subject": { "type": "string" },
                "body":    { "type": "string" },
                "cc":      { "type": "string", "description": "Optional CC address." }
            }
        })
    }

    async fn execute(&self, args: &Value, ctx: &ToolContext) -> ToolResult {
        use crate::secrets_store::{ACCOUNT_SMTP_FROM, ACCOUNT_SMTP_HOST, ACCOUNT_SMTP_PASSWORD, ACCOUNT_SMTP_USERNAME};
        use lettre::{message::header::ContentType, transport::smtp::authentication::Credentials, Message, SmtpTransport, Transport};

        let to      = args.get("to").and_then(|v| v.as_str()).ok_or_else(|| ToolError::ValidationFailed { field: "to".into(), reason: "required".into() })?;
        let subject = args.get("subject").and_then(|v| v.as_str()).ok_or_else(|| ToolError::ValidationFailed { field: "subject".into(), reason: "required".into() })?;
        let body    = args.get("body").and_then(|v| v.as_str()).ok_or_else(|| ToolError::ValidationFailed { field: "body".into(), reason: "required".into() })?;

        let mk_cfg_err = |field: &str| ToolError::Configuration {
            message: format!("SMTP {field} not configured"),
            fix_hint: "Go to Bonsai Buddy → Settings → Email to configure SMTP".into(),
        };

        let host     = ctx.secrets.get(ACCOUNT_SMTP_HOST).map_err(|e| ToolError::Internal { message: e })?.ok_or_else(|| mk_cfg_err("host"))?;
        let username = ctx.secrets.get(ACCOUNT_SMTP_USERNAME).map_err(|e| ToolError::Internal { message: e })?.ok_or_else(|| mk_cfg_err("username"))?;
        let password = ctx.secrets.get(ACCOUNT_SMTP_PASSWORD).map_err(|e| ToolError::Internal { message: e })?.ok_or_else(|| mk_cfg_err("password"))?;
        let from     = ctx.secrets.get(ACCOUNT_SMTP_FROM).map_err(|e| ToolError::Internal { message: e })?.ok_or_else(|| mk_cfg_err("from address"))?;

        let mut builder = Message::builder()
            .from(from.parse().map_err(|e| ToolError::ValidationFailed { field: "from".into(), reason: format!("{e}") })?)
            .to(to.parse().map_err(|e| ToolError::ValidationFailed { field: "to".into(), reason: format!("{e}") })?)
            .subject(subject)
            .header(ContentType::TEXT_PLAIN);

        if let Some(cc) = args.get("cc").and_then(|v| v.as_str()) {
            builder = builder.cc(cc.parse().map_err(|e| ToolError::ValidationFailed { field: "cc".into(), reason: format!("{e}") })?);
        }

        let email = builder.body(body.to_string())
            .map_err(|e| ToolError::Internal { message: format!("build email: {e}") })?;

        let mailer = SmtpTransport::relay(&host)
            .map_err(|e| ToolError::Configuration { message: format!("SMTP relay: {e}"), fix_hint: "Check SMTP host in Settings → Email".into() })?
            .credentials(Credentials::new(username, password))
            .build();

        mailer.send(&email)
            .map_err(|e| ToolError::Transient { message: format!("send: {e}"), retry_after_ms: None })?;

        Ok(ToolOutput::Complete(json!({ "sent": true, "to": to, "subject": subject })))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// run_command
// ─────────────────────────────────────────────────────────────────────────────

pub struct RunCommand;

#[async_trait::async_trait]
impl Tool for RunCommand {
    fn name(&self)        -> &'static str { "run_command" }
    fn description(&self) -> &'static str { "Runs a shell command on the local machine and streams its output. Requires user confirmation." }
    fn tags(&self)        -> &'static [&'static str] { &["shell", "command", "terminal", "run", "execute", "script", "bash", "powershell", "cli"] }
    fn side_effects(&self) -> SideEffectProfile { SideEffectProfile::External }
    fn policy_hint(&self)  -> ToolPolicyHint    { ToolPolicyHint::external() }
    fn cache_ttl_secs(&self) -> Option<u64>     { None }

    fn schema(&self) -> Value {
        json!({
            "type": "object",
            "required": ["command"],
            "properties": {
                "command":      { "type": "string", "description": "The shell command to run." },
                "working_dir":  { "type": "string", "description": "Working directory (defaults to workspace root)." },
                "timeout_secs": { "type": "integer", "description": "Max execution time in seconds (default 30, max 120)." }
            }
        })
    }

    async fn execute(&self, args: &Value, ctx: &ToolContext) -> ToolResult {
        let command  = args.get("command").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::ValidationFailed { field: "command".into(), reason: "required".into() })?;
        let work_dir = args.get("working_dir").and_then(|v| v.as_str())
            .or(ctx.workspace_path.as_deref())
            .unwrap_or(".");
        let timeout  = args.get("timeout_secs").and_then(|v| v.as_u64()).unwrap_or(30).min(120);

        let (prog, arg1) = if cfg!(target_os = "windows") {
            ("cmd", "/C")
        } else {
            ("sh", "-c")
        };

        let output_fut = tokio::process::Command::new(prog)
            .args([arg1, command])
            .current_dir(work_dir)
            .output();

        let output = tokio::time::timeout(
            std::time::Duration::from_secs(timeout),
            output_fut,
        ).await
        .map_err(|_| ToolError::Timeout { duration_ms: timeout * 1000 })?
        .map_err(|e| ToolError::Internal { message: format!("spawn: {e}") })?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let code   = output.status.code().unwrap_or(-1);

        Ok(ToolOutput::Complete(json!({
            "exit_code": code,
            "stdout":    &stdout[..stdout.len().min(8192)],
            "stderr":    &stderr[..stderr.len().min(2048)],
            "timed_out": false,
        })))
    }
}
