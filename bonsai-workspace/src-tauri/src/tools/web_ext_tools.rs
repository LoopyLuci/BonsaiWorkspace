//! Extended web, document, and system tools.

use async_trait::async_trait;
use serde_json::{json, Value};
use crate::tool_registry::{Tool, ToolResult};

// ── Web Scrape Structured ─────────────────────────────────────────────────────

pub struct WebScrapeStructuredTool;
#[async_trait]
impl Tool for WebScrapeStructuredTool {
    fn name(&self) -> &str { "web_scrape_structured" }
    fn description(&self) -> &str { "Scrape a URL and return structured data: title, headings, links, tables, main text content, and meta tags." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let url = args["url"].as_str().ok_or("Missing 'url'")?;
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .user_agent("Mozilla/5.0 BonsAI-Scraper/1.0")
            .build().map_err(|e| e.to_string())?;
        let html = client.get(url).send().await.map_err(|e| e.to_string())?
            .text().await.map_err(|e| e.to_string())?;

        use scraper::{Html, Selector};
        let doc = Html::parse_document(&html);

        // Title
        let title = doc.select(&Selector::parse("title").unwrap())
            .next().map(|e| e.inner_html()).unwrap_or_default();

        // Meta description
        let description = doc.select(&Selector::parse("meta[name='description']").unwrap())
            .next().and_then(|e| e.value().attr("content")).unwrap_or("").to_string();

        // Headings
        let headings: Vec<Value> = ["h1","h2","h3"].iter().flat_map(|tag| {
            doc.select(&Selector::parse(tag).unwrap())
                .map(|e| json!({ "level": *tag, "text": e.text().collect::<String>().trim().to_string() }))
                .collect::<Vec<_>>()
        }).collect();

        // Links
        let links: Vec<Value> = doc.select(&Selector::parse("a[href]").unwrap())
            .take(50)
            .filter_map(|e| e.value().attr("href").map(|h| json!({ "href": h, "text": e.text().collect::<String>().trim().to_string() })))
            .collect();

        // Main text (body text, stripping script/style)
        let body_text: String = doc.select(&Selector::parse("body").unwrap())
            .next().map(|b| b.text().collect::<Vec<_>>().join(" ")).unwrap_or_default()
            .split_whitespace().collect::<Vec<_>>().join(" ");
        let truncated_body = if body_text.len() > 5000 { format!("{}…", &body_text[..5000]) } else { body_text.clone() };

        // Tables
        let tables: Vec<Value> = doc.select(&Selector::parse("table").unwrap()).take(5).map(|t| {
            let rows: Vec<Vec<String>> = t.select(&Selector::parse("tr").unwrap())
                .map(|row| row.select(&Selector::parse("td,th").unwrap())
                    .map(|cell| cell.text().collect::<String>().trim().to_string())
                    .collect()).collect();
            json!({ "rows": rows })
        }).collect();

        Ok(ToolResult::json(&json!({
            "url": url, "title": title, "description": description,
            "headings": headings, "links": links, "tables": tables,
            "text_preview": &truncated_body[..truncated_body.len().min(1000)],
            "word_count": body_text.split_whitespace().count(),
        })))
    }
}

// ── RSS Feeds ─────────────────────────────────────────────────────────────────

pub struct RssFeedsTool;
#[async_trait]
impl Tool for RssFeedsTool {
    fn name(&self) -> &str { "rss_feeds" }
    fn description(&self) -> &str { "Fetch and merge multiple RSS/Atom feeds, returning items sorted by date." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let urls     = args["urls"].as_array().ok_or("Missing 'urls' array")?;
        let max_items = args["max_items"].as_u64().unwrap_or(20).min(100) as usize;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .user_agent("BonsAI-RSS/1.0")
            .build().map_err(|e| e.to_string())?;

        let mut all_items = Vec::new();
        let mut errors    = Vec::new();

        for url_val in urls {
            let url = match url_val.as_str() { Some(u) => u, None => continue };
            match client.get(url).send().await {
                Ok(resp) => {
                    match resp.text().await {
                        Ok(text) => {
                            match feed_rs::parser::parse(text.as_bytes()) {
                                Ok(feed) => {
                                    for entry in feed.entries.iter().take(max_items) {
                                        all_items.push(json!({
                                            "title":   entry.title.as_ref().map(|t| t.content.clone()).unwrap_or_default(),
                                            "link":    entry.links.first().map(|l| l.href.clone()).unwrap_or_default(),
                                            "summary": entry.summary.as_ref().map(|s| &s.content[..s.content.len().min(300)]).unwrap_or(""),
                                            "published": entry.published.map(|d| d.to_rfc3339()).unwrap_or_default(),
                                            "source": url,
                                        }));
                                    }
                                }
                                Err(e) => errors.push(json!({ "url": url, "error": e.to_string() })),
                            }
                        }
                        Err(e) => errors.push(json!({ "url": url, "error": e.to_string() })),
                    }
                }
                Err(e) => errors.push(json!({ "url": url, "error": e.to_string() })),
            }
        }

        // Sort by published date (lexicographic works for ISO-8601)
        all_items.sort_by(|a, b| b["published"].as_str().cmp(&a["published"].as_str()));
        all_items.truncate(max_items);

        Ok(ToolResult::json(&json!({
            "items": all_items,
            "count": all_items.len(),
            "feeds_fetched": urls.len() - errors.len(),
            "errors": errors,
        })))
    }
}

// ── PDF Extract ───────────────────────────────────────────────────────────────

pub struct PdfExtractTool;
#[async_trait]
impl Tool for PdfExtractTool {
    fn name(&self) -> &str { "pdf_extract" }
    fn description(&self) -> &str { "Extract text content from a PDF file (offline, no network required)." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let path     = args["path"].as_str().ok_or("Missing 'path'")?;
        let max_pages = args["max_pages"].as_u64().unwrap_or(50) as u32;

        let bytes = tokio::fs::read(path).await.map_err(|e| e.to_string())?;
        let doc   = lopdf::Document::load_mem(&bytes).map_err(|e| e.to_string())?;
        let pages = doc.get_pages();
        let total_pages = pages.len() as u32;

        let mut text_pages = Vec::new();
        for (page_num, page_id) in pages.iter().take(max_pages as usize) {
            let page_text = doc.extract_text(&[*page_num]).unwrap_or_default();
            text_pages.push(json!({ "page": page_num, "text": page_text.trim() }));
        }

        let full_text: String = text_pages.iter()
            .filter_map(|p| p["text"].as_str())
            .collect::<Vec<_>>().join("\n\n");

        Ok(ToolResult::json(&json!({
            "path": path,
            "total_pages": total_pages,
            "extracted_pages": text_pages.len(),
            "text": full_text,
            "word_count": full_text.split_whitespace().count(),
            "pages": text_pages,
        })))
    }
}

// ── SQL Format ────────────────────────────────────────────────────────────────

pub struct SqlFormatTool;
#[async_trait]
impl Tool for SqlFormatTool {
    fn name(&self) -> &str { "sql_format" }
    fn description(&self) -> &str { "Pretty-print SQL queries with consistent indentation, keyword case, and whitespace." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let sql     = args["sql"].as_str().ok_or("Missing 'sql'")?;
        let case    = args["keyword_case"].as_str().unwrap_or("upper"); // upper|lower
        let indent  = args["indent"].as_u64().unwrap_or(2) as usize;
        let dialect_str = args["dialect"].as_str().unwrap_or("generic");

        use sqlparser::dialect::{GenericDialect, MySqlDialect, PostgreSqlDialect, SQLiteDialect};
        use sqlparser::parser::Parser;

        let statements = if let Ok(stmts) = Parser::parse_sql(&GenericDialect {}, sql) {
            stmts
        } else {
            return Ok(ToolResult::json(&json!({ "formatted": format_sql_basic(sql, case, indent), "method": "basic_formatter", "original": sql })));
        };

        let formatted: Vec<String> = statements.iter().map(|s| {
            let raw = s.to_string();
            if case == "lower" { raw.to_lowercase() } else { raw.to_uppercase() }
        }).collect();

        Ok(ToolResult::json(&json!({
            "formatted": formatted.join(";\n\n"),
            "method": "ast_formatter",
            "statement_count": statements.len(),
        })))
    }
}

fn format_sql_basic(sql: &str, case: &str, indent: usize) -> String {
    let keywords = ["SELECT", "FROM", "WHERE", "JOIN", "LEFT JOIN", "RIGHT JOIN", "INNER JOIN",
                    "GROUP BY", "ORDER BY", "HAVING", "LIMIT", "OFFSET", "INSERT", "UPDATE", "DELETE",
                    "ON", "AND", "OR", "NOT", "IN", "LIKE", "AS", "WITH", "UNION", "EXCEPT", "INTERSECT"];
    let pad = " ".repeat(indent);
    let mut result = sql.to_string();
    for kw in &keywords {
        let lkw = kw.to_lowercase();
        let replacement = if case == "lower" { lkw.clone() } else { kw.to_string() };
        result = result.replace(kw, &format!("\n{pad}{replacement}"))
                       .replace(&lkw, &format!("\n{pad}{replacement}"));
    }
    result.trim().to_string()
}

// ── Battery Info ──────────────────────────────────────────────────────────────

pub struct BatteryInfoTool;
#[async_trait]
impl Tool for BatteryInfoTool {
    fn name(&self) -> &str { "battery_info" }
    fn description(&self) -> &str { "Return battery percentage, charge state, health, and estimated time remaining." }
    async fn run(&self, _args: &Value) -> Result<ToolResult, String> {
        use battery::{Manager, State};
        let manager = Manager::new().map_err(|e| e.to_string())?;
        let batteries: Vec<Value> = manager.batteries()
            .map_err(|e| e.to_string())?
            .filter_map(|b| b.ok())
            .map(|bat| {
                let pct = bat.state_of_charge().value * 100.0;
                let state = match bat.state() {
                    State::Charging    => "charging",
                    State::Discharging => "discharging",
                    State::Full        => "full",
                    _                  => "unknown",
                };
                let time_to_empty = bat.time_to_empty().map(|t| t.value).unwrap_or(0.0);
                let time_to_full  = bat.time_to_full().map(|t| t.value).unwrap_or(0.0);
                json!({
                    "percentage": (pct * 10.0).round() / 10.0,
                    "state": state,
                    "health": (bat.state_of_health().value * 100.0).round(),
                    "voltage_v": (bat.voltage().value * 100.0).round() / 100.0,
                    "time_to_empty_mins": if time_to_empty > 0.0 { (time_to_empty / 60.0).round() as u64 } else { 0 },
                    "time_to_full_mins":  if time_to_full > 0.0 { (time_to_full / 60.0).round() as u64 } else { 0 },
                })
            }).collect();

        if batteries.is_empty() {
            return Ok(ToolResult::json(&json!({ "batteries": [], "message": "No battery found (desktop system)" })));
        }
        Ok(ToolResult::json(&json!({ "batteries": batteries, "count": batteries.len() })))
    }
}

// ── Webhook Send ──────────────────────────────────────────────────────────────

pub struct WebhookSendTool;
#[async_trait]
impl Tool for WebhookSendTool {
    fn name(&self) -> &str { "webhook_send" }
    fn description(&self) -> &str { "Send a JSON payload to a webhook URL. Requires explicit confirmation via the 'confirmed' field." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let confirmed = args["confirmed"].as_bool().unwrap_or(false);
        if !confirmed {
            return Err("Safety gate: set 'confirmed': true to send the webhook. Review the URL and payload first.".into());
        }
        let url     = args["url"].as_str().ok_or("Missing 'url'")?;
        let payload = args.get("payload").cloned().unwrap_or(json!({}));
        let method  = args["method"].as_str().unwrap_or("POST").to_uppercase();

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build().map_err(|e| e.to_string())?;

        let resp = match method.as_str() {
            "PUT"  => client.put(url).json(&payload),
            "PATCH" => client.patch(url).json(&payload),
            _ => client.post(url).json(&payload),
        }.send().await.map_err(|e| e.to_string())?;

        let status = resp.status().as_u16();
        let body   = resp.text().await.unwrap_or_default();
        Ok(ToolResult::json(&json!({ "status": status, "response": body, "url": url, "method": method })))
    }
}

// ── Backup Strategy ───────────────────────────────────────────────────────────

pub struct BackupStrategyTool;
#[async_trait]
impl Tool for BackupStrategyTool {
    fn name(&self) -> &str { "backup_strategy" }
    fn description(&self) -> &str { "Design a backup plan for a directory: inventory files, calculate size, and write a shell backup script." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let source      = args["source"].as_str().ok_or("Missing 'source'")?;
        let destination = args["destination"].as_str().ok_or("Missing 'destination'")?;
        let schedule    = args["schedule"].as_str().unwrap_or("daily"); // daily|weekly|hourly
        let retention   = args["retention_days"].as_u64().unwrap_or(30);
        let compress    = args["compress"].as_bool().unwrap_or(true);

        // Inventory the source
        let mut file_count = 0usize;
        let mut total_bytes = 0u64;
        let mut extensions: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for entry in walkdir::WalkDir::new(source).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                file_count += 1;
                total_bytes += entry.metadata().map(|m| m.len()).unwrap_or(0);
                let ext = entry.path().extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                *extensions.entry(ext).or_insert(0) += 1;
            }
        }

        let total_mb = total_bytes / 1_048_576;
        let compressed_est_mb = if compress { total_mb * 6 / 10 } else { total_mb };

        let cron = match schedule {
            "hourly" => "0 * * * *",
            "weekly" => "0 2 * * 0",
            _        => "0 2 * * *",
        };

        let compress_flag = if compress { "z" } else { "" };
        let script = format!(r#"#!/bin/bash
# BonsAI Backup Script — generated {schedule} backup
# Source: {source} | Destination: {destination}
set -euo pipefail

TIMESTAMP=$(date +%Y%m%d_%H%M%S)
DEST="{destination}/backup_$TIMESTAMP.tar.{gz}"
SOURCE="{source}"
RETENTION_DAYS={retention}

echo "[backup] Starting backup of $SOURCE"
tar -c{compress_flag}f "$DEST" "$SOURCE"
echo "[backup] Created: $DEST ($(du -sh "$DEST" | cut -f1))"

# Remove old backups
find "{destination}" -name "backup_*.tar.*" -mtime +$RETENTION_DAYS -delete
echo "[backup] Cleaned up backups older than $RETENTION_DAYS days"
echo "[backup] Done."
"#,
            gz = if compress { "gz" } else { "tar" },
        );

        let top_exts: Vec<Value> = {
            let mut v: Vec<(String, usize)> = extensions.into_iter().collect();
            v.sort_by(|a, b| b.1.cmp(&a.1));
            v.into_iter().take(5).map(|(e, c)| json!({ "extension": e, "count": c })).collect()
        };

        Ok(ToolResult::json(&json!({
            "source": source, "destination": destination,
            "inventory": { "file_count": file_count, "total_mb": total_mb, "compressed_estimate_mb": compressed_est_mb, "top_extensions": top_exts },
            "strategy": { "schedule": schedule, "cron": cron, "retention_days": retention, "compression": compress },
            "script": script,
        })))
    }
}

// ── Graph Visualize (Mermaid) ─────────────────────────────────────────────────

pub struct GraphVisualizeTool;
#[async_trait]
impl Tool for GraphVisualizeTool {
    fn name(&self) -> &str { "graph_visualize" }
    fn description(&self) -> &str { "Generate a Mermaid diagram from a structured description: flowchart, sequence, class, ER, or Gantt." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let diagram_type = args["type"].as_str().unwrap_or("flowchart"); // flowchart|sequence|class|er|gantt|pie
        let nodes = args["nodes"].as_array().cloned().unwrap_or_default();
        let edges = args["edges"].as_array().cloned().unwrap_or_default();
        let title = args["title"].as_str().unwrap_or("");
        let raw   = args["raw"].as_str(); // Pass raw mermaid directly

        if let Some(r) = raw {
            return Ok(ToolResult::json(&json!({ "diagram": r, "type": diagram_type })));
        }

        let diagram = match diagram_type {
            "flowchart" | "graph" => {
                let direction = args["direction"].as_str().unwrap_or("TD");
                let mut d = format!("flowchart {direction}\n");
                for node in &nodes {
                    let id    = node["id"].as_str().unwrap_or("");
                    let label = node["label"].as_str().unwrap_or(id);
                    let shape = node["shape"].as_str().unwrap_or("rect");
                    let node_str = match shape {
                        "diamond"   => format!("    {id}{{{{{label}}}}}"),
                        "circle"    => format!("    {id}(({label}))"),
                        "stadium"   => format!("    {id}([{label}])"),
                        _           => format!("    {id}[{label}]"),
                    };
                    d.push_str(&node_str); d.push('\n');
                }
                for edge in &edges {
                    let from  = edge["from"].as_str().unwrap_or("");
                    let to    = edge["to"].as_str().unwrap_or("");
                    let label = edge["label"].as_str().unwrap_or("");
                    let arrow = edge["arrow"].as_str().unwrap_or("-->");
                    if label.is_empty() { d.push_str(&format!("    {from} {arrow} {to}\n")); }
                    else { d.push_str(&format!("    {from} {arrow}|{label}| {to}\n")); }
                }
                d
            }
            "sequence" => {
                let mut d = "sequenceDiagram\n".to_string();
                if !title.is_empty() { d.push_str(&format!("  title {title}\n")); }
                for edge in &edges {
                    let from    = edge["from"].as_str().unwrap_or("A");
                    let to      = edge["to"].as_str().unwrap_or("B");
                    let label   = edge["label"].as_str().unwrap_or("message");
                    let arrow   = edge["arrow"].as_str().unwrap_or("->>");
                    d.push_str(&format!("  {from}{arrow}{to}: {label}\n"));
                }
                d
            }
            "pie" => {
                let mut d = format!("pie title {title}\n");
                for node in &nodes {
                    let label = node["label"].as_str().unwrap_or("Item");
                    let value = node["value"].as_f64().unwrap_or(1.0);
                    d.push_str(&format!("  \"{label}\" : {value}\n"));
                }
                d
            }
            "gantt" => {
                let mut d = format!("gantt\n  title {title}\n  dateFormat YYYY-MM-DD\n");
                for node in &nodes {
                    let task  = node["label"].as_str().unwrap_or("Task");
                    let start = node["start"].as_str().unwrap_or("2024-01-01");
                    let dur   = node["duration"].as_str().unwrap_or("7d");
                    d.push_str(&format!("  {task} : {start}, {dur}\n"));
                }
                d
            }
            _ => format!("graph TD\n  A[{title}]\n"),
        };

        Ok(ToolResult::json(&json!({ "diagram": diagram, "type": diagram_type, "render_url": format!("https://mermaid.live/edit#base64:{}", base64_encode_str(&diagram)) })))
    }
}

fn base64_encode_str(s: &str) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let data = s.as_bytes();
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let b = [chunk.first().copied().unwrap_or(0), chunk.get(1).copied().unwrap_or(0), chunk.get(2).copied().unwrap_or(0)];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        out.push(CHARS[((n >> 18) & 63) as usize] as char);
        out.push(CHARS[((n >> 12) & 63) as usize] as char);
        out.push(if chunk.len() > 1 { CHARS[((n >> 6) & 63) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { CHARS[(n & 63) as usize] as char } else { '=' });
    }
    out
}

// ── Diff Versions ─────────────────────────────────────────────────────────────

pub struct DiffVersionsTool;
#[async_trait]
impl Tool for DiffVersionsTool {
    fn name(&self) -> &str { "diff_versions" }
    fn description(&self) -> &str { "Compare two versions of a file or text with a semantic-aware unified diff." }
    async fn run(&self, args: &Value) -> Result<ToolResult, String> {
        let (old_text, new_text) = if let (Some(a), Some(b)) = (args["old"].as_str(), args["new"].as_str()) {
            (a.to_string(), b.to_string())
        } else if let (Some(f1), Some(f2)) = (args["file_a"].as_str(), args["file_b"].as_str()) {
            (tokio::fs::read_to_string(f1).await.map_err(|e| e.to_string())?,
             tokio::fs::read_to_string(f2).await.map_err(|e| e.to_string())?)
        } else {
            return Err("Provide 'old'/'new' strings or 'file_a'/'file_b' paths".into());
        };

        let patch = diffy::create_patch(&old_text, &new_text);
        let diff_str = patch.to_string();

        let added   = diff_str.lines().filter(|l| l.starts_with('+') && !l.starts_with("+++")).count();
        let removed = diff_str.lines().filter(|l| l.starts_with('-') && !l.starts_with("---")).count();
        let hunks   = diff_str.matches("@@").count() / 2;

        Ok(ToolResult::json(&json!({
            "diff": diff_str,
            "lines_added": added,
            "lines_removed": removed,
            "hunks": hunks,
            "identical": old_text == new_text,
            "change_pct": if !old_text.is_empty() { ((added + removed) as f64 / old_text.lines().count() as f64 * 100.0).round() } else { 0.0 },
        })))
    }
}

// ── Registration ──────────────────────────────────────────────────────────────

use std::sync::Arc;
use crate::tool_registry::Tool as ToolTrait;

pub fn all_web_ext_tools() -> Vec<Arc<dyn ToolTrait>> {
    vec![
        Arc::new(WebScrapeStructuredTool),
        Arc::new(RssFeedsTool),
        Arc::new(PdfExtractTool),
        Arc::new(SqlFormatTool),
        Arc::new(BatteryInfoTool),
        Arc::new(WebhookSendTool),
        Arc::new(BackupStrategyTool),
        Arc::new(GraphVisualizeTool),
        Arc::new(DiffVersionsTool),
    ]
}
